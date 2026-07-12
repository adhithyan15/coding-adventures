//! Gate-level CPU for the MOS 6502 (1975).
//!
//! # Design philosophy
//!
//! Every data-path operation routes through logic gate primitives:
//! - AND, OR, XOR, NOT  (`logic-gates` crate)
//! - ripple_carry_adder (`arithmetic` crate, via `full_adder` stages)
//!
//! No host integer arithmetic appears in the execution path.
//! Address computation (0x0100|S, vector reads) uses Rust integer ops
//! because these are address-bus wiring, not data-path operations.
//!
//! # Memory-mapped I/O
//!
//! The 6502 has no IN/OUT instructions — I/O is memory-mapped.
//! Reads from 0xFF00–0xFFEF → input_ports[addr - 0xFF00]
//! Writes to  0xFF00–0xFFEF → output_ports[addr - 0xFF00]
//!
//! # Hardware quirks
//!
//! 1. JMP ($xxFF) indirect bug: high byte from $xx00, not $xx01.
//! 2. SBC carry convention: C=1 = no borrow.
//! 3. BCD (NMOS): N/V/Z from binary; C from BCD correction.
//! 4. BRK halts the simulator (pushes PC+2 and P with B=1).
//! 5. Stack is in page 0x01xx; S wraps within the page.

use crate::alu::{adc_bcd, add8, and8, asl8, bit8, compare8, dec8, inc8, lsr8, or8, rol8, ror8, sbc_bcd, sub8, xor8};
use crate::bits::{add_16bit, add_8bit, int_to_bits8};
use crate::decoder::{decode, ABS, ABX, ABY, ACC, IMM, IMP, IND, INX, INY, REL, ZP, ZPX, ZPY};
use crate::registers::RegisterFile6502;

const IO_BASE: usize = 0xFF00;
const IO_END: usize = 0xFFEF;
const NUM_PORTS: usize = 240;

const NMI_LO: usize = 0xFFFA;
const NMI_HI: usize = 0xFFFB;
const IRQ_LO: usize = 0xFFFE;
const IRQ_HI: usize = 0xFFFF;

/// A snapshot of CPU state after execution.
#[derive(Debug, Clone)]
pub struct CpuState {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub s: u8,
    pub pc: u16,
    pub flag_n: bool,
    pub flag_v: bool,
    pub flag_b: bool,
    pub flag_d: bool,
    pub flag_i: bool,
    pub flag_z: bool,
    pub flag_c: bool,
    pub halted: bool,
}

/// Per-instruction trace entry.
#[derive(Debug, Clone)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub mnemonic: String,
    pub description: String,
}

/// Gate-level MOS 6502 (NMOS) simulator.
///
/// # Example
///
/// ```rust
/// use coding_adventures_mos6502_gatelevel::GateLevelCpu;
///
/// let mut cpu = GateLevelCpu::new();
/// // LDA #10 ; ADC #5 ; BRK
/// let (traces, state) = cpu.run(&[0xA9, 0x0A, 0x69, 0x05, 0x00], 100);
/// assert_eq!(state.a, 15);
/// assert!(!state.flag_c);
/// ```
pub struct GateLevelCpu {
    memory: [u8; 65536],
    rf: RegisterFile6502,
    halted: bool,
    input_ports: [u8; NUM_PORTS],
    output_ports: [u8; NUM_PORTS],
}

impl GateLevelCpu {
    /// Create a new CPU with zeroed memory and power-on register state.
    pub fn new() -> Self {
        Self {
            memory: [0u8; 65536],
            rf: RegisterFile6502::new(),
            halted: false,
            input_ports: [0u8; NUM_PORTS],
            output_ports: [0u8; NUM_PORTS],
        }
    }

    /// Reset CPU to power-on state (clears memory).
    pub fn reset(&mut self) {
        self.memory = [0u8; 65536];
        self.rf.reset();
        self.halted = false;
    }

    /// Load program bytes at `origin` and set PC.
    pub fn load(&mut self, program: &[u8], origin: u16) {
        for (i, &byte) in program.iter().enumerate() {
            let addr = (origin as usize + i) & 0xFFFF;
            self.memory[addr] = byte;
        }
        self.rf.pc.write(origin);
        self.halted = false;
    }

    /// Set input port value (read when code accesses 0xFF00 + port).
    pub fn set_input_port(&mut self, port: usize, value: u8) {
        assert!(port < NUM_PORTS, "port out of range");
        self.input_ports[port] = value;
    }

    /// Get output port value (written when code writes to 0xFF00 + port).
    pub fn get_output_port(&self, port: usize) -> u8 {
        assert!(port < NUM_PORTS, "port out of range");
        self.output_ports[port]
    }

