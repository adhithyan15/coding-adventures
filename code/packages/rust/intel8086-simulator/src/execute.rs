//! Instruction executor for this crate's curated Intel 8086 subset.
//!
//! Direct-but-scoped transcription of the relevant arms of the Python
//! original's big `_exec_op` dispatch (`code/packages/python/
//! intel-8086-simulator/src/intel_8086_simulator/simulator.py`). Takes
//! `&mut Intel8086Simulator` rather than decomposed field parameters —
//! same rationale as `mos6502_simulator::execute::execute` (see that
//! module's doc): several instructions here touch both a register *and*
//! flags together, and re-deriving `&mut Intel8086Simulator`'s field set
//! as a parameter list at every call site would just be indirection.
//!
//! # No memory operands in this crate's curated subset
//!
//! Every instruction `decode::fetch_decode` can produce in this crate
//! either has no memory operand at all (`Implied`, `RegImm16`, `RegImm8`,
//! `RegOnly`, `AccImm16`) or has already rejected the memory-operand
//! ModRM case at decode time (`ModRegOnly16` — see `decode.rs`'s module
//! doc). Consequently `execute` never needs to read or write `sim.mem`
//! for data — only `decode::fetch_decode` (via `simulator::phys_addr`)
//! touches memory, for the instruction *fetch* itself.

use crate::decode::Decoded;
use crate::opcodes::Format;
use crate::simulator::Intel8086Simulator;

/// Execute one already-decoded instruction, mutating `sim` in place.
///
/// `sim.ip` on entry is already the *post-operand* address (`decode::
/// fetch_decode` advanced the caller's local `ip` past every byte the
/// instruction consumes, and `Intel8086Simulator::step` copied it back)
/// — this crate's curated subset has no control-flow instructions
/// (`JMP`/`CALL`/branches are all out of scope — see `opcodes.rs`'s
/// module doc), so unlike `mos6502_simulator::execute::execute`, no arm
/// here ever needs to overwrite `sim.ip` itself.
///
/// Returns the mnemonic, mirroring `mos6502_simulator::execute::
/// execute`'s and `MipsR2000Simulator::step`'s return convention.
pub fn execute(sim: &mut Intel8086Simulator, d: &Decoded) -> &'static str {
    match d.mnemonic {
        "HLT" => {
            sim.halted = true;
            "HLT"
        }

        "NOP" => "NOP",

        "MOV" => {
            match d.format {
                Format::RegImm16 => {
                    sim.set_reg16(d.reg, d.imm.expect("RegImm16 always carries an immediate"));
                }
                Format::RegImm8 => {
                    sim.set_reg8(
                        d.reg,
                        d.imm.expect("RegImm8 always carries an immediate") as u8,
                    );
                }
                Format::ModRegOnly16 => {
                    let src = sim.get_reg16(d.rm_reg.expect("ModRegOnly16 always carries rm_reg"));
                    sim.set_reg16(d.reg, src);
                }
                other => unreachable!("MOV decoded with unexpected format {other:?}"),
            }
            "MOV"
        }

        "ADD" | "SUB" | "AND" | "OR" | "XOR" | "CMP" => {
            let a = sim.get_reg16(d.reg);
            let b = match d.format {
                Format::AccImm16 => d.imm.expect("AccImm16 always carries an immediate"),
                Format::ModRegOnly16 => {
                    sim.get_reg16(d.rm_reg.expect("ModRegOnly16 always carries rm_reg"))
                }
                other => unreachable!("{} decoded with unexpected format {other:?}", d.mnemonic),
            };
            let result = sim.alu16(d.mnemonic, a, b);
            // CMP only updates flags -- mirrors the Python original's
            // `alu_op != 7` "CMP does not write back" special case.
            if d.mnemonic != "CMP" {
                sim.set_reg16(d.reg, result);
            }
            d.mnemonic
        }

        "INC" => {
            let old_cf = sim.flag_cf;
            let old = sim.get_reg16(d.reg);
            let result = sim.alu16("ADD", old, 1);
            sim.set_reg16(d.reg, result);
            sim.flag_cf = old_cf; // INC does not affect CF (real 8086 behaviour)
            "INC"
        }

        "DEC" => {
            let old_cf = sim.flag_cf;
            let old = sim.get_reg16(d.reg);
            let result = sim.alu16("SUB", old, 1);
            sim.set_reg16(d.reg, result);
            sim.flag_cf = old_cf; // DEC does not affect CF
            "DEC"
        }

        other => panic!(
            "intel8086-simulator: unhandled mnemonic {other:?} (opcode {:#04x}) -- every \
             mnemonic in opcodes::lookup's table must have an execute arm",
            d.opcode
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::fetch_decode;
    use crate::opcodes;

    fn step_bytes(sim: &mut Intel8086Simulator, bytes: &[u8]) -> &'static str {
        sim.mem.load_bytes(0, bytes);
        let mut ip = 0u16;
        let d = fetch_decode(&sim.mem, sim.cs, &mut ip).unwrap();
        sim.ip = ip;
        execute(sim, &d)
    }

    #[test]
    fn hlt_sets_halted() {
        let mut sim = Intel8086Simulator::new(65536);
        let mnem = step_bytes(&mut sim, &[0xF4]);
        assert_eq!(mnem, "HLT");
        assert!(sim.halted);
    }

    #[test]
    fn mov_reg_imm16_loads_register() {
        let mut sim = Intel8086Simulator::new(65536);
        step_bytes(&mut sim, &[0xB8, 0x34, 0x12]); // MOV AX, 0x1234
        assert_eq!(sim.ax, 0x1234);
    }

    #[test]
    fn mov_reg_imm8_loads_low_byte_only() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.ax = 0xFFFF;
        step_bytes(&mut sim, &[0xB0, 0x42]); // MOV AL, 0x42
        assert_eq!(sim.ax, 0xFF42);
    }

    #[test]
    fn add_updates_flags_and_result() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.ax = 5;
        step_bytes(&mut sim, &[0x05, 0x03, 0x00]); // ADD AX, 3
        assert_eq!(sim.ax, 8);
        assert!(!sim.flag_zf);
    }

    #[test]
    fn cmp_updates_flags_without_writing_back() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.ax = 5;
        step_bytes(&mut sim, &[0x3D, 0x05, 0x00]); // CMP AX, 5
        assert_eq!(sim.ax, 5, "CMP must not modify the register");
        assert!(sim.flag_zf);
    }

    #[test]
    fn mov_reg_reg16_via_modrm() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.ax = 77;
        step_bytes(&mut sim, &[0x8B, 0xD8]); // MOV BX, AX (mod=11,reg=BX=3,rm=AX=0)
        assert_eq!(sim.bx, 77);
    }

    #[test]
    #[should_panic(expected = "unhandled mnemonic")]
    fn execute_panics_on_a_mnemonic_with_no_arm() {
        // Construct a Decoded directly with a mnemonic execute() cannot
        // handle -- proves the exhaustiveness panic fires (this can't
        // happen through fetch_decode, since opcodes::lookup only ever
        // returns mnemonics execute() handles).
        let mut sim = Intel8086Simulator::new(65536);
        let bogus = Decoded {
            mnemonic: "JMP",
            format: opcodes::Format::Implied,
            reg: 0,
            rm_reg: None,
            imm: None,
            opcode: 0xE9,
        };
        execute(&mut sim, &bogus);
    }
}
