"""End to end, on test ports: SF (headless) <-> Link (MC + GMod doors) <-> Server (plaza world)."""
import os, socket, subprocess, sys, threading, time, urllib.request, json
S = os.environ.get("SIGNET_TEST_DIR", "/tmp/signet-tests"); os.makedirs(S, exist_ok=True); sys_path = os.path.dirname(os.path.abspath(__file__)); B = os.path.expanduser("~/signet-server/target/release/signet")
env = dict(os.environ, HF_HUB_OFFLINE="1", SIGNET_FORGE_DECISIONS=f"{S}/test_decisions.json")
if os.path.exists(f"{S}/test_decisions.json"): os.remove(f"{S}/test_decisions.json")
procs, logs = [], {}
def run(name, cmd, cwd=None, stdin=None):
    f = open(f"{S}/e2e_{name}.log", "w"); p = subprocess.Popen(cmd, cwd=cwd, env=env, stdout=f, stderr=subprocess.STDOUT, stdin=stdin, text=True)
    procs.append(p); return p
def wait_for(path, text, t=120):
    end = time.time() + t
    while time.time() < end:
        if text in open(path).read(): return True
        time.sleep(0.3)
    return False
ok = lambda c, m: print(("PASS " if c else "FAIL ") + m, flush=True)

sf = run("sf", [os.path.expanduser("~/clm-bench/clef/.venv-unsloth/bin/python"), "signet_forge.py", "--headless", "--port", "17796"],
         cwd=os.path.expanduser("~/clm-bench"), stdin=subprocess.PIPE)
assert wait_for(f"{S}/e2e_sf.log", "listening for the Signet Link"), "SF did not start"
run("server", [B, "server", "--bind", "127.0.0.1:17800"])
# fake proto bridge: records what Minecraft would be told; sends a Minecraft player walking
got = []
srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); srv.bind(("127.0.0.1", 17790)); srv.listen(1)
def bridge():
    c, _ = srv.accept()
    threading.Thread(target=lambda: [got.append(l.rstrip()) for l in c.makefile()], daemon=True).start()
    t0 = time.time()
    while True:
        try: c.sendall(f"MC {0.5 + (time.time() - t0) * 0.2:.3f} 64.000 3.500 90.0 1\n".encode()); time.sleep(0.1)
        except OSError: return
threading.Thread(target=bridge, daemon=True).start()
time.sleep(0.5)
run("link", [B, "link", "--server", "127.0.0.1:17800", "--name", "e2e", "--no-lc", "--gmod", "--gmod-addr", "127.0.0.1:17795",
             "--mc-bridge", "127.0.0.1:17790", "--forge", "127.0.0.1:17796"])
time.sleep(1)
# simulated GMod addon: one GMod player standing 2 m east of the spawn
answers = []
def tick():
    body = "S 0 0 0\nP 7 80 0 0 90 0 alyx orux".encode()
    r = urllib.request.urlopen(urllib.request.Request("http://127.0.0.1:17795/tick", data=body, method="POST"), timeout=2)
    answers.append(r.read().decode())
for _ in range(80): tick(); time.sleep(0.1)

worlds = [l for l in got if l.startswith("WORLD")]
boxes = [l for l in got if l.startswith("BOX")]
spawns = [l for l in got if l.startswith("SPAWN")]
ok(len(worlds) >= 1, f"Minecraft got the server's world ({len(worlds)} paints)")
import sys; sys.path.insert(0, sys_path); from mcstate import block_at
ok(block_at(got, 40, 63, 40) == "minecraft:grass_block", f"after SF answered, the lawn is painted with grass_block: {block_at(got, 40, 63, 40)}")
probe = [(40, 63, 40), (40, 60, 40), (40, 58, 40), (0, 63, 0), (5, 63, 5), (-22, 62, 18), (20, 64, 0), (20, 63, 0), (19, 68, 0), (30, 64, 30), (30, 69, 30), (-31, 64, -31), (12, 63, 0)]
stone = [p for p in probe if block_at(got, *p) == "minecraft:stone"]
ok(len(stone) <= 1, f"no material left as stone placeholder (stone at {stone})")
ok(any("minecraft:armor_stand" not in s for s in spawns), "the GMod player is drawn in Minecraft as SF's choice: " + (spawns[-1] if spawns else "-"))
looks = [l.split()[7] for a in answers for l in a.splitlines() if l.startswith("E ")]
ok(looks and looks[-1] != "-", f"the Minecraft player is drawn in Garry's Mod as SF's choice: {looks[-1] if looks else '-'}")

# your choice in SF, pushed live
sf.stdin.write("approve minecraft_26_3|material|green grass lawn => moss_block\n"); sf.stdin.flush()
sf.stdin.write("approve minecraft_26_3|player|garrysmod:alyx => villager\n"); sf.stdin.flush()
n0 = len(got); time.sleep(3)
for _ in range(10): tick(); time.sleep(0.1)
new = got[n0:]
ok(block_at(got, 40, 63, 40) == "minecraft:moss_block", "approving moss_block for the lawn repaints Minecraft's ground at once")
ok(any(l.startswith("SPAWN") and "minecraft:villager" in l for l in new) and any(l.startswith("REMOVE") for l in new),
   "approving villager for GMod players respawns the GMod player in Minecraft as a villager")
dec = json.load(open(f"{S}/test_decisions.json"))
ok(dec.get("minecraft_26_3|material|green grass lawn", {}).get("accepted") == "moss_block", "the approved choice is kept in the decisions file")
# preview: shown in the game, never saved; turning it off goes back to the real choice
n0 = len(got)
sf.stdin.write("preview minecraft_26_3|player|garrysmod:alyx => zombie\n"); sf.stdin.flush(); time.sleep(1.5)
for _ in range(5): tick(); time.sleep(0.1)
ok(any(l.startswith("SPAWN") and "minecraft:zombie" in l for l in got[n0:]), "preview: the GMod player shows as a zombie in Minecraft right away")
n0 = len(got)
sf.stdin.write("preview minecraft_26_3|player|garrysmod:alyx =>\n"); sf.stdin.flush(); time.sleep(1.5)
for _ in range(5): tick(); time.sleep(0.1)
ok(any(l.startswith("SPAWN") and "minecraft:villager" in l for l in got[n0:]), "preview off: back to your approved villager")
ok(json.load(open(f"{S}/test_decisions.json"))["minecraft_26_3|player|garrysmod:alyx"]["accepted"] == "villager", "the preview was never saved")
sf.stdin.write("list\n"); sf.stdin.flush(); time.sleep(1)
for p in procs: p.terminate()
