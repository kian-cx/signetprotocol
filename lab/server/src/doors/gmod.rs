//! Garry's Mod door. Here we *are* the server for real: the official GMod dedicated server (srcds, LAN only) runs our
//! server-side Lua addon (gmod/signet), which talks to this door over local HTTP ~30 times a second (GMod only allows that on dedicated
//! servers started with -allowlocalhttp). The player's game is untouched: console `connect 127.0.0.1:27015`.
//!
//!   addon -> door  POST /tick, text lines:
//!     S x y z                    the map's spawn point (Source units): the shared origin
//!     P uid x y z yaw crouch look name     each GMod player (look = player_manager name, e.g. "kleiner")
//!     C uid text                 chat
//!     G final ox1 oy1 oz1 ox2 oy2 oz2 nx1 ny1 nz1 nx2 ny2 nz2
//!                                a ground box moved with the physics gun: where it was (its /world line) and where it
//!                                is now; final 1 when it is dropped. Becomes two edits of the shared ground: air where
//!                                it was, its material where it is (the other games see it move while it is carried)
//!   door -> addon  answer, text lines:
//!     W version                  the shared world's ground: when it changes, the addon GETs /world
//!   GET /world -> B x1 y1 z1 x2 y2 z2 solid texture   one box per line, Source units (mins, maxs); texture = the GMod
//!                                          surface Signet Forge chose for its material ("-" until it answers)
//!     E id x y z yaw moving look label     each entity from another game, in Source units; look = the GMod player
//!                                          model Signet Forge chose ("-" until it answers: the addon's neutral default)
//!
//! Frames: Source is Z up, X forward, Y left, 1 unit = 1 inch for characters (a player is 72 units = 1.83 m).
//! Universal = metres, Y up (Minecraft-like): u = (sx, sz, -sy) * 0.0254 from the spawn. Yaw: universal (Minecraft
//! convention) = -90 - source yaw.
use crate::forge::Forge;
use crate::world::{Entity, Event, Ground, TBox, World, AIR};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const NAME: &str = "garrysmod";
pub const TARGET: &str = "garry_s_mod";              // this game's catalog (SF Catalog)
pub const ADDR: &str = "127.0.0.1:7795";
const INCH: f64 = 0.0254;
const GONE: Duration = Duration::from_secs(3);

struct Mine { id: u64, seen: Instant, crouch: bool }
struct Other { pos: [f64; 3], yaw: f32, moved: Instant, label: String, key: String, name: String, desc: String }

struct Door {
    spawn: [f64; 3],
    mine: HashMap<String, Mine>,
    others: HashMap<u64, Other>,
    forge: Arc<Forge>,
    last_tick: Option<Instant>,
    ground: Ground,
    generation: u64,                                   // bumps with every change of the ground
    carried: HashMap<String, (String, TBox, TBox)>,    // boxes being moved here: key -> (material, from, to), applied on drop
    cache: Option<(u64, u64, [f64; 3], Built)>,        // (generation, Signet Forge version, spawn) of the last build
}

/// The ground built for GMod: its version, the /world text, and each line's material (to know what a moved box is).
#[derive(Clone)]
struct Built { version: u64, text: String, material: HashMap<String, String>, place: HashMap<String, Aabb> }

pub fn start(world: Arc<World>, addr: String, forge: Arc<Forge>) {
    thread::spawn(move || {
        let l = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => { crate::log(NAME, &format!("cannot listen on {addr}: {e}")); return; }
        };
        crate::log(NAME, &format!("waiting for the GMod server addon on http://{addr} (start srcds with -allowlocalhttp; then in GMod: connect 127.0.0.1:27015)"));
        let inbox = world.subscribe(NAME);
        let mut d = Door { spawn: [0.0; 3], mine: HashMap::new(), others: HashMap::new(), forge, last_tick: None, ground: world.ground(),
                               generation: 1, carried: HashMap::new(), cache: None };
        let mut watchdog = Instant::now();
        let _ = l.set_nonblocking(true);
        loop {
            while let Ok(ev) = inbox.try_recv() { on_world(&mut d, ev); }
            match l.accept() {
                Ok((s, _)) => { let _ = s.set_nonblocking(false); serve(&world, &mut d, s); }
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
            if watchdog.elapsed() > Duration::from_millis(500) {
                watchdog = Instant::now();
                let gone: Vec<String> = d.mine.iter().filter(|(_, m)| m.seen.elapsed() > GONE).map(|(k, _)| k.clone()).collect();
                for k in gone { let m = d.mine.remove(&k).unwrap(); world.publish(NAME, Event::Left { id: m.id }); }
                if d.last_tick.is_some_and(|t| t.elapsed() > GONE) { crate::log(NAME, "the GMod server addon stopped talking"); d.last_tick = None; }
            }
        }
    });
}

