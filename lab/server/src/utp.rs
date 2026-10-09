//! Unity Transport (UTP) over UDP, host side: connection tokens, ping/pong and the reliable sequenced pipeline (3)
//! with acks and resends; the unreliable sequenced pipeline (1) for TimeSync. Mirror of what our LC bot does as a client.
use std::collections::{BTreeMap, HashSet};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

pub const CONN_REQ: u8 = 0; pub const CONN_ACCEPT: u8 = 2; pub const DISCONNECT: u8 = 3; pub const DATA: u8 = 4;
pub const PING: u8 = 5; pub const PONG: u8 = 6;
const RELIABLE: u8 = 3; const UNRELIABLE_SEQ: u8 = 1;

pub struct Conn {
    pub addr: SocketAddr,
    pub client_token: [u8; 8],  // the client puts it in its request; we put it in everything we send
    pub host_token: [u8; 8],    // ours: the client puts it in everything it sends
    send_seq: u16,
    unacked: BTreeMap<u16, (Vec<u8>, Instant, Instant)>,
    got: HashSet<u16>,
    highest: Option<u16>,
    next_deliver: u16,
    pending: BTreeMap<u16, Vec<u8>>,
    pub stream: Vec<u8>,        // in-order reliable bytes from the client, not yet parsed
    useq: u16,
    pub rtt: Duration,
    pub last_rx: Instant,
    pub resends: u64,
}

fn hdr(t: u8, flags: u8, token: &[u8; 8]) -> Vec<u8> { let mut v = vec![t, flags]; v.extend_from_slice(token); v }

impl Conn {
    pub fn accept(sock: &UdpSocket, addr: SocketAddr, client_token: [u8; 8]) -> Conn {
        let mut host_token = [0u8; 8];
        let seed = Instant::now().elapsed().as_nanos() as u64 ^ (std::process::id() as u64) << 20 ^ u64::from_le_bytes(client_token);
        host_token.copy_from_slice(&crate::ngo::xxh64(&seed.to_le_bytes(), 7).to_le_bytes());
        let c = Conn { addr, client_token, host_token, send_seq: 0, unacked: BTreeMap::new(), got: HashSet::new(), highest: None,
                       next_deliver: 0, pending: BTreeMap::new(), stream: Vec::new(), useq: 0, rtt: Duration::from_millis(100),
                       last_rx: Instant::now(), resends: 0 };
        c.send_accept(sock);
        c
    }

    pub fn send_accept(&self, sock: &UdpSocket) {
        let mut p = hdr(CONN_ACCEPT, 1, &self.client_token);
        p.extend_from_slice(&self.host_token);
        let _ = sock.send_to(&p, self.addr);
    }

    pub fn pong(&self, sock: &UdpSocket) { let _ = sock.send_to(&hdr(PONG, 0, &self.client_token), self.addr); }
    pub fn ping(&self, sock: &UdpSocket) { let _ = sock.send_to(&hdr(PING, 0, &self.client_token), self.addr); }
    pub fn disconnect(&self, sock: &UdpSocket) { let _ = sock.send_to(&hdr(DISCONNECT, 0, &self.client_token), self.addr); }

    fn ack_fields(&self) -> (u16, u64) {
        let Some(h) = self.highest else { return (0xFFFF, 0) };
        let mut mask = 0u64;
        for i in 0..64u16 { if self.got.contains(&h.wrapping_sub(i)) { mask |= 1 << i; } }
        (h, mask)
    }

    fn reliable_packet(&self, typ: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
        let (ack, mask) = self.ack_fields();
        let mut p = hdr(DATA, 2, &self.client_token);
        p.push(RELIABLE);
        for v in [typ, 0, seq, ack] { p.extend_from_slice(&v.to_le_bytes()); }
        p.extend_from_slice(&mask.to_le_bytes());
        p.extend_from_slice(payload);
        p
    }

    /// Queue bytes of the reliable stream (one UTP packet each) and send them now.
    pub fn send_reliable(&mut self, sock: &UdpSocket, payload: Vec<u8>) {
        let seq = self.send_seq; self.send_seq = self.send_seq.wrapping_add(1);
        let _ = sock.send_to(&self.reliable_packet(0, seq, &payload), self.addr);
        let now = Instant::now();
        self.unacked.insert(seq, (payload, now, now));
    }

    pub fn in_flight(&self) -> usize { self.unacked.len() }

    pub fn resend(&mut self, sock: &UdpSocket) {
        let rto = (self.rtt * 3 / 2).max(Duration::from_millis(250));
        let now = Instant::now();
        let due: Vec<u16> = self.unacked.iter().filter(|(_, v)| now - v.1 > rto).map(|(k, _)| *k).collect();
        for seq in due {
            let payload = self.unacked[&seq].0.clone();
            let _ = sock.send_to(&self.reliable_packet(0, seq, &payload), self.addr);
            self.unacked.get_mut(&seq).unwrap().1 = now; self.resends += 1;
        }
    }

    pub fn send_unreliable(&mut self, sock: &UdpSocket, payload: &[u8]) {
        let mut p = hdr(DATA, 2, &self.client_token);
        p.push(UNRELIABLE_SEQ);
        p.extend_from_slice(&self.useq.to_le_bytes()); self.useq = self.useq.wrapping_add(1);
        p.extend_from_slice(payload);
        let _ = sock.send_to(&p, self.addr);
    }

    /// A DATA packet from the client; returns true when new in-order bytes reached `stream`.
    pub fn on_data(&mut self, sock: &UdpSocket, p: &[u8]) -> bool {
        self.last_rx = Instant::now();
        if p.len() < 27 || p[10] != RELIABLE { return false; }
        let rd = |i: usize| u16::from_le_bytes([p[i], p[i + 1]]);
        let (typ, seq, ack) = (rd(11), rd(15), rd(17));
        let mask = u64::from_le_bytes(p[19..27].try_into().unwrap());
        for i in 0..64u16 {
            if mask >> i & 1 == 1 {
                if let Some((_, _, first)) = self.unacked.remove(&ack.wrapping_sub(i)) { if i == 0 { self.rtt = first.elapsed(); } }
            }
        }
        if typ != 0 { return false; }
        if self.got.insert(seq) {
            self.pending.insert(seq, p[27..].to_vec());
            if self.highest.map_or(true, |h| seq.wrapping_sub(h) < 0x8000) { self.highest = Some(seq); }
        }
        let (ack, mask) = self.ack_fields();          // acknowledge (typ 1 = ack only)
        let mut a = hdr(DATA, 2, &self.client_token); a.push(RELIABLE);
        for v in [1u16, 0, 0, ack] { a.extend_from_slice(&v.to_le_bytes()); }
        a.extend_from_slice(&mask.to_le_bytes()); a.push(0);
        let _ = sock.send_to(&a, self.addr);
        let mut fresh = false;
        while let Some(d) = self.pending.remove(&self.next_deliver) {
            self.stream.extend(d); self.next_deliver = self.next_deliver.wrapping_add(1); fresh = true;
        }
        fresh
    }
}
