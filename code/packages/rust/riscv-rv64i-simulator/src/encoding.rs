//! Structured encoders for the complete Spec 07y RV64I+M surface.

#[must_use]
pub fn assemble(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

const fn r(opcode: u32, funct3: u32, funct7: u32, rd: u32, rs1: u32, rs2: u32) -> u32 {
    (funct7 << 25)
        | ((rs2 & 31) << 20)
        | ((rs1 & 31) << 15)
        | ((funct3 & 7) << 12)
        | ((rd & 31) << 7)
        | opcode
}

const fn i(opcode: u32, funct3: u32, rd: u32, rs1: u32, immediate: i32) -> u32 {
    (((immediate as u32) & 0xfff) << 20)
        | ((rs1 & 31) << 15)
        | ((funct3 & 7) << 12)
        | ((rd & 31) << 7)
        | opcode
}

const fn s(funct3: u32, rs1: u32, rs2: u32, immediate: i32) -> u32 {
    let immediate = (immediate as u32) & 0xfff;
    ((immediate >> 5) << 25)
        | ((rs2 & 31) << 20)
        | ((rs1 & 31) << 15)
        | ((funct3 & 7) << 12)
        | ((immediate & 31) << 7)
        | 0x23
}

const fn b(funct3: u32, rs1: u32, rs2: u32, offset: i32) -> u32 {
    let immediate = (offset as u32) & 0x1fff;
    (((immediate >> 12) & 1) << 31)
        | (((immediate >> 5) & 0x3f) << 25)
        | ((rs2 & 31) << 20)
        | ((rs1 & 31) << 15)
        | ((funct3 & 7) << 12)
        | (((immediate >> 1) & 0xf) << 8)
        | (((immediate >> 11) & 1) << 7)
        | 0x63
}

#[must_use]
pub const fn encode_lui(rd: u32, immediate20: u32) -> u32 {
    ((immediate20 & 0xfffff) << 12) | ((rd & 31) << 7) | 0x37
}

#[must_use]
pub const fn encode_auipc(rd: u32, immediate20: u32) -> u32 {
    ((immediate20 & 0xfffff) << 12) | ((rd & 31) << 7) | 0x17
}

#[must_use]
pub const fn encode_jal(rd: u32, offset: i32) -> u32 {
    let immediate = (offset as u32) & 0x1f_ffff;
    (((immediate >> 20) & 1) << 31)
        | (((immediate >> 1) & 0x3ff) << 21)
        | (((immediate >> 11) & 1) << 20)
        | (((immediate >> 12) & 0xff) << 12)
        | ((rd & 31) << 7)
        | 0x6f
}

#[must_use]
pub const fn encode_jalr(rd: u32, rs1: u32, immediate: i32) -> u32 {
    i(0x67, 0, rd, rs1, immediate)
}

macro_rules! branch_encoders {
    ($($name:ident => $funct3:expr),+ $(,)?) => {$(
        #[must_use]
        pub const fn $name(rs1: u32, rs2: u32, offset: i32) -> u32 {
            b($funct3, rs1, rs2, offset)
        }
    )+};
}

branch_encoders! {
    encode_beq => 0,
    encode_bne => 1,
    encode_blt => 4,
    encode_bge => 5,
    encode_bltu => 6,
    encode_bgeu => 7,
}

macro_rules! load_encoders {
    ($($name:ident => $funct3:expr),+ $(,)?) => {$(
        #[must_use]
        pub const fn $name(rd: u32, rs1: u32, immediate: i32) -> u32 {
            i(0x03, $funct3, rd, rs1, immediate)
        }
    )+};
}

load_encoders! {
    encode_lb => 0,
    encode_lh => 1,
    encode_lw => 2,
    encode_ld => 3,
    encode_lbu => 4,
    encode_lhu => 5,
    encode_lwu => 6,
}

macro_rules! store_encoders {
    ($($name:ident => $funct3:expr),+ $(,)?) => {$(
        #[must_use]
        pub const fn $name(rs1: u32, rs2: u32, immediate: i32) -> u32 {
            s($funct3, rs1, rs2, immediate)
        }
    )+};
}

store_encoders! {
    encode_sb => 0,
    encode_sh => 1,
    encode_sw => 2,
    encode_sd => 3,
}

macro_rules! immediate_encoders {
    ($($name:ident => $funct3:expr),+ $(,)?) => {$(
        #[must_use]
        pub const fn $name(rd: u32, rs1: u32, immediate: i32) -> u32 {
            i(0x13, $funct3, rd, rs1, immediate)
        }
    )+};
}

immediate_encoders! {
    encode_addi => 0,
    encode_slti => 2,
    encode_sltiu => 3,
    encode_xori => 4,
    encode_ori => 6,
    encode_andi => 7,
}

#[must_use]
pub const fn encode_slli(rd: u32, rs1: u32, shift: u32) -> u32 {
    i(0x13, 1, rd, rs1, (shift & 63) as i32)
}

#[must_use]
pub const fn encode_srli(rd: u32, rs1: u32, shift: u32) -> u32 {
    i(0x13, 5, rd, rs1, (shift & 63) as i32)
}

#[must_use]
pub const fn encode_srai(rd: u32, rs1: u32, shift: u32) -> u32 {
    i(0x13, 5, rd, rs1, (0x400 | (shift & 63)) as i32)
}

macro_rules! register_encoders {
    ($($name:ident => ($funct3:expr, $funct7:expr)),+ $(,)?) => {$(
        #[must_use]
        pub const fn $name(rd: u32, rs1: u32, rs2: u32) -> u32 {
            r(0x33, $funct3, $funct7, rd, rs1, rs2)
        }
    )+};
}

register_encoders! {
    encode_add => (0, 0),
    encode_sub => (0, 0x20),
    encode_sll => (1, 0),
    encode_slt => (2, 0),
    encode_sltu => (3, 0),
    encode_xor => (4, 0),
    encode_srl => (5, 0),
    encode_sra => (5, 0x20),
    encode_or => (6, 0),
    encode_and => (7, 0),
    encode_mul => (0, 1),
    encode_mulh => (1, 1),
    encode_mulhsu => (2, 1),
    encode_mulhu => (3, 1),
    encode_div => (4, 1),
    encode_divu => (5, 1),
    encode_rem => (6, 1),
    encode_remu => (7, 1),
}

#[must_use]
pub const fn encode_addiw(rd: u32, rs1: u32, immediate: i32) -> u32 {
    i(0x1b, 0, rd, rs1, immediate)
}

#[must_use]
pub const fn encode_slliw(rd: u32, rs1: u32, shift: u32) -> u32 {
    i(0x1b, 1, rd, rs1, (shift & 31) as i32)
}

#[must_use]
pub const fn encode_srliw(rd: u32, rs1: u32, shift: u32) -> u32 {
    i(0x1b, 5, rd, rs1, (shift & 31) as i32)
}

#[must_use]
pub const fn encode_sraiw(rd: u32, rs1: u32, shift: u32) -> u32 {
    i(0x1b, 5, rd, rs1, (0x400 | (shift & 31)) as i32)
}

macro_rules! word_register_encoders {
    ($($name:ident => ($funct3:expr, $funct7:expr)),+ $(,)?) => {$(
        #[must_use]
        pub const fn $name(rd: u32, rs1: u32, rs2: u32) -> u32 {
            r(0x3b, $funct3, $funct7, rd, rs1, rs2)
        }
    )+};
}

word_register_encoders! {
    encode_addw => (0, 0),
    encode_subw => (0, 0x20),
    encode_sllw => (1, 0),
    encode_srlw => (5, 0),
    encode_sraw => (5, 0x20),
    encode_mulw => (0, 1),
    encode_divw => (4, 1),
    encode_divuw => (5, 1),
    encode_remw => (6, 1),
    encode_remuw => (7, 1),
}

#[must_use]
pub const fn encode_fence() -> u32 {
    0x0000_000f
}

#[must_use]
pub const fn encode_fence_i() -> u32 {
    0x0000_100f
}

#[must_use]
pub const fn encode_ecall() -> u32 {
    0x0000_0073
}

#[must_use]
pub const fn encode_ebreak() -> u32 {
    0x0010_0073
}

#[must_use]
pub const fn encode_zero_halt() -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_encodings_and_endianness_are_pinned() {
        assert_eq!(encode_addi(1, 0, 1), 0x0010_0093);
        assert_eq!(encode_add(3, 1, 2), 0x0020_81b3);
        assert_eq!(encode_ecall(), 0x0000_0073);
        assert_eq!(assemble(&[0x0010_0093]), [0x93, 0x00, 0x10, 0x00]);
    }
}
