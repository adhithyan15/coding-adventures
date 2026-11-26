//! # Mutant encoders, and deliberately wrong encoders
//!
//! ## The ten mutants (VOS31 section 5.2)
//!
//! Every lemma of `warrant/lemmas.part` has a mutant, `warrant/mutants/<lemma>.part`: the contract
//! with ONE model function replaced by a deliberately faulty one, which the lemma must catch.
//! A lemma whose negation is UNSAT and whose mutant's negation is also UNSAT has proved nothing,
//! so each mutant is the *control* of its lemma: it must be SAT (VOS31 7.2 rule 3, VOS-040b).
//!
//! [`Mutant`] is [`crate::Encoder`] with exactly the one method that the part replaces,
//! replaced by the same change, written for any N.  The correspondence between a `.part` and this
//! Rust is **checked, not trusted**: at N = 8 the encoder's failing case set must equal the
//! evaluator's on the mutant document, case for case (VOS31 6.1).
//!
//! ```text
//!   lemma                                    replaced model    fault (generalised to N)
//!   ---------------------------------------  ----------------  ------------------------------
//!   closure_at_m                             inv               m.p0: not total
//!   insert_outside_pre_changes_nothing       abs_insert        out of range answers Unchanged
//!   remove_outside_pre_changes_nothing       abs_remove        out of range answers Unchanged
//!   insert_then_remove_restores              abs_insert        always sets thread 0
//!   top_is_the_maximum                       top               prefers thread 0 to thread N-1
//!   safety_is_idle_or_a_member               valid_pick        any id below N is valid
//!   safe_dispatch_only_removes_the_choice    after             Run removes every thread
//!   policy_implies_safety                    valid_pick        refuses thread N-1 (7 at N = 8)
//!   policy_admits_only_top                   policy_dispatch   also admits Idle
//!   config_n_is_the_id_range                 pre_insert        admits id N (8 at N = 8)
//! ```
//!
//! The `top` mutant is the same as the base at N = 1 (there is no thread N-1 distinct from
//! thread 0 to prefer), so its SAT control exists only for N >= 2; the ladder starts at 8.
//!
//! ## Wrong encoders ([`Fault`])
//!
//! The negative controls of the cross-checks need encoders that are wrong *without* being a
//! contract mutant (VOS31 6.4).  [`Fault`] has four, each a plausible transcription slip:
//! an off-by-one in `top`, `<=` for `<` in `pre_insert`, a `mem` that ignores thread 0, and a
//! `put` that can only set, never clear.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv;
use bit_blaster::bv::BitVec;

use crate::lemmas::LemmaId;
use crate::model::{id_const, Encoder, SchedModel};
use crate::types::{PickEnc, Ready, Step};

fn tru() -> Edge {
    Edge::constant(true)
}

/// The encoder of the contract with `lemma`'s mutant applied, at width `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mutant {
    /// Whose mutant this is.
    pub lemma: LemmaId,
    /// The thread count N.
    pub n: usize,
}

impl Mutant {
    /// The mutant of `lemma` at width `n` (`1 ..= MAX_N`).
    pub fn new(lemma: LemmaId, n: usize) -> Result<Mutant, crate::BadWidth> {
        crate::model::Encoder::new(n)?;
        Ok(Mutant { lemma, n })
    }

    /// The name of the model function this mutant replaces, as `warrant/lemmas.table` writes it.
    pub fn replaced_model(&self) -> &'static str {
        self.lemma.mutant_model()
    }

    /// The valid-pick test shared by the two `valid_pick` mutants: `limit` is the exclusive id
    /// bound, and `needs_member` says whether membership is also required.
    fn valid_pick_with(
        &self,
        aig: &mut Aig,
        m: &Ready,
        o: &PickEnc,
        limit: usize,
        needs_member: bool,
    ) -> Edge {
        let t_ok = bv::ult(aig, &o.id, &id_const(limit));
        let run_ok = if needs_member {
            let member = self.mem(aig, m, &o.id);
            aig.and(t_ok, member)
        } else {
            t_ok
        };
        aig.mux(o.idle, tru(), run_ok)
    }

    /// The honest encoder at this width: every method a mutant does not replace is the honest
    /// one (a mutant is the contract with ONE model replaced).
    fn honest(&self) -> Encoder {
        Encoder { n: self.n }
    }
}

