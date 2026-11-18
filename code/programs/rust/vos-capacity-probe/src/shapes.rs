//! # `shapes`: the obligation shapes of experiments A, B, C and D as [`Instance`]s
//!
//! Every shape is a function from `(parameters, seed, mutation)` to an [`Instance`]; the same
//! function with a `mutation` is the **G3 control** (one mutated instruction, access or
//! variant), whose miter must be SAT and whose model must replay.  The definitions are
//! the table of VOS30 section 9.10; this file is where they are built.
//!
//! ## What the code side and the reference side are
//!
//! ```text
//!   code side       RV32I words (riscv-encoder) -> riscv-simulator::decode -> the stepper
//!   reference side  the same computation described a second way: alternative circuits
//!                   (B, D), a priority chain (A1), per-bit updates (A2), an array with a
//!                   mux tree (D), a different multiplier (B4)
//!   replay          the code side on the real RiscVSimulator; the reference side on a plain
//!                   Rust evaluator that shares nothing with the circuit builders
//! ```

use crate::cpu::{konst, Cpu, Pool};
use crate::inst::Instance;
use crate::prog::{
    assemble, decode_all, eval_ops, gen_b, mutate_op, mutation_sites, Concrete, Op, Rhs, Rng,
};
use crate::sem::{self, Bin};
use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv::{self, BitVec};
use riscv_simulator::simulator::RiscVSimulator;
use std::collections::HashMap;

/// Where the simulator loads the program (data lives below it).
pub const PROG_BASE: u32 = 0x4000;
/// The table of experiment D.
pub const TABLE_BASE: u32 = 0x400;

/// One shape the runner can build, with its control mutation sites.
pub trait Shape {
    /// Build the instance; `mutation = Some(site)` builds the control variant.
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String>;
    /// Candidate control mutation sites, best first.
    fn sites(&self, seed: u64) -> Vec<usize>;
    /// The AND count of the same generator at divergence 100% (the G2 reference), for the
    /// shapes that have a divergence.
    fn full_divergence_ands(&self, _seed: u64) -> Option<usize> {
        None
    }
}

// ---------------------------------------------------------------------------------------
// Concrete replay helpers
// ---------------------------------------------------------------------------------------

/// The value of the declared input `key` on side `side` (`""`, `"@c"`, `"@r"`, ...).
pub fn val(vals: &HashMap<String, u32>, key: &str, side: &str) -> u32 {
    vals.get(&format!("{key}{side}")).copied().unwrap_or(0)
}

/// The `(address, word)` pairs of every memory input `m<hex>` declared on `side`.
pub fn mem_inputs(vals: &HashMap<String, u32>, side: &str) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for (k, x) in vals {
        if let Some(rest) = k.strip_prefix('m') {
            let (hex, suf) = match rest.find('@') {
                Some(p) => (&rest[..p], &rest[p..]),
                None => (rest, ""),
            };
            if suf == side {
                if let Ok(a) = u32::from_str_radix(hex, 16) {
                    v.push((a, *x));
                }
            }
        }
    }
    v.sort_unstable();
    v
}

/// Run `ops` on the real simulator from the given registers and memory words.
pub fn run_on_simulator(
    ops: &[Op],
    regs: &[(u8, u32)],
    mem: &[(u32, u32)],
) -> Result<RiscVSimulator, String> {
    let mut sim = RiscVSimulator::new(1 << 16);
    let words = assemble(ops);
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    sim.mem.load_bytes(PROG_BASE as usize, &bytes);
    sim.pc = PROG_BASE as i32;
    for &(r, v) in regs {
        if r != 0 {
            sim.regs.write(r as usize, v);
        }
    }
    for &(a, v) in mem {
        sim.mem.write_word(a as usize, v);
    }
    let end = PROG_BASE + 4 * ops.len() as u32;
    let mut budget = 4 * ops.len() + 8;
    while (sim.pc as u32) < end && budget > 0 {
        sim.step();
        budget -= 1;
    }
    if (sim.pc as u32) < end {
        return Err("the program did not finish on the simulator".into());
    }
    Ok(sim)
}

fn regs_of(vals: &HashMap<String, u32>, side: &str) -> Vec<(u8, u32)> {
    let mut v = Vec::new();
    for (k, x) in vals {
        if let Some(rest) = k.strip_prefix('x') {
            let (num, suf) = match rest.find('@') {
                Some(p) => (&rest[..p], &rest[p..]),
                None => (rest, ""),
            };
            if suf == side {
                if let Ok(r) = num.parse::<u8>() {
                    v.push((r, *x));
                }
            }
        }
    }
    v.sort_unstable();
    v
}

/// Compare two output bit lists as the miter does: OR of per-bit XORs.
fn miter_of(aig: &mut Aig, a: &[Edge], b: &[Edge]) -> Edge {
    let diffs: Vec<Edge> = a.iter().zip(b).map(|(&x, &y)| aig.xor(x, y)).collect();
    aig.or_all(&diffs)
}

// ---------------------------------------------------------------------------------------
// B: the program miter (also the segments of B3 and the building block of C)
// ---------------------------------------------------------------------------------------

/// Parameters of a program instance.
#[derive(Clone)]
pub struct ProgParams {
    /// The code side's program.
    pub code: Vec<Op>,
    /// The reference side's program (the original, never mutated).
    pub reference: Vec<Op>,
    /// The reference's per-instruction alternative encodings.
    pub alt: Vec<Option<u32>>,
    /// Two fresh inputs per live-in joined by an equality premise (the cut form).
    pub separate: bool,
    /// Compare only these output bits (`u32::MAX` for all).
    pub mask: u32,
    /// Widths of named inputs (default 32).
    pub widths: Vec<(String, usize)>,
    /// Shuffle seed for the input numbering (None: creation order).
    pub shuffle: Option<u64>,
}