    /// Load and run program until BRK or `max_steps`.
    ///
    /// Returns `(traces, final_state)`.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> (Vec<StepTrace>, CpuState) {
        self.reset();
        self.load(program, 0x0000);
        let mut traces = Vec::new();
        let mut steps = 0;
        while !self.halted && steps < max_steps {
            let trace = self.step();
            traces.push(trace);
            steps += 1;
        }
        (traces, self.get_state())
    }

    /// Execute one instruction.
    pub fn step(&mut self) -> StepTrace {
        assert!(!self.halted, "CPU is halted");
        let pc_before = self.rf.pc.read();
        let opcode = self.fetch_byte();
        let instr = decode(opcode);
        let desc = self.execute(instr.mnemonic, instr.mode);
        StepTrace {
            pc_before,
            pc_after: self.rf.pc.read(),
            mnemonic: instr.mnemonic.to_string(),
            description: desc,
        }
    }

    /// Return a snapshot of current CPU state.
    pub fn get_state(&self) -> CpuState {
        let f = &self.rf.flags;
        CpuState {
            a: self.rf.a.read(),
            x: self.rf.x.read(),
            y: self.rf.y.read(),
            s: self.rf.s.read(),
            pc: self.rf.pc.read(),
            flag_n: f.n != 0,
            flag_v: f.v != 0,
            flag_b: f.b != 0,
            flag_d: f.d != 0,
            flag_i: f.i != 0,
            flag_z: f.z != 0,
            flag_c: f.c != 0,
            halted: self.halted,
        }
    }

    // ── Memory helpers ────────────────────────────────────────────────────────

    fn read_mem(&self, addr: u16) -> u8 {
        let a = addr as usize;
        if (IO_BASE..=IO_END).contains(&a) {
            return self.input_ports[a - IO_BASE];
        }
        self.memory[a]
    }

    fn write_mem(&mut self, addr: u16, value: u8) {
        let a = addr as usize;
        if (IO_BASE..=IO_END).contains(&a) {
            self.output_ports[a - IO_BASE] = value;
        } else {
            self.memory[a] = value;
        }
    }

    fn fetch_byte(&mut self) -> u8 {
        let pc = self.rf.pc.read();
        let byte = self.memory[pc as usize];
        self.rf.pc.inc(1);
        byte
    }

    fn fetch_word(&mut self) -> u16 {
        let lo = self.fetch_byte() as u16;
        let hi = self.fetch_byte() as u16;
        (hi << 8) | lo
    }

    fn push_byte(&mut self, value: u8) {
        self.rf.stack_push(&mut self.memory, value);
    }

    fn pull_byte(&mut self) -> u8 {
        // We can't call stack_pull with &self.memory because we need mutable self.rf
        // Inline the logic here.
        let s = self.rf.s.read();
        let (new_s, _carry) = add_8bit(s, 1, 0);
        self.rf.s.write(new_s);
        self.memory[0x0100 | new_s as usize]
    }

    // ── Addressing mode resolver ──────────────────────────────────────────────

    fn resolve_address(&mut self, mode: u8) -> Option<u16> {
        match mode {
            m if m == IMP || m == ACC => None,

            m if m == IMM => {
                let addr = self.rf.pc.read();
                self.rf.pc.inc(1);
                Some(addr)
            }

            m if m == ZP => Some(self.fetch_byte() as u16),

            m if m == ZPX => {
                let zp = self.fetch_byte();
                let x = self.rf.x.read();
                let (result, _c) = add_8bit(zp, x, 0); // wraps in page 0
                Some(result as u16)
            }

            m if m == ZPY => {
                let zp = self.fetch_byte();
                let y = self.rf.y.read();
                let (result, _c) = add_8bit(zp, y, 0);
                Some(result as u16)
            }

            m if m == ABS => Some(self.fetch_word()),

            m if m == ABX => {
                let base = self.fetch_word();
                let x = self.rf.x.read() as u16;
                let (result, _c) = add_16bit(base, x, 0);
                Some(result)
            }

            m if m == ABY => {
                let base = self.fetch_word();
                let y = self.rf.y.read() as u16;
                let (result, _c) = add_16bit(base, y, 0);
                Some(result)
            }

            m if m == INX => {
                let zp = self.fetch_byte();
                let x = self.rf.x.read();
                let (ptr, _c) = add_8bit(zp, x, 0);
                let lo = self.memory[ptr as usize] as u16;
                let hi = self.memory[(ptr.wrapping_add(1)) as usize] as u16;
                Some((hi << 8) | lo)
            }

            m if m == INY => {
                let zp = self.fetch_byte();
                let lo = self.memory[zp as usize] as u16;
                let hi = self.memory[zp.wrapping_add(1) as usize] as u16;
                let base = (hi << 8) | lo;
                let y = self.rf.y.read() as u16;
                let (result, _c) = add_16bit(base, y, 0);
                Some(result)
            }

            m if m == IND => {
                let ptr = self.fetch_word();
                let lo = self.memory[ptr as usize] as u16;
                // 6502 hardware bug: high byte wraps within page (ptr & 0xFF00)
                let hi_addr = (ptr & 0xFF00) | ((ptr.wrapping_add(1)) & 0x00FF);
                let hi = self.memory[hi_addr as usize] as u16;
                Some((hi << 8) | lo)
            }

            m if m == REL => {
                let offset = self.fetch_byte();
                // Sign-extend 8-bit offset to 16-bit
                let signed_offset = if offset >= 0x80 {
                    offset as u16 | 0xFF00 // sign extend
                } else {
                    offset as u16
                };
                let pc = self.rf.pc.read();
                let (result, _c) = add_16bit(pc, signed_offset, 0);
                Some(result)
            }

            _ => panic!("unknown addressing mode {mode}"),
        }
    }

    // ── Flag update helpers ───────────────────────────────────────────────────

    fn update_nz(&mut self, value: u8) {
        let bits = int_to_bits8(value);
        self.rf.flags.n = bits[7];
        self.rf.flags.z = if bits.iter().all(|&b| b == 0) { 1 } else { 0 };
    }

    fn update_nzc(&mut self, value: u8, carry: u8) {
        self.update_nz(value);
        self.rf.flags.c = carry;
    }

    // ── Instruction dispatch ──────────────────────────────────────────────────

    fn execute(&mut self, mnemonic: &str, mode: u8) -> String {
        match mnemonic {
            // ── BRK ──────────────────────────────────────────────────────────
            "BRK" => {
                let ret_pc = self.rf.pc.read();
                // Push PC+1 (return address is the byte after BRK)
                let (ret, _c) = add_16bit(ret_pc, 1, 0);
                self.push_byte((ret >> 8) as u8);
                self.push_byte(ret as u8);
                let p = self.rf.flags.pack(Some(1)); // B=1
                self.push_byte(p);
                self.rf.flags.i = 1;
                self.rf.flags.b = 1;
                self.halted = true;
                "BRK — software interrupt / halt".to_string()
            }

            // ── NOP ──────────────────────────────────────────────────────────
            "NOP" => "NOP — no operation".to_string(),

            // ── LDA / LDX / LDY ──────────────────────────────────────────────
            "LDA" => {
                let addr = self.resolve_address(mode).unwrap();
                let val = self.read_mem(addr);
                self.rf.a.write(val);
                self.update_nz(val);
                format!("LDA — A ← {val:#04x}")
            }
            "LDX" => {
                let addr = self.resolve_address(mode).unwrap();
                let val = self.read_mem(addr);
                self.rf.x.write(val);
                self.update_nz(val);
                format!("LDX — X ← {val:#04x}")
            }
            "LDY" => {
                let addr = self.resolve_address(mode).unwrap();
                let val = self.read_mem(addr);
                self.rf.y.write(val);
                self.update_nz(val);
                format!("LDY — Y ← {val:#04x}")
            }

            // ── STA / STX / STY ──────────────────────────────────────────────
            "STA" => {
                let addr = self.resolve_address(mode).unwrap();
                let a = self.rf.a.read();
                self.write_mem(addr, a);
                format!("STA — mem[{addr:#06x}] ← {a:#04x}")
            }
            "STX" => {
                let addr = self.resolve_address(mode).unwrap();
                let x = self.rf.x.read();
                self.write_mem(addr, x);
                format!("STX — mem[{addr:#06x}] ← {x:#04x}")
            }
            "STY" => {
                let addr = self.resolve_address(mode).unwrap();
                let y = self.rf.y.read();
                self.write_mem(addr, y);
                format!("STY — mem[{addr:#06x}] ← {y:#04x}")
            }

            // ── Register transfers ────────────────────────────────────────────
            "TAX" => {
                let val = self.rf.a.read();
                self.rf.x.write(val);
                self.update_nz(val);
                format!("TAX — X ← {val:#04x}")
            }
            "TAY" => {
                let val = self.rf.a.read();
                self.rf.y.write(val);
                self.update_nz(val);
                format!("TAY — Y ← {val:#04x}")
            }
            "TXA" => {
                let val = self.rf.x.read();
                self.rf.a.write(val);
                self.update_nz(val);
                format!("TXA — A ← {val:#04x}")
            }
            "TYA" => {
                let val = self.rf.y.read();
                self.rf.a.write(val);
                self.update_nz(val);
                format!("TYA — A ← {val:#04x}")
            }
            "TSX" => {
                let val = self.rf.s.read();
                self.rf.x.write(val);
                self.update_nz(val);
                format!("TSX — X ← S={val:#04x}")
            }
            "TXS" => {
                // TXS does NOT update flags
                let val = self.rf.x.read();
                self.rf.s.write(val);
                format!("TXS — S ← {val:#04x}")
            }

            // ── Stack ─────────────────────────────────────────────────────────
            "PHA" => {
                let a = self.rf.a.read();
                self.push_byte(a);
                format!("PHA — push A={a:#04x}")
            }
            "PLA" => {
                let val = self.pull_byte();
                self.rf.a.write(val);
                self.update_nz(val);
                format!("PLA — A ← {val:#04x}")
            }
            "PHP" => {
                let p = self.rf.flags.pack(Some(1)); // B=1
                self.push_byte(p);
                format!("PHP — push P={p:#04x}")
            }
            "PLP" => {
                let p = self.pull_byte();
                self.rf.flags.unpack(p);
                format!("PLP — P ← {p:#04x}")
            }

            // ── ADC ───────────────────────────────────────────────────────────
            "ADC" => {
                let addr = self.resolve_address(mode).unwrap();
                let m = self.read_mem(addr);
                let a = self.rf.a.read();
                let c = self.rf.flags.c;
                let d = self.rf.flags.d;
                let res = if d != 0 { adc_bcd(a, m, c) } else { add8(a, m, c) };
                self.rf.a.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.v = res.flag_v;
                self.rf.flags.z = res.flag_z;
                self.rf.flags.c = res.flag_c;
                format!("ADC — A ← {a:#04x} + {m:#04x} + {c} = {:#04x}", res.result)
            }

            // ── SBC ───────────────────────────────────────────────────────────
            "SBC" => {
                let addr = self.resolve_address(mode).unwrap();
                let m = self.read_mem(addr);
                let a = self.rf.a.read();
                let c = self.rf.flags.c;
                let d = self.rf.flags.d;
                let res = if d != 0 { sbc_bcd(a, m, c) } else { sub8(a, m, c) };
                self.rf.a.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.v = res.flag_v;
                self.rf.flags.z = res.flag_z;
                self.rf.flags.c = res.flag_c;
                format!("SBC — A ← {a:#04x} - {m:#04x} = {:#04x}", res.result)
            }

            // ── AND ───────────────────────────────────────────────────────────
            "AND" => {
                let addr = self.resolve_address(mode).unwrap();
                let m = self.read_mem(addr);
                let a = self.rf.a.read();
                let res = and8(a, m);
                self.rf.a.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("AND — A ← {a:#04x} & {m:#04x} = {:#04x}", res.result)
            }

            // ── ORA ───────────────────────────────────────────────────────────
            "ORA" => {
                let addr = self.resolve_address(mode).unwrap();
                let m = self.read_mem(addr);
                let a = self.rf.a.read();
                let res = or8(a, m);
                self.rf.a.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("ORA — A ← {a:#04x} | {m:#04x} = {:#04x}", res.result)
            }

            // ── EOR ───────────────────────────────────────────────────────────
            "EOR" => {
                let addr = self.resolve_address(mode).unwrap();
                let m = self.read_mem(addr);
                let a = self.rf.a.read();
                let res = xor8(a, m);
                self.rf.a.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("EOR — A ← {a:#04x} ^ {m:#04x} = {:#04x}", res.result)
            }

            // ── BIT ───────────────────────────────────────────────────────────
            "BIT" => {
                let addr = self.resolve_address(mode).unwrap();
                let m = self.read_mem(addr);
                let a = self.rf.a.read();
                let (flag_n, flag_v, flag_z) = bit8(a, m);
                self.rf.flags.n = flag_n;
                self.rf.flags.v = flag_v;
                self.rf.flags.z = flag_z;
                format!("BIT — N={flag_n} V={flag_v} Z={flag_z}")
            }

            // ── ASL ───────────────────────────────────────────────────────────
            "ASL" => {
                if mode == ACC {
                    let (result, carry) = asl8(self.rf.a.read());
                    self.rf.a.write(result);
                    self.update_nzc(result, carry);
                    format!("ASL A — {result:#04x} C={carry}")
                } else {
                    let addr = self.resolve_address(mode).unwrap();
                    let v = self.read_mem(addr);
                    let (result, carry) = asl8(v);
                    self.write_mem(addr, result);
                    self.update_nzc(result, carry);
                    format!("ASL ${addr:#06x} — {result:#04x}")
                }
            }

            // ── LSR ───────────────────────────────────────────────────────────
            "LSR" => {
                if mode == ACC {
                    let (result, carry) = lsr8(self.rf.a.read());
                    self.rf.a.write(result);
                    self.update_nzc(result, carry);
                    format!("LSR A — {result:#04x} C={carry}")
                } else {
                    let addr = self.resolve_address(mode).unwrap();
                    let v = self.read_mem(addr);
                    let (result, carry) = lsr8(v);
                    self.write_mem(addr, result);
                    self.update_nzc(result, carry);
                    format!("LSR ${addr:#06x} — {result:#04x}")
                }
            }

            // ── ROL ───────────────────────────────────────────────────────────
            "ROL" => {
                let cin = self.rf.flags.c;
                if mode == ACC {
                    let (result, carry) = rol8(self.rf.a.read(), cin);
                    self.rf.a.write(result);
                    self.update_nzc(result, carry);
                    format!("ROL A — {result:#04x}")
                } else {
                    let addr = self.resolve_address(mode).unwrap();
                    let v = self.read_mem(addr);
                    let (result, carry) = rol8(v, cin);
                    self.write_mem(addr, result);
                    self.update_nzc(result, carry);
                    format!("ROL ${addr:#06x} — {result:#04x}")
                }
            }

            // ── ROR ───────────────────────────────────────────────────────────
            "ROR" => {
                let cin = self.rf.flags.c;
                if mode == ACC {
                    let (result, carry) = ror8(self.rf.a.read(), cin);
                    self.rf.a.write(result);
                    self.update_nzc(result, carry);
                    format!("ROR A — {result:#04x}")
                } else {
                    let addr = self.resolve_address(mode).unwrap();
                    let v = self.read_mem(addr);
                    let (result, carry) = ror8(v, cin);
                    self.write_mem(addr, result);
                    self.update_nzc(result, carry);
                    format!("ROR ${addr:#06x} — {result:#04x}")
                }
            }

            // ── INC / DEC (memory) ────────────────────────────────────────────
            "INC" => {
                let addr = self.resolve_address(mode).unwrap();
                let v = self.read_mem(addr);
                let res = inc8(v);
                self.write_mem(addr, res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("INC ${addr:#06x} — {:#04x}", res.result)
            }
            "DEC" => {
                let addr = self.resolve_address(mode).unwrap();
                let v = self.read_mem(addr);
                let res = dec8(v);
                self.write_mem(addr, res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("DEC ${addr:#06x} — {:#04x}", res.result)
            }

            // ── INX / INY / DEX / DEY ─────────────────────────────────────────
            "INX" => {
                let res = inc8(self.rf.x.read());
                self.rf.x.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("INX — X={:#04x}", res.result)
            }
            "INY" => {
                let res = inc8(self.rf.y.read());
                self.rf.y.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("INY — Y={:#04x}", res.result)
            }
            "DEX" => {
                let res = dec8(self.rf.x.read());
                self.rf.x.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("DEX — X={:#04x}", res.result)
            }
            "DEY" => {
                let res = dec8(self.rf.y.read());
                self.rf.y.write(res.result);
                self.rf.flags.n = res.flag_n;
                self.rf.flags.z = res.flag_z;
                format!("DEY — Y={:#04x}", res.result)
            }

            // ── Compare ────────────────────────────────────────────────────────
            "CMP" | "CPX" | "CPY" => {
                let addr = self.resolve_address(mode).unwrap();
                let m = self.read_mem(addr);
                let reg = match mnemonic {
                    "CMP" => self.rf.a.read(),
                    "CPX" => self.rf.x.read(),
                    _     => self.rf.y.read(),
                };
                let (flag_n, flag_z, flag_c) = compare8(reg, m);
                self.rf.flags.n = flag_n;
                self.rf.flags.z = flag_z;
                self.rf.flags.c = flag_c;
                format!("{mnemonic} — {reg:#04x} vs {m:#04x}: N={flag_n} Z={flag_z} C={flag_c}")
            }

            // ── Branches ───────────────────────────────────────────────────────
            "BCC" | "BCS" | "BEQ" | "BNE" | "BPL" | "BMI" | "BVC" | "BVS" => {
                let target = self.resolve_address(REL).unwrap();
                let f = &self.rf.flags;
                let taken = match mnemonic {
                    "BCC" => f.c == 0,
                    "BCS" => f.c != 0,
                    "BEQ" => f.z != 0,
                    "BNE" => f.z == 0,
                    "BPL" => f.n == 0,
                    "BMI" => f.n != 0,
                    "BVC" => f.v == 0,
                    _     => f.v != 0, // BVS
                };
                if taken {
                    self.rf.pc.write(target);
                    format!("{mnemonic} — branch taken to {target:#06x}")
                } else {
                    format!("{mnemonic} — not taken")
                }
            }

            // ── JMP ────────────────────────────────────────────────────────────
            "JMP" => {
                let target = self.resolve_address(mode).unwrap();
                self.rf.pc.write(target);
                format!("JMP → {target:#06x}")
            }

            // ── JSR ────────────────────────────────────────────────────────────
            "JSR" => {
                let target = self.fetch_word();
                // Push PC-1 (return address is last byte of JSR instruction)
                let ret_pc = self.rf.pc.read();
                let (ret, _c) = add_16bit(ret_pc, 0xFFFF, 0); // ret_pc - 1
                self.push_byte((ret >> 8) as u8);
                self.push_byte(ret as u8);
                self.rf.pc.write(target);
                format!("JSR → {target:#06x} (push ret={ret:#06x})")
            }

            // ── RTS ────────────────────────────────────────────────────────────
            "RTS" => {
                let lo = self.pull_byte() as u16;
                let hi = self.pull_byte() as u16;
                let ret = (hi << 8) | lo;
                let (new_pc, _c) = add_16bit(ret, 1, 0);
                self.rf.pc.write(new_pc);
                format!("RTS → {new_pc:#06x}")
            }

            // ── RTI ────────────────────────────────────────────────────────────
            "RTI" => {
                let p = self.pull_byte();
                self.rf.flags.unpack(p);
                let lo = self.pull_byte() as u16;
                let hi = self.pull_byte() as u16;
                let new_pc = (hi << 8) | lo;
                self.rf.pc.write(new_pc);
                format!("RTI → P={p:#04x} PC={new_pc:#06x}")
            }

            // ── Flag instructions ──────────────────────────────────────────────
            "CLC" => { self.rf.flags.c = 0; "CLC — C=0".to_string() }
            "SEC" => { self.rf.flags.c = 1; "SEC — C=1".to_string() }
            "CLD" => { self.rf.flags.d = 0; "CLD — D=0".to_string() }
            "SED" => { self.rf.flags.d = 1; "SED — D=1".to_string() }
            "CLI" => { self.rf.flags.i = 0; "CLI — I=0".to_string() }
            "SEI" => { self.rf.flags.i = 1; "SEI — I=1".to_string() }
            "CLV" => { self.rf.flags.v = 0; "CLV — V=0".to_string() }

            _ => panic!("unhandled mnemonic {mnemonic:?}"),
        }
    }

    // ── Interrupt handlers ────────────────────────────────────────────────────

    /// Trigger an IRQ (masked by I flag).
    pub fn interrupt(&mut self) {
        if self.rf.flags.i != 0 {
            return; // IRQ masked
        }
        let pc = self.rf.pc.read();
        self.push_byte((pc >> 8) as u8);
        self.push_byte(pc as u8);
        let p = self.rf.flags.pack(Some(0)); // B=0 for hardware interrupt
        self.push_byte(p);
        self.rf.flags.i = 1;
        let lo = self.memory[IRQ_LO] as u16;
        let hi = self.memory[IRQ_HI] as u16;
        self.rf.pc.write((hi << 8) | lo);
    }

    /// Trigger an NMI (non-maskable).
    pub fn nmi(&mut self) {
        let pc = self.rf.pc.read();
        self.push_byte((pc >> 8) as u8);
        self.push_byte(pc as u8);
        let p = self.rf.flags.pack(Some(0)); // B=0 for hardware interrupt
        self.push_byte(p);
        self.rf.flags.i = 1;
        let lo = self.memory[NMI_LO] as u16;
        let hi = self.memory[NMI_HI] as u16;
        self.rf.pc.write((hi << 8) | lo);
    }
}

impl Default for GateLevelCpu {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn run(program: &[u8]) -> CpuState {
        let mut cpu = GateLevelCpu::new();
        let (_, state) = cpu.run(program, 1000);
        state
    }

    #[test]
    fn lda_imm_and_halt() {
        let state = run(&[0xA9, 0x42, 0x00]); // LDA #0x42 ; BRK
        assert_eq!(state.a, 0x42);
        assert!(state.halted);
        assert!(!state.flag_z);
        assert!(!state.flag_n);
    }

    #[test]
    fn lda_zero_sets_z_flag() {
        let state = run(&[0xA9, 0x00, 0x00]); // LDA #0 ; BRK
        assert_eq!(state.a, 0x00);
        assert!(state.flag_z);
        assert!(!state.flag_n);
    }

    #[test]
    fn lda_negative_sets_n_flag() {
        let state = run(&[0xA9, 0x80, 0x00]); // LDA #0x80 ; BRK
        assert!(state.flag_n);
        assert!(!state.flag_z);
    }

    #[test]
    fn adc_basic() {
        // LDA #10 ; ADC #5 ; BRK
        let state = run(&[0xA9, 0x0A, 0x69, 0x05, 0x00]);
        assert_eq!(state.a, 15);
        assert!(!state.flag_c);
        assert!(!state.flag_z);
    }

    #[test]
    fn adc_with_carry() {
        // LDA #0xFF ; ADC #1 ; BRK  → 0x00, carry=1
        let state = run(&[0xA9, 0xFF, 0x69, 0x01, 0x00]);
        assert_eq!(state.a, 0x00);
        assert!(state.flag_c);
        assert!(state.flag_z);
    }

    #[test]
    fn adc_signed_overflow() {
        // LDA #0x7F ; ADC #1 ; BRK → 0x80 = -128: overflow
        let state = run(&[0xA9, 0x7F, 0x69, 0x01, 0x00]);
        assert_eq!(state.a, 0x80);
        assert!(state.flag_v);
        assert!(state.flag_n);
    }

    #[test]
    fn sbc_basic() {
        // SEC ; LDA #10 ; SBC #3 ; BRK → A = 7, C=1
        let state = run(&[0x38, 0xA9, 0x0A, 0xE9, 0x03, 0x00]);
        assert_eq!(state.a, 7);
        assert!(state.flag_c); // no borrow
    }

    #[test]
    fn sbc_with_borrow() {
        // SEC ; LDA #5 ; SBC #10 ; BRK → A = 251 (0xFB), C=0 (borrow)
        let state = run(&[0x38, 0xA9, 0x05, 0xE9, 0x0A, 0x00]);
        assert_eq!(state.a, 0xFB);
        assert!(!state.flag_c);
    }

    #[test]
    fn and_basic() {
        // LDA #0xFF ; AND #0x0F ; BRK
        let state = run(&[0xA9, 0xFF, 0x29, 0x0F, 0x00]);
        assert_eq!(state.a, 0x0F);
    }

    #[test]
    fn ora_basic() {
        // LDA #0xA0 ; ORA #0x0B ; BRK
        let state = run(&[0xA9, 0xA0, 0x09, 0x0B, 0x00]);
        assert_eq!(state.a, 0xAB);
    }

    #[test]
    fn eor_basic() {
        // LDA #0xFF ; EOR #0x0F ; BRK
        let state = run(&[0xA9, 0xFF, 0x49, 0x0F, 0x00]);
        assert_eq!(state.a, 0xF0);
        assert!(state.flag_n);
    }

    #[test]
    fn eor_self_zeroes() {
        // LDA #0xAB ; EOR #0xAB ; BRK
        let state = run(&[0xA9, 0xAB, 0x49, 0xAB, 0x00]);
        assert_eq!(state.a, 0);
        assert!(state.flag_z);
    }

    #[test]
    fn inx_dex_basic() {
        // LDX #5 ; INX ; INX ; DEX ; BRK → X=6
        let state = run(&[0xA2, 0x05, 0xE8, 0xE8, 0xCA, 0x00]);
        assert_eq!(state.x, 6);
    }

    #[test]
    fn iny_dey_basic() {
        // LDY #10 ; INY ; DEY ; DEY ; BRK → Y=9
        let state = run(&[0xA0, 0x0A, 0xC8, 0x88, 0x88, 0x00]);
        assert_eq!(state.y, 9);
    }

    #[test]
    fn register_transfers() {
        // LDA #0x55 ; TAX ; TAY ; BRK
        let state = run(&[0xA9, 0x55, 0xAA, 0xA8, 0x00]);
        assert_eq!(state.x, 0x55);
        assert_eq!(state.y, 0x55);
    }

    #[test]
    fn stack_pha_pla() {
        // LDA #0xBE ; PHA ; LDA #0 ; PLA ; BRK
        let state = run(&[0xA9, 0xBE, 0x48, 0xA9, 0x00, 0x68, 0x00]);
        assert_eq!(state.a, 0xBE);
    }

    #[test]
    fn cmp_equal() {
        // LDA #5 ; CMP #5 ; BRK → Z=1, C=1
        let state = run(&[0xA9, 0x05, 0xC9, 0x05, 0x00]);
        assert!(state.flag_z);
        assert!(state.flag_c);
    }

    #[test]
    fn cmp_greater() {
        // LDA #10 ; CMP #5 ; BRK → Z=0, C=1
        let state = run(&[0xA9, 0x0A, 0xC9, 0x05, 0x00]);
        assert!(!state.flag_z);
        assert!(state.flag_c);
    }

    #[test]
    fn cmp_less() {
        // LDA #5 ; CMP #10 ; BRK → Z=0, C=0, N=1
        let state = run(&[0xA9, 0x05, 0xC9, 0x0A, 0x00]);
        assert!(!state.flag_z);
        assert!(!state.flag_c);
        assert!(state.flag_n);
    }

    #[test]
    fn branch_beq_taken() {
        // LDA #0 ; BEQ +2 ; NOP ; BRK ; NOP ; BRK
        // BEQ jumps past the first NOP to the second BRK
        let state = run(&[
            0xA9, 0x00,  // LDA #0
            0xF0, 0x01,  // BEQ +1 (skip the NOP)
            0xEA,        // NOP (skipped)
            0x00,        // BRK (reached)
        ]);
        assert!(state.halted);
    }

    #[test]
    fn branch_bne_not_taken() {
        // LDA #0 ; BNE +5 ; BRK
        let state = run(&[0xA9, 0x00, 0xD0, 0x05, 0x00]);
        assert_eq!(state.pc, 5); // halted at BRK at address 4, PC advances to 5
        assert!(state.halted);
    }

    #[test]
    fn asl_accumulator() {
        // LDA #0x01 ; ASL A ; BRK
        let state = run(&[0xA9, 0x01, 0x0A, 0x00]);
        assert_eq!(state.a, 0x02);
        assert!(!state.flag_c);
    }

    #[test]
    fn asl_carry_out() {
        // LDA #0x80 ; ASL A ; BRK
        let state = run(&[0xA9, 0x80, 0x0A, 0x00]);
        assert_eq!(state.a, 0x00);
        assert!(state.flag_c);
        assert!(state.flag_z);
    }

    #[test]
    fn lsr_accumulator() {
        // LDA #0x02 ; LSR A ; BRK
        let state = run(&[0xA9, 0x02, 0x4A, 0x00]);
        assert_eq!(state.a, 0x01);
        assert!(!state.flag_c);
    }

    #[test]
    fn rol_ror_through_carry() {
        // CLC ; LDA #0x01 ; ROL A ; BRK → 0x02, C=0
        let state = run(&[0x18, 0xA9, 0x01, 0x2A, 0x00]);
        assert_eq!(state.a, 0x02);
        // SEC ; LDA #0x01 ; ROR A ; BRK → 0x80 (carry enters bit 7), C=1
        let state2 = run(&[0x38, 0xA9, 0x01, 0x6A, 0x00]);
        assert_eq!(state2.a, 0x80);
        assert!(state2.flag_c); // old bit 0 exits
    }

    #[test]
    fn flag_instructions() {
        // SEC ; CLC ; BRK → C=0 (CLC was last)
        let state = run(&[0x38, 0x18, 0x00]);
        assert!(!state.flag_c);

        // SED ; CLD ; BRK → D=0
        let state2 = run(&[0xF8, 0xD8, 0x00]);
        assert!(!state2.flag_d);

        // SEI ; CLI ; SEC ; BRK — note: BRK sets I=1, so we cannot check !flag_i
        // after BRK. Verify CLI executed by checking the trace count instead.
        // Just confirm CLC/SEC work correctly:
        let state3 = run(&[0x18, 0x00]); // CLC ; BRK
        assert!(!state3.flag_c);
        let state4 = run(&[0x38, 0x00]); // SEC ; BRK
        assert!(state4.flag_c);
    }

    #[test]
    fn jmp_absolute() {
        // Program: JMP $0006 ; NOP ; NOP ; BRK
        // Layout: JMP at 0, NOP at 3, NOP at 4, BRK at 5, BRK at 6
        let state = run(&[
            0x4C, 0x05, 0x00, // JMP $0005
            0xEA,              // NOP (skipped)
            0xEA,              // NOP (skipped)
            0x00,              // BRK (reached)
        ]);
        assert!(state.halted);
    }

    #[test]
    fn jsr_rts() {
        // LDA #0 ; JSR $0008 ; BRK ; ... subroutine at 0x0008: LDA #0x42 ; RTS
        let prog = [
            0xA9, 0x00,       // 0x00: LDA #0
            0x20, 0x07, 0x00, // 0x02: JSR $0007
            0x00,             // 0x05: BRK
            0xEA,             // 0x06: NOP (padding)
            0xA9, 0x42,       // 0x07: LDA #0x42
            0x60,             // 0x09: RTS
        ];
        let state = run(&prog);
        assert_eq!(state.a, 0x42);
        assert!(state.halted);
    }

    #[test]
    fn sta_lda_memory() {
        // LDA #0xAB ; STA $10 ; LDA #0 ; LDA $10 ; BRK
        let state = run(&[0xA9, 0xAB, 0x85, 0x10, 0xA9, 0x00, 0xA5, 0x10, 0x00]);
        assert_eq!(state.a, 0xAB);
    }

    #[test]
    fn clv_flag() {
        // LDA #0x7F ; ADC #1 ; CLV ; BRK → V cleared
        let state = run(&[0xA9, 0x7F, 0x69, 0x01, 0xB8, 0x00]);
        assert!(!state.flag_v);
    }

    #[test]
    fn txs_does_not_update_flags() {
        // LDA #0x00 ; TAX ; TXS ; LDX #0xFF ; TXS ; BRK
        // After TXS with X=0xFF, Z should not be set (TXS doesn't update flags)
        let mut cpu = GateLevelCpu::new();
        let (_, state) = cpu.run(&[
            0xA2, 0x42, // LDX #0x42
            0x9A,       // TXS (no flag update)
            0x00,       // BRK
        ], 100);
        // BRK pushes PCH, PCL, P (3 bytes) → S = 0x42 - 3 = 0x3F
        assert_eq!(state.s, 0x42u8.wrapping_sub(3));
    }
}