fn on_world(d: &mut Door, ev: Event) {
    match ev {
        Event::Joined(e) if e.kind == "player" => {
            crate::log(NAME, &format!("#{} ({} from {}) will be drawn in Garry's Mod", e.id, e.name, e.origin));
            d.others.insert(e.id, Other { pos: e.pos, yaw: e.yaw, moved: Instant::now(), label: format!("{} ({})", e.name, e.origin),
                                          key: format!("{}:{}", e.origin, e.key), name: format!("{} player", e.origin), desc: e.desc.clone() });
        }
        Event::Moved { id, pos, yaw } => {
            if let Some(o) = d.others.get_mut(&id) {
                if (0..3).any(|i| (o.pos[i] - pos[i]).abs() > 0.01) { o.moved = Instant::now(); }
                o.pos = pos; o.yaw = yaw;
            }
        }
        Event::Left { id } => { d.others.remove(&id); }
        Event::Terrain(t) => {
            crate::log(NAME, &format!("the shared world's ground '{}' will be built in Garry's Mod", t.name));
            d.ground.set_base(t); d.generation += 1;
        }
        Event::Edit { key, b, origin } => {
            if origin != NAME { crate::log(NAME, &format!("{origin} changed the ground: {} ({key})", b.material)); }
            d.ground.edit(&key, b); d.generation += 1;
        }
        _ => {}
    }
}

fn serve(world: &Arc<World>, d: &mut Door, s: TcpStream) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let mut r = BufReader::new(&s);
    let mut len = 0usize;
    let mut line = String::new();
    if r.read_line(&mut line).unwrap_or(0) == 0 { return; }
    let get_world = line.starts_with("GET /world");
    loop {
        line.clear();
        if r.read_line(&mut line).unwrap_or(0) == 0 { return; }
        let l = line.trim_end();
        if l.is_empty() { break; }
        if let Some(v) = l.to_ascii_lowercase().strip_prefix("content-length:") { len = v.trim().parse().unwrap_or(0); }
    }
    let mut body = vec![0u8; len.min(1 << 20)];
    if r.read_exact(&mut body).is_err() { return; }
    if get_world {
        let text = built(d).text;
        let _ = (&s).write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len()).as_bytes());
        return;
    }
    if d.last_tick.is_none() { crate::log(NAME, "the GMod server addon is connected"); }
    d.last_tick = Some(Instant::now());
    on_tick(world, d, &String::from_utf8_lossy(&body));
    let answer = answer(d);
    let _ = (&s).write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}", answer.len()).as_bytes());
}

fn to_universal(spawn: [f64; 3], s: [f64; 3]) -> [f64; 3] {
    let (x, y, z) = (s[0] - spawn[0], s[1] - spawn[1], s[2] - spawn[2]);
    [x * INCH, z * INCH, -y * INCH]
}

fn to_source(spawn: [f64; 3], u: [f64; 3]) -> [f64; 3] {
    [u[0] / INCH + spawn[0], -u[2] / INCH + spawn[1], u[1] / INCH + spawn[2]]
}

fn on_tick(world: &Arc<World>, d: &mut Door, body: &str) {
    for line in body.lines() {
        let f: Vec<&str> = line.splitn(9, ' ').collect();
        let num = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        match f.first().copied() {
            Some("S") if f.len() >= 4 => d.spawn = [num(1), num(2), num(3)],
            Some("P") if f.len() >= 8 => {
                let uid = f[1].to_string();
                let pos = to_universal(d.spawn, [num(2), num(3), num(4)]);
                let yaw = (-90.0 - num(5)) as f32;
                let crouch = f[6] == "1";
                let look = f[7].to_string();
                let name = f.get(8).copied().unwrap_or("").to_string();
                match d.mine.get_mut(&uid) {
                    Some(m) => {
                        m.seen = Instant::now();
                        world.publish(NAME, Event::Moved { id: m.id, pos, yaw });
                        if m.crouch != crouch { m.crouch = crouch; world.publish(NAME, Event::Action { id: m.id, what: "sneak".into(), arg: (crouch as u8).to_string() }); }
                    }
                    None => {
                        let id = world.new_id();
                        world.publish(NAME, Event::Joined(Entity {
                            id, origin: NAME.into(), kind: "player".into(), key: look.clone(),
                            name: if name.is_empty() { "Garry's Mod player".into() } else { name },
                            desc: format!("a Half-Life 2 style human character (Garry's Mod player model \"{look}\")"), pos, yaw }));
                        d.mine.insert(uid, Mine { id, seen: Instant::now(), crouch });
                    }
                }
            }
            Some("G") => { carry(world, d, line); }
            Some("C") if f.len() >= 3 => {
                let (uid, text) = line[2..].split_once(' ').unwrap_or(("", ""));
                if let Some(m) = d.mine.get(uid) { world.publish(NAME, Event::Action { id: m.id, what: "chat".into(), arg: text.into() }); }
            }
            _ => {}
        }
    }
}

