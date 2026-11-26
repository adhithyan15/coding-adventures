//! # Probes: one per model function, for the function check (VOS31 6.2)
//!
//! A lemma exercises only some of the model functions, and only through the shape its claim
//! gives them.  A wrong `top` could hide behind a lemma that never asks for `top` on the inputs
//! that expose it.  So each model function also gets a **probe**: a small description that says,
//! once, four things about the function:
//!
//! ```text
//!   probe "after_run"                            what it says
//!   --------------------------------------------------------------------------------------
//!   binders   m: Ready, k: U8                    the function's whole domain, at N = 8
//!   warrant   after(ready_of(m0..m7), Run(k))    the expression the Warrant evaluator runs
//!   build     |model, aig, args| ...             the same call on the encoder (a circuit)
//!   reference |n, args| ...                      the same call on the plain Rust reference
//! ```
//!
//! The evaluator reads the function's value through the **ground-call interface** that
//! `warrant-grammar`'s `Evaluator::eval` already offers: a probe becomes the claim
//! `claim probe_<name>: forall <binders>. (<warrant> == <warrant>)`, and the left operand of
//! its body is evaluated on every case.  Nothing in `warrant-grammar` is edited (VOS31 section 3).
//! The claim is never asserted: only its left operand is read.  (VOS31 section 16 item 3 is
//! settled by this: the interface is usable.)
//!
//! ## What a probe's domain is
//!
//! The *whole* domain of the function at N = 8, with two exceptions, both recorded here and in
//! the VOS31 amendment of this PR:
//!
//! * `after` and `valid_pick` take a `Pick`; there is no `Pick` binder in a Warrant claim, so each
//!   has two probes, `*_run` (`Run(k)` for every `k`: 256 values, 255 included) and `*_idle`.
//! * `safe_dispatch` and `policy_dispatch` take `(Ready, Pick, Ready)`: 2^8 x 257 x 2^8 = 16.8M
//!   cases per relation, far past the budget of the evaluator route (`LEMMA_BUDGET` 400,000).
//!   Each is probed on every `Ready` and every `pick_of(k)`, and on four candidates for the
//!   second `Ready`, chosen by two `Bool` binders `s0`, `s1`: the successor `after(m, o)`
//!   (where both relations can be true), the unchanged `m`, `empty()`, and the successor with
//!   thread 0 flipped (a near miss).  That is 2^18 = 262,144 cases per relation, and both
//!   truth values occur.
//!
//! ## The case numbering
//!
//! As in [`crate::lemmas`]: binders in order, each taking consecutive bits, least significant
//! first; a `Ready` is N bits (8 on the evaluator side), a `U8` eight, a `Bool` one.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv::BitVec;

use crate::model::{id_const, SchedModel, ID_WIDTH};
use crate::reference as r;
use crate::types::{Out, PickEnc, Ready, Val};

/// What a binder ranges over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A ready set: eight `Bool` binders `<name>0 .. <name>7` on the Warrant side.
    Ready,
    /// A `U8`.
    U8,
    /// A `Bool`.
    Bool,
}

/// One binder of a probe.
#[derive(Clone, Copy, Debug)]
pub struct Binder {
    /// The Warrant name (a `Ready` named `m` is `m0 .. m7`).
    pub name: &'static str,
    /// What it ranges over.
    pub kind: Kind,
}

/// An argument of a probe's circuit.
#[derive(Clone, Debug)]
pub enum Arg {
    /// A ready set.
    Ready(Ready),
    /// A thread id.
    Id(BitVec),
    /// A flag.
    Bool(Edge),
}

impl Arg {
    fn ready(&self) -> &Ready {
        match self {
            Arg::Ready(r) => r,
            _ => unreachable!("probe argument order is fixed by its binders"),
        }
    }
    fn id(&self) -> &BitVec {
        match self {
            Arg::Id(v) => v,
            _ => unreachable!("probe argument order is fixed by its binders"),
        }
    }
    fn flag(&self) -> Edge {
        match self {
            Arg::Bool(e) => *e,
            _ => unreachable!("probe argument order is fixed by its binders"),
        }
    }
}

/// The type of a probe's value, which tells the evaluator side how to read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    /// `Bool`.
    Bool,
    /// `Ready`.
    Ready,
    /// `Upd`.
    Upd,
    /// `Pick`.
    Pick,
}

/// The circuit of a probe: the model function applied to the arguments.
pub type Build = fn(&dyn SchedModel, &mut Aig, &[Arg]) -> Out;
/// The reference value of a probe: `(n, decoded arguments)`.  A `Ready` decodes to its mask, a
/// `U8` to its value and a `Bool` to 0 or 1.
pub type Reference = fn(usize, &[u64]) -> Val;

