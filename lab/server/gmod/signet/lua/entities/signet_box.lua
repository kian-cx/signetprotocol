-- One box of the shared world's ground, drawn with the GMod surface Signet Forge chose for its material.
-- Server: a frozen physics box players can stand on (unless it is a liquid); the physics gun can carry it.
-- Client: a cube mesh with the surface texture, repeated every 128 units, faces shaded a little by direction.
AddCSLuaFile()
ENT.Type = "anim"
ENT.Base = "base_anim"
ENT.RenderGroup = RENDERGROUP_BOTH
ENT.Spawnable = false

function ENT:SetupDataTables()
	self:NetworkVar("Vector", 0, "Half")       -- half size (the entity sits at the box centre)
	self:NetworkVar("String", 0, "Surface")    -- GMod material path, or "-"
	self:NetworkVar("Bool", 0, "Hard")         -- solid
end

function ENT:Initialize()
	local h = self:GetHalf()
	if SERVER then
		if self:GetHard() then
			self:PhysicsInitBox(-h, h)
			self:SetSolid(SOLID_VPHYSICS)
			self:SetMoveType(MOVETYPE_VPHYSICS)       -- frozen, but the physics gun can carry it (an edit of the ground)
			local phys = self:GetPhysicsObject()
			if IsValid(phys) then phys:EnableMotion(false) end
			self:EnableCustomCollisions(true)
		else
			self:SetSolid(SOLID_NONE)
			self:SetMoveType(MOVETYPE_NONE)
		end
	end
	self:SetCollisionBounds(-h, h)
	self:DrawShadow(false)
end

if SERVER then return end

local made = {}
-- brush surfaces are made for the map's lightmaps; a model-like copy (unlit, vertex colours for the shading) draws on meshes
local function surface(path)
	if made[path] then return made[path] end
	local base, tex = path ~= "-" and Material(path) or nil, nil
	if base and not base:IsError() then
		local t = base:GetTexture("$basetexture")
		-- a surface whose texture file is missing draws the purple checkers: flat colour instead
		if t and not t:IsError() and not t:IsErrorTexture() and t:GetName() ~= "error" then tex = t:GetName() end
	end
	local water = path:find("water") ~= nil
	-- foliage and fences are cut out by their alpha: keep that (else the holes draw black)
	local flags = base and not base:IsError() and base:GetInt("$flags") or 0
	local cutout = tex and (bit.band(flags, 256) ~= 0 or bit.band(flags, 2097152) ~= 0 or (base:GetInt("$alphatest") or 0) ~= 0
		or (base:GetInt("$translucent") or 0) ~= 0)
	local m = CreateMaterial("signet_surface_" .. path:gsub("[^%w]", "_"), "UnlitGeneric", {
		["$basetexture"] = tex or "color/white",
		["$vertexcolor"] = 1,
		["$translucent"] = water and 1 or 0,
		["$alpha"] = water and 0.75 or 1,
		["$alphatest"] = (cutout and not water) and 1 or 0,
		["$alphatestreference"] = 0.5,
	})
	made[path] = { mat = m, tint = tex and Color(255, 255, 255) or (water and Color(60, 110, 190) or Color(150, 150, 150)) }
	return made[path]
end

local FACES = {   -- normal, the two axes along the face, shade
	{ Vector(0, 0, 1), Vector(1, 0, 0), Vector(0, 1, 0), 1.0 },
	{ Vector(0, 0, -1), Vector(0, 1, 0), Vector(1, 0, 0), 0.55 },
	{ Vector(1, 0, 0), Vector(0, 1, 0), Vector(0, 0, 1), 0.8 },
	{ Vector(-1, 0, 0), Vector(0, 0, 1), Vector(0, 1, 0), 0.7 },
	{ Vector(0, 1, 0), Vector(0, 0, 1), Vector(1, 0, 0), 0.75 },
	{ Vector(0, -1, 0), Vector(1, 0, 0), Vector(0, 0, 1), 0.65 },
}

function ENT:BuildMesh()
	local h = self:GetHalf()
	if h == vector_origin then return end
	local s = surface(self:GetSurface())
	local verts = {}
	for _, f in ipairs(FACES) do
		local n, u, v, shade = f[1], f[2], f[3], f[4]
		local c = Vector(n.x * h.x, n.y * h.y, n.z * h.z)
		local hu = math.abs(u.x * h.x + u.y * h.y + u.z * h.z)
		local hv = math.abs(v.x * h.x + v.y * h.y + v.z * h.z)
		local corners = { { -1, -1 }, { 1, -1 }, { 1, 1 }, { -1, 1 } }
		local quad = {}
		for _, k in ipairs(corners) do
			local p = c + u * (k[1] * hu) + v * (k[2] * hv)
			local w = self:GetPos() + p
			quad[#quad + 1] = { pos = p, normal = n, u = (w:Dot(u)) / 128, v = (w:Dot(v)) / 128,
				color = Color(s.tint.r * shade, s.tint.g * shade, s.tint.b * shade, s.tint.a) }
		end
		for _, i in ipairs({ 1, 2, 3, 1, 3, 4 }) do verts[#verts + 1] = quad[i] end
	end
	self.signetMesh = Mesh()
	self.signetMesh:BuildFromTriangles(verts)
	self.signetMat = s.mat
	self:SetRenderBounds(-h, h)
end

function ENT:Draw()
	if self.signetBuilt ~= self:GetSurface() then   -- new surface from Signet Forge: rebuild with it
		if self.signetMesh then self.signetMesh:Destroy(); self.signetMesh = nil end
		self.signetBuilt = self:GetSurface()
		self:BuildMesh()
	end
	if not self.signetMesh then return end
	render.SetMaterial(self.signetMat)
	local m = Matrix(); m:SetTranslation(self:GetPos())
	render.CullMode(MATERIAL_CULLMODE_NONE)      -- both sides: no face lost to the winding order
	cam.PushModelMatrix(m); self.signetMesh:Draw(); cam.PopModelMatrix()
	render.CullMode(MATERIAL_CULLMODE_CCW)
end
ENT.DrawTranslucent = ENT.Draw

function ENT:OnRemove()
	if self.signetMesh then self.signetMesh:Destroy() end
end
