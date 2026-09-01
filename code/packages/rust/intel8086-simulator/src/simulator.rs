//! Top-level Intel 8086 simulator: segmented memory + registers +
//! fetch-decode-execute.
//!
//! # Segmented memory — the 8086's defining, structural feature
//!
//! Every physical memory access on the 8086 — instruction fetch, and (in
//! a full port) every data read/write — is computed from a **segment
//! register** and a 16-bit **offset**:
//!
//! ```text
//! physical_address = (segment_register << 4) + offset    (mod 2^20)
//! ```
//!
//! This gives a 1 MiB address space (`0x00000..=0xFFFFF`) built from
//! 16-bit segment×offset pairs, rather than one flat 20-bit program
//! counter. The four segment registers (`CS`/`DS`/`SS`/`ES`) each pick a
//! different 64 KiB *window* into that 1 MiB space; instruction fetch
//! always uses `CS:IP`.
//!
//! **This cannot be flattened or omitted.** Even the trivial two-instruction program this
//! crate's `intel8086-backend` smoke test compiles —
//! `MOV AX,imm16; HLT` — has its very first opcode byte fetched via
//! [`phys_addr`]`(self.cs, self.ip)`, not via `self.ip` alone. A flat-
//! memory shape (the shape every *other* Rust ISA simulator in this repo
//! uses — MIPS R2000, ARM1, MOS 6502, RISC-V) would not be able to
//! represent this at all: it has no separate "segment" concept for
//! `phys_addr` to combine with an offset. Structurally, `Intel8086Simulator`
//! must carry `cs`/`ds`/`ss`/`es` as first-class fields and route every
//! fetch through the segment-aware helper, which is exactly what this
//! module does — mirroring `code/packages/python/intel-8086-simulator`'s
//! `X86Simulator._phys`/`_fetch8`/`_fetch16` faithfully (see that
//! package's `simulator.py` module doc, "MEMORY MODEL" section).
//!
//! # Public API shape
//!
//! Mirrors the repository's `Mos6502Simulator` shape:
//! `new(memory_size)`, public register/flag/`mem`/`halted` fields,
//! `load_program`/`load_program_at`, `run`, `run_loaded_with_limit`
//! returning an `ExecutionResult`, `step() -> String`. The one deviation:
//! `ExecutionResult.pc` becomes `ExecutionResult.ip` (the 8086 term for
//! its program counter — used throughout the Python reference and its
//! own `StepTrace`).

use cpu_simulator::Memory;

use crate::flags;

/// 20-bit physical address mask — a real 8086 has exactly `0xFFFFF`
/// (1,048,575) addressable bytes; `(seg << 4) + offset` can exceed that
/// (e.g. `CS=0xFFFF, IP=0xFFFF` gives `0x10FFEF`), and real silicon wraps
/// it — see `code/packages/python/intel-8086-simulator`'s `_PHYS_MASK`.
const PHYS_MASK: u32 = 0xFFFFF;
const BYTE_MASK: u32 = 0xFF;
const WORD_MASK: u32 = 0xFFFF;
const PORT_COUNT: usize = 256;
const MEMORY_SIZE: usize = 1 << 20;

#[derive(Debug, Clone, Copy)]
struct RmOperand {
    mode: u8,
    rm: u8,
    segment: u16,
    offset: u16,
}

/// Compute the 20-bit physical address `(segment << 4) + offset`, masked
/// to the 8086's 1 MiB address space.
///
/// A free function (not a method) so `decode.rs` can call it without
/// needing a `&Intel8086Simulator` — decode only ever has `cs` and a
/// local `ip`, not the whole simulator (mirrors why `mos6502_simulator::
/// decode::resolve_address` takes `mem`/`pc`/`x`/`y` as plain parameters
/// rather than `&Mos6502Simulator`).
///
/// # Examples
///
/// ```
/// use intel8086_simulator::simulator::phys_addr;
///
/// assert_eq!(phys_addr(0, 0), 0);
/// assert_eq!(phys_addr(0x1000, 0x0010), 0x10010);
/// // CS=0x1000 -> base 0x10000; + offset 0x0010 -> 0x10010.
/// ```
pub fn phys_addr(segment: u16, offset: u16) -> usize {
    ((((segment as u32) << 4) + offset as u32) & PHYS_MASK) as usize
}

/// Complete Intel 8086 simulator: four 16-bit
/// general-purpose registers (with byte-addressable halves), four
/// pointer/index registers, four segment registers, a 16-bit instruction
/// pointer, the six arithmetic/logical flags (CF/PF/AF/ZF/SF/OF, plus
/// TF/IF/DF), and flat byte-addressable memory
/// accessed exclusively through segmented `CS:IP`/`(seg,offset)`
/// addressing.
pub struct Intel8086Simulator {
    // ── General-purpose (16-bit; byte halves via get_reg8/set_reg8) ────
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,

    // ── Index / pointer ──────────────────────────────────────────────
    pub si: u16,
    pub di: u16,
    pub sp: u16,
    pub bp: u16,

    // ── Segment registers ────────────────────────────────────────────
    /// Code segment — `physical instruction fetch = CS<<4 + IP`.
    pub cs: u16,
    pub ds: u16,
    pub ss: u16,
    pub es: u16,

    /// Instruction pointer — 16-bit offset within `CS`.
    pub ip: u16,

    // ── Flags ────────────────────────────────────────────────────────
    pub flag_cf: bool,
    pub flag_pf: bool,
    pub flag_af: bool,
    pub flag_zf: bool,
    pub flag_sf: bool,
    pub flag_tf: bool,
    pub flag_if: bool,
    pub flag_df: bool,
    pub flag_of: bool,

    /// Flat byte-addressable memory, indexed by 20-bit physical address
    /// (see [`phys_addr`]) — NOT by `ip` directly.
    pub mem: Memory,

    /// Byte-wide input and output port latches. The original 8086 has a
    /// 16-bit port address bus; the repository oracle intentionally models
    /// the low 256 ports and wraps word-port accesses at `0xff`.
    pub input_ports: [u8; PORT_COUNT],
    pub output_ports: [u8; PORT_COUNT],

    /// `true` once `HLT` (`opcodes::HLT_OPCODE`) has executed.
    pub halted: bool,

    // Bytes consumed by the current instruction. This is transient trace
    // bookkeeping, not architectural state.
    trace_raw: Vec<u8>,
}

/// A checked Intel 8086 simulator failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intel8086Error {
    ProgramOutOfRange { origin: usize, length: usize },
    InvalidStateMemory { length: usize },
    InvalidPort { port: usize },
    UnknownOpcode { cs: u16, ip: u16, raw: Vec<u8> },
    Halted,
}

impl std::fmt::Display for Intel8086Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProgramOutOfRange { origin, length } => write!(
                formatter,
                "program range {origin:#x}..{:#x} exceeds 1 MiB memory",
                origin.saturating_add(*length)
            ),
            Self::InvalidStateMemory { length } => {
                write!(
                    formatter,
                    "state has {length} memory bytes; expected {MEMORY_SIZE}"
                )
            }
            Self::InvalidPort { port } => write!(formatter, "port {port} is outside 0..255"),
            Self::UnknownOpcode { cs, ip, raw } => {
                write!(formatter, "unknown opcode {raw:02X?} at {cs:04X}:{ip:04X}")
            }
            Self::Halted => formatter.write_str("CPU is halted"),
        }
    }
}

impl std::error::Error for Intel8086Error {}

/// Complete owned architectural state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intel8086State {
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,
    pub si: u16,
    pub di: u16,
    pub sp: u16,
    pub bp: u16,
    pub cs: u16,
    pub ds: u16,
    pub ss: u16,
    pub es: u16,
    pub ip: u16,
    pub flags: u16,
    pub halted: bool,
    pub input_ports: [u8; PORT_COUNT],
    pub output_ports: [u8; PORT_COUNT],
    pub memory: Box<[u8]>,
}

/// Complete before/after record for one successful instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub cs: u16,
    pub ip: u16,
    pub raw: Vec<u8>,
    pub mnemonic: String,
    pub state_before: Intel8086State,
    pub state_after: Intel8086State,
}

/// Observable outcome of a bounded simulator run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub ip: u16,
    pub traces: Vec<StepTrace>,
    pub final_state: Intel8086State,
}

