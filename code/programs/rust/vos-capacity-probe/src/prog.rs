//! # `prog`: hand-assembled RV32I sequences, the generator and the independent evaluator
//!
//! A probe program is a list of [`Op`]s.  An `Op` is the probe's own name for one RV32I
//! instruction of the subset it models (VOS30 section 9.1: the ALU, `lw`/`lbu`/`sw`/`sb` and
//! forward branches).  Three things are done with it:
//!
//! ```text
//!   Vec<Op> --assemble (riscv-encoder)--> u32 words --decode (riscv-simulator)--> Vec<Op>
//!       |                                                   |
//!       | reference side                                    | code side
//!       v                                                   v
//!   alternative encodings                           the stepper reads what the real
//!   of a seeded share                               simulator's decoder says, so the probe
//!                                                   cannot disagree with it about encodings
//! ```
//!
//! The code side of every experiment goes through the `u32` words and the simulator's
//! decoder; the reference side is built from the `Op`s directly.  [`eval_ops`] is the
//! **independent concrete evaluator** of guard G3: plain `u32` arithmetic over an `Op`
//! list, written without the stepper's circuit builder, used to replay a counterexample's
//! reference side.
//!
//! The generator [`gen_b`] makes the seeded straight-line programs of experiments B, C and
//! D with the two liveness rules of guard G2: a fresh symbolic load every
//! [`LOAD_EVERY`] instructions, and at most [`KILL_SHARE`] percent entropy-killing
//! operations (`slt`, `sltu`, shifts, `and` with a small constant).

use crate::sem::{self, Bin};
use riscv_simulator::decode::decode;
use riscv_simulator::encoding as enc;
use std::collections::BTreeMap;

/// A fresh symbolic load every this many instructions (guard G2).
pub const LOAD_EVERY: usize = 6;
/// The cap on entropy-killing operations, in percent of the program (guard G2).
pub const KILL_SHARE: usize = 20;
/// Where generated programs read their fresh symbolic words (word `k` at `DATA_BASE + 4k`).
pub const DATA_BASE: u32 = 0x100;

/// A register number, 0 to 31.
pub type Reg = u8;

/// The second operand of an ALU instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rhs {
    /// `rs2`.
    Reg(Reg),
    /// A sign-extended 12-bit immediate (a 5-bit shift amount for the shifts).
    Imm(i32),
}

/// Branch conditions of the forward branches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    /// `beq`.
    Eq,
    /// `bne`.
    Ne,
    /// `blt`.
    Lt,
    /// `bge`.
    Ge,
    /// `bltu`.
    Ltu,
    /// `bgeu`.
    Geu,
}

/// One instruction of the modelled subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `rd = rs1 <k> rhs`.
    Alu {
        /// The operation.
        k: Bin,
        /// Destination.
        rd: Reg,
        /// First operand.
        rs1: Reg,
        /// Second operand.
        rhs: Rhs,
    },
    /// `lui rd, imm20`.
    Lui {
        /// Destination.
        rd: Reg,
        /// The upper 20 bits.
        imm20: u32,
    },
    /// `lw rd, off(base)`.
    Lw {
        /// Destination.
        rd: Reg,
        /// Base register.
        base: Reg,
        /// Byte offset.
        off: i32,
    },
    /// `lbu rd, off(base)`.
    Lbu {
        /// Destination.
        rd: Reg,
        /// Base register.
        base: Reg,
        /// Byte offset.
        off: i32,
    },
    /// `sw rs, off(base)`.
    Sw {
        /// Value register.
        rs: Reg,
        /// Base register.
        base: Reg,
        /// Byte offset.
        off: i32,
    },
    /// `sb rs, off(base)`.
    Sb {
        /// Value register.
        rs: Reg,
        /// Base register.
        base: Reg,
        /// Byte offset.
        off: i32,
    },
    /// A forward branch over the next `skip` instructions when the condition holds.
    Br {
        /// The condition.
        c: Cond,
        /// First comparand.
        rs1: Reg,
        /// Second comparand.
        rs2: Reg,
        /// Instructions skipped when taken (the branch offset is `4 * (skip + 1)`).
        skip: u8,
    },
}

