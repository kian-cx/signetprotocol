//! One door per game protocol. A door speaks its game's native protocol on one side and universal events on the
//! other (crate::world). Today: Minecraft (through our proto's bridge) and Lethal Company (host emulation over UTP).
pub mod gmod;
pub mod lc;
pub mod mc;

/// Shared spawn point mapping (same numbers our bridge already uses):
/// Minecraft spawn (0.5, 64, 0.5) = the universal origin; Lethal Company: a point on the ship floor (0.5, 0, -6.5),
/// ship-local, with X mirrored (Unity is left-handed). Yaw is the same in both after the mirror.
pub const MC_SPAWN: [f64; 3] = [0.5, 64.0, 0.5];
pub const LC_SHIP_SPOT: [f64; 3] = [0.5, 0.0, -6.5];
pub const LC_SHIP_IN_WORLD: [f64; 3] = [1.271463394165039, 0.2784385681152344, -7.5];
