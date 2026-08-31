//! Alpha instruction encoders used by tests and future compiler consumers.

/// Concatenate instruction words using Alpha's little-endian byte order.
pub fn assemble(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

/// Encode an operate-format instruction with a register source.
pub const fn operate_register(op: u32, function: u32, ra: u32, rb: u32, rc: u32) -> u32 {
    ((op & 0x3f) << 26)
        | ((ra & 0x1f) << 21)
        | ((rb & 0x1f) << 16)
        | ((function & 0x7f) << 5)
        | (rc & 0x1f)
}

/// Encode an operate-format instruction with an unsigned eight-bit literal.
pub const fn operate_literal(op: u32, function: u32, ra: u32, literal: u32, rc: u32) -> u32 {
    ((op & 0x3f) << 26)
        | ((ra & 0x1f) << 21)
        | ((literal & 0xff) << 13)
        | (1 << 12)
        | ((function & 0x7f) << 5)
        | (rc & 0x1f)
}

/// Encode a memory-format instruction.
pub const fn memory(op: u32, ra: u32, rb: u32, displacement: i16) -> u32 {
    ((op & 0x3f) << 26) | ((ra & 0x1f) << 21) | ((rb & 0x1f) << 16) | displacement as u16 as u32
}

/// Encode a branch-format instruction.
pub const fn branch(op: u32, ra: u32, displacement: i32) -> u32 {
    ((op & 0x3f) << 26) | ((ra & 0x1f) << 21) | (displacement as u32 & 0x1f_ffff)
}

/// Encode a jump-format instruction.
pub const fn jump(function: u32, ra: u32, rb: u32) -> u32 {
    (0x1a << 26) | ((ra & 0x1f) << 21) | ((rb & 0x1f) << 16) | ((function & 3) << 14)
}

/// Encode the standard `BIS r31, literal, rd` immediate-load idiom.
pub const fn mov_literal(rd: u32, literal: u32) -> u32 {
    operate_literal(0x11, 0x20, 31, literal, rd)
}

/// Encode `call_pal 0`, the repository halt sentinel.
pub const fn halt() -> u32 {
    0
}

/// Compute a signed branch displacement from instruction and target addresses.
pub const fn branch_displacement(instruction_address: u64, target_address: u64) -> i32 {
    ((target_address as i64 - (instruction_address as i64 + 4)) / 4) as i32
}
