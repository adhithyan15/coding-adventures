//! Shared constants, size-code tables, and condition-code predicates for
//! the Motorola 68000 ISA.
//!
//! Unlike `mos6502-simulator::opcodes` (a flat 256-entry opcode → mnemonic
//! lookup table) or `mips-r2000-simulator` (fixed 3-format decode), the
//! 68000 has **no single opcode table at all** — every instruction is
//! identified by matching bit *fields* within the 16-bit opword (the
//! Python original's own doc calls out the top 4 bits, "line 0" through
//! "line F", as only "a rough category, not a complete opcode"). This
//! module carries the pieces that are genuinely shared across every
//! line's decode/execute logic:
//!
//! - The two competing "size code" tables (`MOVE` numbers byte/word/long
//!   differently from every other instruction family — see
//!   [`sz_arith`]/[`sz_move`]).
//! - Byte/word/long masks and most-significant-bit constants, used by
//!   both `decode.rs` (effective-address widths) and `flags.rs` (N/Z/V/C
//!   computation).
//! - The 16-entry condition-code predicate table shared by `Bcc`/`DBcc`/
//!   `Scc` ([`cc_taken`]/[`CC_NAMES`]).
//! - The HALT sentinel, [`TRAP_15_WORD`] — see the crate-level doc for
//!   why `TRAP #15`, not `STOP #imm`, was chosen.
//! - [`sign_extend32`]-style helpers for the sign extensions that recur
//!   throughout `decode.rs`/`execute.rs` (word→long, byte→word, etc).

// ===========================================================================
// Address space
// ===========================================================================

/// 24-bit address bus (16 MiB) — every computed effective address is
/// masked with this, mirroring the Python original's `_ADDR_MASK`.  The
/// backing `Memory` a caller constructs may be smaller (tests routinely
/// use a few KiB); an access past the backing store's actual size still
/// panics via `cpu_simulator::Memory`'s own bounds check, same as every
/// other Rust ISA simulator in this repo.
pub const ADDR_MASK: u32 = 0x00FF_FFFF;

// ===========================================================================
// HALT sentinel
// ===========================================================================

/// `TRAP #15` — the HALT convention this simulator uses.  See the
/// crate-level doc comment (`lib.rs`) for the full derivation: the
/// Python original's own `state.py` documents *both* `STOP` and
/// `TRAP #15` as halting the CPU ("halted: True after STOP or TRAP #15
/// executes"), but its test suite's own `_stop()` helper (used 100+
/// times across `test_instructions.py`/`test_programs.py`) is
/// `TRAP #15`, not `STOP #imm` — the dominant, already-established
/// idiom this port mirrors rather than inventing a fresh convention.
///
/// Encoding: `0100 1110 0100 1111` = `0x4E4F` (line-4 miscellaneous
/// group, `TRAP #n` sub-encoding with `n = 15`).
pub const TRAP_15_WORD: u16 = 0x4E4F;

// ===========================================================================
// Size codes
// ===========================================================================

/// Decode the 2-bit "arithmetic" size code used by ADD/SUB/AND/OR/EOR/CMP/
/// ADDQ/SUBQ/CLR/NEG/NOT/TST/the immediate group/shifts: `00`=byte,
/// `01`=word, `10`=long.  Returns the size in **bytes** (1/2/4).
///
/// `11` is not a valid arithmetic size — instructions that also use `11`
/// for something else (e.g. `ADDA`/`SUBA`/`CMPA`'s address-register forms,
/// or line 5's `Scc`/`DBcc`) must check for it *before* calling this.
pub fn sz_arith(code: u8) -> Option<u8> {
    match code {
        0 => Some(1),
        1 => Some(2),
        2 => Some(4),
        _ => None,
    }
}

/// Decode `MOVE`'s own 2-bit size code (bits 13-12 of the opword),
/// which — famously, and confusingly — uses a **different** numbering
/// than every other instruction: `01`=byte, `11`=word, `10`=long.
pub fn sz_move(code: u8) -> Option<u8> {
    match code {
        1 => Some(1),
        3 => Some(2),
        2 => Some(4),
        _ => None,
    }
}

/// Unsigned bitmask for an operand of `sz` bytes (1, 2, or 4).
///
/// # Panics
///
/// Panics if `sz` is not 1, 2, or 4 — every call site derives `sz` from
/// [`sz_arith`]/[`sz_move`], which never produce another value.
pub fn mask_for(sz: u8) -> u32 {
    match sz {
        1 => 0xFF,
        2 => 0xFFFF,
        4 => 0xFFFF_FFFF,
        _ => panic!("m68k-simulator: invalid operand size {sz} (must be 1, 2, or 4)"),
    }
}

/// Most-significant-bit mask for an operand of `sz` bytes.
///
/// # Panics
///
/// Same as [`mask_for`].
pub fn msb_for(sz: u8) -> u32 {
    match sz {
        1 => 0x80,
        2 => 0x8000,
        4 => 0x8000_0000,
        _ => panic!("m68k-simulator: invalid operand size {sz} (must be 1, 2, or 4)"),
    }
}

