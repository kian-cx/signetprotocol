"""Edits of the shared ground, end to end on test ports:
SF (headless) <-> Link A (MC + GMod doors) and Link B (MC door, "another PC") <-> Server (plaza world).
- a lava block placed in Minecraft A reaches GMod A (a liquid box) and Minecraft B (a PATCH)
- a block broken in Minecraft A carves GMod's ground
- a tree trunk carried with the physics gun in GMod A: Minecraft A and B see it leave and arrive (PATCH air / trunk)
- a Link that joins later gets the edited world"""
import os, socket, subprocess, threading, time, urllib.request
S = os.environ.get("SIGNET_TEST_DIR", "/tmp/signet-tests"); os.makedirs(S, exist_ok=True); sys_path = os.path.dirname(os.path.abspath(__file__)); B = os.path.expanduser("~/signet-server/target/release/signet")
env = dict(os.environ, HF_HUB_OFFLINE="1", SIGNET_FORGE_DECISIONS=f"{S}/test_decisions_edits.json")
if os.path.exists(f"{S}/test_decisions_edits.json"): os.remove(f"{S}/test_decisions_edits.json")
procs, fails = [], []
INCH = 0.0254
def run(name, cmd, cwd=None, stdin=None):
    f = open(f"{S}/e2e_edits_{name}.log", "w")
    p = subprocess.Popen(cmd, cwd=cwd, env=env, stdout=f, stderr=subprocess.STDOUT, stdin=stdin, text=True); procs.append(p); return p
def wait_for(path, text, t=120):
    end = time.time() + t
    while time.time() < end:
        if text in open(path).read(): return True
        time.sleep(0.3)
    return False
def ok(c, m):
    print(("PASS " if c else "FAIL ") + m, flush=True)
    if not c: fails.append(m)

def fake_bridge(port):
    """A Minecraft proto bridge: records what Minecraft is told; lets the test send MCEV lines."""
    got, conn = [], {}
    srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); srv.bind(("127.0.0.1", port)); srv.listen(1)
    def serve():
        c, _ = srv.accept(); conn["c"] = c
        threading.Thread(target=lambda: [got.append(l.rstrip()) for l in c.makefile()], daemon=True).start()
        while True:
            try: c.sendall(b"MC 0.5 64.0 0.5 0.0 0\n"); time.sleep(0.2)
            except OSError: return
    threading.Thread(target=serve, daemon=True).start()
    return got, lambda line: conn["c"].sendall((line + "\n").encode())