/// One model function (or one `Pick` case of one), described once for all three checkers.
#[derive(Clone)]
pub struct Probe {
    /// The probe's name (also the suffix of its Warrant claim).
    pub name: &'static str,
    /// The domain.
    pub binders: Vec<Binder>,
    /// The Warrant expression whose value is read.
    pub warrant: String,
    /// The value's type.
    pub ty: Ty,
    /// The encoder side.
    pub build: Build,
    /// The reference side.
    pub reference: Reference,
}

impl Probe {
    /// The number of circuit inputs at width `n`.
    pub fn input_count(&self, n: usize) -> usize {
        self.binders
            .iter()
            .map(|b| match b.kind {
                Kind::Ready => n,
                Kind::U8 => ID_WIDTH,
                Kind::Bool => 1,
            })
            .sum()
    }

    /// Fresh inputs for the binders and the circuit of the probe on `model`.
    pub fn circuit(&self, model: &dyn SchedModel, aig: &mut Aig) -> Out {
        let args: Vec<Arg> = self
            .binders
            .iter()
            .map(|b| match b.kind {
                Kind::Ready => Arg::Ready(Ready::input(aig, model.n())),
                Kind::U8 => Arg::Id(BitVec::input(aig, ID_WIDTH)),
                Kind::Bool => Arg::Bool(aig.input()),
            })
            .collect();
        (self.build)(model, aig, &args)
    }

    /// Decode case `case` at width `n` into one number per binder.
    pub fn decode(&self, n: usize, case: u64) -> Vec<u64> {
        let mut shift = 0usize;
        self.binders
            .iter()
            .map(|b| {
                let w = match b.kind {
                    Kind::Ready => n,
                    Kind::U8 => ID_WIDTH,
                    Kind::Bool => 1,
                };
                let v = (case >> shift) & ((1u64 << w) - 1);
                shift += w;
                v
            })
            .collect()
    }

    /// The Warrant binder list, `m0: Bool, ..., m7: Bool, k: U8`.
    pub fn warrant_binders(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for b in &self.binders {
            match b.kind {
                Kind::Ready => {
                    for i in 0..8 {
                        parts.push(format!("{}{i}: Bool", b.name));
                    }
                }
                Kind::U8 => parts.push(format!("{}: U8", b.name)),
                Kind::Bool => parts.push(format!("{}: Bool", b.name)),
            }
        }
        parts.join(", ")
    }

    /// The probe as a Warrant claim: only the left operand of its body is ever evaluated.
    pub fn claim_text(&self) -> String {
        format!(
            "  claim probe_{}:\n    forall {}. (({}) == ({}))\n",
            self.name,
            self.warrant_binders(),
            self.warrant,
            self.warrant
        )
    }
}

/// `ready_of(m0, ..., m7)` for a binder named `name`.
fn rd(name: &str) -> String {
    let bits: Vec<String> = (0..8).map(|i| format!("{name}{i}")).collect();
    format!("ready_of({})", bits.join(", "))
}

fn b(name: &'static str, kind: Kind) -> Binder {
    Binder { name, kind }
}

fn run_pick(k: &BitVec) -> PickEnc {
    PickEnc {
        idle: Edge::constant(false),
        id: k.clone(),
    }
}

fn idle_pick() -> PickEnc {
    PickEnc {
        idle: Edge::constant(true),
        id: id_const(0),
    }
}

fn bool_val(x: bool) -> Val {
    Val::Bool(x)
}

fn pick_val(p: Option<u8>) -> Val {
    Val::Pick(p)
}

/// The four candidates for the second `Ready` of a relation probe (module docs).  `s0`, `s1`
/// select; the Warrant text of [`m2_warrant`] is the same selection.
fn m2_circuit(model: &dyn SchedModel, aig: &mut Aig, m: &Ready, next: &Ready, s: &[Edge]) -> Ready {
    let flipped_bit = model.mem(aig, next, &id_const(0));
    let flipped = model.put(aig, next, &id_const(0), !flipped_bit);
    let empty = model.empty();
    let if_s1 = model.ready_mux(aig, s[1], m, next);
    let else_s1 = model.ready_mux(aig, s[1], &empty, &flipped);
    model.ready_mux(aig, s[0], &if_s1, &else_s1)
}

fn m2_warrant(next: &str) -> String {
    format!(
        "if s0 then (if s1 then {m} else {next}) else (if s1 then empty() else put({next}, 0u8, not mem({next}, 0u8)))",
        m = rd("m"),
        next = next
    )
}

