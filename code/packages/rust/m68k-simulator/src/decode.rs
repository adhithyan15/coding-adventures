//! Effective-address (EA) resolution and PC-relative fetch helpers.
//!
//! # Why EA resolution lives here, not folded into `execute.rs`
//!
//! Every 68000 instruction that touches memory shares the *same* 6-bit
//! `mode:reg` effective-address field and the *same* resolution rules —
//! this is what "orthogonal addressing" means on the 68000, and it's why
//! the Python original centralises `_ea_address`/`_ea_read`/`_ea_write`
//! as three methods every `_exec_line*` dispatcher calls into, rather
//! than each instruction re-deriving its own operand-fetch logic.  This
//! module is the direct Rust transcription of those three methods (plus
//! the PC-fetch helpers `_fetch_word`/`_fetch_long`/`_fetch_word_signed`
//! they depend on).
//!
//! # Addressing modes ported vs. deferred
//!
//! Of the 11 non-register-direct effective-address variants the 68000
//! defines, this port implements 6 (plus the 2 register-direct modes,
//! plus immediate — 9 of 12 rows in the full EA table, or "8 of 11
//! addressing-mode variants" counting only the true *memory* EA rows
//! beyond bare register-direct):
//!
//! | Mode | Syntax | Ported? |
//! |------|--------|---------|
//! | `000` | `Dn` (data register direct) | ✅ |
//! | `001` | `An` (address register direct) | ✅ |
//! | `010` | `(An)` (indirect) | ✅ |
//! | `011` | `(An)+` (postincrement) | ✅ |
//! | `100` | `-(An)` (predecrement) | ✅ |
//! | `101` | `d16(An)` (16-bit displacement) | ✅ |
//! | `110` | `d8(An,Xn.sz)` (indexed) | ❌ deferred |
//! | `111.000` | `(abs).W` (absolute short) | ✅ |
//! | `111.001` | `(abs).L` (absolute long) | ✅ |
//! | `111.010` | `d16(PC)` (PC-relative) | ❌ deferred |
//! | `111.011` | `d8(PC,Xn.sz)` (PC-relative indexed) | ❌ deferred |
//! | `111.100` | `#imm` (immediate) | ✅ |
//!
//! The 3 deferred modes are exactly the ones that require decoding a
//! **second** extension word carrying an index-register selector, a
//! Dn-vs-An bit, and a word/long size bit (`ext = 1 xxx x sz 0 dddddddd`
//! in the Python original's `_ea_address`) — the most intricate
//! addressing-mode machinery on the chip, and not needed by
//! `m68k-backend`'s minimal-viable `MOVE.L #imm, D0` scope.
//! [`decode_ea`] returns `Err` for them (and for reserved `mode=7,reg∈{5,6,7}`
//! encodings) rather than silently misinterpreting the extension word
//! stream — a wrong guess here would desync every subsequent fetch.

use cpu_simulator::Memory;

use crate::opcodes::{sext16, ADDR_MASK};
use crate::simulator::M68kSimulator;

/// A resolved effective-address *mode* — everything [`ea_read`]/
/// [`ea_write`]/[`ea_address`] need to know once the `mode:reg` opword
/// field has been classified.  Deliberately does **not** carry a
/// pre-computed address: several variants (postincrement, predecrement,
/// displacement, absolute) must consume PC-relative extension words or
/// mutate an address register as a *side effect* of being read, so the
/// address can only be computed at the point of use — see
/// [`ea_address`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EaMode {
    /// `Dn` — data register direct.
    Dn(u8),
    /// `An` — address register direct.
    An(u8),
    /// `(An)` — address register indirect.
    Ind(u8),
    /// `(An)+` — indirect with postincrement.
    PostInc(u8),
    /// `-(An)` — indirect with predecrement.
    PreDec(u8),
    /// `d16(An)` — indirect plus a signed 16-bit displacement extension
    /// word.
    Disp16(u8),
    /// `(abs).W` — absolute short (a 16-bit extension word, sign-extended
    /// to the 24-bit address bus).
    AbsShort,
    /// `(abs).L` — absolute long (a 32-bit extension long-word).
    AbsLong,
    /// `#imm` — immediate data (the operand bytes follow the opword
    /// directly; word immediates use a 16-bit extension even for `.B`
    /// operands, matching real 68000 instruction alignment).
    Imm,
}

