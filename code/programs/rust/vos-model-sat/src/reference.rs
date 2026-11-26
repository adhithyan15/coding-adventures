//! # A plain Rust reference model of `sched`
//!
//! The third cross-check (VOS31 6.3) compares the encoder with a model that shares **no code**
//! with it.  This is that model: ordinary integer Rust, no AIG, no `SchedModel`, written from
//! `contract.part` and from the English of VOS10 9.2 rather than from the encoder.
//!
//! ```text
//!   Ready  = a u64 bit mask        bit i set  <=>  thread i ready      (N <= 64)
//!   id     = a u8
//!   Pick   = Option<u8>            None = Idle, Some(t) = Run(t)
//!   Upd    = u8                    0 Changed, 1 Unchanged, 2 OutOfRange
//! ```
//!
//! It is the ONLY thing that relates two widths.  At N = 8 it is compared with the Warrant
//! evaluator (the probes of [`crate::probes`] carry both); at N = 1 to 10 the encoder is compared
//! with it, exhaustively.  So the chain of trust for a width other than 8 is
//! `evaluator =(N=8)= reference =(N=1..10)= encoder`, which is evidence about the encoder
//! *family*, not a proof about `contract.part` (VOS31 section 1; VOS-D072).

use crate::lemmas::LemmaId;

/// `Changed`.
pub const CHANGED: u8 = 0;
/// `Unchanged`.
pub const UNCHANGED: u8 = 1;
/// `OutOfRange`.
pub const OUT_OF_RANGE: u8 = 2;

/// The mask of the N valid thread bits.
pub fn mask(n: usize) -> u64 {
    if n >= 64 {
        u64::MAX
    } else {
        (1u64 << n) - 1
    }
}

/// Is thread `i` ready in `m`?  An id `>= n` is never ready.
pub fn mem(n: usize, m: u64, i: u8) -> bool {
    (i as usize) < n && (m >> i) & 1 == 1
}

/// `m` with thread `i` set to `b`; an id `>= n` changes nothing.
pub fn put(n: usize, m: u64, i: u8, b: bool) -> u64 {
    if (i as usize) >= n {
        m
    } else if b {
        m | (1 << i)
    } else {
        m & !(1 << i)
    }
}

/// Is every thread of `a` in `b`?
pub fn subset(a: u64, b: u64) -> bool {
    a & !b == 0
}

/// The highest-priority (largest id) ready thread, `None` for the empty set.
pub fn top(n: usize, m: u64) -> Option<u8> {
    let m = m & mask(n);
    if m == 0 {
        None
    } else {
        Some((63 - m.leading_zeros()) as u8)
    }
}

/// `pick_of(k)`: 255 is `Idle`.
pub fn pick_of(k: u8) -> Option<u8> {
    if k == 255 {
        None
    } else {
        Some(k)
    }
}

/// The ready set after dispatching `o`.
pub fn after(n: usize, m: u64, o: Option<u8>) -> u64 {
    match o {
        None => m,
        Some(t) => put(n, m, t, false),
    }
}

/// Is `o` a valid choice in `m`?
pub fn valid_pick(n: usize, m: u64, o: Option<u8>) -> bool {
    match o {
        None => true,
        Some(t) => (t as usize) < n && mem(n, m, t),
    }
}

/// `abs_insert`: the `Upd` and the new set.
pub fn abs_insert(n: usize, m: u64, t: u8) -> (u8, u64) {
    if (t as usize) >= n {
        (OUT_OF_RANGE, m)
    } else if mem(n, m, t) {
        (UNCHANGED, m)
    } else {
        (CHANGED, put(n, m, t, true))
    }
}

/// `abs_remove`: the `Upd` and the new set.
pub fn abs_remove(n: usize, m: u64, t: u8) -> (u8, u64) {
    if (t as usize) >= n {
        (OUT_OF_RANGE, m)
    } else if mem(n, m, t) {
        (CHANGED, put(n, m, t, false))
    } else {
        (UNCHANGED, m)
    }
}

/// The caller precondition of insert and of remove: the id names a thread.
pub fn pre(n: usize, t: u8) -> bool {
    (t as usize) < n
}

/// The safety relation.
pub fn safe_dispatch(n: usize, m: u64, o: Option<u8>, m2: u64) -> bool {
    valid_pick(n, m, o) && m2 == after(n, m, o)
}

/// The fixed-priority relation.
pub fn policy_dispatch(n: usize, m: u64, o: Option<u8>, m2: u64) -> bool {
    o == top(n, m) && m2 == after(n, m, o)
}

/// Does `lemma` hold on this case?  `m` is the ready set, `t` the `U8` binder (`t` or `k`);
/// a lemma that does not bind one ignores it.  Each arm is the claim's English, not the encoder.
pub fn lemma_holds(lemma: LemmaId, n: usize, m: u64, t: u8) -> bool {
    let m = m & mask(n);
    match lemma {
        // inv is total, so closure is true.
        LemmaId::ClosureAtM => true,
        LemmaId::InsertOutsidePreChangesNothing => {
            pre(n, t) || abs_insert(n, m, t) == (OUT_OF_RANGE, m)
        }
        LemmaId::RemoveOutsidePreChangesNothing => {
            pre(n, t) || abs_remove(n, m, t) == (OUT_OF_RANGE, m)
        }
        LemmaId::InsertThenRemoveRestores => {
            if !(pre(n, t) && !mem(n, m, t)) {
                return true;
            }
            let (upd, m1) = abs_insert(n, m, t);
            upd == CHANGED && abs_remove(n, m1, t).1 == m
        }
        LemmaId::TopIsTheMaximum => {
            let best = top(n, m);
            let upper = !mem(n, m, t)
                || match best {
                    None => false,
                    Some(h) => t <= h,
                };
            let witness = match best {
                None => !mem(n, m, t),
                Some(h) => mem(n, m, h),
            };
            upper && witness
        }
        LemmaId::SafetyIsIdleOrAMember => {
            let o = pick_of(t);
            safe_dispatch(n, m, o, after(n, m, o)) == (t == 255 || (pre(n, t) && mem(n, m, t)))
        }
        LemmaId::SafeDispatchOnlyRemovesTheChoice => {
            let o = pick_of(t);
            if !valid_pick(n, m, o) {
                return true;
            }
            let next = after(n, m, o);
            subset(next, m)
                && match o {
                    None => next == m,
                    Some(h) => next == put(n, m, h, false) && !mem(n, next, h),
                }
        }
        LemmaId::PolicyImpliesSafety => {
            let best = top(n, m);
            let next = after(n, m, best);
            !policy_dispatch(n, m, best, next) || safe_dispatch(n, m, best, next)
        }
        LemmaId::PolicyAdmitsOnlyTop => {
            let o = pick_of(t);
            !policy_dispatch(n, m, o, after(n, m, o)) || o == top(n, m)
        }
        // pre_insert(empty, t) is `t < n`, and the lemma says it equals `t < cfg_N()` = n.
        LemmaId::ConfigNIsTheIdRange => pre(n, t) == ((t as usize) < n),
    }
}