impl SchedModel for Mutant {
    fn n(&self) -> usize {
        self.n
    }

    fn inv(&self, aig: &mut Aig, m: &Ready) -> Edge {
        let _ = aig;
        match self.lemma {
            // `model inv(m) = m.p0`
            LemmaId::ClosureAtM => m.bit(0),
            _ => tru(),
        }
    }

    fn pre_insert(&self, aig: &mut Aig, _m: &Ready, t: &BitVec) -> Edge {
        match self.lemma {
            // `t < 9u8`: admits id N
            LemmaId::ConfigNIsTheIdRange => bv::ult(aig, t, &id_const(self.n + 1)),
            _ => bv::ult(aig, t, &id_const(self.n)),
        }
    }

    fn abs_insert(&self, aig: &mut Aig, m: &Ready, t: &BitVec) -> Step {
        match self.lemma {
            LemmaId::InsertOutsidePreChangesNothing => {
                // `if t >= 8u8 then (Unchanged, m) else ...`
                let in_range = bv::ult(aig, t, &id_const(self.n));
                let member = self.mem(aig, m, t);
                let with_t = self.put(aig, m, t, tru());
                let changed = aig.and(in_range, !member);
                let inner = self.ready_mux(aig, member, m, &with_t);
                let ready = self.ready_mux(aig, in_range, &inner, m);
                Step {
                    upd: crate::types::UpdEnc {
                        changed,
                        unchanged: !changed,
                        out_of_range: Edge::constant(false),
                    },
                    ready,
                }
            }
            LemmaId::InsertThenRemoveRestores => {
                // `... else (Changed, put(m, 0u8, true))`: always thread 0
                let in_range = bv::ult(aig, t, &id_const(self.n));
                let member = self.mem(aig, m, t);
                let with_zero = self.put(aig, m, &id_const(0), tru());
                let upd = self.upd_of(aig, in_range, member);
                let inner = self.ready_mux(aig, member, m, &with_zero);
                let ready = self.ready_mux(aig, in_range, &inner, m);
                Step { upd, ready }
            }
            _ => self.honest().abs_insert(aig, m, t),
        }
    }

    fn abs_remove(&self, aig: &mut Aig, m: &Ready, t: &BitVec) -> Step {
        match self.lemma {
            LemmaId::RemoveOutsidePreChangesNothing => {
                // `if t >= 8u8 then (Unchanged, m) else ...`
                let in_range = bv::ult(aig, t, &id_const(self.n));
                let member = self.mem(aig, m, t);
                let without_t = self.put(aig, m, t, Edge::constant(false));
                let changed = aig.and(in_range, member);
                let inner = self.ready_mux(aig, member, &without_t, m);
                let ready = self.ready_mux(aig, in_range, &inner, m);
                Step {
                    upd: crate::types::UpdEnc {
                        changed,
                        unchanged: !changed,
                        out_of_range: Edge::constant(false),
                    },
                    ready,
                }
            }
            _ => self.honest().abs_remove(aig, m, t),
        }
    }

    fn after(&self, aig: &mut Aig, m: &Ready, o: &PickEnc) -> Ready {
        match self.lemma {
            LemmaId::SafeDispatchOnlyRemovesTheChoice => {
                // `Run(t) -> empty()`
                let none = self.empty();
                self.ready_mux(aig, o.idle, m, &none)
            }
            _ => self.honest().after(aig, m, o),
        }
    }

    fn valid_pick(&self, aig: &mut Aig, m: &Ready, o: &PickEnc) -> Edge {
        match self.lemma {
            // `Run(t) -> t < 8u8`: membership not required
            LemmaId::SafetyIsIdleOrAMember => self.valid_pick_with(aig, m, o, self.n, false),
            // `Run(t) -> t < 7u8 and mem(m, t)`: refuses the top thread
            LemmaId::PolicyImpliesSafety => {
                self.valid_pick_with(aig, m, o, self.n.saturating_sub(1), true)
            }
            _ => self.valid_pick_with(aig, m, o, self.n, true),
        }
    }

