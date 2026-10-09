//! Ctrl-C (and SIGTERM) without a crash: the first one asks every part to clean up (the Minecraft door removes the
//! players it drew, the Server's Links see the connection close and drop their entities) and exits; a second one
//! exits at once. Standard library only: the C library's signal() is already linked.
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Mutex;

static STOP: AtomicBool = AtomicBool::new(false);
static HOOKS: Mutex<Vec<Box<dyn Fn() + Send>>> = Mutex::new(Vec::new());

extern "C" {
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    fn _exit(code: i32) -> !;
}

extern "C" fn on_signal(_: i32) {
    if STOP.swap(true, SeqCst) { unsafe { _exit(130) } }      // the second Ctrl-C: out now
}

pub fn install() {
    unsafe {
        signal(2, on_signal); // SIGINT, Ctrl-C on Linux, macOS and Windows
        #[cfg(unix)]
        signal(15, on_signal); // SIGTERM
    }
}

pub fn requested() -> bool { STOP.load(SeqCst) }

/// Something to undo before leaving (runs once, on the main thread).
pub fn on_stop(f: impl Fn() + Send + 'static) { HOOKS.lock().unwrap().push(Box::new(f)); }

pub fn run_hooks() { for h in HOOKS.lock().unwrap().iter() { h(); } }
