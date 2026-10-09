def block_at(lines, x, y, z):
    """The block Minecraft shows at x y z after these bridge lines (WORLD resets; BOX and PATCH paint in order)."""
    cur = None
    for l in lines:
        f = l.split()
        if not f: continue
        if f[0] == "WORLD": cur = None
        elif f[0] in ("BOX", "PATCH") and len(f) == 8:
            v = [int(a) for a in f[1:7]]
            if v[0] <= x <= v[3] and v[1] <= y <= v[4] and v[2] <= z <= v[5]: cur = f[7]
    return cur
