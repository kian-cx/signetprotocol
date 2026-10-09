#![allow(dead_code)]
//! Signet: one shared world, one door per game. Each game connects with its own, unmodified client; the world keeps
//! universal entities (what, from which game, how it looks, where) and every door draws the others its own way.
//!
//!   signet server [--bind 127.0.0.1:7800] [--world worlds/plaza.txt|none]
//!       The shared world only (no doors, no game data). Links connect to it with the Signet protocol (src/wire.rs).
//!   signet link [--server HOST:7800] [--name NAME] [--no-mc] [--no-lc] [--mc-bridge 127.0.0.1:7790] [--lc-port 7777] [--lc-script FILE] [--gmod [--gmod-addr 127.0.0.1:7795]] [--forge 127.0.0.1:7796 | --no-forge]
//!       Runs on each player's PC, next to their game: the doors (local only, 127.0.0.1) plus the uplink to a Server.
//!       Without --server it is a self-contained local world (both doors on one PC).
//!   signet forge [--port 7796] [--debug]
//!       The decision screen for this PC. Python and the model come from `signet setup`, not from a fixed path.
//!   signet setup --role server|client|both [--download] [--install-python] [--games minecraft,gmod,lethal]
//!   (no mode = link without --server)
//!
//! The tunnel is the user's choice: put the Server where you like and reach it through your own VPN (OpenVPN,
//! WireGuard, Tailscale, ...). Optional shared token: env SIGNET_TOKEN on both sides.
mod banner;
mod doors;
mod forge;
mod net;
mod setup;
mod stop;
mod ngo;
mod utp;
mod wire;
mod world;

use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static START: OnceLock<Instant> = OnceLock::new();

pub fn log(who: &str, msg: &str) {
    let t = START.get_or_init(Instant::now).elapsed().as_secs_f64();
    println!("{t:8.2} [{who:>13}] {msg}");
}

/// Plaza next to the executable, then next to the source tree, then the working directory.
/// A binary copied out of the Docker image finds /worlds/plaza.txt beside /signet.
fn default_world() -> String {
    if let Ok(p) = std::env::var("SIGNET_WORLD") { return p; }
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() { candidates.push(dir.join("worlds/plaza.txt")); }
    }
    candidates.push(std::path::PathBuf::from(format!("{}/worlds/plaza.txt", env!("CARGO_MANIFEST_DIR"))));
    candidates.push(std::path::PathBuf::from("worlds/plaza.txt"));
    for c in &candidates {
        if c.is_file() { return c.to_string_lossy().into(); }
    }
    candidates[0].to_string_lossy().into()
}

/// The ground of the shared world (--world FILE, default worlds/plaza.txt).
fn load_world(world: &std::sync::Arc<world::World>, path: Option<String>) {
    let path = path.unwrap_or_else(default_world);
    if path == "none" { return; }
    match world::load_terrain(&path) {
        Ok(t) => world.publish("signet", world::Event::Terrain(std::sync::Arc::new(t))),
        Err(e) => log("world", &format!("no terrain: {e}")),
    }
}

fn usage() {
    eprintln!("signet setup  --role server|client|both [--download] [--install-python] [--games minecraft,gmod,lethal]");
    eprintln!("signet server [--bind 127.0.0.1:7800] [--world FILE|none]");
    eprintln!("signet link   [--server HOST:7800] [--name NAME] [--no-mc] [--no-lc] [--gmod] [--no-forge]");
    eprintln!("signet forge  [--port 7796] [--debug]");
    eprintln!("no mode starts a local link (doors on this PC, no Server)");
}

/// The Forge screen named by setup (or SIGNET_PYTHON and SIGNET_FORGE).
fn exec_forge(rest: &[String]) -> ! {
    let (python, script) = match setup::forge_runtime() {
        Ok(pair) => pair,
        Err(e) => { eprintln!("signet forge: {e}"); std::process::exit(1); }
    };
    let mut cmd = Command::new(&python);
    cmd.arg(&script).args(rest).env("HF_HUB_OFFLINE", std::env::var("HF_HUB_OFFLINE").unwrap_or_else(|_| "1".into()));
    // On Unix the screen replaces this process. On Windows it runs as a child and we exit with its code.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        eprintln!("signet forge: cannot start {python}: {err}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        match cmd.status() {
            Ok(s) => std::process::exit(s.code().unwrap_or(1)),
            Err(e) => { eprintln!("signet forge: cannot start {python}: {e}"); std::process::exit(1); }
        }
    }
}

