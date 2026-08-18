//! # `intel8051-simulator::opcodes` — architectural constants.
//!
//! Every memory-size constant, Special Function Register (SFR) address,
//! PSW bit mask, and instruction opcode byte the simulator needs, ported
//! 1:1 from the Python reference (`code/packages/python/intel8051-simulator/
//! src/intel8051_simulator/state.py` and `simulator.py`'s inline opcode
//! literals).  No behaviour lives here — just named numbers, so the
//! `decode`/`execute` modules read like the datasheet table in
//! `code/specs/07p-intel-8051-simulator.md` rather than a wall of magic
//! hex.
//!
//! ## Memory model recap
//!
//! The 8051 is Harvard-architecture: three independent address spaces.
//!
//! ```text
//! code   (64 KiB) — program memory, fetched by PC, read by MOVC
//! iram  (256 B)   — internal RAM (0x00-0x7F) + SFRs (0x80-0xFF)
//! xdata  (64 KiB) — external data memory, accessed only via MOVX
//! ```
//!
//! ## Opcode-family naming convention
//!
//! Many 8051 instructions come in 8-wide or 2-wide "families" that share
//! a fixed set of high bits and use the low 3 (or 1) bits to select an
//! operand register.  Those are named `..._BASE`; the concrete opcode is
//! `BASE + n` (registers `R0..R7`) or `BASE + i` (indirect pointer
//! `@R0`/`@R1`, `i` = 0 or 1).  E.g. `ADD_A_RN_BASE + 3` is `ADD A, R3`.

// ===========================================================================
// Memory-space sizes
// ===========================================================================

/// Harvard code-memory space: 64 KiB, addressed by the 16-bit PC.
pub const CODE_SIZE: usize = 65536;

/// External data-memory space (accessed only via `MOVX`): 64 KiB.
pub const XDATA_SIZE: usize = 65536;

/// Internal RAM + SFR space: 256 bytes total (0x00-0x7F general RAM +
/// bit-addressable area, 0x80-0xFF SFRs).
pub const IRAM_SIZE: usize = 256;

// ===========================================================================
// Special Function Register (SFR) addresses
// ===========================================================================
//
// SFRs live in the *same* 256-byte iram array as general RAM — a direct
// address of 0x80 or above simply lands in SFR space rather than
// scratch RAM.  See `code/specs/07p-intel-8051-simulator.md`'s SFR
// table for the full 21-register set; the ones below are the subset
// this behavioral simulator models with live semantics.

pub const SFR_P0: u8 = 0x80;
pub const SFR_SP: u8 = 0x81;
pub const SFR_DPL: u8 = 0x82;
pub const SFR_DPH: u8 = 0x83;
pub const SFR_PCON: u8 = 0x87;
pub const SFR_TCON: u8 = 0x88;
pub const SFR_TMOD: u8 = 0x89;
pub const SFR_TL0: u8 = 0x8A;
pub const SFR_TL1: u8 = 0x8B;
pub const SFR_TH0: u8 = 0x8C;
pub const SFR_TH1: u8 = 0x8D;
pub const SFR_P1: u8 = 0x90;
pub const SFR_SCON: u8 = 0x98;
pub const SFR_SBUF: u8 = 0x99;
pub const SFR_P2: u8 = 0xA0;
pub const SFR_IE: u8 = 0xA8;
pub const SFR_P3: u8 = 0xB0;
pub const SFR_IP: u8 = 0xB8;
pub const SFR_PSW: u8 = 0xD0;
pub const SFR_ACC: u8 = 0xE0;
pub const SFR_B: u8 = 0xF0;

/// Reset value of the P0-P3 port latches (all bits high on real
/// hardware — an unconfigured pin floats/reads high).
pub const PORT_RESET: u8 = 0xFF;

// ===========================================================================
// PSW (Program Status Word) bit masks
// ===========================================================================
//
// ```text
// Bit  7    6    5    4    3    2    1    0
//      CY   AC   F0   RS1  RS0  OV   -    P
// ```

pub const PSW_CY: u8 = 0x80;
pub const PSW_AC: u8 = 0x40;
pub const PSW_F0: u8 = 0x20;
pub const PSW_RS1: u8 = 0x10;
pub const PSW_RS0: u8 = 0x08;
pub const PSW_OV: u8 = 0x04;
pub const PSW_P: u8 = 0x01;

// ===========================================================================
// HALT convention
// ===========================================================================

