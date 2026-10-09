"""SF starts after the Link, then restarts: defaults first, SF's answers when it comes, approvals survive a restart."""
import os, socket, subprocess, threading, time
S = os.environ.get("SIGNET_TEST_DIR", "/tmp/signet-tests"); os.makedirs(S, exist_ok=True); sys_path = os.path.dirname(os.path.abspath(__file__)); B = os.path.expanduser("~/signet-server/target/release/signet")
env = dict(os.environ, HF_HUB_OFFLINE="1", SIGNET_FORGE_DECISIONS=f"{S}/test_decisions.json")   # keeps the approvals of e2e.py
procs = []
def run(name, cmd, cwd=None, stdin=None):
    p = subprocess.Popen(cmd, cwd=cwd, env=env, stdout=open(f"{S}/late_{name}.log", "w"), stderr=subprocess.STDOUT, stdin=stdin, text=True); procs.append(p); return p
def wait_for(path, text, t=120):
    end = time.time() + t
    while time.time() < end:
        if text in open(path).read(): return True
        time.sleep(0.3)
ok = lambda c, m: print(("PASS " if c else "FAIL ") + m, flush=True)
got = []
srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); srv.bind(("127.0.0.1", 17790)); srv.listen(1)
def bridge():
    c, _ = srv.accept()
    threading.Thread(target=lambda: [got.append(l.rstrip()) for l in c.makefile()], daemon=True).start()
    while True:
        try: c.sendall(b"MC 0.5 64 0.5 0 0\n"); time.sleep(0.2)
        except OSError: return
threading.Thread(target=bridge, daemon=True).start()
run("link", [B, "link", "--name", "late", "--no-lc", "--mc-bridge", "127.0.0.1:17790", "--forge", "127.0.0.1:17796",
             "--world", os.path.expanduser("~/signet-server/worlds/plaza.txt")])
time.sleep(3)
import sys; sys.path.insert(0, sys_path); from mcstate import block_at
lawn = lambda: [block_at(got, 40, 63, 40)]   # a lawn block, as Minecraft shows it now
ok(lawn() and lawn()[-1].endswith("minecraft:stone"), "without SF: the ground is painted with the stone placeholder")
sf = run("sf1", [os.path.expanduser("~/clm-bench/clef/.venv-unsloth/bin/python"), "signet_forge.py", "--headless", "--port", "17796"], cwd=os.path.expanduser("~/clm-bench"), stdin=subprocess.PIPE)
wait_for(f"{S}/late_sf1.log", "listening"); time.sleep(6)
ok(lawn()[-1].endswith("minecraft:moss_block"), f"SF started later: the Link asks again and uses your approved choice ({lawn()[-1]})")
sf.terminate(); sf.wait(); time.sleep(3)
ok("Signet Forge closed" in open(f"{S}/late_link.log").read(), "SF stopped: the Link notices and keeps the last answers")
sf2 = run("sf2", [os.path.expanduser("~/clm-bench/clef/.venv-unsloth/bin/python"), "signet_forge.py", "--headless", "--port", "17796"], cwd=os.path.expanduser("~/clm-bench"), stdin=subprocess.PIPE)
wait_for(f"{S}/late_sf2.log", "listening"); time.sleep(6)
ok(open(f"{S}/late_link.log").read().count("connected to Signet Forge") >= 2 and lawn()[-1].endswith("minecraft:moss_block"),
   "SF restarted: the Link reconnects, and the approval is still there")
for p in procs: p.terminate()