/// Encode one op as an RV32I word with the repository's encoder.
pub fn assemble_op(op: &Op) -> u32 {
    let r = |x: Reg| x as u32;
    match *op {
        Op::Alu {
            k,
            rd,
            rs1,
            rhs: Rhs::Reg(rs2),
        } => match k {
            Bin::Add => enc::encode_add(r(rd), r(rs1), r(rs2)),
            Bin::Sub => enc::encode_sub(r(rd), r(rs1), r(rs2)),
            Bin::Sll => enc::encode_sll(r(rd), r(rs1), r(rs2)),
            Bin::Slt => enc::encode_slt(r(rd), r(rs1), r(rs2)),
            Bin::Sltu => enc::encode_sltu(r(rd), r(rs1), r(rs2)),
            Bin::Xor => enc::encode_xor(r(rd), r(rs1), r(rs2)),
            Bin::Srl => enc::encode_srl(r(rd), r(rs1), r(rs2)),
            Bin::Sra => enc::encode_sra(r(rd), r(rs1), r(rs2)),
            Bin::Or => enc::encode_or(r(rd), r(rs1), r(rs2)),
            Bin::And => enc::encode_and(r(rd), r(rs1), r(rs2)),
            Bin::Mul => enc::encode_mul(r(rd), r(rs1), r(rs2)),
        },
        Op::Alu {
            k,
            rd,
            rs1,
            rhs: Rhs::Imm(i),
        } => match k {
            Bin::Add => enc::encode_addi(r(rd), r(rs1), i),
            Bin::Slt => enc::encode_slti(r(rd), r(rs1), i),
            Bin::Sltu => enc::encode_sltiu(r(rd), r(rs1), i),
            Bin::Xor => enc::encode_xori(r(rd), r(rs1), i),
            Bin::Or => enc::encode_ori(r(rd), r(rs1), i),
            Bin::And => enc::encode_andi(r(rd), r(rs1), i),
            Bin::Sll => enc::encode_slli(r(rd), r(rs1), i as u32 & 31),
            Bin::Srl => enc::encode_srli(r(rd), r(rs1), i as u32 & 31),
            Bin::Sra => enc::encode_srai(r(rd), r(rs1), i as u32 & 31),
            Bin::Sub | Bin::Mul => panic!("no immediate form of {k:?}"),
        },
        Op::Lui { rd, imm20 } => enc::encode_lui(r(rd), imm20),
        Op::Lw { rd, base, off } => enc::encode_lw(r(rd), r(base), off),
        Op::Lbu { rd, base, off } => enc::encode_lbu(r(rd), r(base), off),
        Op::Sw { rs, base, off } => enc::encode_sw(r(rs), r(base), off),
        Op::Sb { rs, base, off } => enc::encode_sb(r(rs), r(base), off),
        Op::Br { c, rs1, rs2, skip } => {
            let off = 4 * (skip as i32 + 1);
            match c {
                Cond::Eq => enc::encode_beq(r(rs1), r(rs2), off),
                Cond::Ne => enc::encode_bne(r(rs1), r(rs2), off),
                Cond::Lt => enc::encode_blt(r(rs1), r(rs2), off),
                Cond::Ge => enc::encode_bge(r(rs1), r(rs2), off),
                Cond::Ltu => enc::encode_bltu(r(rs1), r(rs2), off),
                Cond::Geu => enc::encode_bgeu(r(rs1), r(rs2), off),
            }
        }
    }
}

/// Encode a whole program.
pub fn assemble(ops: &[Op]) -> Vec<u32> {
    ops.iter().map(assemble_op).collect()
}

