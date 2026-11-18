//! # `cpu`: the probe-private minimal symbolic RV32I stepper
//!
//! VOS30 section 9.1: instance generation is a stepper of a few hundred lines, built on the
//! `bit-blaster` (`Aig`, `bv`), reading instructions through `riscv-simulator::decode`.  It
//! is **not** VOS-029's executor and is in no trusted base: it is a measurement instrument.
//!
//! ## The machine
//!
//! ```text
//!   registers   32 x BitVec(32); x0 is the constant 0; a register that has not been written
//!               is read as a *fresh symbolic word* the first time (a live-in)
//!   memory      concrete word addresses: a map address -> word, filled with fresh symbolic
//!               words on first read.  Once a store with a symbolic address has been made,
//!               every later access consults a *store log* (the `sym.rs` eq/ite chain):
//!                   load(a) = ite(a == w_k.addr, w_k.val, ite(a == w_{k-1}.addr, ... base(a)))
//!               and a symbolic read of a never-written word reads the declared table region
//!               as another eq/ite chain over its words.
//!   branches    forward only, with a concrete PC: the caller chooses a direction and the
//!               machine records the path condition (cond or not cond) as a premise.
//! ```
//!
//! ## Symbolic inputs
//!
//! Every fresh input is a named group of AIG inputs in a [`Pool`]; the pool is the probe's
//! *declared input list* (guard G2 counts declared bits from it, never from a generator's
//! self-report).  The two sides of a miter either share the pool's inputs (the monolithic
//! form) or get one input per side with an equality premise (the cut form of section 9.5):
//! the machine's `suffix` (`""`, `"@c"`, `"@r"`) names which.

use crate::prog::{Cond, Op, Rhs, Rng};
use crate::sem;
use bit_blaster::aig::{Aig, Edge, Node};
use bit_blaster::bv::{self, BitVec};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A declared symbolic input: a named group of AIG inputs (least significant bit first).
#[derive(Clone, Debug)]
pub struct Declared {
    /// The name, e.g. `x3`, `m100@c`.
    pub name: String,
    /// Symbolic bits; the machine word is this many bits wide, zero-extended to 32.
    pub bits: Vec<Edge>,
}

/// The inputs of an instance, created on demand.
#[derive(Debug, Default)]
pub struct Pool {
    /// Every declared group, in creation order.
    pub declared: Vec<Declared>,
    by_name: HashMap<String, usize>,
    widths: HashMap<String, usize>,
}

impl Pool {
    /// An empty pool.
    pub fn new() -> Pool {
        Pool::default()
    }

    /// Declare that the group named `name` is `width` bits wide (default 32).
    pub fn set_width(&mut self, name: &str, width: usize) {
        self.widths.insert(name.to_string(), width);
    }

    /// Create the listed groups now, with the AIG input *numbering* shuffled by `rng`: the
    /// seeded source of solver variance for the deterministic shapes (VOS30 section 9.10).
    pub fn declare_shuffled(&mut self, aig: &mut Aig, specs: &[(String, usize)], rng: &mut Rng) {
        let total: usize = specs.iter().map(|s| s.1).sum();
        let ins: Vec<Edge> = (0..total).map(|_| aig.input()).collect();
        let mut perm: Vec<usize> = (0..total).collect();
        rng.shuffle(&mut perm);
        let mut at = 0;
        for (name, w) in specs {
            let bits: Vec<Edge> = (0..*w).map(|b| ins[perm[at + b]]).collect();
            at += w;
            self.by_name.insert(name.clone(), self.declared.len());
            self.widths.insert(name.clone(), *w);
            self.declared.push(Declared {
                name: name.clone(),
                bits,
            });
        }
    }

