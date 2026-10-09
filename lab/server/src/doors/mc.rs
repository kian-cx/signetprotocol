//! Minecraft door. Our Rust proto (~/minecraft-signet/proto) already is a Minecraft server; this door talks to it over
//! its local bridge (127.0.0.1:7790, text lines):
//!   proto -> door: "MC x y z yaw moving" (the Minecraft player), "MCEV place|break|hold|chat|sneak ..." (its actions)
//!   door -> proto: "SPAWN eid minecraft:<creature> x y z yaw label" / "MOVE eid x y z yaw" / "REMOVE eid":
//!                  a player from another game, drawn as the creature Signet Forge chose (name tag = label)
//!                  "WORLD name" · "BOX x1 y1 z1 x2 y2 z2 minecraft:block" (inclusive block ranges) · "WORLDEND":
//!                  the shared world's ground, one block per material, chosen in Signet Forge
//!                  "PATCH x1 y1 z1 x2 y2 z2 minecraft:block": one region repainted (an edit from another game,
//!                  or a new choice in Signet Forge), applied in order like the boxes
//!   proto -> door: "MCEV block x y z minecraft:block": a block placed or broken in Minecraft (air): an edit of the
//!                  shared ground, a 1 m box with the block's name in words ("lava", "oak planks")
//! Until SF answers: armor stands for players, stone for the ground.
use super::MC_SPAWN;
use crate::forge::Forge;
use crate::world::{Entity, Event, Ground, TBox, World, AIR};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const NAME: &str = "minecraft";
pub const BRIDGE: &str = "127.0.0.1:7790";
pub const TARGET: &str = "minecraft_26_3";          // this game's catalog (SF Catalog)
const GONE: Duration = Duration::from_secs(3);
const DEFAULT_PLAYER: &str = "minecraft:armor_stand";
const DEFAULT_BLOCK: &str = "minecraft:stone";

/// The bridge of the current session and the players drawn there: Ctrl-C removes them from Minecraft.
type Drawn = Arc<Mutex<(Option<TcpStream>, Vec<i32>)>>;

pub fn start(world: Arc<World>, bridge: String, forge: Arc<Forge>) {
    let inbox = Arc::new(Mutex::new(world.subscribe(NAME)));
    let drawn: Drawn = Arc::default();
    {
        let drawn = drawn.clone();
        crate::stop::on_stop(move || {
            let mut d = drawn.lock().unwrap();
            let eids = std::mem::take(&mut d.1);
            if let Some(w) = d.0.as_mut() {
                let mut out: String = eids.iter().map(|e| format!("REMOVE {e}\n")).collect();
                out += "SAY [Signet] Signet Link stopped: players from other games are gone until it is back\n";
                let _ = w.write_all(out.as_bytes());
                crate::log(NAME, &format!("removed {} player(s) from other games in Minecraft", eids.len()));
            }
        });
    }
    thread::spawn(move || loop {
        match TcpStream::connect(&bridge) {
            Ok(s) => {
                crate::log(NAME, &format!("connected to the Minecraft proto bridge {bridge}"));
                *drawn.lock().unwrap() = (s.try_clone().ok(), Vec::new());
                session(&world, s, inbox.clone(), forge.clone(), drawn.clone());
                *drawn.lock().unwrap() = (None, Vec::new());
            }
            Err(_) => thread::sleep(Duration::from_secs(2)),
        }
    });
}

fn mc_id(choice: &str) -> String { if choice.contains(':') { choice.into() } else { format!("minecraft:{choice}") } }

struct Other { pos: [f64; 3], yaw: f32, label: String, key: String, name: String, desc: String, eid: i32, drawn: Option<String>, moved: bool }

fn other(e: &Entity, eid: i32) -> Other {
    Other { pos: e.pos, yaw: e.yaw, label: format!("{} ({})", e.name, e.origin), key: format!("{}:{}", e.origin, e.key),
            name: format!("{} player", e.origin), desc: e.desc.clone(), eid, drawn: None, moved: false }
}