/// Classify a 6-bit `mode:reg` effective-address field.  Consumes no
/// bytes and mutates no state — pure decode.
///
/// Returns `Err` for the 3 deferred indexed/PC-relative modes (see the
/// module doc) and for the two reserved `mode=7` sub-encodings
/// (`MOVE #imm,CCR`/`MOVE #imm,SR` claim `reg=4`/`reg=5` in some
/// instruction families, but as a plain *addressing mode* `reg=5..=7`
/// under `mode=7` has no defined meaning here).
pub fn decode_ea(mode: u8, reg: u8) -> Result<EaMode, String> {
    match mode {
        0 => Ok(EaMode::Dn(reg)),
        1 => Ok(EaMode::An(reg)),
        2 => Ok(EaMode::Ind(reg)),
        3 => Ok(EaMode::PostInc(reg)),
        4 => Ok(EaMode::PreDec(reg)),
        5 => Ok(EaMode::Disp16(reg)),
        6 => Err(format!(
            "indexed addressing mode d8(An,Xn.sz) ({mode:#o}/{reg:#o}) is not \
             supported by m68k-simulator v0.1.0 -- see decode.rs's module doc"
        )),
        7 => match reg {
            0 => Ok(EaMode::AbsShort),
            1 => Ok(EaMode::AbsLong),
            2 => Err(
                "PC-relative addressing mode d16(PC) is not supported by \
                 m68k-simulator v0.1.0 -- see decode.rs's module doc"
                    .to_string(),
            ),
            3 => Err(
                "PC-relative indexed addressing mode d8(PC,Xn.sz) is not \
                 supported by m68k-simulator v0.1.0 -- see decode.rs's module doc"
                    .to_string(),
            ),
            4 => Ok(EaMode::Imm),
            _ => Err(format!("reserved EA encoding mode=7,reg={reg}")),
        },
        _ => Err(format!("invalid EA mode {mode} (must be 0-7)")),
    }
}

// ===========================================================================
// PC-relative fetch helpers
// ===========================================================================

/// Fetch a big-endian 16-bit word at `sim.pc`, advance `pc` by 2 (masked
/// to the 24-bit address bus).
pub fn fetch_word(sim: &mut M68kSimulator) -> u16 {
    let addr = sim.pc as usize;
    let w = (u16::from(sim.mem.read_byte(addr)) << 8) | u16::from(sim.mem.read_byte(addr + 1));
    sim.pc = sim.pc.wrapping_add(2) & ADDR_MASK;
    w
}

/// Fetch a big-endian 32-bit longword at `sim.pc`, advance `pc` by 4.
pub fn fetch_long(sim: &mut M68kSimulator) -> u32 {
    let hi = fetch_word(sim);
    let lo = fetch_word(sim);
    (u32::from(hi) << 16) | u32::from(lo)
}

/// Fetch a big-endian 16-bit word and sign-extend it to `i32` — used for
/// branch/DBcc/LINK displacements.
pub fn fetch_word_signed(sim: &mut M68kSimulator) -> i32 {
    sext16(fetch_word(sim))
}

/// Fetch an immediate operand of `sz` bytes.  Byte immediates still
/// consume a full 16-bit extension word (the low byte is the value) —
/// mirrors the Python original's `_fetch_imm` and real 68000 instruction
/// alignment (every extension word is 2 bytes, even for `.B` operands).
pub fn fetch_imm(sim: &mut M68kSimulator, sz: u8) -> u32 {
    if sz == 4 {
        fetch_long(sim)
    } else {
        u32::from(fetch_word(sim)) & crate::opcodes::mask_for(sz)
    }
}

// ===========================================================================
// Big-endian memory access (checked alignment)
// ===========================================================================

/// Read `sz` bytes (1, 2, or 4) from `addr`, big-endian.  Returns `Err`
/// for a misaligned word/long access (real 68000 silicon raises an
/// address-error exception; this simulator has no exception channel, so
/// the caller propagates the error up to `step()`, which halts).
pub fn mem_read(mem: &Memory, addr: u32, sz: u8) -> Result<u32, String> {
    if sz >= 2 && addr & 1 != 0 {
        return Err(format!("misaligned {sz}-byte read at {addr:#08x}"));
    }
    let a = addr as usize;
    Ok(match sz {
        1 => u32::from(mem.read_byte(a)),
        2 => (u32::from(mem.read_byte(a)) << 8) | u32::from(mem.read_byte(a + 1)),
        4 => {
            (u32::from(mem.read_byte(a)) << 24)
                | (u32::from(mem.read_byte(a + 1)) << 16)
                | (u32::from(mem.read_byte(a + 2)) << 8)
                | u32::from(mem.read_byte(a + 3))
        }
        _ => unreachable!("sz is always 1, 2, or 4"),
    })
}

