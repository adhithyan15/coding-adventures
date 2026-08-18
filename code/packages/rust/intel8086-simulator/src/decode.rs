//! Fetch + decode for the curated Intel 8086 instruction subset.
//!
//! # Segmented fetch — `CS:IP`, not a flat program counter
//!
//! Every other fixed/variable-width simulator in this repo
//! (`mips-r2000-simulator`, `arm1-simulator`, `mos6502-simulator`, …)
//! fetches from a flat address space: the program counter *is* the
//! memory address. The 8086 never does this — even fetching the very
//! first opcode byte of the trivial `MOV AX,imm16; HLT` program this
//! crate's `intel8086-backend` smoke test relies on goes through the
//! segmented-memory translation
//!
//! ```text
//! physical_address = (CS << 4) + IP   (masked to 20 bits, 0xFFFFF)
//! ```
//!
//! This is not a decode nicety — it is *the* structural feature that
//! makes the 8086 the 8086 (see `simulator.rs`'s module doc for why this
//! can't be deferred the way, say, `mos6502-backend`'s full addressing-
//! mode support was deferred). [`fetch_decode`] therefore takes `cs`
//! explicitly and reads through [`crate::simulator::phys_addr`] for every
//! byte, including the opcode byte itself.
//!
//! # Why this crate's ModRM decoding is register-only (`mod=11`)
//!
//! The 8086's ModRM byte can address either a register (`mod=11`) or one
//! of eight memory effective-address forms (`mod` ∈ `{00,01,10}`, e.g.
//! `[BX+SI+disp8]`). Effective-address computation is real work (base+
//! index selection, displacement sign-extension, segment-override
//! prefixes, the `mod=00,rm=110` "just `[disp16]`" special case) that is
//! out of scope for this lane's curated core (see `opcodes.rs`'s module
//! doc). [`fetch_decode`] decodes the ModRM byte fully (so callers get a
//! clear diagnostic distinguishing "register operand" from "memory
//! operand, unsupported") but only *resolves* the register case —
//! `mod != 0b11` is a decode error, not a silent misinterpretation.

use cpu_simulator::Memory;

use crate::opcodes::{self, lookup, Format};
use crate::simulator::phys_addr;

/// A fully decoded instruction from this crate's curated subset.
///
/// Field meaning depends on `format`:
///
/// | `format` | `reg` | `rm_reg` | `imm` |
/// |---|---|---|---|
/// | `Implied` | unused (0) | `None` | `None` |
/// | `RegImm16` / `RegImm8` | destination register index | `None` | the immediate |
/// | `RegOnly` | the register `INC`/`DEC` targets | `None` | `None` |
/// | `AccImm16` | `opcodes::REG_AX` | `None` | the immediate |
/// | `ModRegOnly16` | ModRM `reg` field (destination) | ModRM `rm` field (source register) | `None` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoded {
    pub mnemonic: &'static str,
    pub format: Format,
    pub reg: u8,
    pub rm_reg: Option<u8>,
    pub imm: Option<u16>,
    pub opcode: u8,
}

fn read_pc8(mem: &Memory, cs: u16, ip: &mut u16) -> u8 {
    let b = mem.read_byte(phys_addr(cs, *ip));
    *ip = ip.wrapping_add(1);
    b
}

fn read_pc16(mem: &Memory, cs: u16, ip: &mut u16) -> u16 {
    let lo = read_pc8(mem, cs, ip) as u16;
    let hi = read_pc8(mem, cs, ip) as u16;
    lo | (hi << 8)
}