/// A source box (as a /world line has it: mins, maxs, 1 unit above the ground's real top) in universal metres.
fn box_to_universal(spawn: [f64; 3], mn: [f64; 3], mx: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let a = to_universal(spawn, [mn[0], mn[1], mn[2] - 1.0]);
    let b = to_universal(spawn, [mx[0], mx[1], mx[2] - 1.0]);
    ([0, 1, 2].map(|k| a[k].min(b[k])), [0, 1, 2].map(|k| a[k].max(b[k])))
}

/// A box carried with the physics gun: two edits of the shared ground, sent while it moves; applied here on drop.
fn carry(world: &Arc<World>, d: &mut Door, line: &str) {
    let f: Vec<&str> = line.split(' ').collect();
    if f.len() != 14 { return; }
    let v: Vec<f64> = f[2..14].iter().filter_map(|x| x.parse().ok()).collect();
    if v.len() != 12 { return; }
    let geo = f[2..8].join(" ");
    let fin = f[1] == "1";
    let entry = match d.carried.get(&geo) {
        Some(e) => Some((e.0.clone(), (e.1.min, e.1.max))),
        None => { let b = built(d); b.material.get(&geo).cloned().zip(b.place.get(&geo).copied()) }
    };
    let Some((material, (a, b))) = entry else {
        crate::log(NAME, &format!("the physics gun moved a box that is not in the shared ground ({geo}): ignored"));
        return;
    };
    // where it is now: the same box, moved by what the source box moved
    let (o, _) = box_to_universal(d.spawn, [v[0], v[1], v[2]], [v[3], v[4], v[5]]);
    let (n, _) = box_to_universal(d.spawn, [v[6], v[7], v[8]], [v[9], v[10], v[11]]);
    let delta = [0, 1, 2].map(|k| n[k] - o[k]);
    let from = TBox { min: a, max: b, material: AIR.into() };
    let to = TBox { min: [0, 1, 2].map(|k| a[k] + delta[k]), max: [0, 1, 2].map(|k| b[k] + delta[k]), material: material.clone() };
    let first = !d.carried.contains_key(&geo);
    if first {
        crate::log(NAME, &format!("you picked up the ground's '{material}' with the physics gun: the other games see it move"));
        world.publish(NAME, Event::Edit { key: format!("gmod:{geo}:from"), b: from.clone(), origin: NAME.into() });
    }
    world.publish(NAME, Event::Edit { key: format!("gmod:{geo}:to"), b: to.clone(), origin: NAME.into() });
    d.carried.insert(geo.clone(), (material.clone(), from, to));
    if fin {
        let (_, from, to) = d.carried.remove(&geo).unwrap();
        d.ground.edit(&format!("gmod:{geo}:from"), from);
        d.ground.edit(&format!("gmod:{geo}:to"), to.clone());
        d.generation += 1;
        crate::log(NAME, &format!("you dropped '{material}' at {:.1},{:.1},{:.1}", to.min[0], to.min[1], to.min[2]));
    }
}

type Aabb = ([f64; 3], [f64; 3]);

/// a minus b: up to 6 boxes that cover a without b (later boxes win, like in the world file).
fn subtract(a: Aabb, b: Aabb) -> Vec<Aabb> {
    let (mut lo, mut hi) = a;
    if (0..3).any(|k| b.1[k] <= lo[k] || b.0[k] >= hi[k]) { return vec![a]; }      // no overlap
    let mut out = Vec::new();
    for k in 0..3 {                                                                 // peel the slabs outside b, axis by axis
        if b.0[k] > lo[k] { let mut h = hi; h[k] = b.0[k]; out.push((lo, h)); lo[k] = b.0[k]; }
        if b.1[k] < hi[k] { let mut l = lo; l[k] = b.1[k]; out.push((l, hi)); hi[k] = b.1[k]; }
    }
    out
}