/// Read one word back with `riscv-simulator::decode`.  The stepper only ever sees what
/// this function returns, so the probe consumes the same decoder as the simulator.
pub fn decode_word(word: u32) -> Result<Op, String> {
    let d = decode(word, 0);
    let f = |name: &str| -> Result<i32, String> {
        d.fields
            .get(name)
            .copied()
            .ok_or_else(|| format!("{} has no field {name}", d.mnemonic))
    };
    let reg = |name: &str| -> Result<Reg, String> { Ok(f(name)? as Reg) };
    let alu_r = |k: Bin| -> Result<Op, String> {
        Ok(Op::Alu {
            k,
            rd: reg("rd")?,
            rs1: reg("rs1")?,
            rhs: Rhs::Reg(reg("rs2")?),
        })
    };
    let alu_i = |k: Bin| -> Result<Op, String> {
        Ok(Op::Alu {
            k,
            rd: reg("rd")?,
            rs1: reg("rs1")?,
            rhs: Rhs::Imm(f("imm")?),
        })
    };
    let br = |c: Cond| -> Result<Op, String> {
        let imm = f("imm")?;
        if imm < 4 || imm % 4 != 0 || imm > 4 * 256 {
            return Err(format!(
                "{}: only forward branches are modelled (offset {imm})",
                d.mnemonic
            ));
        }
        Ok(Op::Br {
            c,
            rs1: reg("rs1")?,
            rs2: reg("rs2")?,
            skip: (imm / 4 - 1) as u8,
        })
    };
    match d.mnemonic.as_str() {
        "add" => alu_r(Bin::Add),
        "sub" => alu_r(Bin::Sub),
        "sll" => alu_r(Bin::Sll),
        "slt" => alu_r(Bin::Slt),
        "sltu" => alu_r(Bin::Sltu),
        "xor" => alu_r(Bin::Xor),
        "srl" => alu_r(Bin::Srl),
        "sra" => alu_r(Bin::Sra),
        "or" => alu_r(Bin::Or),
        "and" => alu_r(Bin::And),
        "mul" => alu_r(Bin::Mul),
        "addi" => alu_i(Bin::Add),
        "slti" => alu_i(Bin::Slt),
        "sltiu" => alu_i(Bin::Sltu),
        "xori" => alu_i(Bin::Xor),
        "ori" => alu_i(Bin::Or),
        "andi" => alu_i(Bin::And),
        "slli" => alu_i(Bin::Sll),
        "srli" => alu_i(Bin::Srl),
        "srai" => alu_i(Bin::Sra),
        "lui" => Ok(Op::Lui {
            rd: reg("rd")?,
            imm20: (f("imm")? as u32) & 0xF_FFFF,
        }),
        "lw" => Ok(Op::Lw {
            rd: reg("rd")?,
            base: reg("rs1")?,
            off: f("imm")?,
        }),
        "lbu" => Ok(Op::Lbu {
            rd: reg("rd")?,
            base: reg("rs1")?,
            off: f("imm")?,
        }),
        "sw" => Ok(Op::Sw {
            rs: reg("rs2")?,
            base: reg("rs1")?,
            off: f("imm")?,
        }),
        "sb" => Ok(Op::Sb {
            rs: reg("rs2")?,
            base: reg("rs1")?,
            off: f("imm")?,
        }),
        "beq" => br(Cond::Eq),
        "bne" => br(Cond::Ne),
        "blt" => br(Cond::Lt),
        "bge" => br(Cond::Ge),
        "bltu" => br(Cond::Ltu),
        "bgeu" => br(Cond::Geu),
        other => Err(format!("{other} is outside the probe's RV32I subset")),
    }
}

/// Decode a whole program.
pub fn decode_all(words: &[u32]) -> Result<Vec<Op>, String> {
    words.iter().map(|&w| decode_word(w)).collect()
}

/// The concrete machine state of [`eval_ops`]: registers and word-addressed memory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Concrete {
    /// The 32 registers (`x0` stays 0).
    pub regs: [u32; 32],
    /// Memory as byte-addressed little-endian words keyed by the aligned address.
    pub mem: BTreeMap<u32, u32>,
}

fn rhs_value(st: &Concrete, rhs: Rhs) -> u32 {
    match rhs {
        Rhs::Reg(r) => st.regs[r as usize],
        Rhs::Imm(i) => i as u32,
    }
}