/// Write `sz` bytes (1, 2, or 4) to `addr`, big-endian.  Same alignment
/// rule as [`mem_read`].
pub fn mem_write(mem: &mut Memory, addr: u32, sz: u8, val: u32) -> Result<(), String> {
    if sz >= 2 && addr & 1 != 0 {
        return Err(format!("misaligned {sz}-byte write at {addr:#08x}"));
    }
    let a = addr as usize;
    match sz {
        1 => mem.write_byte(a, val as u8),
        2 => {
            mem.write_byte(a, (val >> 8) as u8);
            mem.write_byte(a + 1, val as u8);
        }
        4 => {
            mem.write_byte(a, (val >> 24) as u8);
            mem.write_byte(a + 1, (val >> 16) as u8);
            mem.write_byte(a + 2, (val >> 8) as u8);
            mem.write_byte(a + 3, val as u8);
        }
        _ => unreachable!("sz is always 1, 2, or 4"),
    }
    Ok(())
}

// ===========================================================================
// Effective-address resolution
// ===========================================================================

/// Compute the memory address an [`EaMode`] refers to, consuming any
/// extension words it needs and applying pre-decrement/post-increment to
/// the address register as a side effect.  Only valid for the 4 "true
/// memory" modes (`Ind`/`PostInc`/`PreDec`/`Disp16`) and the 2 absolute
/// modes — `Dn`/`An`/`Imm` have no memory address (mirrors the Python
/// original's `_ea_address` raising `ValueError` for the same cases).
pub fn ea_address(sim: &mut M68kSimulator, ea: EaMode, sz: u8) -> Result<u32, String> {
    match ea {
        EaMode::Ind(r) => Ok(sim.a[r as usize] & ADDR_MASK),
        EaMode::PostInc(r) => {
            let addr = sim.a[r as usize] & ADDR_MASK;
            // The stack pointer (A7) always moves by >= 2, even for byte
            // accesses -- mirrors the Python original's stack-alignment
            // note (real 68000 silicon keeps A7 word-aligned).
            let inc = if r == 7 { sz.max(2) } else { sz };
            sim.a[r as usize] = sim.a[r as usize].wrapping_add(u32::from(inc)) & ADDR_MASK;
            Ok(addr)
        }
        EaMode::PreDec(r) => {
            let dec = if r == 7 { sz.max(2) } else { sz };
            sim.a[r as usize] = sim.a[r as usize].wrapping_sub(u32::from(dec)) & ADDR_MASK;
            Ok(sim.a[r as usize] & ADDR_MASK)
        }
        EaMode::Disp16(r) => {
            let d16 = fetch_word_signed(sim);
            Ok(((i64::from(sim.a[r as usize]) + i64::from(d16)) as u32) & ADDR_MASK)
        }
        EaMode::AbsShort => {
            let w = fetch_word(sim);
            Ok((sext16(w) as u32) & ADDR_MASK)
        }
        EaMode::AbsLong => Ok(fetch_long(sim) & ADDR_MASK),
        EaMode::Dn(_) | EaMode::An(_) | EaMode::Imm => {
            Err(format!("EA mode {ea:?} has no memory address"))
        }
    }
}

/// Read an `sz`-byte value from an effective address — works for every
/// [`EaMode`] variant, including the 3 register/immediate modes
/// [`ea_address`] rejects.
pub fn ea_read(sim: &mut M68kSimulator, ea: EaMode, sz: u8) -> Result<u32, String> {
    match ea {
        EaMode::Dn(r) => Ok(sim.d[r as usize] & crate::opcodes::mask_for(sz)),
        EaMode::An(r) => Ok(sim.a[r as usize]), // always 32-bit, per the ISA
        EaMode::Imm => Ok(fetch_imm(sim, sz)),
        _ => {
            let addr = ea_address(sim, ea, sz)?;
            mem_read(&sim.mem, addr, sz)
        }
    }
}

