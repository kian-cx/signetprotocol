//! Lethal Company door: Signet Server *is the host*. A vanilla Lethal Company started in LAN mode joins
//! 127.0.0.1:7777 when you press "Join" (the address is fixed in the game's transport settings), and in LAN mode the
//! client only sends its game version, no Steam identity. So nothing in the game has to change.
//!
//! Phase 1 (this file): Unity Transport + Netcode host side, and a host *script* replayed from a local capture of your
//! own LAN game: ConnectionApproved, the ship synchronisation, the lobby. TimeSync is
//! generated live. What the client sends is read: its player position/rotation become a universal entity.
//! Phase 2 (next): spawn the other games' players in the ship from universal events (instead of only logging them).
use super::{LC_SHIP_IN_WORLD, LC_SHIP_SPOT};
use crate::ngo::{self, bp_u32};
use crate::utp::{self, Conn};
use crate::world::{Entity, Event, World};
use std::collections::{BTreeMap, HashSet};
use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const NAME: &str = "lethalcompany";
const POS_RPC: u32 = 890_924_887;   // PlayerControllerB.UpdatePlayerPositionRpc(Vector3, inElevator, inShip, ...)
const ROT_RPC: u32 = 588_787_670;   // PlayerControllerB.UpdatePlayerRotationServerRpc(short pitch, short yaw)
const GONE: Duration = Duration::from_secs(10);

pub struct Script { tick: u32, tickrate: u32, join: Vec<Vec<u8>>, synced: Vec<(f64, Vec<u8>)> }

pub fn load_script(path: &str) -> Result<Script, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut s = Script { tick: 0, tickrate: 50, join: vec![], synced: vec![] };
    let hex = |h: &str| (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0)).collect::<Vec<u8>>();
    for l in text.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        match f.as_slice() {
            ["tick", n] => s.tick = n.parse().unwrap_or(0),
            ["tickrate", n] => s.tickrate = n.parse().unwrap_or(50),
            ["R", "join", _seq, _d, h] => s.join.push(hex(h)),
            ["R", "synced", _seq, d, h] => s.synced.push((d.parse().unwrap_or(0.0), hex(h))),
            _ => {}
        }
    }
    if s.join.is_empty() { return Err(format!("{path}: no join packets")); }
    Ok(s)
}

struct Client {
    conn: Conn,
    approved: Option<Instant>,
    synced: Option<Instant>,
    next: usize,
    entity: Option<u64>,
    pos: [f64; 3],
    yaw: f32,
    last_pub: Instant,
    seen: HashSet<(u32, u32)>,   // (message type, method) already logged
    counts: BTreeMap<&'static str, u64>,
}

pub fn start(world: Arc<World>, script: Script, port: u16) {
    thread::spawn(move || run(world, script, port));
}

