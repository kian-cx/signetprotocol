//! The Link's line to Signet Forge (one SF per PC, for every door of this PC). The doors never decide how to draw
//! something from another game: they ask SF, draw a neutral default meanwhile, and redraw when SF answers or when
//! you approve or change a choice in SF (it pushes the new answer).
//!
//!   link -> SF   HELLO  signet-link  <link name>
//!                ASK    target  kind  key  name  description     (target = the receiving game's catalog, e.g.
//!                                                                 minecraft_26_3; kind = player|creature|item|material)
//!   SF -> link   USE    target  kind  key  choice  source  p      (source = approved|dm; choice = an id of the target
//!                                                                 game's catalog, e.g. minecraft:villager, alyx)
//!
//! Plain text, one tab-separated message per line, 127.0.0.1 only. If SF is not running the doors keep their defaults
//! and every question is asked again when it connects.
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const ADDR: &str = "127.0.0.1:7796";

#[derive(Clone, Debug, PartialEq)]
pub struct Answer { pub choice: String, pub source: String, pub p: f32 }

pub struct Forge {
    asked: Mutex<HashMap<String, String>>,      // key -> the ASK line (re-sent when SF reconnects)
    answers: Mutex<HashMap<String, Answer>>,
    conn: Mutex<Option<TcpStream>>,
    version: AtomicU64,                          // bumps on every new or changed answer: doors redraw
    enabled: bool,
}

fn clean(s: &str) -> String { s.replace(['\t', '\n', '\r'], " ") }

pub fn key(target: &str, kind: &str, key: &str) -> String { format!("{target}|{kind}|{key}") }

impl Forge {
    /// No SF: doors use their defaults (signet link --no-forge).
    pub fn off() -> Arc<Forge> {
        Arc::new(Forge { asked: Mutex::default(), answers: Mutex::default(), conn: Mutex::default(), version: AtomicU64::new(0), enabled: false })
    }

    pub fn start(addr: String, link_name: String) -> Arc<Forge> {
        let f = Arc::new(Forge { asked: Mutex::default(), answers: Mutex::default(), conn: Mutex::default(), version: AtomicU64::new(0), enabled: true });
        let me = f.clone();
        thread::spawn(move || {
            let mut warned = false;
            loop {
                match TcpStream::connect(&addr) {
                    Ok(s) => {
                        warned = false;
                        crate::log("forge", &format!("connected to Signet Forge at {addr}"));
                        me.session(s, &link_name);
                        crate::log("forge", "Signet Forge closed: the doors keep the last answers; reconnecting");
                    }
                    Err(_) if !warned => {
                        crate::log("forge", &format!("Signet Forge is not running at {addr}: neutral defaults until it starts \
                                                      (cd ~/clm-bench && HF_HUB_OFFLINE=1 clef/.venv-unsloth/bin/python signet_forge.py)"));
                        warned = true;
                    }
                    Err(_) => {}
                }
                thread::sleep(Duration::from_secs(2));
            }
        });
        f
    }

    fn session(&self, s: TcpStream, link_name: &str) {
        let _ = s.set_nodelay(true);
        let Ok(mut w) = s.try_clone() else { return };
        let hello = format!("HELLO\tsignet-link\t{}\n", clean(link_name));
        let again: String = self.asked.lock().unwrap().values().map(|l| format!("{l}\n")).collect();
        if w.write_all(format!("{hello}{again}").as_bytes()).is_err() { return; }
        *self.conn.lock().unwrap() = Some(w);
        for line in BufReader::new(s).lines() {
            let Ok(line) = line else { break };
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() == 7 && f[0] == "USE" {
                let a = Answer { choice: f[4].into(), source: f[5].into(), p: f[6].parse().unwrap_or(0.0) };
                let k = key(f[1], f[2], f[3]);
                let mut ans = self.answers.lock().unwrap();
                if ans.get(&k) != Some(&a) {
                    crate::log("forge", &format!("{} {} '{}' -> {} ({}{})", f[1], f[2], f[3], a.choice, a.source,
                                                 if a.source == "dm" { format!(" {:.0}%", a.p * 100.0) } else { String::new() }));
                    ans.insert(k, a);
                    self.version.fetch_add(1, SeqCst);
                }
            }
        }
        *self.conn.lock().unwrap() = None;
    }

    /// The current answer for something a door must draw (None until SF answers). The first call sends the question.
    pub fn ask(&self, target: &str, kind: &str, k: &str, name: &str, desc: &str) -> Option<Answer> {
        let id = key(target, kind, k);
        if let Some(a) = self.answers.lock().unwrap().get(&id) { return Some(a.clone()); }
        if !self.enabled { return None; }
        let mut asked = self.asked.lock().unwrap();
        if !asked.contains_key(&id) {
            let line = format!("ASK\t{}\t{}\t{}\t{}\t{}", clean(target), clean(kind), clean(k), clean(name), clean(desc));
            if let Some(w) = self.conn.lock().unwrap().as_mut() { let _ = w.write_all(format!("{line}\n").as_bytes()); }
            asked.insert(id, line);
        }
        None
    }

    pub fn version(&self) -> u64 { self.version.load(SeqCst) }
}

/// Doors that redraw only what changed remember the answers they used.
#[derive(Default)]
pub struct Seen(HashSet<(String, String)>);
impl Seen {
    pub fn changed(&mut self, id: &str, choice: &str) -> bool { self.0.insert((id.to_string(), choice.to_string())) }
}