/// Fetch the opcode byte at `CS:*ip`, look it up, and decode whatever
/// further bytes its [`Format`] requires — advancing `*ip` past the
/// whole instruction (still relative to the fixed `cs` in effect for
/// this fetch; this crate does not port segment-override prefixes, so a
/// single instruction never changes which segment it fetches from
/// mid-decode).
///
/// Returns `Err` for a byte outside this crate's curated opcode subset,
/// or for a `ModRegOnly16`-format instruction whose ModRM byte specifies
/// a memory operand (`mod != 0b11`) — see the module doc for why the
/// latter is out of scope rather than silently misdecoded.
pub fn fetch_decode(mem: &Memory, cs: u16, ip: &mut u16) -> Result<Decoded, String> {
    let opcode = read_pc8(mem, cs, ip);
    let (mnemonic, format) =
        lookup(opcode).ok_or_else(|| format!("unsupported opcode {opcode:#04x}"))?;

    match format {
        Format::Implied => Ok(Decoded {
            mnemonic,
            format,
            reg: 0,
            rm_reg: None,
            imm: None,
            opcode,
        }),

        Format::RegImm16 => {
            let reg = opcode - opcodes::MOV_REG_IMM16_BASE;
            let imm = read_pc16(mem, cs, ip);
            Ok(Decoded {
                mnemonic,
                format,
                reg,
                rm_reg: None,
                imm: Some(imm),
                opcode,
            })
        }

        Format::RegImm8 => {
            let reg = opcode - opcodes::MOV_REG_IMM8_BASE;
            let imm = read_pc8(mem, cs, ip) as u16;
            Ok(Decoded {
                mnemonic,
                format,
                reg,
                rm_reg: None,
                imm: Some(imm),
                opcode,
            })
        }

        Format::RegOnly => {
            let base = if mnemonic == "INC" {
                opcodes::INC_REG16_BASE
            } else {
                opcodes::DEC_REG16_BASE
            };
            let reg = opcode - base;
            Ok(Decoded {
                mnemonic,
                format,
                reg,
                rm_reg: None,
                imm: None,
                opcode,
            })
        }

        Format::AccImm16 => {
            let imm = read_pc16(mem, cs, ip);
            Ok(Decoded {
                mnemonic,
                format,
                reg: opcodes::REG_AX,
                rm_reg: None,
                imm: Some(imm),
                opcode,
            })
        }

        Format::ModRegOnly16 => {
            let modrm = read_pc8(mem, cs, ip);
            let mod_bits = (modrm >> 6) & 0x3;
            let reg = (modrm >> 3) & 0x7;
            let rm = modrm & 0x7;
            if mod_bits != 0b11 {
                return Err(format!(
                    "{mnemonic} with ModRM mod={mod_bits:#04b} (memory operand) is not \
                     supported by this minimal-viable Intel 8086 simulator -- only \
                     register-to-register (mod=11) forms are ported; full effective-\
                     address computation ([BX+SI] etc.) is deferred to a future increment"
                ));
            }
            Ok(Decoded {
                mnemonic,
                format,
                reg,
                rm_reg: Some(rm),
                imm: None,
                opcode,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_hlt() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xF4]);
        let mut ip = 0u16;
        let d = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d.mnemonic, "HLT");
        assert_eq!(ip, 1);
    }

    #[test]
    fn decode_mov_ax_imm16() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xB8, 0x2A, 0x00]); // MOV AX, 42
        let mut ip = 0u16;
        let d = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d.mnemonic, "MOV");
        assert_eq!(d.format, Format::RegImm16);
        assert_eq!(d.reg, opcodes::REG_AX);
        assert_eq!(d.imm, Some(42));
        assert_eq!(ip, 3);
    }

    #[test]
    fn decode_mov_reg_imm16_nonzero_register() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xB9, 0x34, 0x12]); // MOV CX, 0x1234
        let mut ip = 0u16;
        let d = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d.reg, opcodes::REG_CX);
        assert_eq!(d.imm, Some(0x1234));
    }

    #[test]
    fn decode_mov_reg_imm8() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xB0, 0x7F]); // MOV AL, 0x7F
        let mut ip = 0u16;
        let d = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d.format, Format::RegImm8);
        assert_eq!(d.reg, opcodes::REG_AL);
        assert_eq!(d.imm, Some(0x7F));
        assert_eq!(ip, 2);
    }

    #[test]
    fn decode_add_ax_imm16() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0x05, 0x01, 0x00]); // ADD AX, 1
        let mut ip = 0u16;
        let d = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d.mnemonic, "ADD");
        assert_eq!(d.format, Format::AccImm16);
        assert_eq!(d.imm, Some(1));
    }

    #[test]
    fn decode_mov_reg_reg16_register_form() {
        let mut mem = Memory::new(65536);
        // MOV CX, AX -- ModRM = 11 001 000 = 0xC8 (mod=11, reg=CX=1, rm=AX=0)
        mem.load_bytes(0, &[0x8B, 0xC8]);
        let mut ip = 0u16;
        let d = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d.mnemonic, "MOV");
        assert_eq!(d.format, Format::ModRegOnly16);
        assert_eq!(d.reg, opcodes::REG_CX);
        assert_eq!(d.rm_reg, Some(opcodes::REG_AX));
        assert_eq!(ip, 2);
    }

    #[test]
    fn decode_modrm_memory_operand_is_an_error() {
        let mut mem = Memory::new(65536);
        // MOV CX, [BX+SI] -- ModRM = 00 001 000 = 0x08 (mod=00, reg=CX, rm=BX+SI)
        mem.load_bytes(0, &[0x8B, 0x08]);
        let mut ip = 0u16;
        let err = fetch_decode(&mem, 0, &mut ip).unwrap_err();
        assert!(err.contains("memory operand"));
    }

    #[test]
    fn decode_inc_dec_reg16() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0x40, 0x48]); // INC AX; DEC AX
        let mut ip = 0u16;
        let d1 = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d1.mnemonic, "INC");
        assert_eq!(d1.reg, opcodes::REG_AX);
        let d2 = fetch_decode(&mem, 0, &mut ip).unwrap();
        assert_eq!(d2.mnemonic, "DEC");
        assert_eq!(d2.reg, opcodes::REG_AX);
    }

    #[test]
    fn decode_unsupported_opcode_errs() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xE8, 0x00, 0x00]); // CALL near -- not in this subset
        let mut ip = 0u16;
        assert!(fetch_decode(&mem, 0, &mut ip).is_err());
    }

    #[test]
    fn decode_fetches_through_nonzero_cs_segment() {
        // Same bytes, but placed at physical address CS<<4 (CS=0x0010,
        // IP=0) rather than address 0 -- proves fetch_decode reads
        // through segmented CS:IP addressing, not a flat IP-as-address
        // shortcut.
        let mut mem = Memory::new(65536);
        let cs = 0x0010u16;
        let phys = phys_addr(cs, 0);
        mem.load_bytes(phys, &[0xB8, 0x2A, 0x00]); // MOV AX, 42
        let mut ip = 0u16;
        let d = fetch_decode(&mem, cs, &mut ip).unwrap();
        assert_eq!(d.mnemonic, "MOV");
        assert_eq!(d.imm, Some(42));
    }
}
