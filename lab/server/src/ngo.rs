//! Unity Netcode for GameObjects (NGO) wire format: batches of messages (magic 0x1160, xxh64 of the body) and the
//! BytePacker variable-length integers. Same layout our LC bot reads and writes (~/lc-analisis/bot/lcbot.py).

pub const CONNECTION_APPROVED: u32 = 0;
pub const CONNECTION_REQUEST: u32 = 1;
pub const CLIENT_RPC: u32 = 7;
pub const PROXY: u32 = 15;
pub const RPC: u32 = 16;
pub const SCENE_EVENT: u32 = 17;
pub const SERVER_RPC: u32 = 19;
pub const TIME_SYNC: u32 = 20;

pub fn name(t: u32) -> &'static str {
    match t {
        0 => "ConnectionApproved", 1 => "ConnectionRequest", 4 => "ChangeOwnership", 5 => "ClientConnected",
        6 => "ClientDisconnected", 7 => "ClientRpc", 8 => "CreateObject", 9 => "DestroyObject", 10 => "DisconnectReason",
        11 => "Named", 13 => "NetworkVariableDelta", 14 => "ParentSync", 15 => "Proxy", 16 => "Rpc", 17 => "SceneEvent",
        19 => "ServerRpc", 20 => "TimeSync", _ => "other",
    }
}

// ---- BytePacker: the low bits of the first byte say how many bytes follow ----
fn packed(value: u64, bits: u32, full_n: u8, full_w: usize) -> Vec<u8> {
    let v = (value as u128) << bits;
    let n = ((128 - v.leading_zeros() + 7) / 8).max(1) as usize;
    if n > (1 << bits) - 1 || n >= full_n as usize {
        let mut out = vec![full_n];
        out.extend_from_slice(&value.to_le_bytes()[..full_w]);
        return out;
    }
    let v = v | n as u128;
    v.to_le_bytes()[..n].to_vec()
}
pub fn bp_u32(v: u32) -> Vec<u8> { if v > 536_870_911 { let mut o = vec![5]; o.extend_from_slice(&v.to_le_bytes()); o } else { packed(v as u64, 3, 5, 4) } }
pub fn bp_u64(v: u64) -> Vec<u8> { packed(v, 4, 9, 8) }

pub fn read_u32(b: &[u8], i: usize) -> Option<(u32, usize)> {
    let n = (*b.get(i)? & 7) as usize;
    if n == 5 { return Some((u32::from_le_bytes(b.get(i + 1..i + 5)?.try_into().ok()?), i + 5)); }
    let mut v = 0u64;
    for (k, x) in b.get(i..i + n)?.iter().enumerate() { v |= (*x as u64) << (8 * k); }
    Some(((v >> 3) as u32, i + n))
}
pub fn read_u64(b: &[u8], i: usize) -> Option<(u64, usize)> {
    let n = (*b.get(i)? & 15) as usize;
    if n == 9 { return Some((u64::from_le_bytes(b.get(i + 1..i + 9)?.try_into().ok()?), i + 9)); }
    let mut v = 0u128;
    for (k, x) in b.get(i..i + n)?.iter().enumerate() { v |= (*x as u128) << (8 * k); }
    Some(((v >> 4) as u64, i + n))
}
/// bit-packed signed short (zig-zag), as NGO writes rotations
pub fn read_i16(b: &[u8], i: usize) -> Option<(i32, usize)> {
    let n = (*b.get(i)? & 3) as usize;
    let (v, j) = if n == 3 { (u16::from_le_bytes(b.get(i + 1..i + 3)?.try_into().ok()?) as u32, i + 3) } else {
        let mut v = 0u32;
        for (k, x) in b.get(i..i + n)?.iter().enumerate() { v |= (*x as u32) << (8 * k); }
        (v >> 2, i + n)
    };
    Some((((v >> 1) as i32) ^ -((v & 1) as i32), j))
}

// ---- batches ----
pub fn batch(msgs: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (t, m) in msgs { body.extend(bp_u32(*t)); body.extend(bp_u32(m.len() as u32)); body.extend_from_slice(m); }
    let size = 16 + body.len() as i32;
    let mut out = size.to_le_bytes().to_vec();
    out.extend_from_slice(&0x1160u16.to_le_bytes());
    out.extend_from_slice(&(msgs.len() as u16).to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&xxh64(&body, 0).to_le_bytes());
    out.extend(body);
    out
}