    /// The 32-bit word of the group `name`, creating it if need be.
    pub fn word(&mut self, aig: &mut Aig, name: &str) -> BitVec {
        let idx = match self.by_name.get(name) {
            Some(&i) => i,
            None => {
                let w = self.widths.get(name).copied().unwrap_or(32);
                let bits: Vec<Edge> = (0..w).map(|_| aig.input()).collect();
                self.by_name.insert(name.to_string(), self.declared.len());
                self.declared.push(Declared {
                    name: name.to_string(),
                    bits,
                });
                self.declared.len() - 1
            }
        };
        let bits = &self.declared[idx].bits;
        BitVec {
            bits: (0..32)
                .map(|i| bits.get(i).copied().unwrap_or(Edge::FALSE))
                .collect(),
        }
    }

    /// Whether `name` has been declared.
    pub fn has(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// The total number of declared bits.
    pub fn declared_bits(&self) -> usize {
        self.declared.iter().map(|d| d.bits.len()).sum()
    }
}

/// The value of a word whose every bit is a constant.
pub fn as_const(w: &BitVec) -> Option<u32> {
    let mut v = 0u32;
    for (i, e) in w.bits.iter().enumerate() {
        if !e.is_constant() {
            return None;
        }
        // Edge::TRUE is node 0 complemented.
        if *e == Edge::TRUE {
            v |= 1 << i;
        }
    }
    Some(v)
}

/// A 32-bit constant word.
pub fn konst(v: u32) -> BitVec {
    BitVec::constant(32, v as u128)
}

/// A symbolic machine.
pub struct Cpu {
    regs: Vec<Option<BitVec>>,
    conc: BTreeMap<u32, BitVec>,
    log: Vec<(BitVec, BitVec)>,
    region: Option<(u32, usize)>,
    suffix: String,
    /// Path conditions recorded at symbolic branches (premises of the instance).
    pub path: Vec<Edge>,
    /// Registers written, in the order of first write.
    pub written_regs: BTreeSet<u8>,
    /// Concrete word addresses stored to.
    pub written_words: BTreeSet<u32>,
}

impl Cpu {
    /// A machine whose fresh inputs are named with `suffix`.
    pub fn new(suffix: &str) -> Cpu {
        Cpu {
            regs: vec![None; 32],
            conc: BTreeMap::new(),
            log: Vec::new(),
            region: None,
            suffix: suffix.to_string(),
            path: Vec::new(),
            written_regs: BTreeSet::new(),
            written_words: BTreeSet::new(),
        }
    }

    /// Declare the table region (`words` words from `base`) that a symbolic address may read.
    pub fn set_region(&mut self, base: u32, words: usize) {
        self.region = Some((base, words));
    }

    /// Set a register to a given word (e.g. a constant or a premised input).
    pub fn set_reg(&mut self, r: u8, w: BitVec) {
        self.regs[r as usize] = Some(w);
    }

    /// Read a register: `x0` is 0; an unwritten register is a fresh live-in input.
    pub fn read_reg(&mut self, aig: &mut Aig, pool: &mut Pool, r: u8) -> BitVec {
        if r == 0 {
            return konst(0);
        }
        if self.regs[r as usize].is_none() {
            let w = pool.word(aig, &format!("x{r}{}", self.suffix));
            self.regs[r as usize] = Some(w);
        }
        self.regs[r as usize].clone().expect("just set")
    }

    fn write_reg(&mut self, r: u8, w: BitVec) {
        if r != 0 {
            self.regs[r as usize] = Some(w);
            self.written_regs.insert(r);
        }
    }

    /// Whether register `r` has been read or written.
    pub fn touched(&self, r: u8) -> bool {
        self.regs[r as usize].is_some()
    }

    fn base_word(&mut self, aig: &mut Aig, pool: &mut Pool, a: u32) -> BitVec {
        if let Some(w) = self.conc.get(&a) {
            return w.clone();
        }
        let w = pool.word(aig, &format!("m{a:x}{}", self.suffix));
        self.conc.insert(a, w.clone());
        w
    }