/// The real 8051 has no HALT instruction.  This simulator (and its
/// Python reference implementation, `intel8051_simulator.state.
/// HALT_OPCODE`, spec 07p) uses opcode `0xA5` — undefined/reserved in
/// every Intel MCS-51 datasheet's opcode map — as a HALT sentinel.
/// Executing it sets `halted = true` and stops the fetch-decode-execute
/// loop, the same way a PDP-11 program of this codebase's other
/// historical-arch lanes terminates on a reserved trap value rather
/// than a real "power off" instruction.
///
/// `intel8051-backend` reuses this exact convention (see its
/// crate-level doc comment for why self-jump detection was considered
/// and rejected in favour of preserving this already-established,
/// already-tested Python behavioral reference).
pub const HALT_OPCODE: u8 = 0xA5;

// ===========================================================================
// Fixed single-byte / single-opcode instructions
// ===========================================================================

pub const NOP: u8 = 0x00;

// -- Data transfer ----------------------------------------------------------

pub const MOV_A_DIR: u8 = 0xE5;
pub const MOV_A_IMM: u8 = 0x74;
pub const MOV_DIR_A: u8 = 0xF5;
pub const MOV_DIR_DIR: u8 = 0x85;
pub const MOV_DIR_IMM: u8 = 0x75;
pub const MOV_DPTR_IMM: u8 = 0x90;
pub const MOVC_A_AT_A_DPTR: u8 = 0x93;
pub const MOVC_A_AT_A_PC: u8 = 0x83;
pub const MOVX_A_AT_DPTR: u8 = 0xE0;
pub const MOVX_AT_DPTR_A: u8 = 0xF0;
pub const PUSH: u8 = 0xC0;
pub const POP: u8 = 0xD0;
pub const XCH_A_DIR: u8 = 0xC5;

// -- Arithmetic ---------------------------------------------------------

pub const ADD_A_DIR: u8 = 0x25;
pub const ADD_A_IMM: u8 = 0x24;
pub const ADDC_A_DIR: u8 = 0x35;
pub const ADDC_A_IMM: u8 = 0x34;
pub const SUBB_A_DIR: u8 = 0x95;
pub const SUBB_A_IMM: u8 = 0x94;
pub const INC_A: u8 = 0x04;
pub const INC_DIR: u8 = 0x05;
pub const INC_DPTR: u8 = 0xA3;
pub const DEC_A: u8 = 0x14;
pub const DEC_DIR: u8 = 0x15;
pub const MUL_AB: u8 = 0xA4;
pub const DIV_AB: u8 = 0x84;
pub const DA_A: u8 = 0xD4;

// -- Logic ----------------------------------------------------------------

pub const ANL_A_DIR: u8 = 0x55;
pub const ANL_A_IMM: u8 = 0x54;
pub const ANL_DIR_A: u8 = 0x52;
pub const ANL_DIR_IMM: u8 = 0x53;
pub const ORL_A_DIR: u8 = 0x45;
pub const ORL_A_IMM: u8 = 0x44;
pub const ORL_DIR_A: u8 = 0x42;
pub const ORL_DIR_IMM: u8 = 0x43;
pub const XRL_A_DIR: u8 = 0x65;
pub const XRL_A_IMM: u8 = 0x64;
pub const XRL_DIR_A: u8 = 0x62;
pub const XRL_DIR_IMM: u8 = 0x63;
pub const CLR_A: u8 = 0xE4;
pub const CPL_A: u8 = 0xF4;
pub const RL_A: u8 = 0x23;
pub const RLC_A: u8 = 0x33;
pub const RR_A: u8 = 0x03;
pub const RRC_A: u8 = 0x13;
pub const SWAP_A: u8 = 0xC4;

// -- Bit manipulation -------------------------------------------------------

pub const CLR_C: u8 = 0xC3;
pub const CLR_BIT: u8 = 0xC2;
pub const SETB_C: u8 = 0xD3;
pub const SETB_BIT: u8 = 0xD2;
pub const CPL_C: u8 = 0xB3;
pub const CPL_BIT: u8 = 0xB2;
pub const ANL_C_BIT: u8 = 0x82;
pub const ANL_C_NBIT: u8 = 0xB0;
pub const ORL_C_BIT: u8 = 0x72;
pub const ORL_C_NBIT: u8 = 0xA0;
pub const MOV_C_BIT: u8 = 0xA2;
pub const MOV_BIT_C: u8 = 0x92;

// -- Branching --------------------------------------------------------------

pub const LJMP: u8 = 0x02;
pub const SJMP: u8 = 0x80;
pub const JMP_AT_A_DPTR: u8 = 0x73;
pub const JZ: u8 = 0x60;
pub const JNZ: u8 = 0x70;
pub const JC: u8 = 0x40;
pub const JNC: u8 = 0x50;
pub const JB: u8 = 0x20;
pub const JNB: u8 = 0x30;
pub const JBC: u8 = 0x10;
pub const CJNE_A_DIR: u8 = 0xB5;
pub const CJNE_A_IMM: u8 = 0xB4;
pub const DJNZ_DIR: u8 = 0xD5;

// -- Subroutines --------------------------------------------------------