    fn top(&self, aig: &mut Aig, m: &Ready) -> PickEnc {
        match self.lemma {
            LemmaId::TopIsTheMaximum => {
                // `if p0 then Run(0) else (if p6 ... (if p0 then Run(0) else Idle))`:
                // the honest chain without thread N-1, under an outermost test of thread 0.
                let mut result = PickEnc {
                    idle: tru(),
                    id: id_const(0),
                };
                for j in 0..self.n.saturating_sub(1) {
                    result = self.pick_mux(aig, m.bit(j), &id_const(j), &result);
                }
                self.pick_mux(aig, m.bit(0), &id_const(0), &result)
            }
            _ => self.honest().top(aig, m),
        }
    }

    fn policy_dispatch(&self, aig: &mut Aig, m: &Ready, o: &PickEnc, m2: &Ready) -> Edge {
        // The honest body calls `self.top` and `self.after`, so it is spelled out here (not
        // delegated to `honest()`): the `top` and `after` mutants must reach it, as they do
        // in the Warrant document where `policy_dispatch` calls the replaced model.
        let best = self.top(aig, m);
        let is_top = self.pick_eq(aig, o, &best);
        let admitted = if self.lemma == LemmaId::PolicyAdmitsOnlyTop {
            // `(o == top(m) or o == Idle) and ...`: also admits Idle.
            aig.or(is_top, o.idle)
        } else {
            is_top
        };
        let next = self.after(aig, m, o);
        let same = self.ready_eq(aig, m2, &next);
        aig.and(admitted, same)
    }
}

/// A plausible transcription slip: an encoder that is wrong without being any contract mutant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// `top` stops one thread early: it never looks at thread N-1 (a loop bound off by one).
    TopOffByOne,
    /// `pre_insert` is `t <= n` where the contract says `t < n` (`<=` for `<`).
    PreInsertLe,
    /// `mem` ignores thread 0 (an index off by one).
    MemIgnoresZero,
    /// `put` can only set a bit, never clear it (the `b` operand is OR-ed in).
    PutOnlySets,
}

impl Fault {
    /// Every fault, for tests that must catch each.
    pub const ALL: [Fault; 4] = [
        Fault::TopOffByOne,
        Fault::PreInsertLe,
        Fault::MemIgnoresZero,
        Fault::PutOnlySets,
    ];
}

/// The honest encoder with one [`Fault`] injected, at width `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Faulty {
    /// Which slip.
    pub fault: Fault,
    /// The thread count N.
    pub n: usize,
}

impl SchedModel for Faulty {
    fn n(&self) -> usize {
        self.n
    }

    fn top(&self, aig: &mut Aig, m: &Ready) -> PickEnc {
        let mut result = PickEnc {
            idle: tru(),
            id: id_const(0),
        };
        let upto = if self.fault == Fault::TopOffByOne {
            self.n.saturating_sub(1)
        } else {
            self.n
        };
        for j in 0..upto {
            result = self.pick_mux(aig, m.bit(j), &id_const(j), &result);
        }
        result
    }

    fn pre_insert(&self, aig: &mut Aig, _m: &Ready, t: &BitVec) -> Edge {
        if self.fault == Fault::PreInsertLe {
            bv::ule(aig, t, &id_const(self.n))
        } else {
            bv::ult(aig, t, &id_const(self.n))
        }
    }

    fn mem(&self, aig: &mut Aig, m: &Ready, i: &BitVec) -> Edge {
        let first = usize::from(self.fault == Fault::MemIgnoresZero);
        let mut terms = Vec::new();
        for j in first..self.n {
            let at_j = bv::eq(aig, i, &id_const(j));
            terms.push(aig.and(at_j, m.bit(j)));
        }
        aig.or_all(&terms)
    }

    fn put(&self, aig: &mut Aig, m: &Ready, i: &BitVec, b: Edge) -> Ready {
        let bits: Vec<Edge> = (0..self.n)
            .map(|k| {
                let hit = bv::eq(aig, i, &id_const(k));
                let target = if self.fault == Fault::PutOnlySets {
                    aig.or(m.bit(k), b)
                } else {
                    b
                };
                aig.mux(hit, target, m.bit(k))
            })
            .collect();
        self.ready_of(&bits)
    }
}