    /// Read a word at address `addr` (word aligned for a symbolic one).
    pub fn read_word(
        &mut self,
        aig: &mut Aig,
        pool: &mut Pool,
        addr: &BitVec,
    ) -> Result<BitVec, String> {
        let mut base = match as_const(addr) {
            Some(a) => self.base_word(aig, pool, a & !3),
            None => {
                let (rb, n) = self
                    .region
                    .ok_or("a symbolic address needs a declared table region")?;
                let mut acc = konst(0);
                for i in (0..n).rev() {
                    let ai = rb + 4 * i as u32;
                    let wi = self.base_word(aig, pool, ai);
                    let hit = bv::eq(aig, addr, &konst(ai));
                    acc = bv::ite(aig, hit, &wi, &acc);
                }
                acc
            }
        };
        for (la, lv) in &self.log {
            // Oldest first, so a newer store wraps an older one: ite(a == newest, v, ... older ...).
            let hit = bv::eq(aig, addr, la);
            base = bv::ite(aig, hit, lv, &base);
        }
        Ok(base)
    }

    /// Write a word at `addr`.
    pub fn write_word(&mut self, addr: &BitVec, val: BitVec) {
        match as_const(addr) {
            Some(a) if self.log.is_empty() => {
                self.conc.insert(a & !3, val);
                self.written_words.insert(a & !3);
            }
            _ => self.log.push((addr.clone(), val)),
        }
    }

    /// Execute one instruction.  A branch is *not* taken here: its condition is returned and
    /// [`Cpu::run`] decides.  `alt` selects the alternative circuit of an ALU operation.
    pub fn exec(
        &mut self,
        aig: &mut Aig,
        pool: &mut Pool,
        op: &Op,
        alt: Option<u32>,
    ) -> Result<Option<Edge>, String> {
        match *op {
            Op::Alu { k, rd, rs1, rhs } => {
                let a = self.read_reg(aig, pool, rs1);
                let b = match rhs {
                    Rhs::Reg(r) => self.read_reg(aig, pool, r),
                    Rhs::Imm(i) => konst(i as u32),
                };
                let v = match alt.and_then(|v| sem::alternative(aig, k, v, &a, &b)) {
                    Some(w) => w,
                    None => sem::primary(aig, k, &a, &b),
                };
                self.write_reg(rd, v);
                Ok(None)
            }
            Op::Lui { rd, imm20 } => {
                self.write_reg(rd, konst(imm20 << 12));
                Ok(None)
            }
            Op::Lw { rd, base, off } => {
                let b = self.read_reg(aig, pool, base);
                let addr = bv::add(aig, &b, &konst(off as u32));
                let v = self.read_word(aig, pool, &addr)?;
                self.write_reg(rd, v);
                Ok(None)
            }
            Op::Sw { rs, base, off } => {
                let b = self.read_reg(aig, pool, base);
                let addr = bv::add(aig, &b, &konst(off as u32));
                let v = self.read_reg(aig, pool, rs);
                self.write_word(&addr, v);
                Ok(None)
            }
            Op::Lbu { rd, base, off } => {
                let b = self.read_reg(aig, pool, base);
                let addr = bv::add(aig, &b, &konst(off as u32));
                let a = as_const(&addr).ok_or("lbu needs a concrete address")?;
                if !self.log.is_empty() {
                    return Err("lbu after a symbolic store is not modelled".into());
                }
                let w = self.base_word(aig, pool, a & !3);
                let lane = (a & 3) as usize * 8;
                let bits: Vec<Edge> = (0..32)
                    .map(|i| if i < 8 { w.bits[lane + i] } else { Edge::FALSE })
                    .collect();
                self.write_reg(rd, BitVec { bits });
                Ok(None)
            }
            Op::Sb { rs, base, off } => {
                let b = self.read_reg(aig, pool, base);
                let addr = bv::add(aig, &b, &konst(off as u32));
                let a = as_const(&addr).ok_or("sb needs a concrete address")?;
                if !self.log.is_empty() {
                    return Err("sb after a symbolic store is not modelled".into());
                }
                let v = self.read_reg(aig, pool, rs);
                let mut w = self.base_word(aig, pool, a & !3);
                let lane = (a & 3) as usize * 8;
                for i in 0..8 {
                    w.bits[lane + i] = v.bits[i];
                }
                self.conc.insert(a & !3, w);
                self.written_words.insert(a & !3);
                Ok(None)
            }
            Op::Br { c, rs1, rs2, .. } => {
                let a = self.read_reg(aig, pool, rs1);
                let b = self.read_reg(aig, pool, rs2);
                Ok(Some(match c {
                    Cond::Eq => bv::eq(aig, &a, &b),
                    Cond::Ne => !bv::eq(aig, &a, &b),
                    Cond::Lt => bv::slt(aig, &a, &b),
                    Cond::Ge => !bv::slt(aig, &a, &b),
                    Cond::Ltu => bv::ult(aig, &a, &b),
                    Cond::Geu => !bv::ult(aig, &a, &b),
                }))
            }
        }
    }