/// Build the monolithic or cut miter of a program pair.
pub fn prog_instance(p: &ProgParams) -> Result<Instance, String> {
    let mut aig = Aig::new();
    let mut pool = Pool::new();
    for (n, w) in &p.widths {
        pool.set_width(n, *w);
    }
    let (sc, sr) = if p.separate { ("@c", "@r") } else { ("", "") };
    let decoded = decode_all(&assemble(&p.code))?;
    let mut cc = Cpu::new(sc);
    cc.run(&mut aig, &mut pool, &decoded, &[], &mut |_, _| false)?;
    let mut rc = Cpu::new(sr);
    rc.run(&mut aig, &mut pool, &p.reference, &p.alt, &mut |_, _| false)?;
    let regs: Vec<u8> = cc.written_regs.union(&rc.written_regs).copied().collect();
    let words: Vec<u32> = cc.written_words.union(&rc.written_words).copied().collect();
    let mut code_out: Vec<Edge> = Vec::new();
    let mut ref_out: Vec<Edge> = Vec::new();
    let keep = |i: usize| i >= 32 || p.mask >> i & 1 == 1;
    for &r in &regs {
        let a = cc.reg(&mut aig, &mut pool, r);
        let b = rc.reg(&mut aig, &mut pool, r);
        for i in 0..32 {
            if keep(i) {
                code_out.push(a.bits[i]);
                ref_out.push(b.bits[i]);
            }
        }
    }
    for &w in &words {
        let a = cc.read_word(&mut aig, &mut pool, &konst(w))?;
        let b = rc.read_word(&mut aig, &mut pool, &konst(w))?;
        for i in 0..32 {
            if keep(i) {
                code_out.push(a.bits[i]);
                ref_out.push(b.bits[i]);
            }
        }
    }
    let mut premises: Vec<Edge> = cc.path.iter().chain(rc.path.iter()).copied().collect();
    if p.separate {
        // The hypothesis of a cut, as clauses: every live-in is equal on the two sides.
        let names: Vec<String> = pool.declared.iter().map(|d| d.name.clone()).collect();
        for n in names {
            if let Some(stem) = n.strip_suffix("@c") {
                let c = pool.word(&mut aig, &format!("{stem}@c"));
                let r = pool.word(&mut aig, &format!("{stem}@r"));
                premises.push(bv::eq(&mut aig, &c, &r));
            } else if let Some(stem) = n.strip_suffix("@r") {
                let c = pool.word(&mut aig, &format!("{stem}@c"));
                let r = pool.word(&mut aig, &format!("{stem}@r"));
                premises.push(bv::eq(&mut aig, &c, &r));
            }
        }
    }
    let miter = miter_of(&mut aig, &code_out, &ref_out);
    let (code_ops, ref_ops, mask, sep) = (p.code.clone(), p.reference.clone(), p.mask, p.separate);
    let regs2 = regs.clone();
    let words2 = words.clone();
    let replay = Box::new(move |vals: &HashMap<String, u32>| -> Result<bool, String> {
        let (sc, sr) = if sep { ("@c", "@r") } else { ("", "") };
        let sim = run_on_simulator(&code_ops, &regs_of(vals, sc), &mem_inputs(vals, sc))?;
        let mut st = Concrete::default();
        for (r, v) in regs_of(vals, sr) {
            st.regs[r as usize] = v;
        }
        for (a, v) in mem_inputs(vals, sr) {
            st.mem.insert(a, v);
        }
        eval_ops(&ref_ops, &mut st);
        for &r in &regs2 {
            if (sim.regs.read(r as usize) ^ st.regs[r as usize]) & mask != 0 {
                return Ok(true);
            }
        }
        for &w in &words2 {
            let c = sim.mem.read_word(w as usize);
            if (c ^ st.mem.get(&w).copied().unwrap_or(0)) & mask != 0 {
                return Ok(true);
            }
        }
        Ok(false)
    });
    Ok(Instance {
        aig,
        miter,
        premises,
        declared: pool.declared,
        code_out,
        ref_out,
        replay,
        live: None,
    })
}

/// Experiment B2 (and B1 at divergence 0): one program, both sides.
pub struct BShape {
    /// Instructions.
    pub len: usize,
    /// Divergence.
    pub divergence: f64,
}

impl BShape {
    fn params(&self, seed: u64, mutation: Option<usize>) -> Result<ProgParams, String> {
        let g = gen_b(seed, self.len, self.divergence);
        let mut code = g.ops.clone();
        if let Some(site) = mutation {
            code[site] = mutate_op(&code[site]).ok_or("no mutation at this site")?;
        }
        Ok(ProgParams {
            code,
            reference: g.ops,
            alt: g.alt,
            separate: false,
            mask: u32::MAX,
            widths: vec![],
            shuffle: None,
        })
    }
}

impl Shape for BShape {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        prog_instance(&self.params(seed, mutation)?)
    }
    fn sites(&self, seed: u64) -> Vec<usize> {
        mutation_sites(seed, &gen_b(seed, self.len, self.divergence).ops)
    }
    fn full_divergence_ands(&self, seed: u64) -> Option<usize> {
        if self.divergence >= 1.0 {
            return None;
        }
        BShape {
            len: self.len,
            divergence: 1.0,
        }
        .build(seed, None)
        .ok()
        .map(|i| i.cone_ands())
    }
}

/// The segments of experiment B3: `[start, end)` ranges of `every` instructions.
pub fn segments(len: usize, every: usize) -> Vec<(usize, usize)> {
    (0..len)
        .step_by(every)
        .map(|s| (s, (s + every).min(len)))
        .collect()
}

/// One cut segment of a generated program (the hypothesis as premises).
pub struct BSegment {
    /// Instructions of the whole program.
    pub len: usize,
    /// Divergence.
    pub divergence: f64,
    /// The segment `[start, end)`.
    pub range: (usize, usize),
}

impl Shape for BSegment {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        let g = gen_b(seed, self.len, self.divergence);
        let (s, e) = self.range;
        let mut code = g.ops[s..e].to_vec();
        if let Some(site) = mutation {
            code[site] = mutate_op(&code[site]).ok_or("no mutation at this site")?;
        }
        prog_instance(&ProgParams {
            code,
            reference: g.ops[s..e].to_vec(),
            alt: g.alt[s..e].to_vec(),
            separate: true,
            mask: u32::MAX,
            widths: vec![],
            shuffle: None,
        })
    }
    fn sites(&self, seed: u64) -> Vec<usize> {
        let g = gen_b(seed, self.len, self.divergence);
        mutation_sites(seed, &g.ops[self.range.0..self.range.1])
    }
}

// ---------------------------------------------------------------------------------------
// A1: pick (clz against a priority chain)
// ---------------------------------------------------------------------------------------

