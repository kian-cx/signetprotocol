//! The shared world. It only knows universal entities: what they are, which game they come from, how they look
//! (a plain description) and where they are. It never translates: each door draws the others with its own catalog.
//!
//! Frame: metres, Y up, origin at the shared spawn point (Minecraft blocks are metres: universal = MC - spawn).
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: u64,
    pub origin: String,       // the game it comes from: "minecraft", "lethalcompany", ...
    pub kind: String,         // player | creature | item | structure
    pub key: String,          // its id in its own game (catalog key)
    pub name: String,
    pub desc: String,         // plain description, for the receiving game's DM
    pub pos: [f64; 3],
    pub yaw: f32,
}

/// The ground of the shared world: solid boxes in universal metres, each with a material in plain words
/// ("grass", "polished concrete floor"). Every game paints it its own way (Minecraft: blocks chosen per material).
#[derive(Clone, Debug)]
pub struct TBox { pub min: [f64; 3], pub max: [f64; 3], pub material: String }

#[derive(Clone, Debug)]
pub struct Terrain { pub name: String, pub boxes: Vec<TBox> }

/// A world file (ours, plain text, universal metres; the shared spawn is the origin and the ground's top is y 0):
///   name <text>
///   box x1 y1 z1 x2 y2 z2 <material in words>
pub fn load_terrain(path: &str) -> Result<Terrain, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut t = Terrain { name: path.rsplit('/').next().unwrap_or(path).trim_end_matches(".txt").into(), boxes: vec![] };
    for (n, l) in text.lines().enumerate() {
        let l = l.split('#').next().unwrap_or("").trim();
        let f: Vec<&str> = l.split_whitespace().collect();
        match f.first().copied() {
            Some("name") => t.name = f[1..].join(" "),
            Some("box") if f.len() >= 8 => {
                let v: Vec<f64> = f[1..7].iter().filter_map(|x| x.parse().ok()).collect();
                if v.len() != 6 { return Err(format!("{path}:{}: box needs 6 numbers", n + 1)); }
                t.boxes.push(TBox { min: [v[0].min(v[3]), v[1].min(v[4]), v[2].min(v[5])],
                                    max: [v[0].max(v[3]), v[1].max(v[4]), v[2].max(v[5])], material: f[7..].join(" ") });
            }
            None => {}
            _ => return Err(format!("{path}:{}: expected 'name ...' or 'box x1 y1 z1 x2 y2 z2 material'", n + 1)),
        }
    }
    Ok(t)
}

/// The ground as it is now: the Server's terrain plus the edits made from the games (a block placed in Minecraft, a
/// box moved with the physics gun in Garry's Mod). Each edit is a keyed box; a new edit with the same key replaces it
/// and goes last (later boxes win); material "air" carves. A new terrain clears the edits.
#[derive(Clone, Debug, Default)]
pub struct Ground { pub base: Option<Arc<Terrain>>, pub pieces: Vec<(String, TBox)> }

pub const AIR: &str = "air";

impl Ground {
    pub fn set_base(&mut self, t: Arc<Terrain>) { self.base = Some(t); self.pieces.clear(); }
    /// Apply an edit; returns the box it replaced (the region to repaint besides the new one).
    pub fn edit(&mut self, key: &str, b: TBox) -> Option<TBox> {
        let old = self.pieces.iter().position(|(k, _)| k == key).map(|i| self.pieces.remove(i).1);
        self.pieces.push((key.to_string(), b));
        old
    }
    pub fn boxes(&self) -> impl Iterator<Item = &TBox> {
        self.base.iter().flat_map(|t| t.boxes.iter()).chain(self.pieces.iter().map(|(_, b)| b))
    }
    pub fn name(&self) -> String { self.base.as_ref().map(|t| t.name.clone()).unwrap_or_default() }
}

#[derive(Clone, Debug)]
pub enum Event {
    Terrain(Arc<Terrain>),
    Edit { key: String, b: TBox, origin: String },
    Joined(Entity),
    Moved { id: u64, pos: [f64; 3], yaw: f32 },
    Action { id: u64, what: String, arg: String },
    Left { id: u64 },
}

