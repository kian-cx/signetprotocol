-- Signet door, client side (sent by our server like any server addon): a name tag over players from other games.
hook.Add("PostDrawTranslucentRenderables", "signet_labels", function()
	local eye = LocalPlayer():EyeAngles()
	for _, e in ipairs(ents.FindByClass("prop_dynamic")) do
		local t = e:GetNWString("signet_label", "")
		if t ~= "" then
			cam.Start3D2D(e:GetPos() + Vector(0, 0, 82), Angle(0, eye.y - 90, 90), 0.15)
				draw.SimpleTextOutlined(t, "DermaLarge", 0, 0, color_white, TEXT_ALIGN_CENTER, TEXT_ALIGN_CENTER, 1, color_black)
			cam.End3D2D()
		end
	end
end)