/// The branch-free RV32I `clz32` binary search on `a`, leaving `32 - clz(a)` in `out`.
/// Uses `n`, `t`, `c`, `s` as scratch.
fn clz_ops(a: u8, out: u8, n: u8, t: u8, c: u8, s: u8, k32: u8) -> Vec<Op> {
    let mut v = vec![Op::Alu {
        k: Bin::Add,
        rd: n,
        rs1: 0,
        rhs: Rhs::Imm(0),
    }];
    for (h, logh) in [(16, 4), (8, 3), (4, 2), (2, 1), (1, 0)] {
        v.push(Op::Alu {
            k: Bin::Srl,
            rd: t,
            rs1: a,
            rhs: Rhs::Imm(32 - h),
        });
        v.push(Op::Alu {
            k: Bin::Sltu,
            rd: c,
            rs1: t,
            rhs: Rhs::Imm(1),
        });
        v.push(Op::Alu {
            k: Bin::Sll,
            rd: s,
            rs1: c,
            rhs: Rhs::Imm(logh),
        });
        v.push(Op::Alu {
            k: Bin::Sll,
            rd: a,
            rs1: a,
            rhs: Rhs::Reg(s),
        });
        v.push(Op::Alu {
            k: Bin::Add,
            rd: n,
            rs1: n,
            rhs: Rhs::Reg(s),
        });
    }
    v.push(Op::Alu {
        k: Bin::Srl,
        rd: t,
        rs1: a,
        rhs: Rhs::Imm(31),
    });
    v.push(Op::Alu {
        k: Bin::Xor,
        rd: t,
        rs1: t,
        rhs: Rhs::Imm(1),
    });
    v.push(Op::Alu {
        k: Bin::Add,
        rd: n,
        rs1: n,
        rhs: Rhs::Reg(t),
    });
    v.push(Op::Alu {
        k: Bin::Add,
        rd: k32,
        rs1: 0,
        rhs: Rhs::Imm(32),
    });
    v.push(Op::Alu {
        k: Bin::Sub,
        rd: out,
        rs1: k32,
        rhs: Rhs::Reg(n),
    });
    v
}

/// Experiment A1 at `n` bits.
pub struct A1 {
    /// The mask width.
    pub n: usize,
}

impl A1 {
    fn code(&self) -> Vec<Op> {
        let mut ops = clz_ops(1, 10, 2, 3, 4, 5, 6);
        if self.n == 64 {
            ops.extend(clz_ops(11, 12, 13, 14, 15, 16, 17));
            ops.extend([
                Op::Alu {
                    k: Bin::Sltu,
                    rd: 7,
                    rs1: 0,
                    rhs: Rhs::Reg(12),
                },
                Op::Alu {
                    k: Bin::Sub,
                    rd: 7,
                    rs1: 0,
                    rhs: Rhs::Reg(7),
                },
                Op::Alu {
                    k: Bin::Add,
                    rd: 8,
                    rs1: 12,
                    rhs: Rhs::Imm(32),
                },
                Op::Alu {
                    k: Bin::And,
                    rd: 8,
                    rs1: 8,
                    rhs: Rhs::Reg(7),
                },
                Op::Alu {
                    k: Bin::Xor,
                    rd: 9,
                    rs1: 7,
                    rhs: Rhs::Imm(-1),
                },
                Op::Alu {
                    k: Bin::And,
                    rd: 9,
                    rs1: 9,
                    rhs: Rhs::Reg(10),
                },
                Op::Alu {
                    k: Bin::Or,
                    rd: 10,
                    rs1: 8,
                    rhs: Rhs::Reg(9),
                },
            ]);
        }
        ops
    }
}

/// The reference of A1 on concrete words: the position of the highest set member plus one.
pub fn pick_plus_one(lo: u32, hi: u32, n: usize) -> u32 {
    let mut r = 0;
    for i in 0..n {
        let bit = if i < 32 {
            lo >> i & 1
        } else {
            hi >> (i - 32) & 1
        };
        if bit == 1 {
            r = i as u32 + 1;
        }
    }
    r
}

impl Shape for A1 {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let mut rng = Rng::new(seed, 0xA1);
        let mut specs = vec![("x1".to_string(), self.n.min(32))];
        if self.n == 64 {
            specs.push(("x11".to_string(), 32));
        }
        pool.declare_shuffled(&mut aig, &specs, &mut rng);
        let mut code_ops = self.code();
        if let Some(site) = mutation {
            code_ops[site] = mutate_op(&code_ops[site]).ok_or("no mutation at this site")?;
        }
        let decoded = decode_all(&assemble(&code_ops))?;
        let mut cpu = Cpu::new("");
        cpu.run(&mut aig, &mut pool, &decoded, &[], &mut |_, _| false)?;
        let out = cpu.reg(&mut aig, &mut pool, 10);
        // The reference: ite(bit N-1, N, ite(bit N-2, N-1, ... ite(bit 0, 1, 0))).
        let lo = pool.word(&mut aig, "x1");
        let hi = if self.n == 64 {
            pool.word(&mut aig, "x11")
        } else {
            konst(0)
        };
        let mut acc = konst(0);
        for i in 0..self.n {
            let bit = if i < 32 { lo.bits[i] } else { hi.bits[i - 32] };
            acc = bv::ite(&mut aig, bit, &konst(i as u32 + 1), &acc);
        }
        let miter = miter_of(&mut aig, &out.bits, &acc.bits);
        let (n, ops2) = (self.n, code_ops);
        let replay = Box::new(move |vals: &HashMap<String, u32>| -> Result<bool, String> {
            let (lo, hi) = (val(vals, "x1", ""), val(vals, "x11", ""));
            let sim = run_on_simulator(&ops2, &[(1, lo), (11, hi)], &[])?;
            Ok(sim.regs.read(10) != pick_plus_one(lo, hi, n))
        });
        Ok(Instance {
            aig,
            miter,
            premises: vec![],
            declared: pool.declared,
            code_out: out.bits,
            ref_out: acc.bits,
            replay,
            live: None,
        })
    }
    fn sites(&self, seed: u64) -> Vec<usize> {
        // Mutate late instructions of the chain so a counterexample is found quickly.
        mutation_sites(seed, &self.code())
    }
}

// ---------------------------------------------------------------------------------------
// A2: insert and remove on an N-bit mask
// ---------------------------------------------------------------------------------------

/// Experiment A2 at `n` bits.
pub struct A2 {
    /// The mask width.
    pub n: usize,
}