/// Run an op list on concrete state: the independent reference evaluator of guard G3.
///
/// It shares nothing with the circuit builder but [`sem::concrete`], which is plain integer
/// arithmetic.  Branches are followed (forward only); the executed-instruction count is
/// returned.
pub fn eval_ops(ops: &[Op], st: &mut Concrete) -> usize {
    let mut pc = 0usize;
    let mut steps = 0usize;
    while pc < ops.len() {
        steps += 1;
        match ops[pc] {
            Op::Alu { k, rd, rs1, rhs } => {
                let v = sem::concrete(k, st.regs[rs1 as usize], rhs_value(st, rhs));
                if rd != 0 {
                    st.regs[rd as usize] = v;
                }
            }
            Op::Lui { rd, imm20 } => {
                if rd != 0 {
                    st.regs[rd as usize] = imm20 << 12;
                }
            }
            Op::Lw { rd, base, off } => {
                let a = st.regs[base as usize].wrapping_add(off as u32);
                let v = st.mem.get(&(a & !3)).copied().unwrap_or(0);
                if rd != 0 {
                    st.regs[rd as usize] = v;
                }
            }
            Op::Lbu { rd, base, off } => {
                let a = st.regs[base as usize].wrapping_add(off as u32);
                let w = st.mem.get(&(a & !3)).copied().unwrap_or(0);
                if rd != 0 {
                    st.regs[rd as usize] = w >> (8 * (a & 3)) & 0xFF;
                }
            }
            Op::Sw { rs, base, off } => {
                let a = st.regs[base as usize].wrapping_add(off as u32);
                st.mem.insert(a & !3, st.regs[rs as usize]);
            }
            Op::Sb { rs, base, off } => {
                let a = st.regs[base as usize].wrapping_add(off as u32);
                let w = st.mem.get(&(a & !3)).copied().unwrap_or(0);
                let sh = 8 * (a & 3);
                st.mem.insert(
                    a & !3,
                    (w & !(0xFF << sh)) | ((st.regs[rs as usize] & 0xFF) << sh),
                );
            }
            Op::Br { c, rs1, rs2, skip } => {
                let (x, y) = (st.regs[rs1 as usize], st.regs[rs2 as usize]);
                let taken = match c {
                    Cond::Eq => x == y,
                    Cond::Ne => x != y,
                    Cond::Lt => (x as i32) < (y as i32),
                    Cond::Ge => (x as i32) >= (y as i32),
                    Cond::Ltu => x < y,
                    Cond::Geu => x >= y,
                };
                if taken {
                    pc += skip as usize + 1;
                    continue;
                }
            }
        }
        st.regs[0] = 0;
        pc += 1;
    }
    steps
}

/// A small deterministic generator (SplitMix64): the probe's only source of randomness, so a
/// row reproduces from `(experiment, knob, seed)` (guard G4).
#[derive(Clone, Debug)]
pub struct Rng(pub u64);