impl Event {
    pub fn id(&self) -> u64 {
        match self { Event::Terrain(_) | Event::Edit { .. } => 0, Event::Joined(e) => e.id, Event::Moved { id, .. } | Event::Action { id, .. } | Event::Left { id } => *id }
    }
}

struct Inner {
    next: u64,
    ents: BTreeMap<u64, Entity>,
    ground: Ground,
    doors: Vec<(String, Sender<Event>)>,
}

pub struct World {
    inner: Mutex<Inner>,
    pub started: Instant,
}

impl World {
    pub fn new() -> Arc<World> {
        Arc::new(World { inner: Mutex::new(Inner { next: 1, ents: BTreeMap::new(), ground: Ground::default(), doors: Vec::new() }), started: Instant::now() })
    }

    /// A door subscribes to everything the other doors publish. It first receives what already exists.
    pub fn subscribe(&self, door: &str) -> Receiver<Event> {
        let (tx, rx) = channel();
        let mut w = self.inner.lock().unwrap();
        if let Some(t) = &w.ground.base { let _ = tx.send(Event::Terrain(t.clone())); }   // the ground first, with its edits
        for (key, b) in &w.ground.pieces { let _ = tx.send(Event::Edit { key: key.clone(), b: b.clone(), origin: String::new() }); }
        for e in w.ents.values() { let _ = tx.send(Event::Joined(e.clone())); }
        w.doors.push((door.to_string(), tx));
        rx
    }

    pub fn unsubscribe(&self, door: &str) { self.inner.lock().unwrap().doors.retain(|(n, _)| n != door); }

    pub fn new_id(&self) -> u64 {
        let mut w = self.inner.lock().unwrap();
        w.next += 1;
        w.next - 1
    }

    /// Update the world and forward the event to every other door.
    pub fn publish(&self, from: &str, ev: Event) {
        let mut w = self.inner.lock().unwrap();
        match &ev {
            Event::Terrain(t) => { w.ground.set_base(t.clone()); }
            Event::Edit { key, b, .. } => { w.ground.edit(key, b.clone()); }
            Event::Joined(e) => { w.ents.insert(e.id, e.clone()); }
            Event::Moved { id, pos, yaw } => { if let Some(e) = w.ents.get_mut(id) { e.pos = *pos; e.yaw = *yaw; } }
            Event::Left { id } => { w.ents.remove(id); }
            Event::Action { .. } => {}
        }
        if matches!(ev, Event::Terrain(_) | Event::Joined(_) | Event::Left { .. } | Event::Action { .. } | Event::Edit { .. }) {
            crate::log("world", &describe(&ev, &w.ents));
        }
        w.doors.retain(|(name, tx)| name == from || tx.send(ev.clone()).is_ok());
    }

    pub fn ground(&self) -> Ground { self.inner.lock().unwrap().ground.clone() }

    pub fn terrain(&self) -> Option<Arc<Terrain>> { self.inner.lock().unwrap().ground.base.clone() }

    pub fn snapshot(&self) -> Vec<Entity> { self.inner.lock().unwrap().ents.values().cloned().collect() }
}

fn describe(ev: &Event, ents: &BTreeMap<u64, Entity>) -> String {
    match ev {
        Event::Terrain(t) => format!("terrain '{}': {} boxes", t.name, t.boxes.len()),
        Event::Joined(e) => format!("#{} joined: {} {} from {} ({})", e.id, e.kind, e.name, e.origin, e.desc),
        Event::Left { id } => format!("#{id} left"),
        Event::Action { id, what, arg } => {
            let who = ents.get(id).map(|e| format!("{} from {}", e.name, e.origin)).unwrap_or_default();
            format!("#{id} {who}: {what} {arg}")
        }
        Event::Moved { id, .. } => format!("#{id} moved"),
        Event::Edit { key, b, origin } => format!("edit {key}{}: {} at {:.1},{:.1},{:.1}..{:.1},{:.1},{:.1}",
            if origin.is_empty() { String::new() } else { format!(" from {origin}") }, b.material, b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]),
    }
}
