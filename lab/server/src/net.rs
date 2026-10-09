//! Server <-> Link over the Signet protocol (crate::wire), plain TCP.
//!
//! Signet does not ship a tunnel: each user puts the Server wherever they want (this PC, a LAN box, a cloud VM) and
//! protects the path with their own VPN/tunnel (OpenVPN, WireGuard, Tailscale, ...). The Server listens on the address
//! it is given (default 127.0.0.1), and an optional shared token (env SIGNET_TOKEN) rejects strangers.
//!
//! Ids: the Server gives each Link a number n; a Link's own entities travel as n << 32 | local id, so ids never clash
//! and the Server only accepts events about entities that belong to the Link sending them.
use crate::wire::{self, VERSION};
use crate::world::{Event, World};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const BEAT: Duration = Duration::from_secs(5);
const DEAD: Duration = Duration::from_secs(20);
const LOCAL: u64 = 1 << 32; // ids below this are a Link's own

// ------------------------------------------------------------------ server

pub fn serve(world: Arc<World>, bind: &str, token: Option<String>) {
    let l = match TcpListener::bind(bind) {
        Ok(l) => l,
        Err(e) => { crate::log("server", &format!("cannot listen on {bind}: {e}")); std::process::exit(1); }
    };
    let open = !bind.starts_with("127.") && !bind.starts_with("[::1]") && !bind.starts_with("localhost");
    crate::log("server", &format!("listening for Links on {bind} (Signet protocol {VERSION}){}", if token.is_some() { ", token required" } else { "" }));
    if open {
        crate::log("server", "note: this address can be reached from other machines; keep it behind your own VPN/tunnel or firewall");
    }
    let next = Arc::new(AtomicU64::new(1));
    for s in l.incoming().flatten() {
        let (world, token, next) = (world.clone(), token.clone(), next.clone());
        thread::spawn(move || link_session(world, s, token, next));
    }
}

fn link_session(world: Arc<World>, s: TcpStream, token: Option<String>, next: Arc<AtomicU64>) {
    let peer = s.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let _ = s.set_read_timeout(Some(DEAD));
    let _ = s.set_nodelay(true);
    let mut w = match s.try_clone() { Ok(w) => w, Err(_) => return };
    let mut lines = BufReader::new(s).lines();
    let hello = lines.next().and_then(Result::ok).unwrap_or_default();
    let f: Vec<&str> = hello.split('\t').collect();
    let deny = |w: &mut TcpStream, why: &str| { let _ = writeln!(w, "DENIED\t{why}"); crate::log("server", &format!("{peer} denied: {why}")); };
    if f.len() < 3 || f[0] != "HELLO" { return deny(&mut w, "expected HELLO"); }
    if f[1] != VERSION { return deny(&mut w, &format!("protocol {} (this server speaks {VERSION})", f[1])); }
    if token.as_deref().is_some_and(|t| f.get(3).copied() != Some(t)) { return deny(&mut w, "wrong token"); }
    let n = next.fetch_add(1, SeqCst);
    let door = format!("link{n}");
    let who = format!("{} ({door}, {peer})", f[2]);
    if writeln!(w, "WELCOME\t{n}").is_err() { return; }
    crate::log("server", &format!("Link {who} connected"));

    let alive = Arc::new(AtomicBool::new(true));
    let inbox = world.subscribe(&door); // everything that exists, then every event from the other Links
    {
        let alive = alive.clone();
        thread::spawn(move || {
            while alive.load(SeqCst) {
                let line = match inbox.recv_timeout(BEAT) { Ok(ev) => wire::encode(&ev), Err(_) => "PING".into() };
                if writeln!(w, "{line}").is_err() { break; }
            }
            alive.store(false, SeqCst);
            let _ = w.shutdown(std::net::Shutdown::Both);
        });
    }
    let mut owned: HashSet<u64> = HashSet::new();
    for line in lines {
        let Ok(line) = line else { break };
        if !alive.load(SeqCst) { break; }
        if line == "PING" { continue; }
        let Some(ev) = wire::decode(&line) else { continue };   // (terrain lines from a Link are ignored)
        if let Event::Edit { .. } = ev { world.publish(&door, ev); continue; }   // any game may edit the shared ground
        if ev.id() >> 32 != n { continue; } // a Link only speaks for its own entities
        match &ev { Event::Joined(_) => { owned.insert(ev.id()); } Event::Left { id } => { owned.remove(id); } _ => {} }
        world.publish(&door, ev);
    }
    alive.store(false, SeqCst);
    world.unsubscribe(&door);
    for id in owned { world.publish(&door, Event::Left { id }); }
    crate::log("server", &format!("Link {who} disconnected"));
}