impl A2 {
    fn code(&self) -> (Vec<Op>, Vec<u8>) {
        let a = |k, rd, rs1, rhs| Op::Alu { k, rd, rs1, rhs };
        if self.n <= 32 {
            let ops = vec![
                a(Bin::Add, 3, 0, Rhs::Imm(1)),
                a(Bin::Sll, 4, 3, Rhs::Reg(2)),
                a(Bin::Sltu, 5, 2, Rhs::Imm(self.n as i32)),
                a(Bin::Sub, 5, 0, Rhs::Reg(5)),
                a(Bin::And, 4, 4, Rhs::Reg(5)),
                a(Bin::Or, 6, 1, Rhs::Reg(4)),
                a(Bin::Xor, 7, 4, Rhs::Imm(-1)),
                a(Bin::And, 7, 1, Rhs::Reg(7)),
            ];
            (ops, vec![6, 7])
        } else {
            let ops = vec![
                a(Bin::Sltu, 5, 2, Rhs::Imm(64)),
                a(Bin::Sub, 5, 0, Rhs::Reg(5)),
                a(Bin::Add, 3, 0, Rhs::Imm(1)),
                a(Bin::Sll, 4, 3, Rhs::Reg(2)),
                a(Bin::Sltu, 9, 2, Rhs::Imm(32)),
                a(Bin::Sub, 9, 0, Rhs::Reg(9)),
                a(Bin::Xor, 13, 9, Rhs::Imm(-1)),
                a(Bin::And, 14, 4, Rhs::Reg(9)),
                a(Bin::And, 14, 14, Rhs::Reg(5)),
                a(Bin::And, 15, 4, Rhs::Reg(13)),
                a(Bin::And, 15, 15, Rhs::Reg(5)),
                a(Bin::Or, 6, 1, Rhs::Reg(14)),
                a(Bin::Or, 16, 11, Rhs::Reg(15)),
                a(Bin::Xor, 17, 14, Rhs::Imm(-1)),
                a(Bin::And, 7, 1, Rhs::Reg(17)),
                a(Bin::Xor, 18, 15, Rhs::Imm(-1)),
                a(Bin::And, 19, 11, Rhs::Reg(18)),
            ];
            (ops, vec![6, 16, 7, 19])
        }
    }
}

/// The reference of A2 on concrete words: `(insert, remove)` of `p` in the `n`-bit set.
pub fn insert_remove(lo: u32, hi: u32, p: u32, n: usize) -> (u64, u64) {
    let m = lo as u64 | (hi as u64) << 32;
    if (p as usize) < n {
        (m | 1u64 << p, m & !(1u64 << p))
    } else {
        (m, m)
    }
}

impl Shape for A2 {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let mut rng = Rng::new(seed, 0xA2);
        let mut specs = vec![("x1".to_string(), self.n.min(32)), ("x2".to_string(), 8)];
        if self.n == 64 {
            specs.push(("x11".to_string(), 32));
        }
        pool.declare_shuffled(&mut aig, &specs, &mut rng);
        let (mut code_ops, outs) = self.code();
        if let Some(site) = mutation {
            code_ops[site] = mutate_op(&code_ops[site]).ok_or("no mutation at this site")?;
        }
        let decoded = decode_all(&assemble(&code_ops))?;
        let mut cpu = Cpu::new("");
        cpu.run(&mut aig, &mut pool, &decoded, &[], &mut |_, _| false)?;
        let lo = pool.word(&mut aig, "x1");
        let hi = if self.n == 64 {
            pool.word(&mut aig, "x11")
        } else {
            konst(0)
        };
        let p = pool.word(&mut aig, "x2");
        // Per-Bool update: out_i = ite(p == i, b, m_i); p >= N matches no i and changes nothing.
        let mut ref_words: Vec<BitVec> = Vec::new();
        for target in [true, false] {
            for half in 0..(self.n.div_ceil(32)) {
                let base = if half == 0 { &lo } else { &hi };
                let mut bits = base.bits.clone();
                for (i, bit) in bits.iter_mut().enumerate() {
                    let idx = half * 32 + i;
                    if idx < self.n {
                        let hit = bv::eq(&mut aig, &p, &konst(idx as u32));
                        *bit = aig.mux(hit, Edge::constant(target), *bit);
                    }
                }
                ref_words.push(BitVec { bits });
            }
        }
        let mut code_out = Vec::new();
        let mut ref_out = Vec::new();
        for (r, w) in outs.iter().zip(&ref_words) {
            let c = cpu.reg(&mut aig, &mut pool, *r);
            code_out.extend(c.bits);
            ref_out.extend(w.bits.clone());
        }
        let miter = miter_of(&mut aig, &code_out, &ref_out);
        let (n, ops2, outs2) = (self.n, code_ops, outs);
        let replay = Box::new(move |vals: &HashMap<String, u32>| -> Result<bool, String> {
            let (lo, hi, p) = (
                val(vals, "x1", ""),
                val(vals, "x11", ""),
                val(vals, "x2", ""),
            );
            let sim = run_on_simulator(&ops2, &[(1, lo), (11, hi), (2, p)], &[])?;
            let (ins, rem) = insert_remove(lo, hi, p, n);
            let want: Vec<u32> = if n <= 32 {
                vec![ins as u32, rem as u32]
            } else {
                vec![
                    ins as u32,
                    (ins >> 32) as u32,
                    rem as u32,
                    (rem >> 32) as u32,
                ]
            };
            Ok(outs2
                .iter()
                .zip(&want)
                .any(|(r, w)| sim.regs.read(*r as usize) != *w))
        });
        Ok(Instance {
            aig,
            miter,
            premises: vec![],
            declared: pool.declared,
            code_out,
            ref_out,
            replay,
            live: None,
        })
    }
    fn sites(&self, seed: u64) -> Vec<usize> {
        mutation_sites(seed, &self.code().0)
    }
}

// ---------------------------------------------------------------------------------------
// B4: arithmetic with no cut point
// ---------------------------------------------------------------------------------------

/// The three shapes of B4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum B4Kind {
    /// n-term re-association.
    Assoc(usize),
    /// `x * 5` against `(x << 2) + x`.
    Mul5,
    /// `base + i * 12` against shift-add.
    Mul12,
}

/// Experiment B4.
pub struct B4 {
    /// Which shape.
    pub kind: B4Kind,
    /// The width of the inputs and of the compared result.
    pub w: usize,
}

