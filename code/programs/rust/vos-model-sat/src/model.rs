//! # The `sched` contract model as a circuit generator
//!
//! `code/packages/rust/vos-sched-contract/warrant/contract.part` defines the abstract scheduler
//! state `M` (the ready set) and a dozen model functions over it.  [`SchedModel`] has one method
//! per function, and each *default method body is the honest transcription* of the Warrant text
//! beside it, calling the other functions through `self`:
//!
//! ```text
//!   contract.part                        this file
//!   ----------------------------------   ------------------------------------------
//!   model safe_dispatch(m, o, m2) =      fn safe_dispatch(&self, aig, m, o, m2) {
//!     valid_pick(m, o) and                   let valid = self.valid_pick(aig, m, o);
//!     m2 == after(m, o)                      let next = self.after(aig, m, o); ... }
//! ```
//!
//! ## Why the honest model is a set of default methods
//!
//! A mutant of the contract (`warrant/mutants/<lemma>.part`) replaces ONE model by its span, and
//! every *other* model that calls it sees the change.  `safety_is_idle_or_a_member`'s mutant
//! replaces `valid_pick`, and `safe_dispatch` (which calls `valid_pick`) changes with it.  With
//! default methods that call `self.valid_pick(..)`, overriding one method in a [`crate::Mutant`]
//! reproduces exactly that: nothing else is edited.
//!
//! ## N-parametric
//!
//! [`Encoder`] holds only the width `n`.  The same loops (`for j in 0..n`) build the N = 8
//! circuit that the evaluator checks and the N = 64 circuit that a solver will see.  Where
//! `contract.part` writes the number 8 it means *the thread count*, which is N here:
//!
//! ```text
//!   contract.part               here (N = n)
//!   -------------------------   --------------------------------------
//!   t >= 8u8, t < 8u8           !(t < n), t < n
//!   p0 .. p7, Run(7u8) .. 0u8   bits 0 .. n-1 of the Ready vector
//!   `top`'s eight-deep chain    a loop from thread 0 up to thread n-1
//!   cfg_N()                     n
//! ```
//!
//! Section 6.3 of VOS31 is the check that this generalisation is faithful: it compares every
//! method against a plain Rust reference at N = 1 to 10.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv;
use bit_blaster::bv::BitVec;

use crate::types::{PickEnc, Ready, Step, UpdEnc};

/// The thread-id width: a `U8` at every N (see [`crate::types`]).
pub const ID_WIDTH: usize = 8;

/// The sentinel `pick_of` maps to `Idle`.
pub const IDLE_ID: u128 = 255;

/// The constant true edge.
fn tru() -> Edge {
    Edge::constant(true)
}

/// The constant false edge.
fn fls() -> Edge {
    Edge::constant(false)
}

/// The id constant `v` as an 8-bit vector.
pub fn id_const(v: usize) -> BitVec {
    BitVec::constant(ID_WIDTH, v as u128)
}

/// One method per model function of `contract.part`; every default body is the honest
/// transcription (module docs).
pub trait SchedModel {
    /// The thread count N.
    fn n(&self) -> usize;

    // ---- the ready set ------------------------------------------------------------------

    /// `empty()`: no thread is ready.
    fn empty(&self) -> Ready {
        Ready(BitVec::constant(self.n(), 0))
    }

    /// `ready_of(b0, ..., b7)`: building a `Ready` from its bits is the identity here (a `Ready`
    /// already is its bits); named only so the correspondence with `contract.part` is visible.
    fn ready_of(&self, bits: &[Edge]) -> Ready {
        Ready(BitVec {
            bits: bits.to_vec(),
        })
    }

    /// `mem(m, i)`: thread `i` is in `m`.
    ///
    /// ```text
    ///   mem(m, i) = OR over j of ( i == j  AND  m_j )     an id i >= n matches no j: false
    /// ```
    fn mem(&self, aig: &mut Aig, m: &Ready, i: &BitVec) -> Edge {
        let mut terms = Vec::with_capacity(self.n());
        for j in 0..self.n() {
            let at_j = bv::eq(aig, i, &id_const(j));
            terms.push(aig.and(at_j, m.bit(j)));
        }
        aig.or_all(&terms)
    }

    /// `set_at(cur, i, k, b)`: `if i == k then b else cur`.
    fn set_at(&self, aig: &mut Aig, cur: Edge, i: &BitVec, k: usize, b: Edge) -> Edge {
        let hit = bv::eq(aig, i, &id_const(k));
        aig.mux(hit, b, cur)
    }