// ------------------------------------------------------------------ link

/// Keep this Link's local world in sync with a Server: local entities go up (as global ids), the other Links'
/// entities come down and are published locally, so the doors draw them exactly like local ones.
pub fn uplink(world: Arc<World>, server: String, name: String, token: Option<String>) {
    thread::spawn(move || {
        let mut warned = false;
        loop {
            match TcpStream::connect(&server) {
                Ok(s) => { warned = false; session(&world, s, &server, &name, token.as_deref()); }
                Err(e) if !warned => { crate::log("link", &format!("cannot reach the Server at {server} ({e}); retrying (is your VPN/tunnel up?)")); warned = true; }
                Err(_) => {}
            }
            thread::sleep(Duration::from_secs(3));
        }
    });
}

fn session(world: &Arc<World>, s: TcpStream, server: &str, name: &str, token: Option<&str>) {
    let _ = s.set_read_timeout(Some(DEAD));
    let _ = s.set_nodelay(true);
    let Ok(mut w) = s.try_clone() else { return };
    if writeln!(w, "HELLO\t{VERSION}\t{name}\t{}", token.unwrap_or("")).is_err() { return; }
    let mut lines = BufReader::new(s).lines();
    let reply = lines.next().and_then(Result::ok).unwrap_or_default();
    let f: Vec<&str> = reply.split('\t').collect();
    let n: u64 = match (f.first().copied(), f.get(1).and_then(|v| v.parse().ok())) {
        (Some("WELCOME"), Some(n)) => n,
        _ => { crate::log("link", &format!("the Server refused this Link: {}", f.get(1).unwrap_or(&"no answer"))); thread::sleep(Duration::from_secs(10)); return; }
    };
    crate::log("link", &format!("connected to the Server {server} as link{n}"));

    let alive = Arc::new(AtomicBool::new(true));
    let inbox = world.subscribe("uplink"); // our entities first, then every local event
    {
        let alive = alive.clone();
        thread::spawn(move || {
            while alive.load(SeqCst) {
                let line = match inbox.recv_timeout(BEAT) {
                    Ok(Event::Terrain(_)) => continue,   // the ground comes from the Server, never from a Link
                    Ok(ev @ Event::Edit { .. }) => wire::encode(&ev),   // edits made in this PC's games go up
                    Ok(ev) if ev.id() < LOCAL => { let id = (n << 32) | ev.id(); wire::encode(&wire::with_id(ev, id)) }
                    Ok(_) => continue, // other Links' entities came from the Server: never send them back
                    Err(_) => "PING".into(),
                };
                if writeln!(w, "{line}").is_err() { break; }
            }
            alive.store(false, SeqCst);
            let _ = w.shutdown(std::net::Shutdown::Both);
        });
    }
    let mut remote: HashSet<u64> = HashSet::new();
    let mut terrain = wire::TerrainReader::default();
    for line in lines {
        let Ok(line) = line else { break };
        if !alive.load(SeqCst) { break; }
        if line == "PING" { continue; }
        if let Some(done) = terrain.feed(&line) {        // the Server's ground: every door paints it
            if let Some(t) = done { world.publish("uplink", Event::Terrain(Arc::new(t))); }
            continue;
        }
        let Some(ev) = wire::decode(&line) else { continue };
        if let Event::Edit { .. } = ev { world.publish("uplink", ev); continue; }   // another game edited the ground
        if ev.id() < LOCAL || ev.id() >> 32 == n { continue; }
        match &ev { Event::Joined(_) => { remote.insert(ev.id()); } Event::Left { id } => { remote.remove(id); } _ => {} }
        world.publish("uplink", ev);
    }
    alive.store(false, SeqCst);
    world.unsubscribe("uplink");
    for id in remote { world.publish("uplink", Event::Left { id }); }
    crate::log("link", "lost the Server; reconnecting");
}