impl B4 {
    fn code(&self) -> Vec<Op> {
        let a = |k, rd, rs1, rhs| Op::Alu { k, rd, rs1, rhs };
        match self.kind {
            B4Kind::Assoc(n) => {
                let mut v = vec![a(Bin::Add, 10, 1, Rhs::Reg(2))];
                for i in 3..=n {
                    v.push(a(Bin::Add, 10, 10, Rhs::Reg(i as u8)));
                }
                v
            }
            B4Kind::Mul5 => vec![
                a(Bin::Sll, 3, 1, Rhs::Imm(2)),
                a(Bin::Add, 10, 3, Rhs::Reg(1)),
            ],
            B4Kind::Mul12 => vec![
                a(Bin::Sll, 3, 2, Rhs::Imm(3)),
                a(Bin::Sll, 4, 2, Rhs::Imm(2)),
                a(Bin::Add, 3, 3, Rhs::Reg(4)),
                a(Bin::Add, 10, 1, Rhs::Reg(3)),
            ],
        }
    }
    fn n_inputs(&self) -> usize {
        match self.kind {
            B4Kind::Assoc(n) => n,
            B4Kind::Mul5 => 1,
            B4Kind::Mul12 => 2,
        }
    }
    fn mask(&self) -> u32 {
        if self.w >= 32 {
            u32::MAX
        } else {
            (1u32 << self.w) - 1
        }
    }
    /// The reference value on concrete inputs (`x1..xn`).
    pub fn concrete(&self, x: &[u32]) -> u32 {
        let r = match self.kind {
            B4Kind::Assoc(_) => x.iter().fold(0u32, |a, b| a.wrapping_add(*b)),
            B4Kind::Mul5 => x[0].wrapping_mul(5),
            B4Kind::Mul12 => x[0].wrapping_add(x[1].wrapping_mul(12)),
        };
        r & self.mask()
    }
}

impl Shape for B4 {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let n = self.n_inputs();
        let specs: Vec<(String, usize)> = (1..=n).map(|i| (format!("x{i}"), self.w)).collect();
        pool.declare_shuffled(&mut aig, &specs, &mut Rng::new(seed, 0xB4));
        let mut ops = self.code();
        if let Some(site) = mutation {
            ops[site] = mutate_op(&ops[site]).ok_or("no mutation at this site")?;
        }
        let decoded = decode_all(&assemble(&ops))?;
        let mut cpu = Cpu::new("");
        cpu.run(&mut aig, &mut pool, &decoded, &[], &mut |_, _| false)?;
        let out = cpu.reg(&mut aig, &mut pool, 10);
        let xs: Vec<BitVec> = (1..=n)
            .map(|i| pool.word(&mut aig, &format!("x{i}")))
            .collect();
        let reference = match self.kind {
            B4Kind::Assoc(_) => {
                // a1 + (a2 + (... + an)), right-associated.
                let mut acc = xs[n - 1].clone();
                for x in xs[..n - 1].iter().rev() {
                    acc = bv::add(&mut aig, x, &acc);
                }
                acc
            }
            // A constant FIRST operand: the product is a sum of gated constants, not the shift-add.
            B4Kind::Mul5 => bv::mul(&mut aig, &konst(5), &xs[0]),
            B4Kind::Mul12 => {
                let p = bv::mul(&mut aig, &konst(12), &xs[1]);
                bv::add(&mut aig, &xs[0], &p)
            }
        };
        let m = self.mask();
        let keep: Vec<usize> = (0..32).filter(|i| m >> i & 1 == 1).collect();
        let code_out: Vec<Edge> = keep.iter().map(|&i| out.bits[i]).collect();
        let ref_out: Vec<Edge> = keep.iter().map(|&i| reference.bits[i]).collect();
        let miter = miter_of(&mut aig, &code_out, &ref_out);
        let (ops2, me) = (
            ops,
            B4 {
                kind: self.kind,
                w: self.w,
            },
        );
        let replay = Box::new(move |vals: &HashMap<String, u32>| -> Result<bool, String> {
            let xs: Vec<u32> = (1..=n).map(|i| val(vals, &format!("x{i}"), "")).collect();
            let regs: Vec<(u8, u32)> = xs
                .iter()
                .enumerate()
                .map(|(i, v)| (i as u8 + 1, *v))
                .collect();
            let sim = run_on_simulator(&ops2, &regs, &[])?;
            Ok(sim.regs.read(10) & me.mask() != me.concrete(&xs))
        });
        Ok(Instance {
            aig,
            miter,
            premises: vec![],
            declared: pool.declared,
            code_out,
            ref_out,
            replay,
            live: None,
        })
    }
    fn sites(&self, seed: u64) -> Vec<usize> {
        mutation_sites(seed, &self.code())
    }
}

// ---------------------------------------------------------------------------------------
// C1 and C2: self-composition
// ---------------------------------------------------------------------------------------

/// What C1 composes: an A1 pick or a generated B program.
#[derive(Clone, Copy, Debug)]
pub enum C1Base {
    /// A1 at this width.
    A1(usize),
    /// A B program of this length and divergence (per mille).
    B(usize, u32),
}

/// Experiment C1.
pub struct C1 {
    /// The body that is composed with itself.
    pub base: C1Base,
}

/// XOR-fold a list of words into one.
fn fold_words(aig: &mut Aig, ws: &[BitVec]) -> BitVec {
    let mut acc = konst(0);
    for w in ws {
        acc = bv::xor(aig, &acc, w);
    }
    acc
}

/// What [`base_pair`] builds: the two result words and the code program with the registers it
/// is read from (for the replay).
struct BasePair {
    code_word: BitVec,
    ref_word: BitVec,
    code_ops: Vec<Op>,
    regs: Vec<u8>,
}