impl Intel8086Simulator {
    /// Create a simulator with the architectural 1 MiB address space.
    ///
    /// The argument is retained for source compatibility and ignored: an
    /// 8086 physical address is always 20 bits, so caller-selected memory
    /// cannot faithfully model segment wraparound.
    pub fn new(_memory_size: usize) -> Self {
        Self {
            ax: 0,
            bx: 0,
            cx: 0,
            dx: 0,
            si: 0,
            di: 0,
            sp: 0,
            bp: 0,
            cs: 0,
            ds: 0,
            ss: 0,
            es: 0,
            ip: 0,
            flag_cf: false,
            flag_pf: false,
            flag_af: false,
            flag_zf: false,
            flag_sf: false,
            flag_tf: false,
            flag_if: false,
            flag_df: false,
            flag_of: false,
            mem: Memory::new(MEMORY_SIZE),
            input_ports: [0; PORT_COUNT],
            output_ports: [0; PORT_COUNT],
            halted: false,
            trace_raw: Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // Register read/write by index (mirrors opcodes::REG_* encoding)
    // ------------------------------------------------------------------

    pub fn get_reg16(&self, reg: u8) -> u16 {
        match reg {
            0 => self.ax,
            1 => self.cx,
            2 => self.dx,
            3 => self.bx,
            4 => self.sp,
            5 => self.bp,
            6 => self.si,
            _ => self.di, // 7
        }
    }

    pub fn set_reg16(&mut self, reg: u8, val: u16) {
        match reg {
            0 => self.ax = val,
            1 => self.cx = val,
            2 => self.dx = val,
            3 => self.bx = val,
            4 => self.sp = val,
            5 => self.bp = val,
            6 => self.si = val,
            _ => self.di = val, // 7
        }
    }

    pub fn get_reg8(&self, reg: u8) -> u8 {
        match reg {
            0 => (self.ax & 0xFF) as u8, // AL
            1 => (self.cx & 0xFF) as u8, // CL
            2 => (self.dx & 0xFF) as u8, // DL
            3 => (self.bx & 0xFF) as u8, // BL
            4 => (self.ax >> 8) as u8,   // AH
            5 => (self.cx >> 8) as u8,   // CH
            6 => (self.dx >> 8) as u8,   // DH
            _ => (self.bx >> 8) as u8,   // BH (7)
        }
    }

    pub fn set_reg8(&mut self, reg: u8, val: u8) {
        let v = val as u16;
        match reg {
            0 => self.ax = (self.ax & 0xFF00) | v,
            1 => self.cx = (self.cx & 0xFF00) | v,
            2 => self.dx = (self.dx & 0xFF00) | v,
            3 => self.bx = (self.bx & 0xFF00) | v,
            4 => self.ax = (self.ax & 0x00FF) | (v << 8),
            5 => self.cx = (self.cx & 0x00FF) | (v << 8),
            6 => self.dx = (self.dx & 0x00FF) | (v << 8),
            _ => self.bx = (self.bx & 0x00FF) | (v << 8), // BH (7)
        }
    }

    fn get_sreg(&self, reg: u8) -> u16 {
        match reg & 3 {
            0 => self.es,
            1 => self.cs,
            2 => self.ss,
            _ => self.ds,
        }
    }

    fn set_sreg(&mut self, reg: u8, value: u16) {
        match reg & 3 {
            0 => self.es = value,
            1 => self.cs = value,
            2 => self.ss = value,
            _ => self.ds = value,
        }
    }

    fn read_byte(&self, segment: u16, offset: u16) -> u8 {
        self.mem.read_byte(phys_addr(segment, offset))
    }

    fn write_byte(&mut self, segment: u16, offset: u16, value: u8) {
        self.mem.write_byte(phys_addr(segment, offset), value);
    }

    fn read_word(&self, segment: u16, offset: u16) -> u16 {
        let lo = self.read_byte(segment, offset) as u16;
        let hi = self.read_byte(segment, offset.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    fn write_word(&mut self, segment: u16, offset: u16, value: u16) {
        self.write_byte(segment, offset, value as u8);
        self.write_byte(segment, offset.wrapping_add(1), (value >> 8) as u8);
    }

    fn fetch8(&mut self) -> u8 {
        let value = self.read_byte(self.cs, self.ip);
        self.ip = self.ip.wrapping_add(1);
        self.trace_raw.push(value);
        value
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch8() as u16;
        let hi = self.fetch8() as u16;
        lo | (hi << 8)
    }

    fn fetch_s8(&mut self) -> i16 {
        self.fetch8() as i8 as i16
    }

    fn fetch_s16(&mut self) -> i16 {
        self.fetch16() as i16
    }

    fn decode_rm(&mut self, modrm: u8, segment_override: Option<u16>) -> RmOperand {
        let mode = modrm >> 6;
        let rm = modrm & 7;
        if mode == 3 {
            return RmOperand {
                mode,
                rm,
                segment: 0,
                offset: rm as u16,
            };
        }

        let uses_bp = matches!(rm, 2 | 3) || (rm == 6 && mode != 0);
        let segment = segment_override.unwrap_or(if uses_bp { self.ss } else { self.ds });
        let mut offset = match rm {
            0 => self.bx.wrapping_add(self.si),
            1 => self.bx.wrapping_add(self.di),
            2 => self.bp.wrapping_add(self.si),
            3 => self.bp.wrapping_add(self.di),
            4 => self.si,
            5 => self.di,
            6 if mode == 0 => self.fetch16(),
            6 => self.bp,
            _ => self.bx,
        };
        if mode == 1 {
            offset = offset.wrapping_add_signed(self.fetch_s8());
        } else if mode == 2 {
            offset = offset.wrapping_add_signed(self.fetch_s16());
        }
        RmOperand {
            mode,
            rm,
            segment,
            offset,
        }
    }

    fn read_rm(&self, operand: RmOperand, word: bool) -> u16 {
        if operand.mode == 3 {
            if word {
                self.get_reg16(operand.rm)
            } else {
                self.get_reg8(operand.rm) as u16
            }
        } else if word {
            self.read_word(operand.segment, operand.offset)
        } else {
            self.read_byte(operand.segment, operand.offset) as u16
        }
    }

    fn write_rm(&mut self, operand: RmOperand, value: u16, word: bool) {
        if operand.mode == 3 {
            if word {
                self.set_reg16(operand.rm, value);
            } else {
                self.set_reg8(operand.rm, value as u8);
            }
        } else if word {
            self.write_word(operand.segment, operand.offset, value);
        } else {
            self.write_byte(operand.segment, operand.offset, value as u8);
        }
    }

    fn push16(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(2);
        self.write_word(self.ss, self.sp, value);
    }

    fn pop16(&mut self) -> u16 {
        let value = self.read_word(self.ss, self.sp);
        self.sp = self.sp.wrapping_add(2);
        value
    }

    fn set_szp(&mut self, result: u16, word: bool) {
        let (sf, zf, pf) = flags::compute_szp(result, word);
        self.flag_sf = sf;
        self.flag_zf = zf;
        self.flag_pf = pf;
    }

    fn add_value(&mut self, a: u16, b: u16, word: bool, carry: u16) -> u16 {
        let mask = if word { WORD_MASK } else { BYTE_MASK };
        let raw = a as u32 + b as u32 + carry as u32;
        let result = (raw & mask) as u16;
        self.flag_cf = flags::compute_cf_add(raw, word);
        self.flag_af = flags::compute_af_add(a, b, carry);
        self.flag_of = flags::compute_of_add(a, b, result, word);
        self.set_szp(result, word);
        result
    }

    fn sub_value(&mut self, a: u16, b: u16, word: bool, borrow: u16) -> u16 {
        let result = if word {
            a.wrapping_sub(b).wrapping_sub(borrow)
        } else {
            (a as u8).wrapping_sub(b as u8).wrapping_sub(borrow as u8) as u16
        };
        self.flag_cf = flags::compute_cf_sub(a as u32, b as u32, borrow as u32);
        self.flag_af = flags::compute_af_sub(a, b, borrow);
        self.flag_of = flags::compute_of_sub(a, b, result, word);
        self.set_szp(result, word);
        result
    }

    fn logic_value(&mut self, operation: u8, a: u16, b: u16, word: bool) -> u16 {
        let mask = if word { WORD_MASK } else { BYTE_MASK };
        let result = match operation {
            1 => a | b,
            4 => a & b,
            _ => a ^ b,
        } & mask as u16;
        self.flag_cf = false;
        self.flag_of = false;
        self.flag_af = false;
        self.set_szp(result, word);
        result
    }

    fn alu_value(&mut self, operation: u8, a: u16, b: u16, word: bool) -> u16 {
        match operation {
            0 => self.add_value(a, b, word, 0),
            1 | 4 | 6 => self.logic_value(operation, a, b, word),
            2 => self.add_value(a, b, word, u16::from(self.flag_cf)),
            3 => self.sub_value(a, b, word, u16::from(self.flag_cf)),
            5 | 7 => self.sub_value(a, b, word, 0),
            _ => unreachable!(),
        }
    }

    fn flags_value(&self) -> u16 {
        u16::from(self.flag_cf)
            | 2
            | (u16::from(self.flag_pf) << 2)
            | (u16::from(self.flag_af) << 4)
            | (u16::from(self.flag_zf) << 6)
            | (u16::from(self.flag_sf) << 7)
            | (u16::from(self.flag_tf) << 8)
            | (u16::from(self.flag_if) << 9)
            | (u16::from(self.flag_df) << 10)
            | (u16::from(self.flag_of) << 11)
    }

    fn load_flags(&mut self, value: u16) {
        self.flag_cf = value & 1 != 0;
        self.flag_pf = value & 4 != 0;
        self.flag_af = value & 0x10 != 0;
        self.flag_zf = value & 0x40 != 0;
        self.flag_sf = value & 0x80 != 0;
        self.flag_tf = value & 0x100 != 0;
        self.flag_if = value & 0x200 != 0;
        self.flag_df = value & 0x400 != 0;
        self.flag_of = value & 0x800 != 0;
    }

    fn eval_condition(&self, condition: u8) -> bool {
        match condition {
            0 => self.flag_of,
            1 => !self.flag_of,
            2 => self.flag_cf,
            3 => !self.flag_cf,
            4 => self.flag_zf,
            5 => !self.flag_zf,
            6 => self.flag_cf || self.flag_zf,
            7 => !self.flag_cf && !self.flag_zf,
            8 => self.flag_sf,
            9 => !self.flag_sf,
            10 => self.flag_pf,
            11 => !self.flag_pf,
            12 => self.flag_sf != self.flag_of,
            13 => self.flag_sf == self.flag_of,
            14 => self.flag_zf || self.flag_sf != self.flag_of,
            _ => !self.flag_zf && self.flag_sf == self.flag_of,
        }
    }

    fn shift_rotate(&mut self, operation: u8, value: u16, count: u8, word: bool) -> u16 {
        let bits = if word { 16u32 } else { 8u32 };
        let mask = if word { 0xffffu32 } else { 0xffu32 };
        let sign = 1u32 << (bits - 1);
        let original = u32::from(value) & mask;

        match operation {
            0 => {
                let count = u32::from(count) % bits;
                if count == 0 {
                    return original as u16;
                }
                let result = ((original << count) | (original >> (bits - count))) & mask;
                self.flag_cf = result & 1 != 0;
                self.flag_of = (result & sign != 0) != self.flag_cf;
                result as u16
            }
            1 => {
                let count = u32::from(count) % bits;
                if count == 0 {
                    return original as u16;
                }
                let result = ((original >> count) | (original << (bits - count))) & mask;
                self.flag_cf = result & sign != 0;
                self.flag_of = (result & sign != 0) != (result & (sign >> 1) != 0);
                result as u16
            }
            2 | 3 => {
                let width = bits + 1;
                let count = u32::from(count) % width;
                if count == 0 {
                    return original as u16;
                }
                let wide_mask = (1u32 << width) - 1;
                let combined = original | (u32::from(self.flag_cf) << bits);
                let rotated = if operation == 2 {
                    ((combined << count) | (combined >> (width - count))) & wide_mask
                } else {
                    ((combined >> count) | (combined << (width - count))) & wide_mask
                };
                self.flag_cf = rotated & (1 << bits) != 0;
                let result = rotated & mask;
                self.flag_of = if operation == 2 {
                    (result & sign != 0) != self.flag_cf
                } else {
                    (result & sign != 0) != (result & (sign >> 1) != 0)
                };
                result as u16
            }
            4 | 6 => {
                let count = u32::from(count) & 0x1f;
                if count == 0 {
                    return original as u16;
                }
                self.flag_cf = if count <= bits {
                    original & (1 << (bits - count)) != 0
                } else {
                    false
                };
                let result = if count < 32 {
                    (original << count) & mask
                } else {
                    0
                };
                self.flag_of = (result & sign != 0) != self.flag_cf;
                self.set_szp(result as u16, word);
                result as u16
            }
            5 => {
                let count = u32::from(count) & 0x1f;
                if count == 0 {
                    return original as u16;
                }
                self.flag_cf = if count <= bits {
                    original & (1 << (count - 1)) != 0
                } else {
                    false
                };
                self.flag_of = original & sign != 0;
                let result = if count < 32 { original >> count } else { 0 };
                self.set_szp(result as u16, word);
                result as u16
            }
            _ => {
                let count = u32::from(count) & 0x1f;
                if count == 0 {
                    return original as u16;
                }
                self.flag_cf = if count <= bits {
                    original & (1 << (count - 1)) != 0
                } else {
                    original & sign != 0
                };
                self.flag_of = false;
                let signed = if word {
                    i32::from(original as u16 as i16)
                } else {
                    i32::from(original as u8 as i8)
                };
                let result = if count >= bits {
                    if signed < 0 {
                        mask
                    } else {
                        0
                    }
                } else {
                    (signed >> count) as u32 & mask
                };
                self.set_szp(result as u16, word);
                result as u16
            }
        }
    }

    fn string_step(&self, word: bool) -> i16 {
        let amount = if word { 2 } else { 1 };
        if self.flag_df {
            -amount
        } else {
            amount
        }
    }

    fn execute_string(
        &mut self,
        opcode: u8,
        source_segment: u16,
        repeat: Option<u8>,
    ) -> &'static str {
        let word = opcode & 1 != 0;
        let step = self.string_step(word);
        let count = if repeat.is_some() { self.cx } else { 1 };
        for _ in 0..count {
            match opcode {
                0xa4 | 0xa5 => {
                    let value = if word {
                        self.read_word(source_segment, self.si)
                    } else {
                        self.read_byte(source_segment, self.si) as u16
                    };
                    if word {
                        self.write_word(self.es, self.di, value);
                    } else {
                        self.write_byte(self.es, self.di, value as u8);
                    }
                    self.si = self.si.wrapping_add_signed(step);
                    self.di = self.di.wrapping_add_signed(step);
                }
                0xa6 | 0xa7 => {
                    let source = if word {
                        self.read_word(source_segment, self.si)
                    } else {
                        self.read_byte(source_segment, self.si) as u16
                    };
                    let destination = if word {
                        self.read_word(self.es, self.di)
                    } else {
                        self.read_byte(self.es, self.di) as u16
                    };
                    self.sub_value(source, destination, word, 0);
                    self.si = self.si.wrapping_add_signed(step);
                    self.di = self.di.wrapping_add_signed(step);
                }
                0xaa | 0xab => {
                    if word {
                        self.write_word(self.es, self.di, self.ax);
                    } else {
                        self.write_byte(self.es, self.di, self.ax as u8);
                    }
                    self.di = self.di.wrapping_add_signed(step);
                }
                0xac | 0xad => {
                    if word {
                        self.ax = self.read_word(source_segment, self.si);
                    } else {
                        self.set_reg8(0, self.read_byte(source_segment, self.si));
                    }
                    self.si = self.si.wrapping_add_signed(step);
                }
                _ => {
                    let value = if word {
                        self.read_word(self.es, self.di)
                    } else {
                        self.read_byte(self.es, self.di) as u16
                    };
                    self.sub_value(if word { self.ax } else { self.ax & 0xff }, value, word, 0);
                    self.di = self.di.wrapping_add_signed(step);
                }
            }

            if let Some(prefix) = repeat {
                self.cx = self.cx.wrapping_sub(1);
                if self.cx == 0 {
                    break;
                }
                if matches!(opcode, 0xa6 | 0xa7 | 0xae | 0xaf)
                    && ((prefix == 0xf3 && !self.flag_zf) || (prefix == 0xf2 && self.flag_zf))
                {
                    break;
                }
            }
        }
        match opcode {
            0xa4 | 0xa5 => "MOVS",
            0xa6 | 0xa7 => "CMPS",
            0xaa | 0xab => "STOS",
            0xac | 0xad => "LODS",
            _ => "SCAS",
        }
    }

    // ------------------------------------------------------------------
    // Compatibility word-ALU helper used by execute.rs.
    // ------------------------------------------------------------------

    /// Compute `a OP b` for `mnemonic` in `{ADD, SUB, CMP, AND, OR, XOR}`,
    /// updating flags exactly as `execute::execute`'s Python-reference
    /// counterparts (`_add`/`_sub`/`_and`/`_or`/`_xor`) do, and returning
    /// the masked 16-bit result. `CMP` computes flags identically to
    /// `SUB` — the caller is responsible for discarding the result
    /// instead of writing it back (mirrors the Python original's
    /// `alu_op == 7` "CMP does not write" special case).
    pub(crate) fn alu16(&mut self, mnemonic: &str, a: u16, b: u16) -> u16 {
        match mnemonic {
            "ADD" => {
                let raw = a as u32 + b as u32;
                let result = raw as u16;
                self.flag_cf = flags::compute_cf_add(raw, true);
                self.flag_af = flags::compute_af_add(a, b, 0);
                self.flag_of = flags::compute_of_add(a, b, result, true);
                let (sf, zf, pf) = flags::compute_szp(result, true);
                self.flag_sf = sf;
                self.flag_zf = zf;
                self.flag_pf = pf;
                result
            }
            "SUB" | "CMP" => {
                let result = a.wrapping_sub(b);
                self.flag_cf = flags::compute_cf_sub(a as u32, b as u32, 0);
                self.flag_af = flags::compute_af_sub(a, b, 0);
                self.flag_of = flags::compute_of_sub(a, b, result, true);
                let (sf, zf, pf) = flags::compute_szp(result, true);
                self.flag_sf = sf;
                self.flag_zf = zf;
                self.flag_pf = pf;
                result
            }
            "AND" | "OR" | "XOR" => {
                let result = match mnemonic {
                    "AND" => a & b,
                    "OR" => a | b,
                    _ => a ^ b,
                };
                // AND/OR/XOR: CF=0, OF=0, AF undefined (cleared) -- mirrors
                // the Python original's `_set_flags_logic`.
                self.flag_cf = false;
                self.flag_of = false;
                self.flag_af = false;
                let (sf, zf, pf) = flags::compute_szp(result, true);
                self.flag_sf = sf;
                self.flag_zf = zf;
                self.flag_pf = pf;
                result
            }
            other => panic!("intel8086-simulator: alu16 called with non-ALU mnemonic {other:?}"),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn execute_opcode(
        &mut self,
        opcode: u8,
        segment_override: Option<u16>,
        repeat: Option<u8>,
    ) -> String {
        // MOV r/m,reg and reg,r/m.
        if matches!(opcode, 0x88..=0x8b) {
            let word = opcode & 1 != 0;
            let direction_to_reg = opcode & 2 != 0;
            let modrm = self.fetch8();
            let reg = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            if direction_to_reg {
                let value = self.read_rm(operand, word);
                if word {
                    self.set_reg16(reg, value);
                } else {
                    self.set_reg8(reg, value as u8);
                }
            } else {
                let value = if word {
                    self.get_reg16(reg)
                } else {
                    self.get_reg8(reg) as u16
                };
                self.write_rm(operand, value, word);
            }
            return "MOV".into();
        }

        if matches!(opcode, 0xc6 | 0xc7) {
            let word = opcode == 0xc7;
            let modrm = self.fetch8();
            let operand = self.decode_rm(modrm, segment_override);
            let immediate = if word {
                self.fetch16()
            } else {
                self.fetch8() as u16
            };
            self.write_rm(operand, immediate, word);
            return "MOV".into();
        }

        if matches!(opcode, 0xb0..=0xb7) {
            let value = self.fetch8();
            self.set_reg8(opcode - 0xb0, value);
            return "MOV".into();
        }
        if matches!(opcode, 0xb8..=0xbf) {
            let value = self.fetch16();
            self.set_reg16(opcode - 0xb8, value);
            return "MOV".into();
        }

        if matches!(opcode, 0xa0..=0xa3) {
            let word = opcode & 1 != 0;
            let offset = self.fetch16();
            let segment = segment_override.unwrap_or(self.ds);
            if opcode & 2 == 0 {
                let value = if word {
                    self.read_word(segment, offset)
                } else {
                    self.read_byte(segment, offset) as u16
                };
                if word {
                    self.ax = value;
                } else {
                    self.set_reg8(0, value as u8);
                }
            } else if word {
                self.write_word(segment, offset, self.ax);
            } else {
                self.write_byte(segment, offset, self.ax as u8);
            }
            return "MOV".into();
        }

        if matches!(opcode, 0x8c | 0x8e) {
            let modrm = self.fetch8();
            let reg = (modrm >> 3) & 3;
            let operand = self.decode_rm(modrm, segment_override);
            if opcode == 0x8c {
                self.write_rm(operand, self.get_sreg(reg), true);
            } else {
                let value = self.read_rm(operand, true);
                self.set_sreg(reg, value);
            }
            return "MOV".into();
        }

        if matches!(opcode, 0x90..=0x97) {
            let reg = opcode - 0x90;
            if reg == 0 {
                return "NOP".into();
            }
            let value = self.get_reg16(reg);
            self.set_reg16(reg, self.ax);
            self.ax = value;
            return "XCHG".into();
        }

        if matches!(opcode, 0x86 | 0x87) {
            let word = opcode & 1 != 0;
            let modrm = self.fetch8();
            let reg = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            let left = self.read_rm(operand, word);
            let right = if word {
                self.get_reg16(reg)
            } else {
                self.get_reg8(reg) as u16
            };
            self.write_rm(operand, right, word);
            if word {
                self.set_reg16(reg, left);
            } else {
                self.set_reg8(reg, left as u8);
            }
            return "XCHG".into();
        }

        if matches!(opcode, 0x50..=0x57) {
            self.push16(self.get_reg16(opcode - 0x50));
            return "PUSH".into();
        }
        if matches!(opcode, 0x58..=0x5f) {
            let value = self.pop16();
            self.set_reg16(opcode - 0x58, value);
            return "POP".into();
        }
        if matches!(opcode, 0x06 | 0x0e | 0x16 | 0x1e) {
            let reg = match opcode {
                0x06 => 0,
                0x0e => 1,
                0x16 => 2,
                _ => 3,
            };
            self.push16(self.get_sreg(reg));
            return "PUSH".into();
        }
        if matches!(opcode, 0x07 | 0x17 | 0x1f) {
            let reg = match opcode {
                0x07 => 0,
                0x17 => 2,
                _ => 3,
            };
            let value = self.pop16();
            self.set_sreg(reg, value);
            return "POP".into();
        }
        if opcode == 0x8f {
            let modrm = self.fetch8();
            let operand = self.decode_rm(modrm, segment_override);
            let value = self.pop16();
            self.write_rm(operand, value, true);
            return "POP".into();
        }
        if opcode == 0x9c {
            self.push16(self.flags_value());
            return "PUSHF".into();
        }
        if opcode == 0x9d {
            let value = self.pop16();
            self.load_flags(value);
            return "POPF".into();
        }

        if matches!(opcode, 0x8d | 0xc4 | 0xc5) {
            let modrm = self.fetch8();
            let reg = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            if opcode == 0x8d {
                self.set_reg16(reg, operand.offset);
                return "LEA".into();
            }
            let offset = self.read_word(operand.segment, operand.offset);
            let segment = self.read_word(operand.segment, operand.offset.wrapping_add(2));
            self.set_reg16(reg, offset);
            if opcode == 0xc4 {
                self.es = segment;
                return "LES".into();
            }
            self.ds = segment;
            return "LDS".into();
        }

        match opcode {
            0x9f => {
                self.set_reg8(4, self.flags_value() as u8);
                return "LAHF".into();
            }
            0x9e => {
                let low = self.get_reg8(4) as u16;
                self.flag_cf = low & 1 != 0;
                self.flag_pf = low & 4 != 0;
                self.flag_af = low & 0x10 != 0;
                self.flag_zf = low & 0x40 != 0;
                self.flag_sf = low & 0x80 != 0;
                return "SAHF".into();
            }
            0x98 => {
                self.ax = self.get_reg8(0) as i8 as i16 as u16;
                return "CBW".into();
            }
            0x99 => {
                self.dx = if self.ax & 0x8000 != 0 { 0xffff } else { 0 };
                return "CWD".into();
            }
            0xd7 => {
                let segment = segment_override.unwrap_or(self.ds);
                let offset = self.bx.wrapping_add(self.get_reg8(0) as u16);
                let value = self.read_byte(segment, offset);
                self.set_reg8(0, value);
                return "XLAT".into();
            }
            _ => {}
        }

        // Immediate ALU group.
        if matches!(opcode, 0x80..=0x83) {
            let word = matches!(opcode, 0x81 | 0x83);
            let modrm = self.fetch8();
            let operation = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            let immediate = match opcode {
                0x80 | 0x82 => self.fetch8() as u16,
                0x81 => self.fetch16(),
                _ => self.fetch8() as i8 as i16 as u16,
            };
            let old = self.read_rm(operand, word);
            let result = self.alu_value(operation, old, immediate, word);
            if operation != 7 {
                self.write_rm(operand, result, word);
            }
            return ["ADD", "OR", "ADC", "SBB", "AND", "SUB", "XOR", "CMP"][operation as usize]
                .into();
        }

        // The eight regular ALU families: r/m<->reg and accumulator immediate.
        if opcode <= 0x3d {
            let operation = opcode >> 3;
            let form = opcode & 7;
            if operation <= 7 && form <= 5 {
                if form <= 3 {
                    let word = form & 1 != 0;
                    let direction_to_reg = form & 2 != 0;
                    let modrm = self.fetch8();
                    let reg = (modrm >> 3) & 7;
                    let operand = self.decode_rm(modrm, segment_override);
                    let rm_value = self.read_rm(operand, word);
                    let reg_value = if word {
                        self.get_reg16(reg)
                    } else {
                        self.get_reg8(reg) as u16
                    };
                    let (left, right) = if direction_to_reg {
                        (reg_value, rm_value)
                    } else {
                        (rm_value, reg_value)
                    };
                    let result = self.alu_value(operation, left, right, word);
                    if operation != 7 {
                        if direction_to_reg {
                            if word {
                                self.set_reg16(reg, result);
                            } else {
                                self.set_reg8(reg, result as u8);
                            }
                        } else {
                            self.write_rm(operand, result, word);
                        }
                    }
                } else {
                    let word = form == 5;
                    let immediate = if word {
                        self.fetch16()
                    } else {
                        self.fetch8() as u16
                    };
                    let old = if word {
                        self.ax
                    } else {
                        self.get_reg8(0) as u16
                    };
                    let result = self.alu_value(operation, old, immediate, word);
                    if operation != 7 {
                        if word {
                            self.ax = result;
                        } else {
                            self.set_reg8(0, result as u8);
                        }
                    }
                }
                return ["ADD", "OR", "ADC", "SBB", "AND", "SUB", "XOR", "CMP"][operation as usize]
                    .into();
            }
        }

        if matches!(opcode, 0x84 | 0x85) {
            let word = opcode & 1 != 0;
            let modrm = self.fetch8();
            let reg = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            let left = self.read_rm(operand, word);
            let right = if word {
                self.get_reg16(reg)
            } else {
                self.get_reg8(reg) as u16
            };
            self.logic_value(4, left, right, word);
            return "TEST".into();
        }
        if matches!(opcode, 0xa8 | 0xa9) {
            let word = opcode & 1 != 0;
            let immediate = if word {
                self.fetch16()
            } else {
                self.fetch8() as u16
            };
            let value = if word {
                self.ax
            } else {
                self.get_reg8(0) as u16
            };
            self.logic_value(4, value, immediate, word);
            return "TEST".into();
        }

        if matches!(opcode, 0x40..=0x4f) {
            let increment = opcode < 0x48;
            let reg = opcode & 7;
            let old_cf = self.flag_cf;
            let value = self.get_reg16(reg);
            let result = if increment {
                self.add_value(value, 1, true, 0)
            } else {
                self.sub_value(value, 1, true, 0)
            };
            self.flag_cf = old_cf;
            self.set_reg16(reg, result);
            return if increment { "INC" } else { "DEC" }.into();
        }

        if opcode == 0xfe {
            let modrm = self.fetch8();
            let operation = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            let old_cf = self.flag_cf;
            let value = self.read_rm(operand, false);
            let result = if operation == 0 {
                self.add_value(value, 1, false, 0)
            } else {
                self.sub_value(value, 1, false, 0)
            };
            self.flag_cf = old_cf;
            self.write_rm(operand, result, false);
            return if operation == 0 { "INC" } else { "DEC" }.into();
        }

        if opcode == 0xff {
            let modrm = self.fetch8();
            let operation = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            let value = self.read_rm(operand, true);
            match operation {
                0 | 1 => {
                    let old_cf = self.flag_cf;
                    let result = if operation == 0 {
                        self.add_value(value, 1, true, 0)
                    } else {
                        self.sub_value(value, 1, true, 0)
                    };
                    self.flag_cf = old_cf;
                    self.write_rm(operand, result, true);
                    return if operation == 0 { "INC" } else { "DEC" }.into();
                }
                2 => {
                    self.push16(self.ip);
                    self.ip = value;
                    return "CALL".into();
                }
                3 => {
                    let new_ip = self.read_word(operand.segment, operand.offset);
                    let new_cs = self.read_word(operand.segment, operand.offset.wrapping_add(2));
                    self.push16(self.cs);
                    self.push16(self.ip);
                    self.cs = new_cs;
                    self.ip = new_ip;
                    return "CALL FAR".into();
                }
                4 => {
                    self.ip = value;
                    return "JMP".into();
                }
                5 => {
                    self.ip = self.read_word(operand.segment, operand.offset);
                    self.cs = self.read_word(operand.segment, operand.offset.wrapping_add(2));
                    return "JMP FAR".into();
                }
                6 => {
                    self.push16(value);
                    return "PUSH".into();
                }
                _ => {}
            }
        }

        if matches!(opcode, 0xf6 | 0xf7) {
            let word = opcode == 0xf7;
            let modrm = self.fetch8();
            let operation = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            let value = self.read_rm(operand, word);
            match operation {
                0 => {
                    let immediate = if word {
                        self.fetch16()
                    } else {
                        self.fetch8() as u16
                    };
                    self.logic_value(4, value, immediate, word);
                    return "TEST".into();
                }
                2 => {
                    let mask = if word { 0xffff } else { 0xff };
                    self.write_rm(operand, !value & mask, word);
                    return "NOT".into();
                }
                3 => {
                    let result = self.sub_value(0, value, word, 0);
                    self.flag_cf = value != 0;
                    self.write_rm(operand, result, word);
                    return "NEG".into();
                }
                4 => {
                    if word {
                        let result = u32::from(self.ax) * u32::from(value);
                        self.ax = result as u16;
                        self.dx = (result >> 16) as u16;
                        self.flag_cf = self.dx != 0;
                    } else {
                        self.ax = u16::from(self.get_reg8(0)) * (value & 0xff);
                        self.flag_cf = self.ax >> 8 != 0;
                    }
                    self.flag_of = self.flag_cf;
                    return "MUL".into();
                }
                5 => {
                    if word {
                        let result = i32::from(self.ax as i16) * i32::from(value as i16);
                        self.ax = result as u16;
                        self.dx = (result >> 16) as u16;
                        let expected = if self.ax & 0x8000 != 0 { 0xffff } else { 0 };
                        self.flag_cf = self.dx != expected;
                    } else {
                        let result =
                            i16::from(self.get_reg8(0) as i8) * i16::from(value as u8 as i8);
                        self.ax = result as u16;
                        let expected = if self.ax & 0x80 != 0 { 0xff } else { 0 };
                        self.flag_cf = self.ax >> 8 != expected;
                    }
                    self.flag_of = self.flag_cf;
                    return "IMUL".into();
                }
                6 => {
                    if value == 0 {
                        self.halted = true;
                        return "DIV /0".into();
                    }
                    if word {
                        let dividend = (u32::from(self.dx) << 16) | u32::from(self.ax);
                        self.ax = (dividend / u32::from(value)) as u16;
                        self.dx = (dividend % u32::from(value)) as u16;
                    } else {
                        let quotient = self.ax / (value & 0xff);
                        let remainder = self.ax % (value & 0xff);
                        self.ax = ((remainder & 0xff) << 8) | (quotient & 0xff);
                    }
                    return "DIV".into();
                }
                7 => {
                    let divisor = if word {
                        i64::from(value as i16)
                    } else {
                        i64::from(value as u8 as i8)
                    };
                    if divisor == 0 {
                        self.halted = true;
                        return "IDIV /0".into();
                    }
                    if word {
                        let bits = (u32::from(self.dx) << 16) | u32::from(self.ax);
                        let dividend = i64::from(bits as i32);
                        let quotient = dividend / divisor;
                        let remainder = dividend - quotient * divisor;
                        self.ax = quotient as u16;
                        self.dx = remainder as u16;
                    } else {
                        let dividend = i64::from(self.ax as i16);
                        let quotient = dividend / divisor;
                        let remainder = dividend - quotient * divisor;
                        self.ax = ((remainder as u16 & 0xff) << 8) | (quotient as u16 & 0xff);
                    }
                    return "IDIV".into();
                }
                _ => {}
            }
        }

        match opcode {
            0x27 | 0x2f => {
                let subtract = opcode == 0x2f;
                let mut al = self.get_reg8(0);
                let old_al = al;
                let old_cf = self.flag_cf;
                if al & 0x0f > 9 || self.flag_af {
                    al = if subtract {
                        al.wrapping_sub(6)
                    } else {
                        al.wrapping_add(6)
                    };
                    self.flag_af = true;
                } else {
                    self.flag_af = false;
                }
                if old_al > 0x99 || old_cf {
                    al = if subtract {
                        al.wrapping_sub(0x60)
                    } else {
                        al.wrapping_add(0x60)
                    };
                    self.flag_cf = true;
                } else {
                    self.flag_cf = false;
                }
                self.set_reg8(0, al);
                self.set_szp(al as u16, false);
                return if subtract { "DAS" } else { "DAA" }.into();
            }
            0x37 | 0x3f => {
                let subtract = opcode == 0x3f;
                let mut al = self.get_reg8(0);
                let mut ah = self.get_reg8(4);
                if al & 0x0f > 9 || self.flag_af {
                    al = if subtract {
                        al.wrapping_sub(6)
                    } else {
                        al.wrapping_add(6)
                    };
                    ah = if subtract {
                        ah.wrapping_sub(1)
                    } else {
                        ah.wrapping_add(1)
                    };
                    self.flag_af = true;
                    self.flag_cf = true;
                } else {
                    self.flag_af = false;
                    self.flag_cf = false;
                }
                self.ax = (u16::from(ah) << 8) | u16::from(al & 0x0f);
                return if subtract { "AAS" } else { "AAA" }.into();
            }
            0xd4 | 0xd5 => {
                let base = self.fetch8();
                if base == 0 {
                    self.halted = true;
                    return "AAM /0".into();
                }
                if opcode == 0xd4 {
                    let al = self.get_reg8(0);
                    self.ax = (u16::from(al / base) << 8) | u16::from(al % base);
                } else {
                    let result = self
                        .get_reg8(4)
                        .wrapping_mul(base)
                        .wrapping_add(self.get_reg8(0));
                    self.ax = u16::from(result);
                }
                self.set_szp(self.get_reg8(0) as u16, false);
                return if opcode == 0xd4 { "AAM" } else { "AAD" }.into();
            }
            _ => {}
        }

        if matches!(opcode, 0xd0..=0xd3) {
            let word = opcode & 1 != 0;
            let count = if opcode < 0xd2 { 1 } else { self.get_reg8(1) };
            let modrm = self.fetch8();
            let operation = (modrm >> 3) & 7;
            let operand = self.decode_rm(modrm, segment_override);
            let value = self.read_rm(operand, word);
            let result = self.shift_rotate(operation, value, count, word);
            self.write_rm(operand, result, word);
            return ["ROL", "ROR", "RCL", "RCR", "SHL", "SHR", "SHL", "SAR"][operation as usize]
                .into();
        }

        match opcode {
            0xeb => {
                let displacement = self.fetch_s8();
                self.ip = self.ip.wrapping_add_signed(displacement);
                return "JMP SHORT".into();
            }
            0xe9 => {
                let displacement = self.fetch_s16();
                self.ip = self.ip.wrapping_add_signed(displacement);
                return "JMP NEAR".into();
            }
            0xea => {
                let new_ip = self.fetch16();
                let new_cs = self.fetch16();
                self.ip = new_ip;
                self.cs = new_cs;
                return "JMP FAR".into();
            }
            0xe8 => {
                let displacement = self.fetch_s16();
                self.push16(self.ip);
                self.ip = self.ip.wrapping_add_signed(displacement);
                return "CALL".into();
            }
            0x9a => {
                let new_ip = self.fetch16();
                let new_cs = self.fetch16();
                self.push16(self.cs);
                self.push16(self.ip);
                self.ip = new_ip;
                self.cs = new_cs;
                return "CALL FAR".into();
            }
            0xc3 => {
                self.ip = self.pop16();
                return "RET".into();
            }
            0xc2 => {
                let amount = self.fetch16();
                self.ip = self.pop16();
                self.sp = self.sp.wrapping_add(amount);
                return "RET".into();
            }
            0xcb => {
                self.ip = self.pop16();
                self.cs = self.pop16();
                return "RETF".into();
            }
            0xca => {
                let amount = self.fetch16();
                self.ip = self.pop16();
                self.cs = self.pop16();
                self.sp = self.sp.wrapping_add(amount);
                return "RETF".into();
            }
            0x70..=0x7f => {
                let displacement = self.fetch_s8();
                if self.eval_condition(opcode - 0x70) {
                    self.ip = self.ip.wrapping_add_signed(displacement);
                }
                return "JCC".into();
            }
            0xe0..=0xe3 => {
                let displacement = self.fetch_s8();
                let taken = if opcode == 0xe3 {
                    self.cx == 0
                } else {
                    self.cx = self.cx.wrapping_sub(1);
                    match opcode {
                        0xe0 => self.cx != 0 && !self.flag_zf,
                        0xe1 => self.cx != 0 && self.flag_zf,
                        _ => self.cx != 0,
                    }
                };
                if taken {
                    self.ip = self.ip.wrapping_add_signed(displacement);
                }
                return "LOOP".into();
            }
            0xcc | 0xce => {
                self.halted = true;
                return "INT".into();
            }
            0xcd => {
                self.fetch8();
                self.halted = true;
                return "INT".into();
            }
            0xcf => {
                self.ip = self.pop16();
                self.cs = self.pop16();
                let flags = self.pop16();
                self.load_flags(flags);
                return "IRET".into();
            }
            _ => {}
        }

        if matches!(opcode, 0xa4..=0xa7 | 0xaa..=0xad | 0xae..=0xaf) {
            let source = segment_override.unwrap_or(self.ds);
            return self.execute_string(opcode, source, repeat).into();
        }

        match opcode {
            0xf4 => {
                self.halted = true;
                return "HLT".into();
            }
            0xf8 => self.flag_cf = false,
            0xf9 => self.flag_cf = true,
            0xf5 => self.flag_cf = !self.flag_cf,
            0xfc => self.flag_df = false,
            0xfd => self.flag_df = true,
            0xfa => self.flag_if = false,
            0xfb => self.flag_if = true,
            0x9b => return "WAIT".into(),
            0xe4 | 0xe5 => {
                let port = self.fetch8();
                let lo = self.input_ports[port as usize];
                if opcode == 0xe4 {
                    self.set_reg8(0, lo);
                } else {
                    let hi = self.input_ports[port.wrapping_add(1) as usize];
                    self.ax = u16::from(lo) | (u16::from(hi) << 8);
                }
                return "IN".into();
            }
            0xec | 0xed => {
                let port = self.dx as u8;
                let lo = self.input_ports[port as usize];
                if opcode == 0xec {
                    self.set_reg8(0, lo);
                } else {
                    let hi = self.input_ports[port.wrapping_add(1) as usize];
                    self.ax = u16::from(lo) | (u16::from(hi) << 8);
                }
                return "IN".into();
            }
            0xe6 | 0xe7 => {
                let port = self.fetch8();
                self.output_ports[port as usize] = self.ax as u8;
                if opcode == 0xe7 {
                    self.output_ports[port.wrapping_add(1) as usize] = (self.ax >> 8) as u8;
                }
                return "OUT".into();
            }
            0xee | 0xef => {
                let port = self.dx as u8;
                self.output_ports[port as usize] = self.ax as u8;
                if opcode == 0xef {
                    self.output_ports[port.wrapping_add(1) as usize] = (self.ax >> 8) as u8;
                }
                return "OUT".into();
            }
            _ => {
                self.halted = true;
                return "illegal".into();
            }
        }

        match opcode {
            0xf8 => "CLC",
            0xf9 => "STC",
            0xf5 => "CMC",
            0xfc => "CLD",
            0xfd => "STD",
            0xfa => "CLI",
            _ => "STI",
        }
        .into()
    }

    // ------------------------------------------------------------------
    // Load / run
    // ------------------------------------------------------------------

    /// Reset all registers, flags, memory, ports, and control state.
    pub fn reset(&mut self) {
        *self = Self::new(MEMORY_SIZE);
    }

    /// Return a complete owned architectural snapshot.
    pub fn snapshot(&self) -> Intel8086State {
        Intel8086State {
            ax: self.ax,
            bx: self.bx,
            cx: self.cx,
            dx: self.dx,
            si: self.si,
            di: self.di,
            sp: self.sp,
            bp: self.bp,
            cs: self.cs,
            ds: self.ds,
            ss: self.ss,
            es: self.es,
            ip: self.ip,
            flags: self.flags_value(),
            halted: self.halted,
            input_ports: self.input_ports,
            output_ports: self.output_ports,
            memory: (0..MEMORY_SIZE)
                .map(|address| self.mem.read_byte(address))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Restore a complete snapshot atomically.
    pub fn restore(&mut self, state: &Intel8086State) -> Result<(), Intel8086Error> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(Intel8086Error::InvalidStateMemory {
                length: state.memory.len(),
            });
        }
        self.ax = state.ax;
        self.bx = state.bx;
        self.cx = state.cx;
        self.dx = state.dx;
        self.si = state.si;
        self.di = state.di;
        self.sp = state.sp;
        self.bp = state.bp;
        self.cs = state.cs;
        self.ds = state.ds;
        self.ss = state.ss;
        self.es = state.es;
        self.ip = state.ip;
        self.load_flags(state.flags);
        self.halted = state.halted;
        self.input_ports = state.input_ports;
        self.output_ports = state.output_ports;
        self.mem.load_bytes(0, &state.memory);
        self.trace_raw.clear();
        Ok(())
    }

    /// Checked input-port latch update.
    pub fn set_input_port(&mut self, port: usize, value: u8) -> Result<(), Intel8086Error> {
        let latch = self
            .input_ports
            .get_mut(port)
            .ok_or(Intel8086Error::InvalidPort { port })?;
        *latch = value;
        Ok(())
    }

    /// Checked output-port latch read.
    pub fn get_output_port(&self, port: usize) -> Result<u8, Intel8086Error> {
        self.output_ports
            .get(port)
            .copied()
            .ok_or(Intel8086Error::InvalidPort { port })
    }

    /// Checked, atomic program load at a physical address.
    pub fn load_program_checked_at(
        &mut self,
        program: &[u8],
        origin: usize,
    ) -> Result<(), Intel8086Error> {
        let end = origin
            .checked_add(program.len())
            .ok_or(Intel8086Error::ProgramOutOfRange {
                origin,
                length: program.len(),
            })?;
        if end > MEMORY_SIZE {
            return Err(Intel8086Error::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        self.mem.load_bytes(origin, program);
        self.ip = 0;
        self.halted = false;
        Ok(())
    }

    /// Checked, atomic program load at physical address zero.
    pub fn load_program_checked(&mut self, program: &[u8]) -> Result<(), Intel8086Error> {
        self.load_program_checked_at(program, 0)
    }

    /// Legacy source-compatible load. Invalid ranges are rejected without
    /// modifying the machine; use [`Self::load_program_checked_at`] to inspect
    /// the typed error.
    pub fn load_program_at(&mut self, program: &[u8], origin: usize) {
        let _ = self.load_program_checked_at(program, origin);
    }

    /// Load a program at physical address 0 — the convention every other
    /// Rust ISA simulator in this repo uses for `load_program`. With
    /// `CS=0` (the power-on / [`Self::new`] default), `CS:IP = 0:0`
    /// fetches from physical address 0, so this matches
    /// `load_program_at(program, 0)` exactly when `CS` hasn't been
    /// changed since construction.
    pub fn load_program(&mut self, program: &[u8]) {
        self.load_program_at(program, 0);
    }

    /// Load and run a program from a fresh state, until halted or 10000
    /// steps (safety limit) — convenience wrapper mirroring
    /// `Mos6502Simulator::run`.
    pub fn run(&mut self, program: &[u8]) {
        let mut candidate = Self::new(MEMORY_SIZE);
        candidate.input_ports = self.input_ports;
        if candidate.load_program_checked(program).is_err() {
            return;
        }
        let mut steps = 0;
        while steps < 10_000 && !candidate.halted {
            candidate.step();
            steps += 1;
        }
        *self = candidate;
    }

    /// Run a fresh program transactionally with a caller-supplied bound.
    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8086Error> {
        let mut candidate = Self::new(MEMORY_SIZE);
        candidate.input_ports = self.input_ports;
        candidate.load_program_checked(program)?;
        let result = candidate.run_loaded_checked(max_steps)?;
        *self = candidate;
        Ok(result)
    }

    /// Run the already-loaded program for at most `max_steps`
    /// instructions.
    ///
    /// A non-halting result means the step budget was exhausted — surfaces
    /// runaway/looping programs to the caller instead of silently treating
    /// them as success (same contract as `Mos6502Simulator::
    /// run_loaded_with_limit`).
    pub fn run_loaded_with_limit(&mut self, max_steps: usize) -> ExecutionResult {
        let mut traces = Vec::new();
        while traces.len() < max_steps && !self.halted {
            let cs = self.cs;
            let ip = self.ip;
            let state_before = self.snapshot();
            let mnemonic = self.step();
            traces.push(StepTrace {
                cs,
                ip,
                raw: self.trace_raw.clone(),
                mnemonic,
                state_before,
                state_after: self.snapshot(),
            });
        }
        ExecutionResult {
            halted: self.halted,
            steps: traces.len(),
            ip: self.ip,
            traces,
            final_state: self.snapshot(),
        }
    }

    /// Run already-loaded code transactionally. An invalid instruction
    /// restores the complete entry state.
    pub fn run_loaded_checked(
        &mut self,
        max_steps: usize,
    ) -> Result<ExecutionResult, Intel8086Error> {
        let entry = self.snapshot();
        let mut traces = Vec::new();
        while traces.len() < max_steps && !self.halted {
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.restore(&entry)?;
                    return Err(error);
                }
            }
        }
        Ok(ExecutionResult {
            halted: self.halted,
            steps: traces.len(),
            ip: self.ip,
            traces,
            final_state: self.snapshot(),
        })
    }

    /// Execute a single instruction and return its mnemonic.
    ///
    /// Fetch-decodes at `CS:IP` (which `decode::fetch_decode` advances
    /// past the whole instruction as a side effect via its local `ip`
    /// copy), then dispatches to `execute::execute`. An opcode outside
    /// the specified opcode surface is a **fail-closed halt** — mirrors
    /// `mos6502-simulator`'s illegal-opcode handling (no exception
    /// channel exists through `step() -> String`, so the simulator stops
    /// rather than silently corrupting state or panicking); the Python
    /// original instead falls through to treating any unrecognised
    /// opcode as `HLT` (see `simulator.py`'s final `# Unknown opcode --
    /// treat as HLT` fallback) -- functionally equivalent (`halted`
    /// becomes `true` either way), just surfaced through `Err` first.
    pub fn step(&mut self) -> String {
        if self.halted {
            return "halted".to_string();
        }

        self.trace_raw.clear();

        let mut segment_override = None;
        let mut repeat = None;
        loop {
            let opcode = self.fetch8();
            match opcode {
                0x26 => segment_override = Some(self.es),
                0x2e => segment_override = Some(self.cs),
                0x36 => segment_override = Some(self.ss),
                0x3e => segment_override = Some(self.ds),
                0xf2 | 0xf3 => repeat = Some(opcode),
                0xf0 => {}
                _ => return self.execute_opcode(opcode, segment_override, repeat),
            }
        }
    }

    /// Execute one instruction atomically and return a complete trace.
    pub fn step_checked(&mut self) -> Result<StepTrace, Intel8086Error> {
        if self.halted {
            return Err(Intel8086Error::Halted);
        }
        let cs = self.cs;
        let ip = self.ip;
        let state_before = self.snapshot();
        let mnemonic = self.step();
        let raw = self.trace_raw.clone();
        if mnemonic == "illegal" {
            self.restore(&state_before)?;
            return Err(Intel8086Error::UnknownOpcode { cs, ip, raw });
        }
        Ok(StepTrace {
            cs,
            ip,
            raw,
            mnemonic,
            state_before,
            state_after: self.snapshot(),
        })
    }

    /// Run a list of raw instruction bytes (convenience for tests).
    pub fn run_bytes(&mut self, bytes: &[u8]) {
        self.run(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes;
    use crate::opcodes::{HLT_OPCODE, MOV_REG_IMM16_BASE, REG_AX};

    // ── Segmentation math itself ────────────────────────────────────

    #[test]
    fn phys_addr_zero_segment_zero_offset() {
        assert_eq!(phys_addr(0, 0), 0);
    }

    #[test]
    fn phys_addr_combines_segment_shifted_and_offset() {
        // CS=0x1000 -> base 0x10000; + offset 0x0010 -> 0x10010.
        assert_eq!(phys_addr(0x1000, 0x0010), 0x10010);
    }

    #[test]
    fn phys_addr_matches_python_reference_formula() {
        // physical = (segment * 16 + offset) & 0xFFFFF -- from
        // code/packages/python/intel-8086-simulator's `_phys`.
        for (seg, off, expected) in [
            (0u16, 0u16, 0usize),
            (0x07C0, 0x0000, 0x07C00), // classic boot-sector CS
            (0xFFFF, 0x0020, 0x00010), // beyond 1MB, masked (wraps)
        ] {
            assert_eq!(phys_addr(seg, off), expected);
        }
    }

    #[test]
    fn phys_addr_wraps_at_20_bits() {
        // CS=0xFFFF, IP=0xFFFF -> (0xFFFF<<4)+0xFFFF = 0x10FFEF,
        // masked to 20 bits -> 0x0FFEF.
        assert_eq!(phys_addr(0xFFFF, 0xFFFF), 0x0FFEF);
    }

    // ── Register read/write ─────────────────────────────────────────

    #[test]
    fn reg16_round_trip_all_eight() {
        let mut sim = Intel8086Simulator::new(65536);
        for reg in 0u8..8 {
            sim.set_reg16(reg, 0x1000 + reg as u16);
        }
        for reg in 0u8..8 {
            assert_eq!(sim.get_reg16(reg), 0x1000 + reg as u16);
        }
    }

    #[test]
    fn reg8_halves_share_storage_with_reg16() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.set_reg16(REG_AX, 0x1234);
        assert_eq!(sim.get_reg8(opcodes::REG_AL), 0x34);
        assert_eq!(sim.get_reg8(opcodes::REG_AH), 0x12);
        sim.set_reg8(opcodes::REG_AL, 0xFF);
        assert_eq!(sim.ax, 0x12FF);
    }

    // ── The trivial "load immediate + halt" sequence the
    // intel8086-backend smoke test relies on ──────────────────────────

    #[test]
    fn mov_ax_immediate_then_hlt_halts_with_ax_equals_42() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.run(&[MOV_REG_IMM16_BASE + REG_AX, 42, 0x00, HLT_OPCODE]);
        assert!(sim.halted);
        assert_eq!(sim.ax, 42);
    }

    #[test]
    fn bounded_run_reports_halt_and_instruction_count() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&[0xB8, 42, 0x00, 0xF4]);
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(result.steps, 2); // MOV AX,42, then HLT
    }

    #[test]
    fn power_on_state_is_all_zero() {
        let sim = Intel8086Simulator::new(65536);
        assert_eq!(sim.ax, 0);
        assert_eq!(sim.cs, 0);
        assert_eq!(sim.ip, 0);
        assert!(!sim.halted);
        assert!(!sim.flag_cf && !sim.flag_zf && !sim.flag_sf);
    }

    #[test]
    fn add_ax_immediate_sets_carry_on_overflow() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,0xFFFF; ADD AX,1; HLT -> AX=0, CF=1, ZF=1
        sim.run(&[0xB8, 0xFF, 0xFF, 0x05, 0x01, 0x00, 0xF4]);
        assert_eq!(sim.ax, 0);
        assert!(sim.flag_cf);
        assert!(sim.flag_zf);
    }

    #[test]
    fn sub_ax_immediate_basic() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,10; SUB AX,3; HLT -> AX=7
        sim.run(&[0xB8, 10, 0x00, 0x2D, 3, 0x00, 0xF4]);
        assert_eq!(sim.ax, 7);
        assert!(!sim.flag_cf);
    }

    #[test]
    fn cmp_ax_immediate_does_not_write_back() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,5; CMP AX,5; HLT -> AX still 5, ZF=1
        sim.run(&[0xB8, 5, 0x00, 0x3D, 5, 0x00, 0xF4]);
        assert_eq!(sim.ax, 5);
        assert!(sim.flag_zf);
    }

