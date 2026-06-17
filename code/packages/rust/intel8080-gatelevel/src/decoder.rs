//! Combinational instruction decoder for the Intel 8080.
//!
//! # Opcode structure
//!
//! ```text
//! Bit  7   6  |  5   4   3  |  2   1   0
//! ──────────────────────────────────────────
//!  group[1:0]  | dst/alu_op  |   src
//! ```
//!
//! Group decode (bits 7–6):
//! - `00` → Group 0: misc (MVI, LXI, INR, DCR, MOV, INX, DCX, …)
//! - `01` → Group 1: MOV r,r (register-to-register); `0x76` = HLT
//! - `10` → Group 2: ALU register ops (ADD, SUB, ANA, ORA, …)
//! - `11` → Group 3: branches, stack, control (JMP, CALL, RET, PUSH, POP, …)
//!
//! # Register codes (3-bit)
//!
//! | Code | Register |
//! |------|----------|
//! | 000  | B        |
//! | 001  | C        |
//! | 010  | D        |
//! | 011  | E        |
//! | 100  | H        |
//! | 101  | L        |
//! | 110  | M (mem)  |
//! | 111  | A        |
//!
//! # Register pair codes (2-bit, bits 5–4)
//!
//! | Code | Pair |
//! |------|------|
//! | 00   | BC   |
//! | 01   | DE   |
//! | 10   | HL   |
//! | 11   | SP   |
//!
//! # Gate-level group decode
//!
//! Using AND/NOT on bits 7 and 6:
//! ```text
//! is_g0 = AND(NOT(b7), NOT(b6))
//! is_g1 = AND(NOT(b7), b6)
//! is_g2 = AND(b7, NOT(b6))
//! is_g3 = AND(b7, b6)
//! ```

use logic_gates::gates::{and_gate, not_gate};

/// Control signals produced by the combinational decoder for one opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoded {
    /// Opcode group 0–3 (bits 7–6).
    pub group: u8,
    /// Destination register code 0–7 (bits 5–3).
    pub dst: u8,
    /// Source register code 0–7 (bits 2–0).
    pub src: u8,
    /// ALU operation code (same bit field as `dst` for group-2 instructions).
    pub alu_op: u8,
    /// Register pair code 0–3 (bits 5–4).
    pub reg_pair: u8,
    /// True when opcode is 0x76 (HLT — MOV M,M re-purposed as halt).
    pub is_halt: bool,
    /// True when src field == 6 (M pseudo-register, memory indirect via HL).
    pub is_mem_src: bool,
    /// True when dst field == 6 (M pseudo-register, memory write).
    pub is_mem_dst: bool,
    /// Number of additional bytes to fetch after the opcode: 0, 1, or 2.
    pub extra_bytes: u8,
    /// Raw opcode byte.
    pub opcode: u8,
}

/// Decode one opcode byte into control signals using AND/NOT gate logic.
///
/// # Example
/// ```
/// use coding_adventures_intel8080_gatelevel::decoder::decode;
/// let d = decode(0x80); // ADD B
/// assert_eq!(d.group, 2);
/// assert_eq!(d.alu_op, 0); // ADD
/// assert_eq!(d.src, 0);    // register B
///
/// let h = decode(0x76); // HLT
/// assert!(h.is_halt);
/// ```
pub fn decode(opcode: u8) -> Decoded {
    // ── Extract individual opcode bits (model as wire reads) ─────────────────
    let b7 = (opcode >> 7) & 1;
    let b6 = (opcode >> 6) & 1;
    let b5 = (opcode >> 5) & 1;
    let b4 = (opcode >> 4) & 1;
    let b3 = (opcode >> 3) & 1;
    let b2 = (opcode >> 2) & 1;
    let b1 = (opcode >> 1) & 1;
    let b0 = opcode & 1;

    // ── Group decode: AND/NOT on bits 7–6 ────────────────────────────────────
    let nb7 = not_gate(b7);
    let nb6 = not_gate(b6);
    let is_g0 = and_gate(nb7, nb6);
    let is_g1 = and_gate(nb7, b6);
    let is_g2 = and_gate(b7, nb6);
    let is_g3 = and_gate(b7, b6);

    let group = if is_g3 != 0 { 3 } else if is_g2 != 0 { 2 } else if is_g1 != 0 { 1 } else { 0 };

    // ── Field extraction ──────────────────────────────────────────────────────
    let dst = (b5 << 2) | (b4 << 1) | b3;     // bits 5–3
    let src = (b2 << 2) | (b1 << 1) | b0;     // bits 2–0
    let alu_op = dst;                           // group-2: ALU op = bits 5–3
    let reg_pair = (b5 << 1) | b4;             // bits 5–4

    // ── HLT detection: opcode 0x76 = 0b01110110 ──────────────────────────────
    // b7=0 b6=1 b5=1 b4=1 b3=0 b2=1 b1=1 b0=0
    let nb3 = not_gate(b3);
    let nb0 = not_gate(b0);
    let is_halt_val = and_gate(
        is_g1,
        and_gate(
            and_gate(b5, b4),
            and_gate(nb3, and_gate(b2, and_gate(b1, nb0))),
        ),
    );
    let is_halt = is_halt_val != 0;

    // ── Memory operand detection ──────────────────────────────────────────────
    // M pseudo-register code = 6 = 0b110: bits2=1, bits1=1, bits0=0
    let nb2_src = and_gate(b2, and_gate(b1, not_gate(b0)));
    let is_mem_src = and_gate(nb2_src, not_gate(is_halt_val)) != 0;
    // dst == 6: bits5=1, bits4=1, bits3=0
    let nb5 = and_gate(b5, and_gate(b4, not_gate(b3)));
    let is_mem_dst = and_gate(nb5, not_gate(is_halt_val)) != 0;

    // ── Instruction length ────────────────────────────────────────────────────
    let extra_bytes = extra_bytes(opcode, is_g0, is_g3);

    Decoded {
        group,
        dst,
        src,
        alu_op,
        reg_pair,
        is_halt,
        is_mem_src,
        is_mem_dst,
        extra_bytes,
        opcode,
    }
}