const TILE: f64 = 2048.0;   // Source units: physics boxes stay a size the engine likes

/// The ground in Source units, with the surface SF chose for each material, and a version that changes with it.
/// Boxes are made disjoint (a later box cuts the earlier ones: the pond is a hole full of water, not water on grass),
/// all solid but water, big ones cut in tiles; it stands on its own, whatever the map has under it.
fn built(d: &mut Door) -> Built {
    let fv = d.forge.version();
    if let Some((g, v, sp, b)) = &d.cache { if *g == d.generation && *v == fv && *sp == d.spawn { return b.clone(); } }
    let b = ground(d);
    d.cache = Some((d.generation, fv, d.spawn, b.clone()));
    b
}

fn liquid(material: &str) -> bool { ["water", "lava"].iter().any(|w| material.contains(w)) }

fn ground(d: &Door) -> Built {
    let boxes: Vec<&TBox> = d.ground.boxes().collect();
    if boxes.is_empty() { return Built { version: 0, text: String::new(), material: HashMap::new(), place: HashMap::new() } }
    let mut pieces: Vec<(Aabb, usize)> = Vec::new();
    for (i, b) in boxes.iter().enumerate() {
        let cut = (b.min, b.max);
        pieces = pieces.into_iter().flat_map(|(p, m)| subtract(p, cut).into_iter().map(move |q| (q, m))).collect();
        pieces.push((cut, i));
    }
    let mut out = String::new();
    let (mut material, mut place) = (HashMap::new(), HashMap::new());
    for ((lo, hi), i) in pieces {
        if (0..3).any(|k| hi[k] - lo[k] < 1e-3) { continue; }      // (slivers under a millimetre: nothing to build)
        let b = boxes[i];
        if b.material == AIR { continue; }                 // carved: nothing to build
        let tex = d.forge.ask(TARGET, "material", &b.material, &b.material, "").map(|a| a.choice).unwrap_or("-".into());
        let solid = !liquid(&b.material);
        let (p, q) = (to_source(d.spawn, lo), to_source(d.spawn, hi));
        let (mn, mx) = ([p[0].min(q[0]), p[1].min(q[1]), p[2].min(q[2]) + 1.0], [p[0].max(q[0]), p[1].max(q[1]), p[2].max(q[2]) + 1.0]);
        let (nx, ny) = (((mx[0] - mn[0]) / TILE).ceil().max(1.0) as usize, ((mx[1] - mn[1]) / TILE).ceil().max(1.0) as usize);
        for ix in 0..nx {
            for iy in 0..ny {
                let x0 = mn[0] + (mx[0] - mn[0]) * ix as f64 / nx as f64;
                let x1 = mn[0] + (mx[0] - mn[0]) * (ix + 1) as f64 / nx as f64;
                let y0 = mn[1] + (mx[1] - mn[1]) * iy as f64 / ny as f64;
                let y1 = mn[1] + (mx[1] - mn[1]) * (iy + 1) as f64 / ny as f64;
                let geo = format!("{x0:.2} {y0:.2} {:.2} {x1:.2} {y1:.2} {:.2}", mn[2], mx[2]);
                out += &format!("B {geo} {} {tex}\n", solid as u8);
                // the exact universal box of this tile (a carried box leaves exactly this hole)
                let (u0, u1) = box_to_universal(d.spawn, [x0, y0, mn[2]], [x1, y1, mx[2]]);
                let exact = ([0, 1, 2].map(|k| u0[k].max(lo[k])), [0, 1, 2].map(|k| u1[k].min(hi[k])));
                let snap = |v: [f64; 3], w: [f64; 3]| [0, 1, 2].map(|k| if (v[k] - w[k]).abs() < 1e-3 { w[k] } else { v[k] });
                place.insert(geo.clone(), (snap(exact.0, lo), snap(exact.1, hi)));
                material.insert(geo, b.material.clone());
            }
        }
    }
    let version = out.bytes().fold(1469598103934665603u64, |h, c| (h ^ c as u64).wrapping_mul(1099511628211)) | 1;
    Built { version, text: out, material, place }
}