// ===========================================================================
// Sign extension helpers
// ===========================================================================

/// Sign-extend an 8-bit value to a 32-bit signed integer (byte → i32).
pub fn sext8(b: u8) -> i32 {
    i32::from(b as i8)
}

/// Sign-extend a 16-bit value to a 32-bit signed integer (word → i32).
pub fn sext16(w: u16) -> i32 {
    i32::from(w as i16)
}

// ===========================================================================
// Condition codes (shared by Bcc / DBcc / Scc)
// ===========================================================================

/// Human-readable mnemonic suffix for each of the 16 condition codes,
/// indexed the same way as [`cc_taken`]'s `cc` parameter.  Direct
/// transcription of the Python original's `_CC_NAMES`.
pub const CC_NAMES: [&str; 16] = [
    "T", "F", "HI", "LS", "CC", "CS", "NE", "EQ", "VC", "VS", "PL", "MI", "GE", "LT", "GT", "LE",
];

/// Evaluate condition code `cc` (0-15) against the current N/Z/V/C flags.
/// Returns `true` if the condition is satisfied (branch/set/no-decrement).
///
/// Direct transcription of the Python original's `_CC_FUNCS` table —
/// see `simulator.py`'s `_cc_*` free functions for the derivation of
/// each predicate (e.g. `GE` = `N == V`, the classic signed
/// greater-or-equal test).
///
/// # Panics
///
/// Panics if `cc > 15` — every call site derives `cc` from a 4-bit
/// opword field, which can never exceed 15.
pub fn cc_taken(cc: u8, n: bool, z: bool, v: bool, c: bool) -> bool {
    match cc {
        0 => true,             // T  -- always
        1 => false,            // F  -- never
        2 => !c && !z,         // HI -- higher (unsigned >)
        3 => c || z,           // LS -- lower or same (unsigned <=)
        4 => !c,               // CC -- carry clear (unsigned >=)
        5 => c,                // CS -- carry set (unsigned <)
        6 => !z,                // NE -- not equal
        7 => z,                 // EQ -- equal
        8 => !v,                // VC -- overflow clear
        9 => v,                 // VS -- overflow set
        10 => !n,                // PL -- plus (non-negative)
        11 => n,                 // MI -- minus (negative)
        12 => n == v,             // GE -- signed >=
        13 => n != v,             // LT -- signed <
        14 => !z && (n == v),     // GT -- signed >
        15 => z || (n != v),      // LE -- signed <=
        _ => panic!("m68k-simulator: invalid condition code {cc} (must be 0-15)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sz_arith_table() {
        assert_eq!(sz_arith(0), Some(1));
        assert_eq!(sz_arith(1), Some(2));
        assert_eq!(sz_arith(2), Some(4));
        assert_eq!(sz_arith(3), None);
    }

    #[test]
    fn sz_move_table() {
        assert_eq!(sz_move(1), Some(1));
        assert_eq!(sz_move(3), Some(2));
        assert_eq!(sz_move(2), Some(4));
        assert_eq!(sz_move(0), None);
    }

    #[test]
    fn mask_and_msb_values() {
        assert_eq!(mask_for(1), 0xFF);
        assert_eq!(mask_for(2), 0xFFFF);
        assert_eq!(mask_for(4), 0xFFFF_FFFF);
        assert_eq!(msb_for(1), 0x80);
        assert_eq!(msb_for(2), 0x8000);
        assert_eq!(msb_for(4), 0x8000_0000);
    }

    #[test]
    fn sext_examples() {
        assert_eq!(sext8(0xFF), -1);
        assert_eq!(sext8(0x7F), 127);
        assert_eq!(sext16(0x8000), -32768);
        assert_eq!(sext16(0x7FFF), 32767);
    }

    #[test]
    fn trap_15_word_value() {
        assert_eq!(TRAP_15_WORD, 0x4E4F);
    }

    #[test]
    fn cc_t_always_true_cc_f_always_false() {
        assert!(cc_taken(0, false, false, false, false));
        assert!(!cc_taken(1, true, true, true, true));
    }

    #[test]
    fn cc_eq_ne() {
        assert!(cc_taken(7, false, true, false, false)); // EQ: Z set
        assert!(!cc_taken(6, false, true, false, false)); // NE: Z set -> false
    }

    #[test]
    fn cc_ge_lt_gt_le_signed() {
        // N == V -> GE true, LT false
        assert!(cc_taken(12, true, false, true, false));
        assert!(!cc_taken(13, true, false, true, false));
        // N != V -> LT true, GE false
        assert!(cc_taken(13, true, false, false, false));
        assert!(!cc_taken(12, true, false, false, false));
    }

    #[test]
    fn cc_names_len_and_order() {
        assert_eq!(CC_NAMES.len(), 16);
        assert_eq!(CC_NAMES[0], "T");
        assert_eq!(CC_NAMES[7], "EQ");
        assert_eq!(CC_NAMES[15], "LE");
    }
}