    #[test]
    fn mov_reg_reg16_copies_between_registers() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,99; MOV CX,AX; HLT
        sim.run(&[0xB8, 99, 0x00, 0x8B, 0xC8, 0xF4]);
        assert_eq!(sim.cx, 99);
        assert_eq!(sim.ax, 99);
    }

    #[test]
    fn inc_dec_reg16_preserve_carry_flag() {
        let mut sim = Intel8086Simulator::new(65536);
        // MOV AX,0xFFFF; ADD AX,1 (sets CF=1); INC CX (must NOT clear CF); HLT
        sim.run(&[0xB8, 0xFF, 0xFF, 0x05, 0x01, 0x00, 0x41, 0xF4]);
        assert!(sim.flag_cf, "INC must not affect CF");
        assert_eq!(sim.cx, 1);
    }

    #[test]
    fn illegal_opcode_halts_fail_closed() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.run(&[0x0F]); // POP CS on 8086; deliberately unsupported here
        assert!(sim.halted);
    }

    #[test]
    fn step_after_halt_returns_halted() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&[HLT_OPCODE]);
        sim.step();
        assert!(sim.halted);
        assert_eq!(sim.step(), "halted");
    }

    #[test]
    fn step_returns_mnemonic() {
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&[0xB8, 42, 0x00, 0xF4]);
        assert_eq!(sim.step(), "MOV");
        assert_eq!(sim.ax, 42);
        assert_eq!(sim.step(), "HLT");
        assert!(sim.halted);
    }

    #[test]
    fn load_program_at_nonzero_origin_with_matching_cs() {
        // Loading at a non-zero physical origin only fetches correctly if
        // CS:0 maps to that same physical address -- set CS accordingly
        // first (mirrors real boot-sector-style loading at CS=0x07C0).
        let mut sim = Intel8086Simulator::new(1 << 20);
        let cs = 0x0010u16;
        let origin = phys_addr(cs, 0);
        sim.cs = cs;
        sim.load_program_at(&[0xB8, 42, 0x00, 0xF4], origin);
        let result = sim.run_loaded_with_limit(10);
        assert!(result.halted);
        assert_eq!(sim.ax, 42);
    }
}