    /// `put(m, i, b)`: `m` with thread `i` set to `b`.  An id `>= n` changes nothing.
    fn put(&self, aig: &mut Aig, m: &Ready, i: &BitVec, b: Edge) -> Ready {
        let bits: Vec<Edge> = (0..self.n())
            .map(|k| self.set_at(aig, m.bit(k), i, k, b))
            .collect();
        self.ready_of(&bits)
    }

    /// `subset(a, b)`: every thread of `a` is in `b`.
    fn subset(&self, aig: &mut Aig, a: &Ready, b: &Ready) -> Edge {
        let terms: Vec<Edge> = (0..self.n())
            .map(|k| aig.implies(a.bit(k), b.bit(k)))
            .collect();
        aig.and_all(&terms)
    }

    /// Structural equality of two `Ready`s (Warrant `==` on the record): every bit equal.
    fn ready_eq(&self, aig: &mut Aig, a: &Ready, b: &Ready) -> Edge {
        let terms: Vec<Edge> = (0..self.n())
            .map(|k| aig.xnor(a.bit(k), b.bit(k)))
            .collect();
        aig.and_all(&terms)
    }

    /// `if c then a else b` on a `Ready`.
    fn ready_mux(&self, aig: &mut Aig, c: Edge, a: &Ready, b: &Ready) -> Ready {
        Ready(bv::ite(aig, c, &a.0, &b.0))
    }

    // ---- invariants and operations ------------------------------------------------------

    /// `inv(m)`: total, `true` (so `closure_at_m` is a trivial lemma, and says so).
    fn inv(&self, _aig: &mut Aig, _m: &Ready) -> Edge {
        tru()
    }

    /// `init_m()`: the empty set.
    fn init_m(&self) -> Ready {
        self.empty()
    }

    /// The `Upd` of `if in_range then (if unchanged then Unchanged else Changed) else
    /// OutOfRange`.
    fn upd_of(&self, aig: &mut Aig, in_range: Edge, unchanged: Edge) -> UpdEnc {
        UpdEnc {
            changed: aig.and(in_range, !unchanged),
            unchanged: aig.and(in_range, unchanged),
            out_of_range: !in_range,
        }
    }

    /// `abs_insert(m, t)`:
    ///
    /// ```text
    ///   if t >= n then (OutOfRange, m)
    ///   else (if mem(m, t) then (Unchanged, m) else (Changed, put(m, t, true)))
    /// ```
    fn abs_insert(&self, aig: &mut Aig, m: &Ready, t: &BitVec) -> Step {
        let in_range = bv::ult(aig, t, &id_const(self.n()));
        let member = self.mem(aig, m, t);
        let with_t = self.put(aig, m, t, tru());
        let upd = self.upd_of(aig, in_range, member);
        let inner = self.ready_mux(aig, member, m, &with_t);
        let ready = self.ready_mux(aig, in_range, &inner, m);
        Step { upd, ready }
    }

    /// `abs_remove(m, t)`:
    ///
    /// ```text
    ///   if t >= n then (OutOfRange, m)
    ///   else (if mem(m, t) then (Changed, put(m, t, false)) else (Unchanged, m))
    /// ```
    fn abs_remove(&self, aig: &mut Aig, m: &Ready, t: &BitVec) -> Step {
        let in_range = bv::ult(aig, t, &id_const(self.n()));
        let member = self.mem(aig, m, t);
        let without_t = self.put(aig, m, t, fls());
        let upd = self.upd_of(aig, in_range, !member);
        let inner = self.ready_mux(aig, member, &without_t, m);
        let ready = self.ready_mux(aig, in_range, &inner, m);
        Step { upd, ready }
    }

    // ---- picks and the dispatch relations -----------------------------------------------

    /// `pick_of(k)`: 255 is `Idle`, anything else is `Run(k)`.
    fn pick_of(&self, aig: &mut Aig, k: &BitVec) -> PickEnc {
        PickEnc {
            idle: bv::eq(aig, k, &BitVec::constant(ID_WIDTH, IDLE_ID)),
            id: k.clone(),
        }
    }

    /// `after(m, o)`: `Idle` leaves `m`, `Run(t)` removes `t`.
    fn after(&self, aig: &mut Aig, m: &Ready, o: &PickEnc) -> Ready {
        let without = self.put(aig, m, &o.id, fls());
        self.ready_mux(aig, o.idle, m, &without)
    }

