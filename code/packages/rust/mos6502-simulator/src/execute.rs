//! Instruction executor for the MOS 6502 ISA.
//!
//! Direct Rust transcription of the big `_execute` dispatch in the Python
//! original (`mos6502_simulator::simulator::MOS6502Simulator._execute`).
//!
//! # Why `execute` takes `&mut Mos6502Simulator` (unlike `mips-r2000`'s
//! decomposed-field style)
//!
//! `mips-r2000-simulator::execute::execute` takes `regs`/`mem`/`hi`/`lo`
//! as separate parameters because MIPS's register file, memory, and HI/LO
//! are independent pieces of state.  The 6502's instruction set is far more
//! cross-cutting: stack ops (`PHA`/`JSR`/`BRK`/…) touch both `S` *and*
//! memory together, and almost every instruction touches the flag byte.
//! Decomposing that into a MIPS-style parameter list would just mean
//! re-deriving `&mut Mos6502Simulator`'s field set at every call site.
//! `execute` therefore takes the whole simulator by mutable reference —
//! the same shape `arm1-simulator::ARM1::execute_data_processing` (an
//! inherent method taking `&mut self`) uses for the same reason.
//!
//! # No memory-mapped I/O
//!
//! The Python original maps `0xFF00-0xFFEF` to 240 input/output "ports"
//! (there being no dedicated `IN`/`OUT` instructions on the 6502).  This
//! Rust port omits that layer — it is orthogonal to instruction semantics
//! and unused by anything in this lane (the `mos6502-backend` v0.1.0 scope
//! is `LDA #imm` + `BRK`, and the simulator's own unit tests only need
//! ordinary RAM) — every 6502 address here is plain `Memory`.  A future
//! increment can port the port-mapping layer alongside genuine I/O-driving
//! CIR ops if a backend ever needs it.
//!
//! # BCD (decimal-mode) `ADC`/`SBC`
//!
//! Ported faithfully from `flags::bcd_add`/`bcd_sub`, including the classic
//! NMOS gotcha: in decimal mode, `N`/`V`/`Z` still reflect the **binary**
//! result computed before BCD correction — only `C` comes from the
//! BCD-corrected result.  The 65C02 fixes this; NMOS (which this simulator
//! models) does not.  See `flags.rs`'s module doc for the full derivation.

use cpu_simulator::Memory;

use crate::decode::Decoded;
use crate::flags::{
    bcd_add, bcd_sub, compute_nz, compute_overflow_add, compute_overflow_sub, pack_p, unpack_p,
};
use crate::opcodes::AddrMode;
use crate::simulator::Mos6502Simulator;

fn read_mem(mem: &Memory, addr: u16) -> u8 {
    mem.read_byte(addr as usize)
}

fn write_mem(mem: &mut Memory, addr: u16, value: u8) {
    mem.write_byte(addr as usize, value);
}

fn push(sim: &mut Mos6502Simulator, value: u8) {
    let addr = 0x0100 | sim.s as usize;
    sim.mem.write_byte(addr, value);
    sim.s = sim.s.wrapping_sub(1);
}

fn pull(sim: &mut Mos6502Simulator) -> u8 {
    sim.s = sim.s.wrapping_add(1);
    sim.mem.read_byte(0x0100 | sim.s as usize)
}

fn set_nz(sim: &mut Mos6502Simulator, value: u8) {
    let (n, z) = compute_nz(value);
    sim.flag_n = n;
    sim.flag_z = z;
}

fn p_byte(sim: &Mos6502Simulator, b: bool) -> u8 {
    pack_p(sim.flag_n, sim.flag_v, b, sim.flag_d, sim.flag_i, sim.flag_z, sim.flag_c)
}

fn apply_unpacked_p(sim: &mut Mos6502Simulator, p: u8) {
    let (n, v, b, d, i, z, c) = unpack_p(p);
    sim.flag_n = n;
    sim.flag_v = v;
    sim.flag_b = b;
    sim.flag_d = d;
    sim.flag_i = i;
    sim.flag_z = z;
    sim.flag_c = c;
}