try:
    sf = run("sf", [os.path.expanduser("~/clm-bench/clef/.venv-unsloth/bin/python"), "signet_forge.py", "--headless", "--port", "17796"],
             cwd=os.path.expanduser("~/clm-bench"), stdin=subprocess.PIPE)
    assert wait_for(f"{S}/e2e_edits_sf.log", "listening for the Signet Link"), "SF did not start"
    run("server", [B, "server", "--bind", "127.0.0.1:17800"])
    got_a, send_a = fake_bridge(17790)
    got_b, send_b = fake_bridge(17791)
    time.sleep(0.5)
    link = lambda name, extra: run(name, [B, "link", "--server", "127.0.0.1:17800", "--name", name, "--no-lc", "--forge", "127.0.0.1:17796"] + extra)
    link("a", ["--gmod", "--gmod-addr", "127.0.0.1:17795", "--mc-bridge", "127.0.0.1:17790"])
    link("b", ["--mc-bridge", "127.0.0.1:17791"])
    def tick(extra=""):
        body = ("S 0 0 0\nP 7 80 0 0 90 0 alyx orux" + ("\n" + extra if extra else "")).encode()
        return urllib.request.urlopen(urllib.request.Request("http://127.0.0.1:17795/tick", data=body, method="POST"), timeout=2).read().decode()
    def world():
        return [l.split() for l in urllib.request.urlopen("http://127.0.0.1:17795/world", timeout=2).read().decode().splitlines()]
    def covering(w, p):   # the /world boxes that contain a source point
        return [b for b in w if all(float(b[1 + k]) <= p[k] <= float(b[4 + k]) for k in range(3))]
    for _ in range(60): tick(); time.sleep(0.1)          # SF answers the plaza's materials
    ok(any(l.startswith("WORLD") for l in got_a) and any(l.startswith("WORLD") for l in got_b), "both Minecrafts got the world")

    # 1. lava placed in Minecraft A (block 20 64 20: on the lawn, universal 19.5..20.5, 0..1, 19.5..20.5)
    n_a, n_b = len(got_a), len(got_b)
    send_a("MCEV block 20 64 20 minecraft:lava")
    time.sleep(3)
    for _ in range(20): tick(); time.sleep(0.1)          # SF answers 'lava' for GMod and Minecraft
    src = [20 / INCH, -20 / INCH, 0.5 / INCH + 1]          # its centre in GMod (source units, spawn 0 0 0)
    lava = covering(world(), src)
    ok(len(lava) == 1 and lava[0][7] == "0" and lava[0][8] != "-", f"GMod A: the lava is a liquid box with SF's surface: {lava}")
    pb = [l for l in got_b[n_b:] if l.startswith("PATCH 20 64 20 20 64 20")]
    ok(len(pb) >= 1 and pb[-1].split()[-1] != "minecraft:air", f"Minecraft B (another PC) paints it: {pb[-1:] }")
    ok(not any(l.startswith("PATCH 20 64 20") for l in got_a[n_a:]), "Minecraft A is not told again (it already shows the lava)")

    # 2. a lawn block broken in Minecraft A carves GMod's ground (block 25 63 25: universal 24.5..25.5, -1..0)
    hole = [25 / INCH, -25 / INCH, -0.5 / INCH + 1]
    ok(len(covering(world(), hole)) == 1, "GMod A: the lawn is there before")
    send_a("MCEV block 25 63 25 minecraft:air")
    time.sleep(1.5); tick()
    ok(len(covering(world(), hole)) == 0, "GMod A: breaking the block in Minecraft leaves a hole in GMod's ground")
    ok(any(l == "PATCH 25 63 25 25 63 25 minecraft:air" for l in got_b), "Minecraft B: the block is gone there too")

    # 3. the oak trunk (universal 30..31, 0..4 under its leaves, 30..31) carried 5 m (196.85 units) along +x with the physics gun in GMod A
    trunk = covering(world(), [30.5 / INCH, -30.5 / INCH, 2.5 / INCH + 1])
    ok(len(trunk) == 1, f"GMod A: the oak trunk is one box: {trunk}")
    geo = " ".join(trunk[0][1:7]); g = [float(v) for v in trunk[0][1:7]]
    n_a, n_b = len(got_a), len(got_b)
    for step in (50, 100, 150, 5 / INCH):
        moved = " ".join(f"{v:.2f}" for v in (g[0] + step, g[1], g[2], g[3] + step, g[4], g[5]))
        tick(f"G 0 {geo} {moved}"); time.sleep(0.15)
    tick(f"G 1 {geo} {moved}"); time.sleep(1.5)
    pa, pb = got_a[n_a:], got_b[n_b:]
    ok(any(l.startswith("PATCH 30 64 30 30 67 30 minecraft:air") for l in pa), "Minecraft A: the trunk leaves its place while carried")
    ok(sum(1 for l in pa if l.startswith("PATCH") and "log" in l) >= 3, f"Minecraft A: and is drawn along the way ({sum(1 for l in pa if l.startswith('PATCH') and 'log' in l)} steps)")
    ok(any(l.startswith("PATCH 35 64 30 35 67 30") and "log" in l for l in pa), "Minecraft A: it ends 5 blocks east (x 35)")
    ok(any(l.startswith("PATCH 35 64 30 35 67 30") and "log" in l for l in pb), "Minecraft B (another PC): the same")
    tick(); w = world()
    ok(len(covering(w, [30.5 / INCH, -30.5 / INCH, 2.5 / INCH + 1])) == 0, "GMod A: after the drop the old place is empty")
    ok(len(covering(w, [35.5 / INCH, -30.5 / INCH, 2.5 / INCH + 1])) == 1, "GMod A: and the trunk stands in its new place")

    # 4. a Link that joins later gets the edited world
    got_c, _ = fake_bridge(17792)
    link("c", ["--mc-bridge", "127.0.0.1:17792"])
    time.sleep(4)
    boxes_c = [l for l in got_c if l.startswith("BOX")]
    ok(any(l.startswith("BOX 20 64 20 20 64 20") for l in boxes_c), "a late Link paints the lava")
    ok(any(l.split()[0] in ("BOX", "PATCH") and l.split()[1:7] == ["35", "64", "30", "35", "67", "30"] and "log" in l for l in got_c),
       "and the moved trunk (painted once Signet Forge answers)")
    ok(any(l == "BOX 25 63 25 25 63 25 minecraft:air" for l in boxes_c), "and the hole")
    for l in (f"{S}/e2e_edits_server.log",):
        ok("edit mc:20,64,20 from minecraft: lava" in open(l).read(), "the Server logged the edits")
finally:
    for p in procs: p.terminate()
print("ALL PASS" if not fails else f"{len(fails)} FAILED")
