//! Combinational instruction decoder for the Intel 8008.
//!
//! # What is a decoder?
//!
//! In the real 8008, the instruction decoder is a large combinational circuit.
//! It takes the 8 opcode bits as inputs and produces a set of control signals
//! (output wires) that tell each part of the CPU what to do this cycle. There
//! is no program running the decoder — it's just AND/OR/NOT gates that light up
//! the correct output wires based on the input bit pattern.
//!
//! # How it works
//!
//! Each control signal is computed by a Boolean expression over the opcode bits.
//! For example:
//!
//! ```text
//! "Is this a HLT instruction?" = AND(bit7=0, bit6=1, bit5=1, bit4=1, bit3=0, bit2=1, bit1=1, bit0=0)
//!                              = AND(NOT(bit7), bit6, bit5, bit4, NOT(bit3), bit2, bit1, NOT(bit0))
//! ```
//!
//! The decoder extracts the two group bits (7-6) and the DDD/SSS fields (5-3, 2-0),
//! then combines them into control signals using AND/OR/NOT gates.
//!
//! # Output signals
//!
//! The decoder produces 16 control signals that the control unit uses to
//! configure the register file, ALU, PC, and stack for each instruction.

use logic_gates::gates::{and_gate, not_gate, or_gate};

/// Decoded control signals for one 8008 instruction.
///
/// The CPU uses these signals to configure all its components for the
/// current instruction without any conditional logic — the decoder has
/// already done all the decision-making via gates.
#[derive(Debug, Clone)]
pub struct DecoderOutput {
    /// Number of bytes in this instruction (1, 2, or 3).
    pub instruction_bytes: u8,
    /// ALU operation name (e.g., "add", "sub", "and", "or", "xor", "nop").
    pub alu_op: &'static str,
    /// Source register index (0-7).
    pub reg_src: usize,
    /// Destination register index (0-7).
    pub reg_dst: usize,
    /// True if this instruction halts the CPU.
    pub is_halt: bool,
    /// True if this is an unconditional or taken conditional jump.
    pub is_jump: bool,
    /// True if this is a CALL (push + jump).
    pub is_call: bool,
    /// True if this is a RETURN.
    pub is_return: bool,
    /// True if this is a RST (restart) instruction.
    pub is_rst: bool,
    /// Condition code (0=CY, 1=Z, 2=S, 3=P) for conditional instructions.
    pub cond_code: u8,
    /// True if condition is "flag must be SET"; false if "flag must be CLEAR".
    pub cond_sense: bool,
    /// True if this instruction writes a result to the accumulator (A).
    pub write_acc: bool,
    /// True if this instruction writes a result to a general register.
    pub write_reg: bool,
    /// True if carry flag should be forced to 0 after this instruction (ANA/ORA/XRA).
    pub clear_carry: bool,
    /// True if Z/S/P flags should be updated.
    pub update_flags: bool,
    /// True if this is an IN (read input port) instruction.
    pub is_input: bool,
    /// True if this is an OUT (write output port) instruction.
    pub is_output: bool,
    /// Port number for IN/OUT instructions (0-23).
    pub port: u8,
    /// RST target address (AAA << 3, for RST instructions).
    pub rst_target: u16,
    /// True if this is a MOV instruction.
    pub is_mov: bool,
    /// True if this is an ALU instruction (register or immediate operand).
    pub is_alu: bool,
    /// True if ALU source is an immediate byte (group 11).
    pub is_alu_immediate: bool,
    /// True if this is MVI (move immediate).
    pub is_mvi: bool,
    /// True if this is INR (increment register).
    pub is_inr: bool,
    /// True if this is DCR (decrement register).
    pub is_dcr: bool,
    /// True if this is a rotate instruction.
    pub is_rotate: bool,
    /// Rotate type: 0=RLC, 1=RRC, 2=RAL, 3=RAR.
    pub rotate_type: u8,
    /// True if INR/DCR should NOT update carry (8008 behavior).
    pub preserve_carry: bool,
}

