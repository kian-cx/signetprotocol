-- Signet door, server side. Runs only on our own LAN dedicated server (sv_lan 1, -allowlocalhttp), never on a public
-- one and never in the player's game. Ten times a second it tells the Signet Link (doors/gmod.rs) where the Garry's Mod
-- players are, and the Link answers with the players of the other games, already in Source units, to be drawn here.
if not SERVER then return end
AddCSLuaFile("autorun/client/signet_cl.lua")

local URL = "http://127.0.0.1:7795/tick"
local WORLD_URL = "http://127.0.0.1:7795/world"
local busy, chat, drawn, spawnpos, warned = false, {}, {}, nil, false
local world_ver, world_busy, boxes = nil, false, {}   -- boxes: "geo solid" -> signet_box
local carried, moves = {}, {}                        -- ground boxes in the physics gun; their moves for the Link

-- the shared world's ground: one signet_box per box. Only what changed is touched: a box whose place is the same
-- keeps its entity (its surface is repainted if Signet Forge chose another), boxes gone are removed, new ones made.
local function build_world(body)
	local want, order = {}, {}
	for line in string.gmatch(body, "[^\n]+") do
		local x1, y1, z1, x2, y2, z2, solid, tex = string.match(line, "^B (%S+) (%S+) (%S+) (%S+) (%S+) (%S+) (%d) (%S+)$")
		if x1 then
			local geo = table.concat({ x1, y1, z1, x2, y2, z2 }, " ")
			want[geo .. " " .. solid] = { geo = geo, mn = Vector(tonumber(x1), tonumber(y1), tonumber(z1)),
				mx = Vector(tonumber(x2), tonumber(y2), tonumber(z2)), solid = solid == "1", tex = tex }
			order[#order + 1] = geo .. " " .. solid
		end
	end
	local first, made, removed = next(boxes) == nil, 0, 0
	for key, e in pairs(boxes) do
		if not want[key] or not IsValid(e) then
			if IsValid(e) and not carried[e] then e:Remove() end   -- (one still in the physics gun stays until dropped)
			boxes[key] = nil; removed = removed + 1
		end
	end
	for _, key in ipairs(order) do
		local w, e = want[key], boxes[key]
		if IsValid(e) then
			if e:GetSurface() ~= w.tex then e:SetSurface(w.tex) end
		else
			e = ents.Create("signet_box")
			e:SetPos((w.mn + w.mx) / 2)
			e:SetHalf((w.mx - w.mn) / 2)
			e:SetSurface(w.tex)
			e:SetHard(w.solid)
			e:Spawn()
			e.signetGeo = w.geo
			boxes[key] = e; made = made + 1
		end
	end
	if made + removed > 0 then print("[signet] the shared world's ground: " .. #order .. " boxes (" .. made .. " new, " .. removed .. " removed)") end
	if first then for _, ply in ipairs(player.GetHumans()) do ply:SetPos(ply:GetPos() + Vector(0, 0, 8)) end end   -- nobody stays inside a box
end

-- a ground box carried with the physics gun: the Link hears where it goes (the other games see it move) and,
-- when it is dropped, makes it an edit of the shared ground; the box is rebuilt in its new place
local function geo_of(e)
	local mn, mx = e:GetPos() - e:GetHalf(), e:GetPos() + e:GetHalf()
	return string.format("%.2f %.2f %.2f %.2f %.2f %.2f", mn.x, mn.y, mn.z, mx.x, mx.y, mx.z)
end
hook.Add("PhysgunPickup", "signet_carry", function(ply, e)
	if IsValid(e) and e:GetClass() == "signet_box" and e:GetHard() then
		carried[e] = { last = e:GetPos(), at = 0 }
		-- free while carried: no gravity, and it passes through the rest of the ground (the leaves on a trunk, the grass)
		e:SetCollisionGroup(COLLISION_GROUP_IN_VEHICLE)   -- collides with nothing while carried
		local phys = e:GetPhysicsObject()
		if IsValid(phys) then
			e.signetMass = e.signetMass or phys:GetMass()
			phys:SetMass(50); phys:EnableMotion(true); phys:EnableGravity(false); phys:Wake()
		end
		print("[signet] " .. ply:Nick() .. " picked up a ground box: " .. tostring(e.signetGeo))
		return true
	end
end)
hook.Add("PhysgunDrop", "signet_drop", function(ply, e)
	if not carried[e] then return end
	carried[e] = nil
	print("[signet] " .. ply:Nick() .. " dropped it at " .. geo_of(e))
	e:SetAngles(Angle(0, 0, 0))                  -- the shared ground is made of upright boxes
	e:SetCollisionGroup(COLLISION_GROUP_NONE)
	local phys = e:GetPhysicsObject()
	if IsValid(phys) then phys:EnableMotion(false); phys:EnableGravity(true); if e.signetMass then phys:SetMass(e.signetMass) end end
	moves[#moves + 1] = "G 1 " .. e.signetGeo .. " " .. geo_of(e)
	timer.Simple(3, function()                   -- normally replaced by the rebuilt box; if the Link did not take it, back home
		if not IsValid(e) then return end
		for _, b in pairs(boxes) do
			if b == e then
				local g = string.Split(e.signetGeo, " ")
				e:SetPos(Vector(tonumber(g[1]) + tonumber(g[4]), tonumber(g[2]) + tonumber(g[5]), tonumber(g[3]) + tonumber(g[6])) / 2)
				return
			end
		end
	end)
end)

local function fetch_world(ver)
	if world_busy then return end
	world_busy = true
	HTTP({ url = WORLD_URL, method = "GET",
		success = function(code, body) world_busy = false; world_ver = ver; build_world(body or "") end,
		failed = function() world_busy = false end })
end

hook.Add("PlayerSpawn", "signet_spawn_on_top", function(ply)   -- the map's spawn is under the ground's boxes: step up
	timer.Simple(0, function() if IsValid(ply) then ply:SetPos(ply:GetPos() + Vector(0, 0, 8)) end end)
end)

local function spawnPoint()
	if not spawnpos then
		local sp = ents.FindByClass("info_player_start")[1]
		spawnpos = sp and sp:GetPos() or Vector(0, 0, 0)
	end
	return spawnpos
end

hook.Add("PlayerSay", "signet_chat", function(ply, text)
	chat[#chat + 1] = "C " .. ply:UserID() .. " " .. (string.gsub(text, "[\r\n]", " "))
end)

local function draw_one(id, x, y, z, yaw, moving, look, label)
	local model = player_manager.TranslatePlayerModel(look ~= "-" and look or "kleiner")
	local e = drawn[id]
	if not IsValid(e) or e.signetModel ~= model then
		if IsValid(e) then e:Remove() end
		e = ents.Create("prop_dynamic")
		e:SetModel(model)
		e:SetKeyValue("solid", "0")
		e:SetKeyValue("DefaultAnim", "idle_all_01")
		e:Spawn()
		e.signetModel, e.signetSeq = model, nil
		drawn[id] = e
		print("[signet] drawing " .. label .. " as " .. model)
	end
	e:SetNWString("signet_label", label)
	e:SetPos(Vector(x, y, z))
	e:SetAngles(Angle(0, yaw, 0))
	local seq = moving and "walk_all" or "idle_all_01"
	if e.signetSeq ~= seq then
		e:Fire("SetDefaultAnimation", seq)
		e:Fire("SetAnimation", seq)
		e.signetSeq = seq
	end
end

local function apply(body)
	local seen = {}
	local ver = string.match(body, "^W (%d+)") or string.match(body, "\nW (%d+)")
	if ver and ver ~= world_ver then fetch_world(ver) end
	for line in string.gmatch(body, "[^\n]+") do
		local id, x, y, z, yaw, moving, look, label = string.match(line, "^E (%d+) (%S+) (%S+) (%S+) (%S+) (%d) (%S+) (.*)$")
		if id then
			seen[id] = true
			draw_one(id, tonumber(x), tonumber(y), tonumber(z), tonumber(yaw), moving == "1", look, label)
		end
	end
	for id, e in pairs(drawn) do
		if not seen[id] then
			if IsValid(e) then e:Remove() end
			drawn[id] = nil
			print("[signet] #" .. id .. " left")
		end
	end
end

timer.Create("signet_tick", 0.033, 0, function()   -- ~30 times a second: movements cross over with little delay
	if busy then return end
	local s = spawnPoint()
	local lines = { string.format("S %.1f %.1f %.1f", s.x, s.y, s.z) }
	for _, ply in ipairs(player.GetHumans()) do
		local p = ply:GetPos()
		local look = player_manager.TranslateToPlayerModelName(ply:GetModel()) or "kleiner"
		lines[#lines + 1] = string.format("P %d %.1f %.1f %.1f %.1f %d %s %s", ply:UserID(), p.x, p.y, p.z,
			ply:EyeAngles().y, ply:Crouching() and 1 or 0, look, ply:Nick())
	end
	for _, c in ipairs(chat) do lines[#lines + 1] = c end
	chat = {}
	for e, c in pairs(carried) do                -- while carried: where it is, ~10 times a second when it moves
		if not IsValid(e) then carried[e] = nil
		elseif CurTime() - c.at > 0.1 and e:GetPos():DistToSqr(c.last) > 4 then
			c.at, c.last = CurTime(), e:GetPos()
			lines[#lines + 1] = "G 0 " .. e.signetGeo .. " " .. geo_of(e)
		end
	end
	for _, m in ipairs(moves) do lines[#lines + 1] = m end
	moves = {}
	busy = true
	HTTP({
		url = URL, method = "POST", body = table.concat(lines, "\n"), type = "text/plain",
		success = function(code, body)
			busy = false
			if warned then print("[signet] the Link answers again"); warned = false end
			apply(body or "")
		end,
		failed = function(err)
			busy = false
			if not warned then
				print("[signet] cannot reach the Signet Link at " .. URL .. " (" .. tostring(err) .. "): is it running with --gmod?")
				warned = true
				local n = 0   -- the Link stopped: players from other games leave instead of staying frozen
				for id, e in pairs(drawn) do if IsValid(e) then e:Remove() end; drawn[id] = nil; n = n + 1 end
				world_ver = nil   -- rebuilt (with the Link's current choices) when it is back
				if n > 0 then print("[signet] removed " .. n .. " player(s) from other games until the Link is back") end
			end
		end,
	})
end)

print("[signet] door loaded: talking to the Signet Link at " .. URL)