/// Execute one already-decoded instruction, mutating `sim` in place.
///
/// `sim.pc` on entry is already the *post-operand* address (`decode::
/// fetch_decode` advanced it past every byte the instruction consumes) —
/// exactly matching the Python original's `self._pc` state at the point
/// `_execute` is invoked from `step()`.  Control-flow instructions
/// (`JMP`/`JSR`/`RTS`/`RTI`/branches) overwrite `sim.pc` explicitly; every
/// other instruction leaves it alone.
///
/// Returns the mnemonic, mirroring `mips_r2000_simulator::simulator::
/// MipsR2000Simulator::step`'s return convention.
#[allow(clippy::too_many_lines)]
pub fn execute(sim: &mut Mos6502Simulator, d: &Decoded) -> &'static str {
    match d.mnemonic {
        "BRK" => {
            // BRK is conventionally 1 byte on the wire but the 6502 treats
            // it as if followed by a signature/padding byte: the pushed
            // return address is PC+1 relative to the already-advanced
            // post-opcode PC (i.e. two bytes past where BRK started).
            let ret = sim.pc.wrapping_add(1);
            push(sim, (ret >> 8) as u8);
            push(sim, ret as u8);
            let p = p_byte(sim, true); // B=1 in the pushed copy
            push(sim, p);
            sim.flag_i = true;
            sim.flag_b = true;
            sim.halted = true;
            "BRK"
        }

        "NOP" => "NOP",

        // ── Load ─────────────────────────────────────────────────────────
        "LDA" => {
            sim.a = read_mem(&sim.mem, d.addr.expect("LDA needs an address"));
            set_nz(sim, sim.a);
            "LDA"
        }
        "LDX" => {
            sim.x = read_mem(&sim.mem, d.addr.expect("LDX needs an address"));
            set_nz(sim, sim.x);
            "LDX"
        }
        "LDY" => {
            sim.y = read_mem(&sim.mem, d.addr.expect("LDY needs an address"));
            set_nz(sim, sim.y);
            "LDY"
        }

        // ── Store ────────────────────────────────────────────────────────
        "STA" => {
            write_mem(&mut sim.mem, d.addr.expect("STA needs an address"), sim.a);
            "STA"
        }
        "STX" => {
            write_mem(&mut sim.mem, d.addr.expect("STX needs an address"), sim.x);
            "STX"
        }
        "STY" => {
            write_mem(&mut sim.mem, d.addr.expect("STY needs an address"), sim.y);
            "STY"
        }

        // ── Register transfers ──────────────────────────────────────────
        "TAX" => { sim.x = sim.a; set_nz(sim, sim.x); "TAX" }
        "TAY" => { sim.y = sim.a; set_nz(sim, sim.y); "TAY" }
        "TXA" => { sim.a = sim.x; set_nz(sim, sim.a); "TXA" }
        "TYA" => { sim.a = sim.y; set_nz(sim, sim.a); "TYA" }
        "TSX" => { sim.x = sim.s; set_nz(sim, sim.x); "TSX" }
        "TXS" => { sim.s = sim.x; "TXS" } // TXS does NOT set flags

        // ── Stack ────────────────────────────────────────────────────────
        "PHA" => { push(sim, sim.a); "PHA" }
        "PLA" => { sim.a = pull(sim); set_nz(sim, sim.a); "PLA" }
        "PHP" => { let p = p_byte(sim, true); push(sim, p); "PHP" }
        "PLP" => { let p = pull(sim); apply_unpacked_p(sim, p); "PLP" }

        // ── ADC / SBC ────────────────────────────────────────────────────
        "ADC" => {
            let m = read_mem(&sim.mem, d.addr.expect("ADC needs an address"));
            let a = sim.a;
            if sim.flag_d {
                let (result, c_out) = bcd_add(a, m, sim.flag_c);
                let bin_result = a.wrapping_add(m).wrapping_add(u8::from(sim.flag_c));
                set_nz(sim, bin_result);
                sim.flag_v = compute_overflow_add(a, m, bin_result);
                sim.flag_c = c_out;
                sim.a = result;
            } else {
                let total = u16::from(a) + u16::from(m) + u16::from(sim.flag_c);
                let result = total as u8;
                set_nz(sim, result);
                sim.flag_v = compute_overflow_add(a, m, result);
                sim.flag_c = total > 0xFF;
                sim.a = result;
            }
            "ADC"
        }
        "SBC" => {
            let m = read_mem(&sim.mem, d.addr.expect("SBC needs an address"));
            let a = sim.a;
            if sim.flag_d {
                let (result, c_out) = bcd_sub(a, m, sim.flag_c);
                let bin_result = a.wrapping_sub(m).wrapping_sub(u8::from(!sim.flag_c));
                set_nz(sim, bin_result);
                sim.flag_v = compute_overflow_sub(a, m, bin_result);
                sim.flag_c = c_out;
                sim.a = result;
            } else {
                // SBC = ADC with the operand inverted.
                let m_inv = !m;
                let total = u16::from(a) + u16::from(m_inv) + u16::from(sim.flag_c);
                let result = total as u8;
                set_nz(sim, result);
                sim.flag_v = compute_overflow_add(a, m_inv, result);
                sim.flag_c = total > 0xFF;
                sim.a = result;
            }
            "SBC"
        }

        // ── Logical ──────────────────────────────────────────────────────
        "AND" => {
            sim.a &= read_mem(&sim.mem, d.addr.expect("AND needs an address"));
            set_nz(sim, sim.a);
            "AND"
        }
        "ORA" => {
            sim.a |= read_mem(&sim.mem, d.addr.expect("ORA needs an address"));
            set_nz(sim, sim.a);
            "ORA"
        }
        "EOR" => {
            sim.a ^= read_mem(&sim.mem, d.addr.expect("EOR needs an address"));
            set_nz(sim, sim.a);
            "EOR"
        }
        "BIT" => {
            let m = read_mem(&sim.mem, d.addr.expect("BIT needs an address"));
            sim.flag_n = m & 0x80 != 0;
            sim.flag_v = m & 0x40 != 0;
            sim.flag_z = (sim.a & m) == 0;
            "BIT"
        }

        // ── Shift / rotate ───────────────────────────────────────────────
        "ASL" => {
            if d.mode == AddrMode::Acc {
                let c = sim.a & 0x80 != 0;
                sim.a <<= 1;
                sim.flag_c = c;
                set_nz(sim, sim.a);
            } else {
                let addr = d.addr.expect("ASL (mem) needs an address");
                let v = read_mem(&sim.mem, addr);
                let c = v & 0x80 != 0;
                let result = v << 1;
                write_mem(&mut sim.mem, addr, result);
                sim.flag_c = c;
                set_nz(sim, result);
            }
            "ASL"
        }
        "LSR" => {
            if d.mode == AddrMode::Acc {
                let c = sim.a & 0x01 != 0;
                sim.a >>= 1;
                sim.flag_c = c;
                set_nz(sim, sim.a);
            } else {
                let addr = d.addr.expect("LSR (mem) needs an address");
                let v = read_mem(&sim.mem, addr);
                let c = v & 0x01 != 0;
                let result = v >> 1;
                write_mem(&mut sim.mem, addr, result);
                sim.flag_c = c;
                set_nz(sim, result);
            }
            "LSR"
        }
        "ROL" => {
            let cin = u8::from(sim.flag_c);
            if d.mode == AddrMode::Acc {
                let c = sim.a & 0x80 != 0;
                sim.a = (sim.a << 1) | cin;
                sim.flag_c = c;
                set_nz(sim, sim.a);
            } else {
                let addr = d.addr.expect("ROL (mem) needs an address");
                let v = read_mem(&sim.mem, addr);
                let c = v & 0x80 != 0;
                let result = (v << 1) | cin;
                write_mem(&mut sim.mem, addr, result);
                sim.flag_c = c;
                set_nz(sim, result);
            }
            "ROL"
        }
        "ROR" => {
            let cin = u8::from(sim.flag_c) << 7;
            if d.mode == AddrMode::Acc {
                let c = sim.a & 0x01 != 0;
                sim.a = (sim.a >> 1) | cin;
                sim.flag_c = c;
                set_nz(sim, sim.a);
            } else {
                let addr = d.addr.expect("ROR (mem) needs an address");
                let v = read_mem(&sim.mem, addr);
                let c = v & 0x01 != 0;
                let result = (v >> 1) | cin;
                write_mem(&mut sim.mem, addr, result);
                sim.flag_c = c;
                set_nz(sim, result);
            }
            "ROR"
        }

        // ── INC/DEC (memory) ─────────────────────────────────────────────
        "INC" => {
            let addr = d.addr.expect("INC needs an address");
            let v = read_mem(&sim.mem, addr).wrapping_add(1);
            write_mem(&mut sim.mem, addr, v);
            set_nz(sim, v);
            "INC"
        }
        "DEC" => {
            let addr = d.addr.expect("DEC needs an address");
            let v = read_mem(&sim.mem, addr).wrapping_sub(1);
            write_mem(&mut sim.mem, addr, v);
            set_nz(sim, v);
            "DEC"
        }

        // ── INX/INY/DEX/DEY ──────────────────────────────────────────────
        "INX" => { sim.x = sim.x.wrapping_add(1); set_nz(sim, sim.x); "INX" }
        "INY" => { sim.y = sim.y.wrapping_add(1); set_nz(sim, sim.y); "INY" }
        "DEX" => { sim.x = sim.x.wrapping_sub(1); set_nz(sim, sim.x); "DEX" }
        "DEY" => { sim.y = sim.y.wrapping_sub(1); set_nz(sim, sim.y); "DEY" }

        // ── Compare ──────────────────────────────────────────────────────
        "CMP" => {
            let m = read_mem(&sim.mem, d.addr.expect("CMP needs an address"));
            let diff = sim.a.wrapping_sub(m);
            set_nz(sim, diff);
            sim.flag_c = sim.a >= m;
            "CMP"
        }
        "CPX" => {
            let m = read_mem(&sim.mem, d.addr.expect("CPX needs an address"));
            let diff = sim.x.wrapping_sub(m);
            set_nz(sim, diff);
            sim.flag_c = sim.x >= m;
            "CPX"
        }
        "CPY" => {
            let m = read_mem(&sim.mem, d.addr.expect("CPY needs an address"));
            let diff = sim.y.wrapping_sub(m);
            set_nz(sim, diff);
            sim.flag_c = sim.y >= m;
            "CPY"
        }

        // ── Branches ─────────────────────────────────────────────────────
        "BCC" => branch(sim, d, !sim.flag_c),
        "BCS" => branch(sim, d, sim.flag_c),
        "BEQ" => branch(sim, d, sim.flag_z),
        "BNE" => branch(sim, d, !sim.flag_z),
        "BPL" => branch(sim, d, !sim.flag_n),
        "BMI" => branch(sim, d, sim.flag_n),
        "BVC" => branch(sim, d, !sim.flag_v),
        "BVS" => branch(sim, d, sim.flag_v),

        // ── Jumps / calls ────────────────────────────────────────────────
        "JMP" => {
            sim.pc = d.addr.expect("JMP needs a target");
            "JMP"
        }
        "JSR" => {
            let target = d.addr.expect("JSR needs a target");
            // JSR pushes (return_address - 1); RTS adds 1 back.
            let ret = sim.pc.wrapping_sub(1);
            push(sim, (ret >> 8) as u8);
            push(sim, ret as u8);
            sim.pc = target;
            "JSR"
        }
        "RTS" => {
            let lo = pull(sim) as u16;
            let hi = pull(sim) as u16;
            sim.pc = ((hi << 8) | lo).wrapping_add(1);
            "RTS"
        }
        "RTI" => {
            let p = pull(sim);
            apply_unpacked_p(sim, p);
            let lo = pull(sim) as u16;
            let hi = pull(sim) as u16;
            sim.pc = (hi << 8) | lo; // RTI does NOT add 1
            "RTI"
        }

        // ── Flag instructions ────────────────────────────────────────────
        "CLC" => { sim.flag_c = false; "CLC" }
        "SEC" => { sim.flag_c = true; "SEC" }
        "CLD" => { sim.flag_d = false; "CLD" }
        "SED" => { sim.flag_d = true; "SED" }
        "CLI" => { sim.flag_i = false; "CLI" }
        "SEI" => { sim.flag_i = true; "SEI" }
        "CLV" => { sim.flag_v = false; "CLV" }

        other => panic!("mos6502-simulator: unhandled mnemonic {other:?} (opcode {:#04x}) -- \
                          every mnemonic in opcodes::lookup's table must have an execute arm", d.opcode),
    }
}

fn branch(sim: &mut Mos6502Simulator, d: &Decoded, condition: bool) -> &'static str {
    if condition {
        sim.pc = d.addr.expect("branch needs a target");
    }
    // else: sim.pc already sits at the fallthrough address (decode
    // consumed the offset byte but did not apply it).
    match d.mnemonic {
        "BCC" => "BCC", "BCS" => "BCS", "BEQ" => "BEQ", "BNE" => "BNE",
        "BPL" => "BPL", "BMI" => "BMI", "BVC" => "BVC", "BVS" => "BVS",
        _ => unreachable!(),
    }
}