fn run(world: Arc<World>, script: Script, port: u16) {
    let sock = match UdpSocket::bind(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => { crate::log(NAME, &format!("cannot listen on UDP 127.0.0.1:{port} ({e}); is a Lethal Company host already running? Close it: Signet is the host")); return; }
    };
    sock.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
    crate::log(NAME, &format!("host listening on UDP 127.0.0.1:{port}: in Lethal Company choose LAN, then Join ({} + {} scripted packets, tick {})",
                              script.join.len(), script.synced.len(), script.tick));
    let inbox = world.subscribe(NAME);
    let mut drawn: HashSet<u64> = HashSet::new();
    let mut c: Option<Client> = None;
    let (mut last_ping, mut last_sync, mut last_diag) = (Instant::now(), Instant::now(), Instant::now());
    let mut buf = [0u8; 2048];
    loop {
        // ---- what the other games publish (phase 2 will spawn them in the ship) ----
        while let Ok(ev) = inbox.try_recv() {
            if let Event::Joined(e) = &ev {
                if e.kind == "player" && drawn.insert(e.id) {
                    crate::log(NAME, &format!("#{} ({} from {}) should appear in the ship: host-side spawning is phase 2", e.id, e.name, e.origin));
                }
            }
        }
        // ---- packets ----
        if let Ok((n, addr)) = sock.recv_from(&mut buf) {
            let p = &buf[..n];
            match p.first().copied() {
                Some(utp::CONN_REQ) if n >= 10 => {
                    let token: [u8; 8] = p[2..10].try_into().unwrap();
                    match &c {
                        Some(cl) if cl.conn.client_token == token => cl.conn.send_accept(&sock),
                        _ => {
                            if let Some(old) = c.take() { leave(&world, old, "replaced by a new connection"); }
                            crate::log(NAME, &format!("Unity Transport: connection request from {addr}: accepted"));
                            c = Some(Client { conn: Conn::accept(&sock, addr, token), approved: None, synced: None, next: 0, entity: None,
                                              pos: [0.0; 3], yaw: 0.0, last_pub: Instant::now(), seen: HashSet::new(), counts: BTreeMap::new() });
                        }
                    }
                }
                Some(utp::PING) => if let Some(cl) = &mut c { cl.conn.last_rx = Instant::now(); cl.conn.pong(&sock); },
                Some(utp::PONG) => if let Some(cl) = &mut c { cl.conn.last_rx = Instant::now(); },
                Some(utp::DISCONNECT) => if let Some(cl) = c.take() { leave(&world, cl, "the client disconnected"); },
                Some(utp::DATA) => if let Some(cl) = &mut c {
                    if cl.conn.on_data(&sock, p) {
                        let mut msgs = Vec::new();
                        let used = ngo::parse_stream(&cl.conn.stream, |t, b| msgs.push((t, b.to_vec())));
                        cl.conn.stream.drain(..used);
                        for (t, b) in msgs { on_message(&world, &sock, &script, cl, t, &b); }
                    }
                },
                _ => {}
            }
        }
        // ---- timers ----
        if let Some(cl) = &mut c {
            if cl.conn.last_rx.elapsed() > GONE { let cl = c.take().unwrap(); leave(&world, cl, "no packets for 10 s"); continue; }
            cl.conn.resend(&sock);
            if let Some(t0) = cl.synced {                       // the lobby, paced like the real host
                while cl.next < script.synced.len() && script.synced[cl.next].0 <= t0.elapsed().as_secs_f64() && cl.conn.in_flight() < 32 {
                    let payload = script.synced[cl.next].1.clone();
                    cl.conn.send_reliable(&sock, payload); cl.next += 1;
                    if cl.next == script.synced.len() { crate::log(NAME, "host script finished: the client should be standing in the ship"); }
                }
            }
            if let Some(t0) = cl.approved {
                if last_sync.elapsed() >= Duration::from_secs(1) {   // TimeSync: the host's tick, once per second
                    last_sync = Instant::now();
                    let tick = script.tick + (t0.elapsed().as_secs_f64() * script.tickrate as f64) as u32;
                    let b = ngo::batch(&[(ngo::TIME_SYNC, bp_u32(tick))]);
                    cl.conn.send_unreliable(&sock, &b);
                }
            }
            if last_ping.elapsed() >= Duration::from_secs(1) { last_ping = Instant::now(); cl.conn.ping(&sock); }
            if last_diag.elapsed() >= Duration::from_secs(10) {
                last_diag = Instant::now();
                crate::log(NAME, &format!("[diag] rtt {} ms · resends {} · in flight {} · scripted {}/{} · from the client: {:?}",
                    cl.conn.rtt.as_millis(), cl.conn.resends, cl.conn.in_flight(), cl.next, script.synced.len(), cl.counts));
            }
        }
    }
}