fn session(world: &Arc<World>, s: TcpStream, inbox: Arc<Mutex<Receiver<Event>>>, forge: Arc<Forge>, drawn: Drawn) {
    let player: Arc<Mutex<Option<(u64, Instant)>>> = Arc::new(Mutex::new(None));
    let alive = Arc::new(Mutex::new(true));
    let (own_tx, own_rx): (Sender<(String, TBox, String)>, Receiver<(String, TBox, String)>) = channel();   // edits made in this Minecraft
    // writer: the ground and the other games' players, as Signet Forge says Minecraft draws them
    {
        let mut w = s.try_clone().unwrap();
        let (alive, world, drawn) = (alive.clone(), world.clone(), drawn.clone());
        thread::spawn(move || {
            let rx = inbox.lock().unwrap();
            let mut next_eid = 100;
            let mut others: HashMap<u64, Other> = HashMap::new();
            for e in world.snapshot() {     // a new proto session: everyone already in the world
                if e.kind == "player" && e.origin != NAME { others.insert(e.id, other(&e, next_eid)); next_eid += 1; }
            }
            let mut ground = world.ground();
            let mut painted: Option<HashMap<String, String>> = None;    // material -> block of the last paint
            let mut regions: Vec<TBox> = Vec::new();                    // edited by other games: repaint these
            let mut native: HashMap<String, String> = HashMap::new();   // edits made in this Minecraft: key -> its own block
            let mut changed_at: Option<Instant> = None;
            let mut last = Instant::now();
            while *alive.lock().unwrap() {
                // every event as soon as it comes (movements go out at once: no batching delay)
                let mut evs: Vec<Event> = rx.recv_timeout(Duration::from_millis(20)).into_iter().collect();
                while let Ok(ev) = rx.try_recv() { evs.push(ev); }
                let mut out = String::new();
                for ev in evs {
                    match ev {
                        Event::Terrain(t) => { ground.set_base(t); painted = None; regions.clear(); }
                        Event::Edit { key, b, .. } => {
                            if let Some(old) = ground.edit(&key, b.clone()) { regions.push(old); }
                            regions.push(b);
                        }
                        Event::Joined(e) if e.kind == "player" => {
                            crate::log(NAME, &format!("drawing #{} ({} from {}) in Minecraft", e.id, e.name, e.origin));
                            others.insert(e.id, other(&e, next_eid)); next_eid += 1;
                        }
                        Event::Moved { id, pos, yaw } => { if let Some(o) = others.get_mut(&id) { o.pos = pos; o.yaw = yaw; o.moved = true; } }
                        Event::Left { id } => {
                            if let Some(o) = others.remove(&id) {
                                drawn.lock().unwrap().1.retain(|e| *e != o.eid);
                                if o.drawn.is_some() { out += &format!("REMOVE {}\n", o.eid); }
                            }
                        }
                        _ => {}
                    }
                }
                while let Ok((key, b, block)) = own_rx.try_recv() { native.insert(key.clone(), block); ground.edit(&key, b); }   // already drawn by Minecraft itself
                // the edits from other games: only their regions, at once
                if ground.base.is_some() && painted.is_some() && !regions.is_empty() {
                    let blocks = materials(&ground, &forge);
                    for r in std::mem::take(&mut regions) { out += &patch(&ground, &r, &blocks, &native); }
                }
                // SF's answers (new choices, your approvals, previews) and the ground: checked every 0.1 s
                let check = last.elapsed() >= Duration::from_millis(100);
                if check {
                    last = Instant::now();
                    // the ground: paint at once; when SF's blocks change (after they settle for 0.4 s), repaint only the
                    // boxes of the materials that changed
                    if ground.base.is_some() {
                        let blocks = materials(&ground, &forge);
                        match &painted {
                            None => { if paint(&mut w, &ground, &blocks, &native).is_err() { break; } painted = Some(blocks); changed_at = None; regions.clear(); }
                            Some(p) if *p != blocks => {
                                let at = *changed_at.get_or_insert_with(Instant::now);
                                if at.elapsed() > Duration::from_millis(400) {
                                    let changed: Vec<&String> = blocks.iter().filter(|(m, b)| p.get(*m) != Some(*b)).map(|(m, _)| m).collect();
                                    for m in changed.iter().filter(|m| m.as_str() != AIR && p.contains_key(m.as_str())) {
                                        crate::log(NAME, &format!("'{m}' is now {} in Minecraft", blocks[*m]));
                                    }
                                    for (_, b) in keyed(&ground).filter(|(k, b)| changed.contains(&&b.material) && !native.contains_key(*k)) { out += &patch(&ground, b, &blocks, &native); }
                                    painted = Some(blocks); changed_at = None;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // the other games' players: drawn as SF says, moved the moment they move
                for o in others.values_mut() {
                    let (x, y, z) = (o.pos[0] + MC_SPAWN[0], o.pos[1] + MC_SPAWN[1], o.pos[2] + MC_SPAWN[2]);
                    if check || o.drawn.is_none() {
                        let kind = forge.ask(TARGET, "player", &o.key, &o.name, &o.desc).map(|a| mc_id(&a.choice)).unwrap_or(DEFAULT_PLAYER.into());
                        if o.drawn.as_deref() != Some(kind.as_str()) {
                            if o.drawn.is_some() { out += &format!("REMOVE {}\n", o.eid); }
                            out += &format!("SPAWN {} {kind} {x:.3} {y:.3} {z:.3} {:.1} {}\n", o.eid, o.yaw, o.label);
                            if o.drawn.is_none() { drawn.lock().unwrap().1.push(o.eid); }
                            o.drawn = Some(kind); o.moved = false;
                        }
                    }
                    if o.moved { out += &format!("MOVE {} {x:.3} {y:.3} {z:.3} {:.1}\n", o.eid, o.yaw); o.moved = false; }
                }
                if !out.is_empty() && w.write_all(out.as_bytes()).is_err() { break; }
            }
        });
    }
    // watchdog: no "MC" line for a while = the Minecraft player left
    {
        let (alive, player, world) = (alive.clone(), player.clone(), world.clone());
        thread::spawn(move || while *alive.lock().unwrap() {
            thread::sleep(Duration::from_millis(500));
            let mut p = player.lock().unwrap();
            if let Some((id, t)) = *p { if t.elapsed() > GONE { world.publish(NAME, Event::Left { id }); *p = None; } }
        });
    }
    for line in BufReader::new(s).lines() {
        let Ok(line) = line else { break };
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.first().copied() {
            Some("MC") if f.len() >= 5 => {
                let n = |i: usize| f[i].parse::<f64>().unwrap_or(0.0);
                let pos = [n(1) - MC_SPAWN[0], n(2) - MC_SPAWN[1], n(3) - MC_SPAWN[2]];
                let yaw = n(4) as f32;
                let mut p = player.lock().unwrap();
                match *p {
                    Some((id, _)) => { world.publish(NAME, Event::Moved { id, pos, yaw }); *p = Some((id, Instant::now())); }
                    None => {
                        let id = world.new_id();
                        world.publish(NAME, Event::Joined(Entity {
                            id, origin: NAME.into(), kind: "player".into(), key: "player".into(), name: "Minecraft player".into(),
                            desc: "a blocky Minecraft player character with a square head, a cyan shirt and blue trousers".into(), pos, yaw }));
                        *p = Some((id, Instant::now()));
                    }
                }
            }
            Some("MCEV") if f.len() == 6 && f[1] == "block" => {      // a block placed or broken: the shared ground changes
                let (Ok(x), Ok(y), Ok(z)) = (f[2].parse::<i32>(), f[3].parse::<i32>(), f[4].parse::<i32>()) else { continue };
                let (key, b) = block_edit(x, y, z, f[5]);
                crate::log(NAME, &format!("you {} {} at {x} {y} {z}: the shared world changes", if b.material == AIR { "broke" } else { "placed" },
                                          if b.material == AIR { "a block".into() } else { b.material.clone() }));
                let _ = own_tx.send((key.clone(), b.clone(), mc_id(f[5])));
                world.publish(NAME, Event::Edit { key, b, origin: NAME.into() });
            }
            Some("MCEV") if f.len() >= 2 => {
                if let Some((id, _)) = *player.lock().unwrap() {
                    world.publish(NAME, Event::Action { id, what: f[1].into(), arg: f.get(2..).map(|r| r.join(" ")).unwrap_or_default() });
                }
            }
            _ => {}
        }
    }
    *alive.lock().unwrap() = false;
    if let Some((id, _)) = player.lock().unwrap().take() { world.publish(NAME, Event::Left { id }); }
    crate::log(NAME, "bridge closed (the proto stopped); retrying");
}

/// material -> Minecraft block: Signet Forge's answer (approved, or the DM's), stone until it answers; air is air.
fn materials(g: &Ground, forge: &Forge) -> HashMap<String, String> {
    g.boxes().map(|b| {
        let block = if b.material == AIR { "minecraft:air".into() } else {
            forge.ask(TARGET, "material", &b.material, &b.material, "").map(|a| mc_id(&a.choice)).unwrap_or(DEFAULT_BLOCK.into()) };
        (b.material.clone(), block)
    }).collect()
}

/// A Minecraft block as an edit of the shared ground: the 1 m box it fills (universal = Minecraft - spawn) and its
/// name in plain words, which every game paints its own way.
fn block_edit(x: i32, y: i32, z: i32, block: &str) -> (String, TBox) {
    let min = [x as f64 - MC_SPAWN[0], y as f64 - MC_SPAWN[1], z as f64 - MC_SPAWN[2]];   // the block cell [x, x+1) - spawn
    let words = block.trim_start_matches("minecraft:").split('[').next().unwrap_or("").replace('_', " ");
    let material = if words.is_empty() || words == "air" || words == "cave air" || words == "void air" { AIR.into() } else { words };
    (format!("mc:{x},{y},{z}"), TBox { min, max: [min[0] + 1.0, min[1] + 1.0, min[2] + 1.0], material })
}

fn ranges(b: &TBox) -> Option<[(i32, i32); 3]> {
    let r = [0, 1, 2].map(|i| block_range(b.min[i], b.max[i], MC_SPAWN[i]));
    r.iter().all(|(a, z)| a <= z).then_some(r)
}

/// Every box of the ground with its edit key ("" for the Server's terrain), in painting order.
fn keyed(g: &Ground) -> impl Iterator<Item = (&str, &TBox)> {
    g.base.iter().flat_map(|t| t.boxes.iter().map(|b| ("", b))).chain(g.pieces.iter().map(|(k, b)| (k.as_str(), b)))
}

/// The block of a box: this Minecraft's own edits keep their block; everything else is Signet Forge's choice.
fn block_of<'a>(key: &str, b: &TBox, blocks: &'a HashMap<String, String>, native: &'a HashMap<String, String>) -> &'a str {
    native.get(key).or_else(|| blocks.get(&b.material)).map(String::as_str).unwrap_or(DEFAULT_BLOCK)
}

/// One region of the ground repainted: air first, then every box that touches it, clipped, in order (later wins).
fn patch(g: &Ground, region: &TBox, blocks: &HashMap<String, String>, native: &HashMap<String, String>) -> String {
    let Some(r) = ranges(region) else { return String::new() };
    let line = |c: [(i32, i32); 3], block: &str| format!("PATCH {} {} {} {} {} {} {block}\n", c[0].0, c[1].0, c[2].0, c[0].1, c[1].1, c[2].1);
    let mut out = line(r, "minecraft:air");
    for (key, b) in keyed(g) {
        let Some(br) = ranges(b) else { continue };
        let c = [0, 1, 2].map(|i| (br[i].0.max(r[i].0), br[i].1.min(r[i].1)));
        if c.iter().all(|(a, z)| a <= z) { out += &line(c, block_of(key, b, blocks, native)); }
    }
    out
}

/// Blocks whose centre lies in [min, max) on each axis (universal metres -> Minecraft blocks around the spawn).
fn block_range(min: f64, max: f64, spawn: f64) -> (i32, i32) {
    ((min + spawn - 0.5).ceil() as i32, (max + spawn - 0.5).ceil() as i32 - 1)
}

fn paint(w: &mut TcpStream, g: &Ground, blocks: &HashMap<String, String>, native: &HashMap<String, String>) -> std::io::Result<()> {
    let mut out = format!("WORLD {}\n", g.name().replace('\n', " "));
    let (mut n, mut count) = (0i64, 0);
    for (key, b) in keyed(g) {
        let Some(r) = ranges(b) else { continue };
        n += r.iter().map(|(a, z)| (z - a + 1) as i64).product::<i64>();
        count += 1;
        out += &format!("BOX {} {} {} {} {} {} {}\n", r[0].0, r[1].0, r[2].0, r[0].1, r[1].1, r[2].1, block_of(key, b, blocks, native));
    }
    out += "WORLDEND\n";
    w.write_all(out.as_bytes())?;
    let undecided = blocks.values().filter(|b| *b == DEFAULT_BLOCK).count();
    crate::log(NAME, &format!("painting '{}' in Minecraft: {count} boxes, {n} blocks, {} materials{}", g.name(), blocks.len(),
                              if undecided > 0 { format!(" ({undecided} still stone: waiting for Signet Forge)") } else { String::new() }));
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn ranges() {
        // the ground's top layer (universal y -1..0) is Minecraft y 63; x -10..10 around the spawn (0.5) is -10..9
        assert_eq!(super::block_range(-1.0, 0.0, 64.0), (63, 63));
        assert_eq!(super::block_range(-10.0, 10.0, 0.5), (-10, 9));
    }
    #[test]
    fn blocks_become_edits_and_back() {
        use super::*;
        let (key, b) = block_edit(3, 64, -2, "minecraft:lava");
        assert_eq!((key.as_str(), b.material.as_str()), ("mc:3,64,-2", "lava"));
        assert_eq!(ranges(&b), Some([(3, 3), (64, 64), (-2, -2)]));        // the same block, back in Minecraft
        assert_eq!(block_edit(0, 63, 0, "minecraft:air").1.material, AIR);
        assert_eq!(block_edit(0, 63, 0, "minecraft:oak_planks").1.material, "oak planks");
        // a region repainted: air, then the boxes under it in order, clipped
        let mut g = Ground::default();
        g.set_base(std::sync::Arc::new(crate::world::Terrain { name: "t".into(), boxes: vec![
            TBox { min: [-10.0, -1.0, -10.0], max: [10.0, 0.0, 10.0], material: "grass".into() }] }));
        g.edit("mc:0,63,0", block_edit(0, 63, 0, "minecraft:lava").1);
        let blocks: HashMap<String, String> = [("grass", "minecraft:grass_block"), ("lava", "minecraft:lava")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let p = patch(&g, &block_edit(0, 63, 0, "minecraft:lava").1, &blocks, &HashMap::new());
        assert_eq!(p, "PATCH 0 63 0 0 63 0 minecraft:air\nPATCH 0 63 0 0 63 0 minecraft:grass_block\nPATCH 0 63 0 0 63 0 minecraft:lava\n");
    }
}