/// Combinational instruction-length decoder.
///
/// Returns 0, 1, or 2 extra bytes to fetch after the opcode.
///
/// The 8080 has three instruction lengths:
/// - 1 byte: register-register ops, ALU register, single-byte control
/// - 2 bytes: MVI, ADI/ACI/…/CPI, IN port, OUT port
/// - 3 bytes: LXI, LDA/STA, LHLD/SHLD, JMP, CALL, conditional J/CALL
fn extra_bytes(opcode: u8, is_g0: u8, is_g3: u8) -> u8 {
    if is_g0 != 0 {
        // LXI rp,d16 — pattern 00rp0001
        if (opcode & 0x0F) == 0x01 { return 2; }
        // MVI r,d8 — pattern 00ddd110 (src=6), but not HLT (0x76)
        if (opcode & 0x07) == 0x06 && opcode != 0x76 { return 1; }
        // LDA (0x3A), STA (0x32), LHLD (0x2A), SHLD (0x22)
        if matches!(opcode, 0x3A | 0x32 | 0x2A | 0x22) { return 2; }
        return 0;
    }
    if is_g3 != 0 {
        // Unconditional JMP (0xC3), CALL (0xCD) → 3 bytes
        if matches!(opcode, 0xC3 | 0xCD) { return 2; }
        // Conditional JMP: Ccc010 pattern
        if (opcode & 0xC7) == 0xC2 { return 2; }
        // Conditional CALL: Ccc100 pattern
        if (opcode & 0xC7) == 0xC4 { return 2; }
        // IN (0xDB), OUT (0xD3) → 2 bytes
        if matches!(opcode, 0xDB | 0xD3) { return 1; }
        // Immediate ALU: ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI → low 3 bits = 110
        if (opcode & 0x07) == 0x06 { return 1; }
        return 0;
    }
    // Groups 1 (MOV) and 2 (ALU register) are all 1-byte
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_decode() {
        assert_eq!(decode(0x00).group, 0); // NOP (group 0)
        assert_eq!(decode(0x41).group, 1); // MOV B,C
        assert_eq!(decode(0x80).group, 2); // ADD B
        assert_eq!(decode(0xC3).group, 3); // JMP addr16
    }

    #[test]
    fn hlt_detection() {
        assert!(decode(0x76).is_halt);
        assert!(!decode(0x46).is_halt); // MOV B,M
    }

    #[test]
    fn memory_src() {
        assert!(decode(0x46).is_mem_src); // MOV B,M
        assert!(!decode(0x41).is_mem_src); // MOV B,C
        assert!(!decode(0x76).is_mem_src); // HLT
    }

    #[test]
    fn memory_dst() {
        assert!(decode(0x70).is_mem_dst); // MOV M,B
        assert!(!decode(0x41).is_mem_dst); // MOV B,C
    }

    #[test]
    fn extra_bytes() {
        assert_eq!(decode(0xC3).extra_bytes, 2); // JMP addr16
        assert_eq!(decode(0x3E).extra_bytes, 1); // MVI A,d8
        assert_eq!(decode(0x01).extra_bytes, 2); // LXI BC,d16
        assert_eq!(decode(0x80).extra_bytes, 0); // ADD B
        assert_eq!(decode(0xC6).extra_bytes, 1); // ADI d8
        assert_eq!(decode(0x3A).extra_bytes, 2); // LDA addr16
    }

    #[test]
    fn reg_fields() {
        let d = decode(0x80); // ADD B → group2, alu_op=0 (ADD), src=0 (B)
        assert_eq!(d.alu_op, 0);
        assert_eq!(d.src, 0);

        let d2 = decode(0x41); // MOV B,C → dst=0 (B), src=1 (C)
        assert_eq!(d2.dst, 0);
        assert_eq!(d2.src, 1);
    }

    #[test]
    fn reg_pair() {
        assert_eq!(decode(0x01).reg_pair, 0); // LXI BC → pair 0
        assert_eq!(decode(0x11).reg_pair, 1); // LXI DE → pair 1
        assert_eq!(decode(0x21).reg_pair, 2); // LXI HL → pair 2
        assert_eq!(decode(0x31).reg_pair, 3); // LXI SP → pair 3
    }
}