/// Write an `sz`-byte value to an effective address.  Writing to `Imm`
/// is not a real destination on the 68000; it falls through to
/// [`ea_address`]'s "no memory address" error.
pub fn ea_write(sim: &mut M68kSimulator, ea: EaMode, sz: u8, val: u32) -> Result<(), String> {
    match ea {
        EaMode::Dn(r) => {
            crate::execute::set_dn(sim, r, val, sz);
            Ok(())
        }
        EaMode::An(r) => {
            // Word writes to An sign-extend to the full 32 bits.
            let v = if sz == 2 {
                sext16(val as u16) as u32
            } else {
                val
            };
            sim.a[r as usize] = v;
            Ok(())
        }
        _ => {
            let addr = ea_address(sim, ea, sz)?;
            mem_write(&mut sim.mem, addr, sz, val)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::M68kSimulator;

    #[test]
    fn decode_register_direct_modes() {
        assert_eq!(decode_ea(0, 3).unwrap(), EaMode::Dn(3));
        assert_eq!(decode_ea(1, 7).unwrap(), EaMode::An(7));
    }

    #[test]
    fn decode_indirect_family() {
        assert_eq!(decode_ea(2, 0).unwrap(), EaMode::Ind(0));
        assert_eq!(decode_ea(3, 0).unwrap(), EaMode::PostInc(0));
        assert_eq!(decode_ea(4, 0).unwrap(), EaMode::PreDec(0));
        assert_eq!(decode_ea(5, 0).unwrap(), EaMode::Disp16(0));
    }

    #[test]
    fn decode_absolute_and_immediate() {
        assert_eq!(decode_ea(7, 0).unwrap(), EaMode::AbsShort);
        assert_eq!(decode_ea(7, 1).unwrap(), EaMode::AbsLong);
        assert_eq!(decode_ea(7, 4).unwrap(), EaMode::Imm);
    }

    #[test]
    fn decode_deferred_modes_error() {
        assert!(decode_ea(6, 0).is_err());
        assert!(decode_ea(7, 2).is_err());
        assert!(decode_ea(7, 3).is_err());
    }

    #[test]
    fn fetch_word_advances_pc_by_two() {
        let mut sim = M68kSimulator::new(64);
        sim.mem.load_bytes(0, &[0x12, 0x34]);
        sim.pc = 0;
        assert_eq!(fetch_word(&mut sim), 0x1234);
        assert_eq!(sim.pc, 2);
    }

    #[test]
    fn fetch_long_big_endian() {
        let mut sim = M68kSimulator::new(64);
        sim.mem.load_bytes(0, &[0xDE, 0xAD, 0xBE, 0xEF]);
        sim.pc = 0;
        assert_eq!(fetch_long(&mut sim), 0xDEAD_BEEF);
        assert_eq!(sim.pc, 4);
    }

    #[test]
    fn ea_read_immediate_long() {
        let mut sim = M68kSimulator::new(64);
        sim.mem.load_bytes(0, &[0x00, 0x00, 0x00, 0x2A]);
        sim.pc = 0;
        let v = ea_read(&mut sim, EaMode::Imm, 4).unwrap();
        assert_eq!(v, 42);
        assert_eq!(sim.pc, 4);
    }

    #[test]
    fn ea_read_write_data_register() {
        let mut sim = M68kSimulator::new(64);
        ea_write(&mut sim, EaMode::Dn(2), 4, 0x1234_5678).unwrap();
        assert_eq!(sim.d[2], 0x1234_5678);
        assert_eq!(ea_read(&mut sim, EaMode::Dn(2), 4).unwrap(), 0x1234_5678);
    }

    #[test]
    fn postinc_and_predec_move_address_register() {
        let mut sim = M68kSimulator::new(64);
        sim.a[0] = 8;
        let addr = ea_address(&mut sim, EaMode::PostInc(0), 4).unwrap();
        assert_eq!(addr, 8);
        assert_eq!(sim.a[0], 12);

        let addr2 = ea_address(&mut sim, EaMode::PreDec(0), 4).unwrap();
        assert_eq!(addr2, 8);
        assert_eq!(sim.a[0], 8);
    }

    #[test]
    fn stack_pointer_postinc_always_moves_by_at_least_two() {
        let mut sim = M68kSimulator::new(64);
        sim.a[7] = 8;
        ea_address(&mut sim, EaMode::PostInc(7), 1).unwrap();
        assert_eq!(sim.a[7], 10, "byte access on A7 still bumps by 2");
    }

    #[test]
    fn misaligned_word_access_errors() {
        let mut sim = M68kSimulator::new(64);
        sim.a[0] = 1; // odd address
        assert!(ea_read(&mut sim, EaMode::Ind(0), 2).is_err());
    }

    #[test]
    fn absolute_short_sign_extends() {
        let mut sim = M68kSimulator::new(0x1_0000);
        sim.mem.load_bytes(0, &[0xFF, 0xF0]); // -16 as i16
        sim.pc = 0;
        let addr = ea_address(&mut sim, EaMode::AbsShort, 2).unwrap();
        assert_eq!(addr, (-16i32 as u32) & ADDR_MASK);
    }
}