/// The code-side and reference-side result words of the base body on the inputs named with
/// `suffix` (both sides share those inputs), plus the code program for replay.
fn base_pair(
    base: C1Base,
    seed: u64,
    aig: &mut Aig,
    pool: &mut Pool,
    suffix: &str,
) -> Result<BasePair, String> {
    match base {
        C1Base::A1(n) => {
            let a1 = A1 { n };
            // The A1 code program reads x1 (and x11): rename through a machine suffix.
            let mut cpu = Cpu::new(suffix);
            let ops = a1.code();
            let decoded = decode_all(&assemble(&ops))?;
            pool.set_width(&format!("x1{suffix}"), n.min(32));
            cpu.run(aig, pool, &decoded, &[], &mut |_, _| false)?;
            let out = cpu.reg(aig, pool, 10);
            let lo = pool.word(aig, &format!("x1{suffix}"));
            let hi = if n == 64 {
                pool.word(aig, &format!("x11{suffix}"))
            } else {
                konst(0)
            };
            let mut acc = konst(0);
            for i in 0..n {
                let bit = if i < 32 { lo.bits[i] } else { hi.bits[i - 32] };
                acc = bv::ite(aig, bit, &konst(i as u32 + 1), &acc);
            }
            Ok(BasePair {
                code_word: out,
                ref_word: acc,
                code_ops: ops,
                regs: vec![10],
            })
        }
        C1Base::B(len, div) => {
            let g = gen_b(seed, len, div as f64 / 1000.0);
            let decoded = decode_all(&assemble(&g.ops))?;
            let mut c = Cpu::new(suffix);
            c.run(aig, pool, &decoded, &[], &mut |_, _| false)?;
            let mut r = Cpu::new(suffix);
            r.run(aig, pool, &g.ops, &g.alt, &mut |_, _| false)?;
            let regs: Vec<u8> = c.written_regs.iter().copied().collect();
            let cw: Vec<BitVec> = regs.iter().map(|&x| c.reg(aig, pool, x)).collect();
            let rw: Vec<BitVec> = regs.iter().map(|&x| r.reg(aig, pool, x)).collect();
            let (cf, rf) = (fold_words(aig, &cw), fold_words(aig, &rw));
            Ok(BasePair {
                code_word: cf,
                ref_word: rf,
                code_ops: g.ops,
                regs,
            })
        }
    }
}

/// The concrete fold of the base's code side on the simulator for the inputs of `suffix`.
fn base_code_value(
    ops: &[Op],
    regs: &[u8],
    vals: &HashMap<String, u32>,
    suffix: &str,
) -> Result<u32, String> {
    let sim = run_on_simulator(ops, &regs_of(vals, suffix), &mem_inputs(vals, suffix))?;
    Ok(regs
        .iter()
        .fold(0u32, |a, r| a ^ sim.regs.read(*r as usize)))
}

impl Shape for C1 {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        // mutation = Some(_) builds the leaking variant (the secret-dependence does not cancel).
        let leak = mutation.is_some();
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let BasePair {
            code_word: pc,
            code_ops: ops,
            regs,
            ..
        } = base_pair(self.base, seed, &mut aig, &mut pool, "@p")?;
        let mut outs = Vec::new();
        let mut obs_code = Vec::new();
        let (mut live_a, mut live_b) = (pc.clone(), pc.clone());
        for c in ["@s1", "@s2"] {
            let BasePair {
                code_word: cw,
                ref_word: rw,
                ..
            } = base_pair(self.base, seed, &mut aig, &mut pool, c)?;
            live_a = bv::xor(&mut aig, &live_a, &cw);
            live_b = bv::xor(&mut aig, &live_b, &rw);
            let cancel = if leak {
                cw.clone()
            } else {
                bv::xor(&mut aig, &cw, &rw)
            };
            let out = bv::xor(&mut aig, &pc, &cancel);
            obs_code.push(out.bits.clone());
            outs.push(out);
        }
        let miter = miter_of(&mut aig, &outs[0].bits, &outs[1].bits);
        let (ops2, regs2) = (ops, regs);
        let replay = Box::new(move |vals: &HashMap<String, u32>| -> Result<bool, String> {
            // Two concrete runs that agree on the public inputs and differ in the secrets.
            let p = base_code_value(&ops2, &regs2, vals, "@p")?;
            let o1 = p ^ base_code_value(&ops2, &regs2, vals, "@s1")?;
            let o2 = p ^ base_code_value(&ops2, &regs2, vals, "@s2")?;
            Ok(o1 != o2)
        });
        Ok(Instance {
            aig,
            miter,
            premises: vec![],
            declared: pool.declared,
            code_out: obs_code[0].clone(),
            ref_out: obs_code[1].clone(),
            replay,
            live: Some((live_a.bits, live_b.bits)),
        })
    }
    fn sites(&self, _seed: u64) -> Vec<usize> {
        vec![0]
    }
}

/// Experiment C2: a secret-dependent load index against a padded constant-time variant.
pub struct C2 {
    /// Table words.
    pub t: usize,
}

impl Shape for C2 {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        let leak = mutation.is_some();
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let mut obs: Vec<Vec<BitVec>> = Vec::new();
        let (mut live_a, mut live_b) = (konst(0), konst(0));
        for c in ["@s1", "@s2"] {
            // A short generated body whose code and reference sides cancel semantically.
            let g = gen_b(seed, 8, 1.0);
            let decoded = decode_all(&assemble(&g.ops))?;
            let mut cc = Cpu::new(c);
            cc.run(&mut aig, &mut pool, &decoded, &[], &mut |_, _| false)?;
            let mut rc = Cpu::new(c);
            rc.run(&mut aig, &mut pool, &g.ops, &g.alt, &mut |_, _| false)?;
            let regs: Vec<u8> = cc.written_regs.iter().copied().collect();
            let cw: Vec<BitVec> = regs
                .iter()
                .map(|&x| cc.reg(&mut aig, &mut pool, x))
                .collect();
            let rw: Vec<BitVec> = regs
                .iter()
                .map(|&x| rc.reg(&mut aig, &mut pool, x))
                .collect();
            let (cf, rf) = (fold_words(&mut aig, &cw), fold_words(&mut aig, &rw));
            live_a = bv::xor(&mut aig, &live_a, &cf);
            live_b = bv::xor(&mut aig, &live_b, &rf);
            let cancel = bv::xor(&mut aig, &cf, &rf);
            let secret = pool.word(&mut aig, &format!("x1{c}"));
            let mut addrs = Vec::new();
            if leak {
                // table[s & (T-1)]: the address depends on the secret.
                let idx = bv::and(&mut aig, &secret, &konst(self.t as u32 - 1));
                let off = bv::shl_const(&idx, 2);
                addrs.push(bv::add(&mut aig, &konst(TABLE_BASE), &off));
            } else {
                // Load every word at a constant address, perturbed by the (zero) cancel term.
                for j in 0..self.t {
                    addrs.push(bv::add(
                        &mut aig,
                        &konst(TABLE_BASE + 4 * j as u32),
                        &cancel,
                    ));
                }
            }
            obs.push(addrs);
        }
        let a: Vec<Edge> = obs[0].iter().flat_map(|w| w.bits.clone()).collect();
        let b: Vec<Edge> = obs[1].iter().flat_map(|w| w.bits.clone()).collect();
        let miter = miter_of(&mut aig, &a, &b);
        let t = self.t;
        let replay = Box::new(move |vals: &HashMap<String, u32>| -> Result<bool, String> {
            // The load program on the real simulator; the address is rs1 + imm of the load.
            let prog = [
                Op::Alu {
                    k: Bin::And,
                    rd: 2,
                    rs1: 1,
                    rhs: Rhs::Imm(t as i32 - 1),
                },
                Op::Alu {
                    k: Bin::Sll,
                    rd: 2,
                    rs1: 2,
                    rhs: Rhs::Imm(2),
                },
            ];
            let mut addrs = Vec::new();
            for c in ["@s1", "@s2"] {
                let sim = run_on_simulator(&prog, &[(1, val(vals, "x1", c))], &[])?;
                addrs.push(sim.regs.read(2).wrapping_add(TABLE_BASE));
            }
            // The independent evaluator: base + 4 * (s & (T-1)).
            let want: Vec<u32> = ["@s1", "@s2"]
                .iter()
                .map(|c| TABLE_BASE + 4 * (val(vals, "x1", c) & (t as u32 - 1)))
                .collect();
            if addrs != want {
                return Err("the simulator and the evaluator disagree about the address".into());
            }
            Ok(addrs[0] != addrs[1])
        });
        Ok(Instance {
            aig,
            miter,
            premises: vec![],
            declared: pool.declared,
            code_out: a,
            ref_out: b,
            replay,
            live: Some((live_a.bits, live_b.bits)),
        })
    }
    fn sites(&self, _seed: u64) -> Vec<usize> {
        vec![0]
    }
}

