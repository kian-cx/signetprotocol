//! The Signet protocol between a Link and the Server: one universal event per line, fields separated by tabs.
//! It carries only universal entities (what, from which game, a plain description, where), never game data.
//!
//!   HELLO  signet/1  <link name>  [token]      link -> server, first line
//!   WELCOME <n>    |  DENIED <reason>           server -> link (n = this link's number: its ids are n << 32 | local id)
//!   J id origin kind key name desc x y z yaw    joined
//!   M id x y z yaw                              moved
//!   A id what arg                               action
//!   L id                                        left
//!   W BEGIN name  ·  B x1 y1 z1 x2 y2 z2 material  ·  W END
//!                                               the terrain (server -> link): the ground every game paints
//!   E key x1 y1 z1 x2 y2 z2 material origin    an edit of the ground (both ways): a keyed box that replaces the
//!                                               edit with the same key; material "air" carves
//!   PING                                        keep-alive (both ways, every 5 s)
use crate::world::{Entity, Event, TBox, Terrain};

pub const VERSION: &str = "signet/1";

fn clean(s: &str) -> String { s.replace(['\t', '\n', '\r'], " ") }

pub fn encode(ev: &Event) -> String {
    match ev {
        Event::Terrain(t) => {
            let mut out = format!("W\tBEGIN\t{}\n", clean(&t.name));
            for b in &t.boxes {
                out += &format!("B\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n", b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2], clean(&b.material));
            }
            out + "W\tEND"
        }
        Event::Joined(e) => format!("J\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.1}", e.id, clean(&e.origin), clean(&e.kind),
                                    clean(&e.key), clean(&e.name), clean(&e.desc), e.pos[0], e.pos[1], e.pos[2], e.yaw),
        Event::Moved { id, pos, yaw } => format!("M\t{id}\t{:.3}\t{:.3}\t{:.3}\t{yaw:.1}", pos[0], pos[1], pos[2]),
        Event::Action { id, what, arg } => format!("A\t{id}\t{}\t{}", clean(what), clean(arg)),
        Event::Left { id } => format!("L\t{id}"),
        Event::Edit { key, b, origin } => format!("E\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", clean(key), b.min[0], b.min[1], b.min[2],
                                                  b.max[0], b.max[1], b.max[2], clean(&b.material), clean(origin)),
    }
}

pub fn decode(line: &str) -> Option<Event> {
    let f: Vec<&str> = line.split('\t').collect();
    if f[0] == "E" && f.len() == 10 {
        let v: Vec<f64> = f[2..8].iter().filter_map(|x| x.parse().ok()).collect();
        if v.len() != 6 { return None; }
        return Some(Event::Edit { key: f[1].into(), origin: f[9].into(), b: TBox { min: [v[0].min(v[3]), v[1].min(v[4]), v[2].min(v[5])],
                                  max: [v[0].max(v[3]), v[1].max(v[4]), v[2].max(v[5])], material: f[8].into() } });
    }
    let id = f.get(1)?.parse::<u64>().ok()?;
    let num = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok());
    Some(match *f.first()? {
        "J" if f.len() == 11 => Event::Joined(Entity {
            id, origin: f[2].into(), kind: f[3].into(), key: f[4].into(), name: f[5].into(), desc: f[6].into(),
            pos: [num(7)?, num(8)?, num(9)?], yaw: num(10)? as f32 }),
        "M" if f.len() == 6 => Event::Moved { id, pos: [num(2)?, num(3)?, num(4)?], yaw: num(5)? as f32 },
        "A" if f.len() == 4 => Event::Action { id, what: f[2].into(), arg: f[3].into() },
        "L" if f.len() == 2 => Event::Left { id },
        _ => return None,
    })
}

/// Reassembles a terrain from its W/B lines. feed(): None = not a terrain line; Some(None) = taken, not finished.
#[derive(Default)]
pub struct TerrainReader { cur: Option<Terrain> }

impl TerrainReader {
    pub fn feed(&mut self, line: &str) -> Option<Option<Terrain>> {
        let f: Vec<&str> = line.split('\t').collect();
        match (f[0], f.get(1).copied()) {
            ("W", Some("BEGIN")) => { self.cur = Some(Terrain { name: f.get(2).unwrap_or(&"").to_string(), boxes: vec![] }); Some(None) }
            ("W", Some("END")) => Some(self.cur.take()),
            ("B", _) if f.len() == 8 => {
                let v: Vec<f64> = f[1..7].iter().filter_map(|x| x.parse().ok()).collect();
                if let (Some(t), 6) = (&mut self.cur, v.len()) {
                    t.boxes.push(TBox { min: [v[0], v[1], v[2]], max: [v[3], v[4], v[5]], material: f[7].into() });
                }
                Some(None)
            }
            _ => None,
        }
    }
}

/// The same event with its id changed (a Link's local ids <-> global ids).
pub fn with_id(ev: Event, id: u64) -> Event {
    match ev {
        Event::Joined(mut e) => { e.id = id; Event::Joined(e) }
        Event::Moved { pos, yaw, .. } => Event::Moved { id, pos, yaw },
        Event::Action { what, arg, .. } => Event::Action { id, what, arg },
        Event::Left { .. } => Event::Left { id },
        t @ (Event::Terrain(_) | Event::Edit { .. }) => t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let e = Event::Joined(Entity { id: (3 << 32) | 7, origin: "minecraft".into(), kind: "player".into(), key: "player".into(),
                                       name: "Steve".into(), desc: "a blocky\tplayer".into(), pos: [1.5, -2.0, 3.25], yaw: 90.0 });
        let Some(Event::Joined(d)) = decode(&encode(&e)) else { panic!() };
        assert_eq!((d.id, d.desc.as_str(), d.pos, d.yaw), ((3 << 32) | 7, "a blocky player", [1.5, -2.0, 3.25], 90.0));
        for ev in [Event::Moved { id: 9, pos: [1.0, 2.0, 3.0], yaw: -45.0 }, Event::Action { id: 9, what: "chat".into(), arg: "hola mundo".into() }, Event::Left { id: 9 }] {
            assert_eq!(encode(&decode(&encode(&ev)).unwrap()), encode(&ev));
        }
        assert!(decode("J\t1\tx").is_none() && decode("garbage").is_none());
        let ed = Event::Edit { key: "mc:1,63,2".into(), origin: "minecraft".into(), b: TBox { min: [0.5, -1.0, 1.5], max: [1.5, 0.0, 2.5], material: "lava".into() } };
        let Some(Event::Edit { key, b, origin }) = decode(&encode(&ed)) else { panic!() };
        assert_eq!((key.as_str(), b.min, b.max, b.material.as_str(), origin.as_str()), ("mc:1,63,2", [0.5, -1.0, 1.5], [1.5, 0.0, 2.5], "lava", "minecraft"));
        let t = Terrain { name: "plaza".into(), boxes: vec![TBox { min: [-5.0, -1.0, -5.0], max: [5.0, 0.0, 5.0], material: "green grass".into() }] };
        let mut r = TerrainReader::default();
        let mut got = None;
        for l in encode(&Event::Terrain(std::sync::Arc::new(t))).lines() { if let Some(Some(t)) = r.feed(l) { got = Some(t); } }
        let g = got.unwrap();
        assert_eq!((g.name.as_str(), g.boxes.len(), g.boxes[0].material.as_str(), g.boxes[0].max), ("plaza", 1, "green grass", [5.0, 0.0, 5.0]));
    }
}