fn answer(d: &mut Door) -> String {
    let mut out = String::new();
    if d.ground.base.is_some() { out += &format!("W {}\n", built(d).version); }
    for (id, o) in &d.others {
        let p = to_source(d.spawn, o.pos);
        let yaw = -90.0 - o.yaw as f64;
        let moving = o.moved.elapsed() < Duration::from_millis(300);
        let look = d.forge.ask(TARGET, "player", &o.key, &o.name, &o.desc).map(|a| a.choice).unwrap_or("-".into());
        out += &format!("E {id} {:.1} {:.1} {:.1} {:.1} {} {look} {}\n", p[0], p[1], p[2], yaw, moving as u8, o.label.replace('\n', " "));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn later_boxes_cut_earlier_ones() {
        let vol = |b: &Aabb| (0..3).map(|k| b.1[k] - b.0[k]).product::<f64>();
        let ground = ([-10.0, -1.0, -10.0], [10.0, 0.0, 10.0]);
        let pond = ([-2.0, -3.0, -2.0], [2.0, 0.0, 2.0]);
        let rest = subtract(ground, pond);
        assert!((rest.iter().map(vol).sum::<f64>() - (400.0 - 16.0)).abs() < 1e-9);     // the hole is exactly the pond
        assert!(rest.iter().all(|r| subtract(*r, pond).len() == 1));                   // and nothing overlaps it
        assert_eq!(subtract(ground, ([20.0, 0.0, 0.0], [21.0, 1.0, 1.0])), vec![ground]);
    }
    #[test]
    fn carried_boxes_become_edits() {
        let world = World::new();
        let rx = world.subscribe("test");
        let mut d = Door { spawn: [0.0, 0.0, -12288.0], mine: HashMap::new(), others: HashMap::new(), forge: crate::forge::Forge::off(),
                           last_tick: None, ground: Ground::default(), generation: 1, carried: HashMap::new(), cache: None };
        d.ground.set_base(Arc::new(crate::world::Terrain { name: "t".into(), boxes: vec![
            TBox { min: [-10.0, -1.0, -10.0], max: [10.0, 0.0, 10.0], material: "grass".into() },
            TBox { min: [2.0, 0.0, 2.0], max: [3.0, 5.0, 3.0], material: "oak tree trunk bark".into() },
            TBox { min: [4.0, -3.0, 4.0], max: [6.0, 0.0, 6.0], material: "lava".into() }] }));
        let b = built(&mut d);
        assert!(b.text.lines().any(|l| l.ends_with(" 0 -")) && b.material.values().any(|m| m == "lava"));   // lava: not solid
        let (geo, _) = b.material.iter().find(|(_, m)| *m == "oak tree trunk bark").unwrap();
        let g: Vec<f64> = geo.split(' ').map(|x| x.parse().unwrap()).collect();
        let moved = format!("{} {} {} {} {} {}", g[0] + 200.0, g[1], g[2], g[3] + 200.0, g[4], g[5]);
        carry(&world, &mut d, &format!("G 0 {geo} {moved}"));
        carry(&world, &mut d, &format!("G 1 {geo} {moved}"));
        let edits: Vec<(String, TBox)> = rx.try_iter().filter_map(|e| match e { Event::Edit { key, b, .. } => Some((key, b)), _ => None }).collect();
        assert_eq!(edits.len(), 3);                                          // air where it was, then where it is (twice)
        assert_eq!(edits[0].1.material, AIR);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-3;                     // (source lines carry 0.01 units)
        assert!(near(edits[0].1.min[0], 2.0) && near(edits[0].1.min[1], 0.0) && near(edits[0].1.max[1], 5.0), "{:?}", edits[0].1);   // the trunk's place
        assert!(near(edits[2].1.min[0], 2.0 + 200.0 * INCH) && edits[2].1.material == "oak tree trunk bark");
        let after = built(&mut d);                                           // applied here on drop: the trunk is in its new place
        let trunks: Vec<f64> = after.material.iter().filter(|(_, m)| *m == "oak tree trunk bark")
            .map(|(k, _)| k.split(' ').next().unwrap().parse().unwrap()).collect();
        assert!(trunks.len() == 1 && (trunks[0] - (g[0] + 200.0)).abs() < 0.05, "{trunks:?}");
    }
    #[test]
    fn frames_roundtrip() {
        let spawn = [100.0, -200.0, 64.0];
        let s = [140.0, -260.0, 64.0];
        let u = to_universal(spawn, s);
        assert!((u[0] - 40.0 * INCH).abs() < 1e-9 && u[1].abs() < 1e-9 && (u[2] - 60.0 * INCH).abs() < 1e-9);
        let back = to_source(spawn, u);
        assert!((0..3).all(|i| (back[i] - s[i]).abs() < 1e-9));
    }
}