    /// `valid_pick(m, o)`: `Idle` is valid; `Run(t)` is valid iff `t < n and mem(m, t)`.
    fn valid_pick(&self, aig: &mut Aig, m: &Ready, o: &PickEnc) -> Edge {
        let t_ok = bv::ult(aig, &o.id, &id_const(self.n()));
        let member = self.mem(aig, m, &o.id);
        let run_ok = aig.and(t_ok, member);
        aig.mux(o.idle, tru(), run_ok)
    }

    /// `if c then Run(id) else other` on a `Pick`.
    fn pick_mux(&self, aig: &mut Aig, c: Edge, id: &BitVec, other: &PickEnc) -> PickEnc {
        PickEnc {
            idle: aig.mux(c, fls(), other.idle),
            id: bv::ite(aig, c, id, &other.id),
        }
    }

    /// `top(m)`: the largest ready id, or `Idle` for the empty set.
    ///
    /// ```text
    ///   if p7 then Run(7) else (if p6 then Run(6) else ( ... (if p0 then Run(0) else Idle)))
    /// ```
    /// Built from the inside out: start at `Idle`, then wrap thread 0, 1, ..., n-1, so that
    /// thread n-1 is the outermost test.
    fn top(&self, aig: &mut Aig, m: &Ready) -> PickEnc {
        let mut result = PickEnc {
            idle: tru(),
            id: id_const(0),
        };
        for j in 0..self.n() {
            result = self.pick_mux(aig, m.bit(j), &id_const(j), &result);
        }
        result
    }

    /// Structural equality of two `Pick`s: both `Idle`, or both `Run` of the same id.
    fn pick_eq(&self, aig: &mut Aig, a: &PickEnc, b: &PickEnc) -> Edge {
        let both_idle = aig.and(a.idle, b.idle);
        let same_id = bv::eq(aig, &a.id, &b.id);
        let neither_idle = aig.and(!a.idle, !b.idle);
        let both_run = aig.and(neither_idle, same_id);
        aig.or(both_idle, both_run)
    }

    // ---- caller preconditions and the two relations -------------------------------------

    /// `pre_insert(m, t)`: the id names a thread, `t < n`.
    fn pre_insert(&self, aig: &mut Aig, _m: &Ready, t: &BitVec) -> Edge {
        bv::ult(aig, t, &id_const(self.n()))
    }

    /// `pre_remove(m, t)`: the id names a thread, `t < n`.
    fn pre_remove(&self, aig: &mut Aig, _m: &Ready, t: &BitVec) -> Edge {
        bv::ult(aig, t, &id_const(self.n()))
    }

    /// `safe_dispatch(m, o, m2)`: `valid_pick(m, o) and m2 == after(m, o)`.
    fn safe_dispatch(&self, aig: &mut Aig, m: &Ready, o: &PickEnc, m2: &Ready) -> Edge {
        let valid = self.valid_pick(aig, m, o);
        let next = self.after(aig, m, o);
        let same = self.ready_eq(aig, m2, &next);
        aig.and(valid, same)
    }

    /// `policy_dispatch(m, o, m2)`: `o == top(m) and m2 == after(m, o)`.
    fn policy_dispatch(&self, aig: &mut Aig, m: &Ready, o: &PickEnc, m2: &Ready) -> Edge {
        let best = self.top(aig, m);
        let is_top = self.pick_eq(aig, o, &best);
        let next = self.after(aig, m, o);
        let same = self.ready_eq(aig, m2, &next);
        aig.and(is_top, same)
    }
}

/// The honest encoder: every model function exactly as `contract.part` states it, at width `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoder {
    /// The thread count N, `1 ..= MAX_N`.
    pub n: usize,
}

impl Encoder {
    /// An encoder at width `n`.  `n` must be in `1 ..= MAX_N`: it is a loop bound and a `U8`
    /// constant, and the id sentinel 255 must stay above it.
    pub fn new(n: usize) -> Result<Encoder, crate::BadWidth> {
        if (1..=crate::MAX_N).contains(&n) {
            Ok(Encoder { n })
        } else {
            Err(crate::BadWidth(n))
        }
    }
}

impl SchedModel for Encoder {
    fn n(&self) -> usize {
        self.n
    }
}