// ---------------------------------------------------------------------------------------
// D: memory with no array theory
// ---------------------------------------------------------------------------------------

/// What D builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DKind {
    /// D1: symbolic index, eq/ite chain against a mux tree.
    Sym,
    /// D2 / one obligation of D3: the index is the constant `c`.
    Concrete(u32),
    /// D4: access number `j` alone, fresh tables joined by premises, shared mux structure.
    Cut(usize),
}

/// Experiments D1 to D4.
pub struct D {
    /// Table words (a power of two).
    pub t: usize,
    /// Accesses.
    pub k: usize,
    /// Which variant.
    pub kind: DKind,
}

impl D {
    fn lg(&self) -> usize {
        self.t.trailing_zeros() as usize
    }

    /// The RV32I program of accesses `range` (access `j` touches word `(i + j) mod T`).
    pub fn code(&self, range: std::ops::Range<usize>) -> Vec<Op> {
        let a = |k, rd, rs1, rhs| Op::Alu { k, rd, rs1, rhs };
        let mut v = Vec::new();
        for j in range {
            v.push(a(Bin::Add, 3, 1, Rhs::Imm(j as i32)));
            v.push(a(Bin::And, 3, 3, Rhs::Imm(self.t as i32 - 1)));
            v.push(a(Bin::Sll, 3, 3, Rhs::Imm(2)));
            v.push(Op::Lw {
                rd: 4,
                base: 3,
                off: TABLE_BASE as i32,
            });
            v.push(a(Bin::Add, 5, 2, Rhs::Imm(j as i32 + 1)));
            v.push(a(Bin::Add, 4, 4, Rhs::Reg(5)));
            v.push(Op::Sw {
                rs: 4,
                base: 3,
                off: TABLE_BASE as i32,
            });
        }
        v
    }

    /// The indices (into [`D::code`]) of the data `add` of each access, the mutation sites.
    fn data_sites(&self, n_acc: usize) -> Vec<usize> {
        (0..n_acc).rev().map(|j| 7 * j + 5).collect()
    }

    /// Binary mux tree over the table: `tbl[idx]`.
    fn mux_tree(aig: &mut Aig, tbl: &[BitVec], idx: &BitVec) -> BitVec {
        let mut level: Vec<BitVec> = tbl.to_vec();
        // One tree level per index bit, least significant first; iterating the bits directly
        // (rather than `0..width` and indexing) is the same walk without the index variable.
        for &bit in idx.bits.iter().take(idx.width()) {
            let mut next = Vec::new();
            for pair in level.chunks(2) {
                next.push(if pair.len() == 2 {
                    bv::ite(aig, bit, &pair[1], &pair[0])
                } else {
                    pair[0].clone()
                });
            }
            level = next;
        }
        level[0].clone()
    }

    /// Apply access `j` to a table in the array-and-mux formulation.  `data_alt` is the
    /// alternative encoding of the data `add` (`None`: the primary circuit); `sub_mut` swaps
    /// the add for a subtract (the control mutation).
    #[allow(clippy::too_many_arguments)]
    fn array_access(
        &self,
        aig: &mut Aig,
        tbl: &mut [BitVec],
        i: &BitVec,
        v: &BitVec,
        j: usize,
        data_alt: Option<u32>,
        sub_mut: bool,
    ) {
        let lg = self.lg();
        let iw = BitVec {
            bits: i.bits[..lg].to_vec(),
        };
        let idx = bv::add(aig, &iw, &BitVec::constant(lg, j as u128));
        let old = Self::mux_tree(aig, tbl, &idx);
        let vj = bv::add(aig, v, &konst(j as u32 + 1));
        let new = if sub_mut {
            bv::sub(aig, &old, &vj)
        } else {
            match data_alt {
                Some(var) => {
                    sem::alternative(aig, Bin::Add, var, &old, &vj).expect("add has alternatives")
                }
                None => bv::add(aig, &old, &vj),
            }
        };
        for (e, slot) in tbl.iter_mut().enumerate() {
            let hit = bv::eq(aig, &idx, &BitVec::constant(lg, e as u128));
            *slot = bv::ite(aig, hit, &new, slot);
        }
    }
}

/// The concrete reference of D: the table after `k` accesses (`j0..j1`) from index `i`.
pub fn d_concrete(t: usize, j0: usize, j1: usize, i: u32, v: u32, mut tbl: Vec<u32>) -> Vec<u32> {
    for j in j0..j1 {
        let idx = (i as usize + j) & (t - 1);
        tbl[idx] = tbl[idx].wrapping_add(v.wrapping_add(j as u32 + 1));
    }
    tbl
}