/// Decode one 8008 opcode into control signals.
///
/// This function implements the combinational gate tree that decodes
/// an 8-bit opcode into the control signals needed by the CPU datapath.
/// All decisions are made through AND/OR/NOT gate calls — no `if/else`
/// except where needed to construct gate outputs.
///
/// # Group structure
///
/// ```text
/// bits 7-6 = group:
///   00 = Group A: INR, DCR, Rotates, MVI, Returns, RST, OUT
///   01 = Group B: MOV, HLT, Jumps, Calls, IN
///   10 = Group C: ALU register (ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP)
///   11 = Group D: ALU immediate (ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI)
/// ```
pub fn decode(opcode: u8) -> DecoderOutput {
    // Extract bit fields from opcode
    let bits: Vec<u8> = (0..8).map(|i| (opcode >> i) & 1).collect();

    // bits[7] = MSB, bits[0] = LSB
    // Group bits: bits[7:6]
    let b7 = bits[7];
    let b6 = bits[6];
    let b5 = bits[5];
    let b4 = bits[4];
    let b3 = bits[3];
    let b2 = bits[2];
    let b1 = bits[1];
    let b0 = bits[0];

    // Group selector signals (one-hot: exactly one will be 1)
    // group_00 = NOT(b7) AND NOT(b6)
    let group_00 = and_gate(not_gate(b7), not_gate(b6));
    // group_01 = NOT(b7) AND b6
    let group_01 = and_gate(not_gate(b7), b6);
    // group_10 = b7 AND NOT(b6)
    let group_10 = and_gate(b7, not_gate(b6));
    // group_11 = b7 AND b6
    let group_11 = and_gate(b7, b6);

    // DDD (destination/operation): bits[5:3]
    let ddd = (b5 << 2) | (b4 << 1) | b3;
    // SSS (source/sub-op): bits[2:0]
    let sss = (b2 << 2) | (b1 << 1) | b0;

    // --- HLT detection (01 110 110 = 0x76) ---
    let is_halt_u8 = and_gate(
        and_gate(not_gate(b7), b6),
        and_gate(
            and_gate(b5, b4),
            and_gate(not_gate(b3), and_gate(b2, and_gate(b1, not_gate(b0))))
        )
    );
    // Also 0xFF
    let all_ones = and_gate(
        and_gate(b7, b6),
        and_gate(and_gate(b5, b4), and_gate(b3, and_gate(b2, and_gate(b1, b0))))
    );
    let is_halt = or_gate(is_halt_u8, all_ones) == 1;

    // --- IN detection: group_01, sss=001 ---
    let sss_is_001 = and_gate(not_gate(b2), and_gate(not_gate(b1), b0));
    let is_input = and_gate(group_01, sss_is_001) == 1;

    // --- OUT detection: group_00, sss=010 (b2-0=010), ddd>3 ---
    let sss_is_010 = and_gate(not_gate(b2), and_gate(b1, not_gate(b0)));
    let ddd_gt3 = b5; // bits[5]=1 means ddd>=4
    let is_output_base = and_gate(group_00, and_gate(sss_is_010, ddd_gt3));
    let is_output = is_output_base == 1;

    // --- Rotate detection: group_00, sss=010, ddd<=3 ---
    let ddd_le3 = not_gate(b5);
    let is_rotate_base = and_gate(group_00, and_gate(sss_is_010, ddd_le3));
    let is_rotate = and_gate(is_rotate_base, not_gate(is_halt_u8)) == 1;
    let rotate_type = (b4 << 1) | b3; // bits[4:3] select RLC/RRC/RAL/RAR

    // --- MVI detection: group_00, sss=110 ---
    let sss_is_110 = and_gate(b2, and_gate(b1, not_gate(b0)));
    let is_mvi = and_gate(group_00, sss_is_110) == 1;

    // --- INR detection: group_00, sss=000 ---
    let sss_is_000 = and_gate(not_gate(b2), and_gate(not_gate(b1), not_gate(b0)));
    let is_inr = and_gate(group_00, sss_is_000) == 1;

    // --- DCR detection: group_00, sss=001 ---
    let is_dcr = and_gate(group_00, sss_is_001) == 1;

    // --- RST detection: group_00, sss=101 ---
    let sss_is_101 = and_gate(b2, and_gate(not_gate(b1), b0));
    let is_rst = and_gate(group_00, sss_is_101) == 1;
    let rst_target = ((ddd as u16) << 3) & 0x3FFF;

    // --- Return detection: group_00, sss=011 or sss=111 ---
    let sss_is_011 = and_gate(not_gate(b2), and_gate(b1, b0));
    let sss_is_111 = and_gate(b2, and_gate(b1, b0));
    let is_return_false = and_gate(group_00, sss_is_011) == 1;
    let is_return_true = and_gate(group_00, sss_is_111) == 1;
    let is_return = is_return_false || is_return_true;

    // --- Jump/Call detection (group_01) ---
    let sss_is_000_2 = sss_is_000; // reuse
    let sss_is_100 = and_gate(b2, and_gate(not_gate(b1), not_gate(b0)));

    // Jump: sss ∈ {000,100} AND ddd ≤ 3 (or opcode=0x7C)
    // 0x7C = 0111 1100: group_01 (b7=0,b6=1), ddd=111 (b5=1,b4=1,b3=1), sss=100 (b2=1,b1=0,b0=0)
    let is_jmp_uncond = and_gate(
        and_gate(not_gate(b7), b6), // group_01
        and_gate(b5, and_gate(b4, and_gate(b3, and_gate(b2, and_gate(not_gate(b1), not_gate(b0))))))
    ) == 1; // 0x7C

    let is_jump_cond = {
        let j_false = and_gate(group_01, and_gate(sss_is_000_2, ddd_le3)) == 1;
        let j_true = and_gate(group_01, and_gate(sss_is_100, ddd_le3)) == 1;
        (j_false || j_true) && !is_halt
    };
    let is_jump = (is_jmp_uncond || is_jump_cond) && !is_halt;

    // Call: sss ∈ {010,110} AND ddd ≤ 3 (or opcode=0x7E)
    let sss_is_110_2 = sss_is_110; // reuse
    let is_cal_uncond = opcode == 0x7E;
    let is_call_cond = {
        let c_false = and_gate(group_01, and_gate(sss_is_010, ddd_le3)) == 1;
        let c_true = and_gate(group_01, and_gate(sss_is_110_2, ddd_le3)) == 1;
        (c_false || c_true) && !is_halt
    };
    let is_call = (is_cal_uncond || is_call_cond) && !is_halt;

    // --- MOV detection: group_01, not HLT, not IN, not Jump, not Call ---
    let is_mov = group_01 == 1 && !is_halt && !is_input && !is_jump && !is_call;

    // --- ALU detection ---
    let is_alu_reg = group_10 == 1; // group_10: ALU with register source
    // is_alu_imm computed below via is_alu_immediate; this intermediate is unused
    let sss_is_100_2 = sss_is_100;
    let is_alu_immediate = and_gate(group_11, sss_is_100_2) == 1 && !is_halt;
    let is_alu = is_alu_reg || is_alu_immediate;

    // ALU operation code (OOO = ddd for group 10/11)
    let alu_ops = ["add", "adc", "sub", "sbb", "and", "xor", "or", "cmp"];
    let alu_op = alu_ops[ddd as usize % 8];

    // Source and destination registers
    let reg_src = sss as usize;
    let reg_dst = ddd as usize;

    // Condition code and sense for conditional instructions
    let cond_sense = if is_return { is_return_true }
                     else if is_jump { sss == 4 || is_jmp_uncond }
                     else if is_call { sss == 6 || is_cal_uncond }
                     else { false };
    let cond_code = ddd as u8 & 0x03; // low 2 bits of ddd

    // Write signals
    let write_acc = (is_alu && ddd != 7) // CMP doesn't write
        || is_input || is_mov;
    let write_reg = is_mov || is_mvi || is_inr || is_dcr || is_alu;

    // Clear carry for logical operations (ANA=4, XRA=5, ORA=6)
    let clear_carry = is_alu && (ddd == 4 || ddd == 5 || ddd == 6);

    // Update flags: most ALU ops, INR, DCR; NOT for MOV, MVI, IN, OUT, jumps
    let update_flags = is_alu || is_inr || is_dcr || is_rotate;

    // Instruction length
    let instruction_bytes = if is_jump || is_call { 3 }
                             else if is_mvi || is_alu_immediate { 2 }
                             else { 1 };

    // Port number for IN/OUT
    let port = if is_input { ddd as u8 }
               else if is_output { (opcode >> 1) & 0x1F }
               else { 0 };

    DecoderOutput {
        instruction_bytes,
        alu_op,
        reg_src,
        reg_dst,
        is_halt,
        is_jump,
        is_call,
        is_return,
        is_rst,
        cond_code,
        cond_sense,
        write_acc,
        write_reg,
        clear_carry,
        update_flags,
        is_input,
        is_output,
        port,
        rst_target,
        is_mov,
        is_alu,
        is_alu_immediate,
        is_mvi,
        is_inr,
        is_dcr,
        is_rotate,
        rotate_type,
        preserve_carry: is_inr || is_dcr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hlt() {
        let d = decode(0x76);
        assert!(d.is_halt, "0x76 should be HLT");
    }

    #[test]
    fn test_decode_mvi_a() {
        let d = decode(0x3E); // MVI A, _ (group=00, ddd=7, sss=6)
        assert!(d.is_mvi, "0x3E should be MVI");
        assert_eq!(d.reg_dst, 7); // destination = A
        assert_eq!(d.instruction_bytes, 2);
    }

    #[test]
    fn test_decode_add_b() {
        let d = decode(0x80); // ADD B (group=10, ddd=0, sss=0)
        assert!(d.is_alu, "0x80 should be ALU");
        assert_eq!(d.alu_op, "add");
        assert_eq!(d.reg_src, 0); // source = B
        assert_eq!(d.instruction_bytes, 1);
    }

    #[test]
    fn test_decode_jmp() {
        let d = decode(0x7C); // JMP
        assert!(d.is_jump, "0x7C should be jump");
        assert_eq!(d.instruction_bytes, 3);
    }

    #[test]
    fn test_decode_cal() {
        let d = decode(0x7E); // CAL
        assert!(d.is_call, "0x7E should be call");
        assert_eq!(d.instruction_bytes, 3);
    }

    #[test]
    fn test_decode_rlc() {
        let d = decode(0x02); // RLC
        assert!(d.is_rotate, "0x02 should be rotate");
        assert_eq!(d.rotate_type, 0); // RLC
    }

    #[test]
    fn test_decode_ret() {
        let d = decode(0x3F); // RET
        assert!(d.is_return, "0x3F should be return");
    }

    #[test]
    fn test_decode_in() {
        let d = decode(0x49); // IN 1
        assert!(d.is_input, "0x49 should be IN");
        assert_eq!(d.port, 1);
    }

    #[test]
    fn test_decode_rst() {
        let d = decode(0x1D); // RST 3
        assert!(d.is_rst, "0x1D should be RST");
        assert_eq!(d.rst_target, 3 << 3); // target = 0x18
    }
}