fn on_message(world: &Arc<World>, sock: &UdpSocket, script: &Script, cl: &mut Client, t: u32, b: &[u8]) {
    *cl.counts.entry(ngo::name(t)).or_default() += 1;
    match t {
        ngo::CONNECTION_REQUEST => {
            let version: String = longest_digits(b);
            crate::log(NAME, &format!("Netcode ConnectionRequest ({} B, game version {}): approving and sending the ship",
                                       b.len(), if version.is_empty() { "?".into() } else { version }));
            if cl.approved.is_none() {
                for p in &script.join { cl.conn.send_reliable(sock, p.clone()); }
                cl.approved = Some(Instant::now());
                let id = world.new_id();
                cl.entity = Some(id);
                world.publish(NAME, Event::Joined(Entity {
                    id, origin: NAME.into(), kind: "player".into(), key: "PlayerControllerB".into(), name: "Lethal Company player".into(),
                    desc: "a company employee in an orange work jumpsuit and a helmet".into(), pos: [0.0; 3], yaw: 0.0 }));
            }
        }
        ngo::SCENE_EVENT => {
            let kind = b.first().copied().unwrap_or(255);
            let what = match kind { 3 => "ReSynchronize", 6 => "LoadComplete", 7 => "UnloadComplete", 8 => "SynchronizeComplete", _ => "other" };
            crate::log(NAME, &format!("client SceneEvent {kind} ({what}, {} B)", b.len()));
            if cl.synced.is_none() { cl.synced = Some(Instant::now()); crate::log(NAME, "the client loaded the ship: playing the lobby"); }
        }
        ngo::PROXY | ngo::RPC | ngo::SERVER_RPC => {
            if let Some((method, args)) = rpc(t, b) {
                if cl.seen.insert((t, method)) {
                    crate::log(NAME, &format!("client {} method {method} ({} B of arguments)", ngo::name(t), args.len()));
                }
                if method == POS_RPC && args.len() >= 13 {
                    let f = |i: usize| f32::from_le_bytes(args[i..i + 4].try_into().unwrap()) as f64;
                    let (mut x, mut y, mut z) = (f(0), f(4), f(8));
                    if args[12] == 0 { x -= LC_SHIP_IN_WORLD[0]; y -= LC_SHIP_IN_WORLD[1]; z -= LC_SHIP_IN_WORLD[2]; } // world -> ship-local
                    // ship-local -> universal (X mirrored: Unity is left-handed)
                    cl.pos = [-(x - LC_SHIP_SPOT[0]), y - LC_SHIP_SPOT[1], z - LC_SHIP_SPOT[2]];
                    publish_move(world, cl);
                } else if method == ROT_RPC {
                    if let Some((_pitch, j)) = ngo::read_i16(args, 0) { if let Some((yaw, _)) = ngo::read_i16(args, j) { cl.yaw = yaw as f32; publish_move(world, cl); } }
                }
            }
        }
        _ => {}
    }
}

fn publish_move(world: &Arc<World>, cl: &mut Client) {
    if let Some(id) = cl.entity {
        if cl.last_pub.elapsed() >= Duration::from_millis(50) { cl.last_pub = Instant::now(); world.publish(NAME, Event::Moved { id, pos: cl.pos, yaw: cl.yaw }); }
    }
}

/// (method id, arguments) of an RPC sent by the client: [Proxy targets] [sender] object behaviour method args
fn rpc(t: u32, b: &[u8]) -> Option<(u32, &[u8])> {
    let mut i = 0;
    if t == ngo::PROXY {
        let n = i32::from_le_bytes(b.get(0..4)?.try_into().ok()?) as usize;
        i = 4 + 8 * n + 1;
    }
    if t != ngo::SERVER_RPC { i = ngo::read_u64(b, i)?.1; }   // sender
    i = ngo::read_u64(b, i)?.1;                                 // network object
    i += (*b.get(i)? & 3) as usize;                             // behaviour index (bit-packed ushort)
    let (method, j) = if *b.get(i)? == 5 { (u32::from_le_bytes(b.get(i + 1..i + 5)?.try_into().ok()?), i + 5) } else { ngo::read_u32(b, i)? };
    Some((method, b.get(j..)?))
}

fn longest_digits(b: &[u8]) -> String {
    let mut best = String::new(); let mut cur = String::new();
    for &x in b { if x.is_ascii_digit() { cur.push(x as char) } else { if cur.len() > best.len() { best = cur.clone(); } cur.clear(); } }
    if cur.len() > best.len() { cur } else { best }
}

fn leave(world: &Arc<World>, cl: Client, why: &str) {
    crate::log(NAME, &format!("client gone: {why}"));
    if let Some(id) = cl.entity { world.publish(NAME, Event::Left { id }); }
}