impl Shape for D {
    fn build(&self, seed: u64, mutation: Option<usize>) -> Result<Instance, String> {
        let mut aig = Aig::new();
        let mut pool = Pool::new();
        let lg = self.lg();
        let mut rng = Rng::new(seed, 0xD0);
        let variant = (seed % 2) as u32;
        let (cut, concrete) = match self.kind {
            DKind::Cut(j) => (Some(j), None),
            DKind::Concrete(c) => (None, Some(c)),
            DKind::Sym => (None, None),
        };
        let range = match cut {
            Some(j) => j..j + 1,
            None => 0..self.k,
        };
        let sep = cut.is_some();
        let (sc, sr) = if sep { ("@c", "@r") } else { ("", "") };
        let sub_site = mutation.map(|m| m % range.len().max(1));
        // Declared inputs: the index, the data word and the table words (created up front, shuffled).
        let mut specs: Vec<(String, usize)> = Vec::new();
        if concrete.is_none() {
            for s in [sc, sr].iter().take(if sep { 2 } else { 1 }) {
                specs.push((format!("x1{s}"), lg));
            }
        }
        for s in [sc, sr].iter().take(if sep { 2 } else { 1 }) {
            specs.push((format!("x2{s}"), 32));
            for e in 0..self.t {
                specs.push((format!("m{:x}{s}", TABLE_BASE + 4 * e as u32), 32));
            }
        }
        pool.declare_shuffled(&mut aig, &specs, &mut rng);
        let ref_tbl_name = |e: usize, s: &str| format!("m{:x}{s}", TABLE_BASE + 4 * e as u32);
        // Reference side: array + mux tree, data add through the alternative encoding.
        let i_ref = match concrete {
            Some(c) => konst(c),
            None => pool.word(&mut aig, &format!("x1{sr}")),
        };
        let v_ref = pool.word(&mut aig, &format!("x2{sr}"));
        let mut rtbl: Vec<BitVec> = (0..self.t)
            .map(|e| pool.word(&mut aig, &ref_tbl_name(e, sr)))
            .collect();
        for j in range.clone() {
            self.array_access(&mut aig, &mut rtbl, &i_ref, &v_ref, j, Some(variant), false);
        }
        // Code side.
        let mut code_ops = self.code(range.clone());
        let mut ctbl: Vec<BitVec> = Vec::new();
        let mut premises: Vec<Edge> = Vec::new();
        if sep {
            // Cut form: the code side is the same mux structure with the primary add.
            let code_i = pool.word(&mut aig, &format!("x1{sc}"));
            let v_c = pool.word(&mut aig, &format!("x2{sc}"));
            ctbl = (0..self.t)
                .map(|e| pool.word(&mut aig, &ref_tbl_name(e, sc)))
                .collect();
            for j in range.clone() {
                let mutated = sub_site.is_some();
                self.array_access(&mut aig, &mut ctbl, &code_i, &v_c, j, None, mutated);
            }
            // The hypothesis as clauses: tables, index and data equal on the two sides.
            let (ci, ri) = (code_i.clone(), pool.word(&mut aig, &format!("x1{sr}")));
            premises.push(bv::eq(&mut aig, &ci, &ri));
            let (cv, rv) = (v_c, v_ref.clone());
            premises.push(bv::eq(&mut aig, &cv, &rv));
            let pairs: Vec<(BitVec, BitVec)> = (0..self.t)
                .map(|e| {
                    (
                        pool.word(&mut aig, &ref_tbl_name(e, sc)),
                        pool.word(&mut aig, &ref_tbl_name(e, sr)),
                    )
                })
                .collect();
            for (c, r) in pairs {
                premises.push(bv::eq(&mut aig, &c, &r));
            }
            if mutation.is_some() {
                // The same mutation in the program the replay runs: add becomes sub.
                code_ops[5] = mutate_op(&code_ops[5]).unwrap_or(code_ops[5]);
            }
        } else {
            if let Some(site) = mutation {
                let sites = self.data_sites(range.len());
                let at = sites[site % sites.len()];
                code_ops[at] = mutate_op(&code_ops[at]).ok_or("no mutation at this site")?;
            }
            let decoded = decode_all(&assemble(&code_ops))?;
            let mut cpu = Cpu::new("");
            cpu.set_region(TABLE_BASE, self.t);
            if let Some(c) = concrete {
                cpu.set_reg(1, konst(c));
            }
            cpu.run(&mut aig, &mut pool, &decoded, &[], &mut |_, _| false)?;
            for e in 0..self.t {
                ctbl.push(cpu.read_word(&mut aig, &mut pool, &konst(TABLE_BASE + 4 * e as u32))?);
            }
        }
        let code_out: Vec<Edge> = ctbl.iter().flat_map(|w| w.bits.clone()).collect();
        let ref_out: Vec<Edge> = rtbl.iter().flat_map(|w| w.bits.clone()).collect();
        let miter = miter_of(&mut aig, &code_out, &ref_out);
        let (t, j0, j1) = (self.t, range.start, range.end);
        let (ops2, conc) = (code_ops, concrete);
        let replay = Box::new(move |vals: &HashMap<String, u32>| -> Result<bool, String> {
            let (sc, sr) = if sep { ("@c", "@r") } else { ("", "") };
            let mem_c: Vec<(u32, u32)> = mem_inputs(vals, sc);
            let i_c = conc.unwrap_or(val(vals, "x1", sc)) & (t as u32 - 1);
            let sim = run_on_simulator(&ops2, &[(1, i_c), (2, val(vals, "x2", sc))], &mem_c)?;
            let tbl_r: Vec<u32> = (0..t)
                .map(|e| {
                    mem_inputs(vals, sr)
                        .iter()
                        .find(|(a, _)| *a == TABLE_BASE + 4 * e as u32)
                        .map(|x| x.1)
                        .unwrap_or(0)
                })
                .collect();
            let i_r = conc.unwrap_or(val(vals, "x1", sr)) & (t as u32 - 1);
            let want = d_concrete(t, j0, j1, i_r, val(vals, "x2", sr), tbl_r);
            Ok((0..t).any(|e| sim.mem.read_word((TABLE_BASE + 4 * e as u32) as usize) != want[e]))
        });
        Ok(Instance {
            aig,
            miter,
            premises,
            declared: pool.declared,
            code_out,
            ref_out,
            replay,
            live: None,
        })
    }
    fn sites(&self, _seed: u64) -> Vec<usize> {
        let n = match self.kind {
            DKind::Cut(_) => 1,
            _ => self.k,
        };
        (0..n.max(1)).collect()
    }
}