fn main() {
    START.get_or_init(Instant::now);
    stop::install();
    let args: Vec<String> = std::env::args().collect();
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let has = |k: &str| args.iter().any(|a| a == k);
    let token = std::env::var("SIGNET_TOKEN").ok().filter(|t| !t.is_empty());
    let world = world::World::new();
    if matches!(args.get(1).map(String::as_str), Some("help") | Some("--help") | Some("-h")) {
        usage();
        std::process::exit(0);
    }
    let mode = args.get(1).map(String::as_str).filter(|a| !a.starts_with('-'));

    if mode == Some("setup") {
        std::process::exit(setup::run(&args[2..]));
    }
    if mode == Some("forge") {
        exec_forge(&args[2..]);
    }
    if let Some(m) = mode {
        if m != "server" && m != "link" {
            eprintln!("signet: unknown mode '{m}'. Expected setup, server, link, or forge.");
            usage();
            std::process::exit(2);
        }
    }

    if mode == Some("server") {
        banner::print("Signet Server");
        let bind = opt("--bind").unwrap_or("127.0.0.1:7800".into());
        load_world(&world, opt("--world"));
        let w = world.clone();
        std::thread::spawn(move || net::serve(w, &bind, token));
    } else {
        let cfg = setup::load();
        let script_path = opt("--lc-script").or_else(|| if cfg.lc_script.is_empty() { None } else { Some(cfg.lc_script.clone()) });
        let lc_port: u16 = opt("--lc-port").and_then(|p| p.parse().ok()).unwrap_or(7777);
        let server = opt("--server");
        banner::print("Signet Link");
        log("signet", &match &server {
            Some(s) => format!("Server {s}"),
            None => "local world (no Server)".into(),
        });
        let name = opt("--name").unwrap_or_else(|| std::env::var("USER").unwrap_or("link".into()));
        // one Signet Forge per PC decides, for every door here, how the others are drawn
        let forge = if has("--no-forge") { forge::Forge::off() } else { forge::Forge::start(opt("--forge").unwrap_or(forge::ADDR.into()), name.clone()) };
        if !has("--no-mc") { doors::mc::start(world.clone(), opt("--mc-bridge").unwrap_or(doors::mc::BRIDGE.into()), forge.clone()); }
        if has("--gmod") { doors::gmod::start(world.clone(), opt("--gmod-addr").unwrap_or(doors::gmod::ADDR.into()), forge.clone()); }
        let want_lc = !has("--no-lc") && script_path.is_some();
        if want_lc {
            match doors::lc::load_script(script_path.as_deref().unwrap_or("")) {
                Ok(s) => doors::lc::start(world.clone(), s, lc_port),
                Err(e) => log("lethalcompany", &format!("door off: {e} (pass --lc-script from a capture of your own LAN game; Signet does not ship one)")),
            }
        }
        if server.is_none() { load_world(&world, opt("--world")); }   // a local world can have its own ground too
        if let Some(s) = server {
            net::uplink(world.clone(), s, name, token);
        }
    }
    let what = if mode == Some("server") { "Signet Server" } else { "Signet Link" };
    let mut last = String::new();
    loop {
        for _ in 0..50 {                                  // 5 s, listening for Ctrl-C every 0.1 s
            std::thread::sleep(Duration::from_millis(100));
            if stop::requested() {
                println!();
                log("signet", "stopping (Ctrl-C): cleaning up what this program drew in your games… (Ctrl-C again: quit now)");
                stop::run_hooks();
                log("signet", &format!("{what} stopped"));
                std::process::exit(0);
            }
        }
        let snap = world.snapshot();
        let s = snap.iter().map(|e| format!("#{} {} ({}) at {:.1},{:.1},{:.1}", e.id, e.name, e.origin, e.pos[0], e.pos[1], e.pos[2])).collect::<Vec<_>>().join(" · ");
        if s != last { log("world", &if s.is_empty() { "empty".into() } else { s.clone() }); last = s; }
    }
}