    /// Run a whole program with a concrete PC.  At each branch `choose(pc, cond)` says whether
    /// it is taken, and `cond` (or its negation) joins [`Cpu::path`].
    pub fn run(
        &mut self,
        aig: &mut Aig,
        pool: &mut Pool,
        ops: &[Op],
        alts: &[Option<u32>],
        choose: &mut dyn FnMut(usize, Edge) -> bool,
    ) -> Result<(), String> {
        let mut pc = 0usize;
        while pc < ops.len() {
            let alt = alts.get(pc).copied().flatten();
            match self.exec(aig, pool, &ops[pc], alt)? {
                Some(cond) => {
                    let skip = match ops[pc] {
                        Op::Br { skip, .. } => skip as usize,
                        _ => unreachable!("only a branch returns a condition"),
                    };
                    let taken = choose(pc, cond);
                    self.path.push(if taken { cond } else { !cond });
                    pc += if taken { skip + 1 } else { 1 };
                }
                None => pc += 1,
            }
        }
        Ok(())
    }

    /// The final value of register `r`.
    pub fn reg(&mut self, aig: &mut Aig, pool: &mut Pool, r: u8) -> BitVec {
        self.read_reg(aig, pool, r)
    }
}

/// The input index of an AIG edge that is an input node.
pub fn input_index(aig: &Aig, e: Edge) -> Option<usize> {
    match aig.node(e.node()) {
        Node::Input(i) => Some(i),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prog::{assemble, decode_all, eval_ops, Concrete};
    use crate::sem::Bin;

    fn bool_inputs(vals: &[(usize, u32)], n: usize) -> Vec<bool> {
        let mut v = vec![false; n];
        for &(start, x) in vals {
            for i in 0..32 {
                v[start + i] = x >> i & 1 == 1;
            }
        }
        v
    }

    #[test]
    fn the_stepper_agrees_with_the_concrete_evaluator_on_random_programs() {
        for seed in 0..5u64 {
            let p = crate::prog::gen_b(seed, 40, 0.0);
            // The code side reads what the real decoder says.
            let ops = decode_all(&assemble(&p.ops)).unwrap();
            let mut aig = Aig::new();
            let mut pool = Pool::new();
            let mut cpu = Cpu::new("");
            cpu.run(&mut aig, &mut pool, &ops, &[], &mut |_, _| false)
                .unwrap();
            let outs: Vec<(u8, BitVec)> = cpu
                .written_regs
                .clone()
                .into_iter()
                .map(|r| (r, cpu.reg(&mut aig, &mut pool, r)))
                .collect();
            let mut rng = Rng::new(seed, 99);
            for _ in 0..4 {
                let mut st = Concrete::default();
                let mut assign: Vec<(usize, u32)> = Vec::new();
                for d in &pool.declared {
                    let v = rng.next_u64() as u32;
                    let start = match aig.node(d.bits[0].node()) {
                        Node::Input(i) => i,
                        _ => unreachable!(),
                    };
                    assign.push((start, v));
                    if let Some(r) = d.name.strip_prefix('x') {
                        st.regs[r.parse::<usize>().unwrap()] = v;
                    } else if let Some(a) = d.name.strip_prefix('m') {
                        st.mem.insert(u32::from_str_radix(a, 16).unwrap(), v);
                    }
                }
                let inputs = bool_inputs(&assign, aig.num_inputs());
                eval_ops(&p.ops, &mut st);
                for (r, w) in &outs {
                    assert_eq!(
                        sem::eval_word(&aig, w, &inputs),
                        st.regs[*r as usize],
                        "seed {seed} x{r}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_symbolic_store_log_reads_back_like_an_array() {
        // table of 4 words at 0x40; a symbolic index i: sw v, (i<<2); lw r, 0x40+... via the log.
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        pool.set_width("x1", 2);
        let mut cpu = Cpu::new("");
        cpu.set_region(0x40, 4);
        let prog = [
            Op::Alu {
                k: Bin::Sll,
                rd: 2,
                rs1: 1,
                rhs: Rhs::Imm(2),
            },
            Op::Sw {
                rs: 3,
                base: 2,
                off: 0x40,
            },
            Op::Lw {
                rd: 4,
                base: 2,
                off: 0x40,
            },
        ];
        cpu.run(&mut aig, &mut pool, &prog, &[], &mut |_, _| false)
            .unwrap();
        let r4 = cpu.reg(&mut aig, &mut pool, 4);
        let r3 = cpu.reg(&mut aig, &mut pool, 3);
        // r4 == r3 is a tautology: what was just stored at i is what is loaded from i.
        let same = bv::eq(&mut aig, &r4, &r3);
        let mut cnf = cdcl_sat::Cnf::new(0);
        bit_blaster::tseitin::Tseitin::new().assert_true(&aig, !same, &mut cnf);
        let mut s = cdcl_sat::Solver::from_cnf(&cnf);
        assert_eq!(s.solve(), cdcl_sat::SolveResult::Unsat);
    }

    #[test]
    fn a_symbolic_address_without_a_region_is_refused() {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let mut cpu = Cpu::new("");
        let r = cpu.run(
            &mut aig,
            &mut pool,
            &[Op::Lw {
                rd: 2,
                base: 1,
                off: 0,
            }],
            &[],
            &mut |_, _| false,
        );
        assert!(r.is_err());
    }

    #[test]
    fn byte_lanes_round_trip_and_branches_record_the_path() {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let mut cpu = Cpu::new("");
        let prog = [
            Op::Sb {
                rs: 1,
                base: 0,
                off: 0x101,
            },
            Op::Lbu {
                rd: 2,
                base: 0,
                off: 0x101,
            },
            Op::Br {
                c: Cond::Eq,
                rs1: 1,
                rs2: 2,
                skip: 1,
            },
            Op::Alu {
                k: Bin::Add,
                rd: 3,
                rs1: 0,
                rhs: Rhs::Imm(1),
            },
        ];
        cpu.run(&mut aig, &mut pool, &prog, &[], &mut |_, _| true)
            .unwrap();
        assert_eq!(cpu.path.len(), 1);
        assert!(!cpu.touched(3), "the taken branch skipped the add");
        assert!(cpu.written_words.contains(&0x100));
    }

    #[test]
    fn an_unsupported_byte_access_after_a_symbolic_store_is_an_error() {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let mut cpu = Cpu::new("");
        cpu.set_region(0x40, 2);
        cpu.write_word(&konst(0), konst(0)); // concrete: goes to the map
        let sym = pool.word(&mut aig, "x9");
        cpu.write_word(&sym, konst(1)); // symbolic: starts the log
        let r = cpu.exec(
            &mut aig,
            &mut pool,
            &Op::Lbu {
                rd: 1,
                base: 0,
                off: 0x100,
            },
            None,
        );
        assert!(r.is_err());
    }
}