impl Rng {
    /// A generator from a seed and a stream tag (distinct tags give unrelated streams).
    pub fn new(seed: u64, tag: u64) -> Rng {
        let mut r = Rng(seed ^ tag.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        r.next_u64();
        r
    }
    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A value below `n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// A float in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

/// A generated program together with the reference side's per-instruction encodings.
#[derive(Clone, Debug)]
pub struct BProg {
    /// The instructions (the code side's program).
    pub ops: Vec<Op>,
    /// For each instruction: `Some(v)` if the reference encodes it with alternative `v`.
    pub alt: Vec<Option<u32>>,
}

/// Whether an op is entropy-killing for guard G2: comparisons, shifts, `and` with a small
/// constant.  (They map many inputs to few outputs.)
pub fn is_killing(op: &Op) -> bool {
    matches!(
        op,
        Op::Alu {
            k: Bin::Slt | Bin::Sltu | Bin::Sll | Bin::Srl | Bin::Sra,
            ..
        } | Op::Alu {
            k: Bin::And,
            rhs: Rhs::Imm(_),
            ..
        }
    )
}

/// The alternative encodings the reference may use for `op`: how many, and for an operation
/// with a constant operand which single one survives hash-consing (see
/// [`sem::imm_alternative`]).
pub fn alt_choices(op: &Op) -> Vec<u32> {
    match op {
        Op::Alu {
            k,
            rhs: Rhs::Reg(_),
            ..
        } => (0..sem::alt_count(*k)).collect(),
        Op::Alu {
            k,
            rhs: Rhs::Imm(_),
            ..
        } => sem::imm_alternative(*k).into_iter().collect(),
        _ => vec![],
    }
}

/// Whether the reference can encode `op` differently.
pub fn has_alt(op: &Op) -> bool {
    !alt_choices(op).is_empty()
}

/// Generate a seeded straight-line program of `len` instructions over `x1..x8`.
///
/// The program is built from three kinds of unit so that it stays *live* (guard G2): no unit
/// destroys the history a register has accumulated.
///
/// ```text
///   plain   accR = accR  op  accS      op in add, sub, xor     (bijective in accR)
///           accR = accR  op  imm       op in addi, xori
///   load    lw   x10, DATA_BASE + 4k(x0)      a fresh symbolic word, every LOAD_EVERY
///           accR = accR (add|xor|sub) x10     instructions, absorbed at once
///   kill    x9 = (sltu|slt|slli|srli|srai|andi 0xFF) of the accumulators   an entropy-killing
///           accR = accR (add|xor) x9          operation, its lossy result absorbed at once
/// ```
///
/// Entropy-killing operations are capped at [`KILL_SHARE`] percent of the program, and they
/// write the scratch register `x9`, never an accumulator.  (An earlier draft let `and`, `or`,
/// shifts and comparisons overwrite accumulators; its programs lost entropy within a few
/// hundred instructions, exactly the failure of the advisor's table B2 at n of 200 or more.)
///
/// `round(divergence * candidates)` of the *live* instructions that have an alternative (at
/// least one for a positive `divergence`) are marked for the reference's alternative circuit.
pub fn gen_b(seed: u64, len: usize, divergence: f64) -> BProg {
    let mut rng = Rng::new(seed, 0xB0);
    let mut ops: Vec<Op> = Vec::with_capacity(len + 2);
    let mut killers = 0usize;
    let mut loads = 0u32;
    let mut next_load = LOAD_EVERY - 1;
    let acc = |rng: &mut Rng| 1 + rng.below(8) as Reg;
    let merge = |rng: &mut Rng, src: Reg, dst: Reg| match rng.below(3) {
        0 => Op::Alu {
            k: Bin::Add,
            rd: dst,
            rs1: dst,
            rhs: Rhs::Reg(src),
        },
        1 => Op::Alu {
            k: Bin::Xor,
            rd: dst,
            rs1: dst,
            rhs: Rhs::Reg(src),
        },
        _ => Op::Alu {
            k: Bin::Sub,
            rd: dst,
            rs1: dst,
            rhs: Rhs::Reg(src),
        },
    };
    while ops.len() < len {
        let at = ops.len();
        let room = len - at;
        if at == next_load {
            ops.push(Op::Lw {
                rd: 10,
                base: 0,
                off: (DATA_BASE + 4 * loads) as i32,
            });
            loads += 1;
            next_load += LOAD_EVERY;
            if room >= 2 {
                let dst = acc(&mut rng);
                ops.push(merge(&mut rng, 10, dst));
            }
            continue;
        }
        let two_fit = room >= 2 && at + 2 <= next_load;
        let may_kill = two_fit && (killers + 1) * 100 <= KILL_SHARE * (at + 2) && rng.unit() < 0.25;
        if may_kill {
            let (a, b) = (acc(&mut rng), acc(&mut rng));
            let kill = match rng.below(6) {
                0 => Op::Alu {
                    k: Bin::Sltu,
                    rd: 9,
                    rs1: a,
                    rhs: Rhs::Reg(b),
                },
                1 => Op::Alu {
                    k: Bin::Slt,
                    rd: 9,
                    rs1: a,
                    rhs: Rhs::Reg(b),
                },
                2 => Op::Alu {
                    k: Bin::Sll,
                    rd: 9,
                    rs1: a,
                    rhs: Rhs::Imm(1 + rng.below(7) as i32),
                },
                3 => Op::Alu {
                    k: Bin::Srl,
                    rd: 9,
                    rs1: a,
                    rhs: Rhs::Imm(1 + rng.below(7) as i32),
                },
                4 => Op::Alu {
                    k: Bin::Sra,
                    rd: 9,
                    rs1: a,
                    rhs: Rhs::Imm(1 + rng.below(7) as i32),
                },
                _ => Op::Alu {
                    k: Bin::And,
                    rd: 9,
                    rs1: a,
                    rhs: Rhs::Imm(0xFF),
                },
            };
            killers += 1;
            ops.push(kill);
            let dst = acc(&mut rng);
            ops.push(if rng.below(2) == 0 {
                Op::Alu {
                    k: Bin::Add,
                    rd: dst,
                    rs1: dst,
                    rhs: Rhs::Reg(9),
                }
            } else {
                Op::Alu {
                    k: Bin::Xor,
                    rd: dst,
                    rs1: dst,
                    rhs: Rhs::Reg(9),
                }
            });
            continue;
        }
        let a = acc(&mut rng);
        let mut b = acc(&mut rng);
        if b == a {
            b = 1 + (a % 8);
        }
        ops.push(match rng.below(10) {
            0..=3 => Op::Alu {
                k: Bin::Add,
                rd: a,
                rs1: a,
                rhs: Rhs::Reg(b),
            },
            4..=5 => Op::Alu {
                k: Bin::Sub,
                rd: a,
                rs1: a,
                rhs: Rhs::Reg(b),
            },
            6..=7 => Op::Alu {
                k: Bin::Xor,
                rd: a,
                rs1: a,
                rhs: Rhs::Reg(b),
            },
            8 => Op::Alu {
                k: Bin::Add,
                rd: a,
                rs1: a,
                rhs: Rhs::Imm(1 + rng.below(100) as i32),
            },
            _ => Op::Alu {
                k: Bin::Xor,
                rd: a,
                rs1: a,
                rhs: Rhs::Imm(1 + rng.below(2000) as i32),
            },
        });
    }
    ops.truncate(len);
    let mut alt = vec![None; len];
    // Only live instructions can be diverged: a different encoding of dead code is invisible.
    let live = live_ops(&ops);
    let mut candidates: Vec<usize> = (0..len).filter(|&i| has_alt(&ops[i]) && live[i]).collect();
    rng.shuffle(&mut candidates);
    let want = if divergence <= 0.0 {
        0
    } else {
        ((divergence * candidates.len() as f64).round() as usize).max(1)
    };
    for &i in candidates.iter().take(want) {
        let choices = alt_choices(&ops[i]);
        alt[i] = Some(choices[rng.below(choices.len())]);
    }
    BProg { ops, alt }
}

/// Which instructions of a straight-line program can influence a final register: a backward
/// slice from every register the program writes.
pub fn live_ops(ops: &[Op]) -> Vec<bool> {
    let mut live_regs = [false; 32];
    for op in ops {
        match *op {
            Op::Alu { rd, .. } | Op::Lui { rd, .. } | Op::Lw { rd, .. } | Op::Lbu { rd, .. } => {
                live_regs[rd as usize] = true
            }
            _ => {}
        }
    }
    let mut live = vec![false; ops.len()];
    for i in (0..ops.len()).rev() {
        let (writes, reads): (Option<Reg>, Vec<Reg>) = match ops[i] {
            Op::Alu {
                rd,
                rs1,
                rhs: Rhs::Reg(r2),
                ..
            } => (Some(rd), vec![rs1, r2]),
            Op::Alu {
                rd,
                rs1,
                rhs: Rhs::Imm(_),
                ..
            } => (Some(rd), vec![rs1]),
            Op::Lui { rd, .. } => (Some(rd), vec![]),
            Op::Lw { rd, base, .. } | Op::Lbu { rd, base, .. } => (Some(rd), vec![base]),
            Op::Sw { rs, base, .. } | Op::Sb { rs, base, .. } => (None, vec![rs, base]),
            Op::Br { rs1, rs2, .. } => (None, vec![rs1, rs2]),
        };
        let needed = match writes {
            Some(rd) => rd != 0 && live_regs[rd as usize],
            None => true,
        };
        if needed {
            live[i] = true;
            if let Some(rd) = writes {
                live_regs[rd as usize] = false;
            }
            for r in reads {
                live_regs[r as usize] = true;
            }
        }
    }
    live
}

/// A one-instruction change that makes the code side compute something else (the G3
/// mutation): the kind of a register operation, or the immediate of an immediate one.
pub fn mutate_op(op: &Op) -> Option<Op> {
    match *op {
        Op::Alu {
            k,
            rd,
            rs1,
            rhs: Rhs::Reg(rs2),
        } => {
            let k2 = match k {
                Bin::Add => Bin::Sub,
                Bin::Sub => Bin::Add,
                Bin::Xor => Bin::Or,
                Bin::Or => Bin::Xor,
                Bin::And => Bin::Or,
                Bin::Sll => Bin::Srl,
                Bin::Srl => Bin::Sll,
                Bin::Sra => Bin::Srl,
                Bin::Slt => Bin::Sltu,
                Bin::Sltu => Bin::Slt,
                Bin::Mul => Bin::Add,
            };
            Some(Op::Alu {
                k: k2,
                rd,
                rs1,
                rhs: Rhs::Reg(rs2),
            })
        }
        Op::Alu {
            k,
            rd,
            rs1,
            rhs: Rhs::Imm(i),
        } => {
            let i2 = if matches!(k, Bin::Sll | Bin::Srl | Bin::Sra) {
                (i + 1) & 31
            } else {
                i ^ 1
            };
            Some(Op::Alu {
                k,
                rd,
                rs1,
                rhs: Rhs::Imm(i2),
            })
        }
        _ => None,
    }
}

/// The indices at which to try a control mutation, best first: the last instructions of the
/// program that have a mutation, seeded (a late mutation is read by few later instructions
/// and so is found quickly by the solver).
pub fn mutation_sites(seed: u64, ops: &[Op]) -> Vec<usize> {
    let mut rng = Rng::new(seed, 0xC7);
    let mut sites: Vec<usize> = (0..ops.len())
        .rev()
        .filter(|&i| mutate_op(&ops[i]).is_some())
        .take(12)
        .collect();
    let n = sites.len().min(4);
    rng.shuffle(&mut sites[..n]);
    sites
}

#[cfg(test)]
mod tests {
    use super::*;
    use riscv_simulator::simulator::RiscVSimulator;

    #[test]
    fn decode_inverts_assemble_for_every_op_shape() {
        let samples = [
            Op::Alu {
                k: Bin::Add,
                rd: 3,
                rs1: 4,
                rhs: Rhs::Reg(5),
            },
            Op::Alu {
                k: Bin::Sub,
                rd: 3,
                rs1: 4,
                rhs: Rhs::Reg(5),
            },
            Op::Alu {
                k: Bin::Mul,
                rd: 3,
                rs1: 4,
                rhs: Rhs::Reg(5),
            },
            Op::Alu {
                k: Bin::Sra,
                rd: 1,
                rs1: 2,
                rhs: Rhs::Imm(7),
            },
            Op::Alu {
                k: Bin::Sltu,
                rd: 1,
                rs1: 2,
                rhs: Rhs::Imm(-1),
            },
            Op::Alu {
                k: Bin::Xor,
                rd: 1,
                rs1: 2,
                rhs: Rhs::Imm(-5),
            },
            Op::Lui {
                rd: 9,
                imm20: 0x12345,
            },
            Op::Lw {
                rd: 2,
                base: 0,
                off: 0x100,
            },
            Op::Lbu {
                rd: 2,
                base: 1,
                off: 3,
            },
            Op::Sw {
                rs: 2,
                base: 0,
                off: 0x104,
            },
            Op::Sb {
                rs: 2,
                base: 0,
                off: 0x105,
            },
            Op::Br {
                c: Cond::Ltu,
                rs1: 1,
                rs2: 2,
                skip: 3,
            },
        ];
        for op in samples {
            assert_eq!(decode_word(assemble_op(&op)).unwrap(), op);
        }
    }

    #[test]
    fn decode_refuses_what_is_outside_the_subset() {
        assert!(decode_word(enc::encode_ecall()).is_err());
        assert!(
            decode_word(enc::encode_beq(1, 2, -8)).is_err(),
            "a backward branch is not modelled"
        );
        assert!(decode_word(enc::encode_lb(1, 2, 0)).is_err());
    }

    #[test]
    fn the_independent_evaluator_agrees_with_the_real_simulator() {
        // Every generated program is run on both; they must end in the same registers.
        for seed in 0..6u64 {
            let p = gen_b(seed, 60, 0.0);
            let mut st = Concrete::default();
            let mut s = Rng::new(seed, 1);
            for reg in st.regs.iter_mut().take(9).skip(1) {
                *reg = s.next_u64() as u32;
            }
            for k in 0..12u32 {
                st.mem.insert(DATA_BASE + 4 * k, s.next_u64() as u32);
            }
            let init = st.clone();
            let mut st2 = init.clone();
            eval_ops(&p.ops, &mut st2);

            let mut sim = RiscVSimulator::new(1 << 16);
            let words = assemble(&p.ops);
            let prog: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            sim.mem.load_bytes(0x4000, &prog);
            sim.pc = 0x4000;
            for (r, v) in init.regs.iter().enumerate().take(9).skip(1) {
                sim.regs.write(r, *v);
            }
            for (a, v) in &init.mem {
                sim.mem.write_word(*a as usize, *v);
            }
            for _ in 0..p.ops.len() {
                sim.step();
            }
            for (r, v) in st2.regs.iter().enumerate().take(32).skip(1) {
                assert_eq!(sim.regs.read(r), *v, "seed {seed} x{r}");
            }
        }
    }

    #[test]
    fn forward_branches_skip_and_fall_through() {
        let prog = [
            Op::Alu {
                k: Bin::Add,
                rd: 1,
                rs1: 0,
                rhs: Rhs::Imm(5),
            },
            Op::Br {
                c: Cond::Eq,
                rs1: 1,
                rs2: 0,
                skip: 1,
            },
            Op::Alu {
                k: Bin::Add,
                rd: 2,
                rs1: 0,
                rhs: Rhs::Imm(7),
            },
            Op::Br {
                c: Cond::Ne,
                rs1: 1,
                rs2: 0,
                skip: 1,
            },
            Op::Alu {
                k: Bin::Add,
                rd: 3,
                rs1: 0,
                rhs: Rhs::Imm(9),
            },
        ];
        let mut st = Concrete::default();
        let steps = eval_ops(&prog, &mut st);
        assert_eq!(
            (st.regs[2], st.regs[3]),
            (7, 0),
            "first branch falls through, second is taken"
        );
        assert_eq!(steps, 4);
    }

    #[test]
    fn the_generator_obeys_the_liveness_rules_and_is_deterministic() {
        let p = gen_b(3, 300, 0.25);
        assert_eq!(p.ops.len(), 300);
        let loads = p.ops.iter().filter(|o| matches!(o, Op::Lw { .. })).count();
        assert_eq!(
            loads,
            300 / LOAD_EVERY,
            "one fresh load per LOAD_EVERY instructions"
        );
        let kill = p.ops.iter().filter(|o| is_killing(o)).count();
        assert!(
            kill * 100 <= KILL_SHARE * 300,
            "{kill} entropy-killing operations"
        );
        assert_eq!(gen_b(3, 300, 0.25).ops, p.ops);
        assert_ne!(gen_b(4, 300, 0.25).ops, p.ops);
        let marked = p.alt.iter().filter(|a| a.is_some()).count();
        let cand = p.ops.iter().filter(|o| has_alt(o)).count();
        assert_eq!(marked, (0.25 * cand as f64).round() as usize);
        assert!(gen_b(3, 300, 0.0).alt.iter().all(|a| a.is_none()));
        assert!(
            gen_b(3, 25, 0.05).alt.iter().any(|a| a.is_some()),
            "a positive divergence marks at least one"
        );
    }

    #[test]
    fn a_mutation_changes_the_op_and_only_that_op() {
        let p = gen_b(1, 40, 0.0);
        let sites = mutation_sites(1, &p.ops);
        assert!(!sites.is_empty());
        for s in sites {
            let m = mutate_op(&p.ops[s]).unwrap();
            assert_ne!(m, p.ops[s]);
        }
        assert!(mutate_op(&Op::Lw {
            rd: 1,
            base: 0,
            off: 0
        })
        .is_none());
    }
}