fn m2_reference(n: usize, m: u64, next: u64, s0: bool, s1: bool) -> u64 {
    match (s0, s1) {
        (true, true) => m,
        (true, false) => next,
        (false, true) => 0,
        (false, false) => r::put(n, next, 0, !r::mem(n, next, 0)),
    }
}

/// Every probe, in a fixed order.
pub fn all_probes() -> Vec<Probe> {
    let m = rd("m");
    let mut v: Vec<Probe> = Vec::new();

    v.push(Probe {
        name: "mem",
        binders: vec![b("m", Kind::Ready), b("i", Kind::U8)],
        warrant: format!("mem({m}, i)"),
        ty: Ty::Bool,
        build: |md, aig, a| Out::Bool(md.mem(aig, a[0].ready(), a[1].id())),
        reference: |n, a| bool_val(r::mem(n, a[0], a[1] as u8)),
    });
    v.push(Probe {
        name: "put",
        binders: vec![b("m", Kind::Ready), b("i", Kind::U8), b("v", Kind::Bool)],
        warrant: format!("put({m}, i, v)"),
        ty: Ty::Ready,
        build: |md, aig, a| Out::Ready(md.put(aig, a[0].ready(), a[1].id(), a[2].flag())),
        reference: |n, a| Val::Ready(r::put(n, a[0], a[1] as u8, a[2] == 1)),
    });
    v.push(Probe {
        name: "subset",
        binders: vec![b("a", Kind::Ready), b("c", Kind::Ready)],
        warrant: format!("subset({}, {})", rd("a"), rd("c")),
        ty: Ty::Bool,
        build: |md, aig, a| Out::Bool(md.subset(aig, a[0].ready(), a[1].ready())),
        reference: |_, a| bool_val(r::subset(a[0], a[1])),
    });
    v.push(Probe {
        name: "inv",
        binders: vec![b("m", Kind::Ready)],
        warrant: format!("inv({m})"),
        ty: Ty::Bool,
        build: |md, aig, a| Out::Bool(md.inv(aig, a[0].ready())),
        reference: |_, _| bool_val(true),
    });
    v.push(Probe {
        name: "init_m",
        binders: vec![b("d", Kind::Bool)],
        warrant: "init_m()".to_string(),
        ty: Ty::Ready,
        build: |md, _aig, _a| Out::Ready(md.init_m()),
        reference: |_, _| Val::Ready(0),
    });
    v.push(Probe {
        name: "abs_insert_upd",
        binders: vec![b("m", Kind::Ready), b("t", Kind::U8)],
        warrant: format!("abs_insert({m}, t).0"),
        ty: Ty::Upd,
        build: |md, aig, a| Out::Upd(md.abs_insert(aig, a[0].ready(), a[1].id()).upd),
        reference: |n, a| Val::Upd(r::abs_insert(n, a[0], a[1] as u8).0),
    });
    v.push(Probe {
        name: "abs_insert_ready",
        binders: vec![b("m", Kind::Ready), b("t", Kind::U8)],
        warrant: format!("abs_insert({m}, t).1"),
        ty: Ty::Ready,
        build: |md, aig, a| Out::Ready(md.abs_insert(aig, a[0].ready(), a[1].id()).ready),
        reference: |n, a| Val::Ready(r::abs_insert(n, a[0], a[1] as u8).1),
    });
    v.push(Probe {
        name: "abs_remove_upd",
        binders: vec![b("m", Kind::Ready), b("t", Kind::U8)],
        warrant: format!("abs_remove({m}, t).0"),
        ty: Ty::Upd,
        build: |md, aig, a| Out::Upd(md.abs_remove(aig, a[0].ready(), a[1].id()).upd),
        reference: |n, a| Val::Upd(r::abs_remove(n, a[0], a[1] as u8).0),
    });
    v.push(Probe {
        name: "abs_remove_ready",
        binders: vec![b("m", Kind::Ready), b("t", Kind::U8)],
        warrant: format!("abs_remove({m}, t).1"),
        ty: Ty::Ready,
        build: |md, aig, a| Out::Ready(md.abs_remove(aig, a[0].ready(), a[1].id()).ready),
        reference: |n, a| Val::Ready(r::abs_remove(n, a[0], a[1] as u8).1),
    });
    v.push(Probe {
        name: "after_run",
        binders: vec![b("m", Kind::Ready), b("k", Kind::U8)],
        warrant: format!("after({m}, Run(k))"),
        ty: Ty::Ready,
        build: |md, aig, a| Out::Ready(md.after(aig, a[0].ready(), &run_pick(a[1].id()))),
        reference: |n, a| Val::Ready(r::after(n, a[0], Some(a[1] as u8))),
    });
    v.push(Probe {
        name: "after_idle",
        binders: vec![b("m", Kind::Ready)],
        warrant: format!("after({m}, Idle)"),
        ty: Ty::Ready,
        build: |md, aig, a| Out::Ready(md.after(aig, a[0].ready(), &idle_pick())),
        reference: |n, a| Val::Ready(r::after(n, a[0], None)),
    });
    v.push(Probe {
        name: "valid_pick_run",
        binders: vec![b("m", Kind::Ready), b("k", Kind::U8)],
        warrant: format!("valid_pick({m}, Run(k))"),
        ty: Ty::Bool,
        build: |md, aig, a| Out::Bool(md.valid_pick(aig, a[0].ready(), &run_pick(a[1].id()))),
        reference: |n, a| bool_val(r::valid_pick(n, a[0], Some(a[1] as u8))),
    });
    v.push(Probe {
        name: "valid_pick_idle",
        binders: vec![b("m", Kind::Ready)],
        warrant: format!("valid_pick({m}, Idle)"),
        ty: Ty::Bool,
        build: |md, aig, a| Out::Bool(md.valid_pick(aig, a[0].ready(), &idle_pick())),
        reference: |n, a| bool_val(r::valid_pick(n, a[0], None)),
    });
    v.push(Probe {
        name: "top",
        binders: vec![b("m", Kind::Ready)],
        warrant: format!("top({m})"),
        ty: Ty::Pick,
        build: |md, aig, a| Out::Pick(md.top(aig, a[0].ready())),
        reference: |n, a| pick_val(r::top(n, a[0])),
    });
    v.push(Probe {
        name: "pick_of",
        binders: vec![b("k", Kind::U8)],
        warrant: "pick_of(k)".to_string(),
        ty: Ty::Pick,
        build: |md, aig, a| Out::Pick(md.pick_of(aig, a[0].id())),
        reference: |_, a| pick_val(r::pick_of(a[0] as u8)),
    });
    v.push(Probe {
        name: "pre_insert",
        binders: vec![b("m", Kind::Ready), b("t", Kind::U8)],
        warrant: format!("pre_insert({m}, t)"),
        ty: Ty::Bool,
        build: |md, aig, a| Out::Bool(md.pre_insert(aig, a[0].ready(), a[1].id())),
        reference: |n, a| bool_val(r::pre(n, a[1] as u8)),
    });
    v.push(Probe {
        name: "pre_remove",
        binders: vec![b("m", Kind::Ready), b("t", Kind::U8)],
        warrant: format!("pre_remove({m}, t)"),
        ty: Ty::Bool,
        build: |md, aig, a| Out::Bool(md.pre_remove(aig, a[0].ready(), a[1].id())),
        reference: |n, a| bool_val(r::pre(n, a[1] as u8)),
    });
    let rel_binders = || {
        vec![
            b("m", Kind::Ready),
            b("k", Kind::U8),
            b("s0", Kind::Bool),
            b("s1", Kind::Bool),
        ]
    };
    let next = format!("after({m}, pick_of(k))");
    v.push(Probe {
        name: "safe_dispatch",
        binders: rel_binders(),
        warrant: format!("safe_dispatch({m}, pick_of(k), {})", m2_warrant(&next)),
        ty: Ty::Bool,
        build: |md, aig, a| {
            let o = md.pick_of(aig, a[1].id());
            let next = md.after(aig, a[0].ready(), &o);
            let m2 = m2_circuit(md, aig, a[0].ready(), &next, &[a[2].flag(), a[3].flag()]);
            Out::Bool(md.safe_dispatch(aig, a[0].ready(), &o, &m2))
        },
        reference: |n, a| {
            let o = r::pick_of(a[1] as u8);
            let next = r::after(n, a[0], o);
            let m2 = m2_reference(n, a[0], next, a[2] == 1, a[3] == 1);
            bool_val(r::safe_dispatch(n, a[0], o, m2))
        },
    });
    v.push(Probe {
        name: "policy_dispatch",
        binders: rel_binders(),
        warrant: format!("policy_dispatch({m}, pick_of(k), {})", m2_warrant(&next)),
        ty: Ty::Bool,
        build: |md, aig, a| {
            let o = md.pick_of(aig, a[1].id());
            let next = md.after(aig, a[0].ready(), &o);
            let m2 = m2_circuit(md, aig, a[0].ready(), &next, &[a[2].flag(), a[3].flag()]);
            Out::Bool(md.policy_dispatch(aig, a[0].ready(), &o, &m2))
        },
        reference: |n, a| {
            let o = r::pick_of(a[1] as u8);
            let next = r::after(n, a[0], o);
            let m2 = m2_reference(n, a[0], next, a[2] == 1, a[3] == 1);
            bool_val(r::policy_dispatch(n, a[0], o, m2))
        },
    });
    v
}