pub const LCALL: u8 = 0x12;
pub const RET: u8 = 0x22;
pub const RETI: u8 = 0x32;

// ===========================================================================
// Opcode-family bases (opcode = BASE + n, n = 0..=7, or BASE + i, i = 0..=1)
// ===========================================================================

pub const MOV_A_RN_BASE: u8 = 0xE8;
pub const MOV_A_AT_RI_BASE: u8 = 0xE6;
pub const MOV_RN_A_BASE: u8 = 0xF8;
pub const MOV_RN_DIR_BASE: u8 = 0xA8;
pub const MOV_RN_IMM_BASE: u8 = 0x78;
pub const MOV_DIR_RN_BASE: u8 = 0x88;
pub const MOV_DIR_AT_RI_BASE: u8 = 0x86;
pub const MOV_AT_RI_A_BASE: u8 = 0xF6;
pub const MOV_AT_RI_DIR_BASE: u8 = 0xA6;
pub const MOV_AT_RI_IMM_BASE: u8 = 0x76;
pub const MOVX_A_AT_RI_BASE: u8 = 0xE2;
pub const MOVX_AT_RI_A_BASE: u8 = 0xF2;
pub const XCH_A_RN_BASE: u8 = 0xC8;
pub const XCH_A_AT_RI_BASE: u8 = 0xC6;
pub const XCHD_A_AT_RI_BASE: u8 = 0xD6;
pub const ADD_A_RN_BASE: u8 = 0x28;
pub const ADD_A_AT_RI_BASE: u8 = 0x26;
pub const ADDC_A_RN_BASE: u8 = 0x38;
pub const ADDC_A_AT_RI_BASE: u8 = 0x36;
pub const SUBB_A_RN_BASE: u8 = 0x98;
pub const SUBB_A_AT_RI_BASE: u8 = 0x96;
pub const INC_RN_BASE: u8 = 0x08;
pub const INC_AT_RI_BASE: u8 = 0x06;
pub const DEC_RN_BASE: u8 = 0x18;
pub const DEC_AT_RI_BASE: u8 = 0x16;
pub const ANL_A_RN_BASE: u8 = 0x58;
pub const ANL_A_AT_RI_BASE: u8 = 0x56;
pub const ORL_A_RN_BASE: u8 = 0x48;
pub const ORL_A_AT_RI_BASE: u8 = 0x46;
pub const XRL_A_RN_BASE: u8 = 0x68;
pub const XRL_A_AT_RI_BASE: u8 = 0x66;
pub const CJNE_RN_IMM_BASE: u8 = 0xB8;
pub const CJNE_AT_RI_IMM_BASE: u8 = 0xB6;
pub const DJNZ_RN_BASE: u8 = 0xD8;

/// `AJMP`'s opcode encodes an 11-bit absolute address split across two
/// bytes: the top 3 bits of the address occupy bits 7:5 of the opcode
/// byte itself, and the low 5 bits of the opcode are the fixed pattern
/// `00001`.  So `opcode & 0x1F == AJMP_PATTERN`, not a simple base.
pub const AJMP_PATTERN: u8 = 0x01;

/// `ACALL`'s opcode uses the same addr[10:8]-in-high-bits trick as
/// `AJMP`, with fixed low-5-bit pattern `10010`.
pub const ACALL_PATTERN: u8 = 0x11;

// ===========================================================================
// Misc capacity constants
// ===========================================================================

/// Number of working registers per bank (R0-R7).
pub const GP_REGISTER_COUNT: usize = 8;

/// Number of register banks selectable via PSW.RS1:RS0.
pub const REGISTER_BANK_COUNT: usize = 4;

/// Maximum unsigned 8-bit immediate.
pub const IMM8_MAX: u8 = 255;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_opcode_is_reserved_a5() {
        assert_eq!(HALT_OPCODE, 0xA5);
    }

    #[test]
    fn sfr_acc_address() {
        assert_eq!(SFR_ACC, 0xE0);
    }

    #[test]
    fn psw_bit_masks_are_disjoint() {
        let masks = [PSW_CY, PSW_AC, PSW_F0, PSW_RS1, PSW_RS0, PSW_OV, PSW_P];
        let mut union = 0u8;
        for m in masks {
            assert_eq!(union & m, 0, "PSW bit masks must not overlap");
            union |= m;
        }
        // Bit 1 (0x02) is reserved/always-0, so the union should be
        // every bit except 0x02.
        assert_eq!(union, !0x02u8);
    }

    #[test]
    fn ajmp_and_acall_patterns_distinct() {
        assert_ne!(AJMP_PATTERN, ACALL_PATTERN);
        assert_eq!(AJMP_PATTERN, 0x01);
        assert_eq!(ACALL_PATTERN, 0x11);
    }
}