/// Split a reliable stream into messages; returns the bytes consumed.
pub fn parse_stream(stream: &[u8], mut on_msg: impl FnMut(u32, &[u8])) -> usize {
    let mut i = 0;
    while i + 4 <= stream.len() {
        let ln = i32::from_le_bytes(stream[i..i + 4].try_into().unwrap()) as usize;
        if i + 4 + ln > stream.len() { break; }
        let b = &stream[i + 4..i + 4 + ln];
        i += 4 + ln;
        if b.len() < 16 || u16::from_le_bytes([b[0], b[1]]) != 0x1160 { continue; }
        let count = u16::from_le_bytes([b[2], b[3]]);
        let mut j = 16;
        for _ in 0..count {
            let Some((t, k)) = read_u32(b, j) else { break };
            let Some((n, k)) = read_u32(b, k) else { break };
            let Some(body) = b.get(k..k + n as usize) else { break };
            on_msg(t, body);
            j = k + n as usize;
        }
    }
    i
}

// ---- xxHash64 (the batch header carries it) ----
const P1: u64 = 11400714785074694791; const P2: u64 = 14029467366897019727; const P3: u64 = 1609587929392839161;
const P4: u64 = 9650029242287828579; const P5: u64 = 2870177450012600261;
fn round(acc: u64, v: u64) -> u64 { acc.wrapping_add(v.wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1) }
fn merge(acc: u64, v: u64) -> u64 { (acc ^ round(0, v)).wrapping_mul(P1).wrapping_add(P4) }
pub fn xxh64(d: &[u8], seed: u64) -> u64 {
    let rd8 = |i: usize| u64::from_le_bytes(d[i..i + 8].try_into().unwrap());
    let rd4 = |i: usize| u32::from_le_bytes(d[i..i + 4].try_into().unwrap()) as u64;
    let n = d.len(); let mut i = 0;
    let mut h = if n >= 32 {
        let (mut v1, mut v2, mut v3, mut v4) = (seed.wrapping_add(P1).wrapping_add(P2), seed.wrapping_add(P2), seed, seed.wrapping_sub(P1));
        while i + 32 <= n { v1 = round(v1, rd8(i)); v2 = round(v2, rd8(i + 8)); v3 = round(v3, rd8(i + 16)); v4 = round(v4, rd8(i + 24)); i += 32; }
        let h = v1.rotate_left(1).wrapping_add(v2.rotate_left(7)).wrapping_add(v3.rotate_left(12)).wrapping_add(v4.rotate_left(18));
        merge(merge(merge(merge(h, v1), v2), v3), v4)
    } else { seed.wrapping_add(P5) };
    h = h.wrapping_add(n as u64);
    while i + 8 <= n { h = (h ^ round(0, rd8(i))).rotate_left(27).wrapping_mul(P1).wrapping_add(P4); i += 8; }
    if i + 4 <= n { h = (h ^ rd4(i).wrapping_mul(P1)).rotate_left(23).wrapping_mul(P2).wrapping_add(P3); i += 4; }
    while i < n { h = (h ^ (d[i] as u64).wrapping_mul(P5)).rotate_left(11).wrapping_mul(P1); i += 1; }
    h ^= h >> 33; h = h.wrapping_mul(P2); h ^= h >> 29; h = h.wrapping_mul(P3); h ^= h >> 32;
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xxh64_known() {
        assert_eq!(xxh64(b"", 0), 0xEF46DB3751D8E999);
        assert_eq!(xxh64(b"abc", 0), 0x44BC2CF5AD770999);
        assert_eq!(xxh64(b"Nobody inspects the spammish repetition", 0), 0xFBCEA83C8A378BF1);
    }
    #[test]
    fn packing_roundtrip() {
        for v in [0u32, 1, 31, 32, 1000, 1814, 536_870_911, 890_924_887, u32::MAX] { assert_eq!(read_u32(&bp_u32(v), 0).unwrap().0, v); }
        for v in [0u64, 1, 15, 16, 70000, u64::MAX] { assert_eq!(read_u64(&bp_u64(v), 0).unwrap().0, v); }
    }
}
