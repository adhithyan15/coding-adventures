//! Hand-written behaviour tests (the generated evidence suite is separate and needs the
//! `ghost` feature).  Everything here goes through the public trait, as the spine does.
//!
//! A state is only ever made by `init` and the operations (its fields are private).  Every
//! reachable state is "some ready set, with the pointer one past the thread last served", so
//! [`state_of`] builds `(mask, ptr)` the way a real run would: serve thread `ptr - 1` once
//! (that leaves the pointer at `ptr`), then insert the threads of `mask`.
//!
//! The reference model is the obvious scan written with a loop: from `ptr`, look at
//! `ptr, ptr+1, ... ptr+7` modulo 8.  It is the same semantics as the op-free `rr_pick` /
//! `rr_next` models of the contract-check round-robin witness
//! (`vos-sched-contract-check/tests/fixtures/impl-rr-models.part`).

use vos_sched_contract::{Pick, Sched, Upd};
use vos_sched_rr::{RoundRobin, Rr};

type S = RoundRobin;

/// The state whose ready set is `mask` and whose pointer is `ptr`, built by real operations.
fn state_of(mask: u8, ptr: u8) -> Rr {
    assert!(ptr < 8);
    let mut s = S::init();
    if ptr != 0 {
        let served = ptr - 1;
        s = S::insert(s, served).1;
        let (p, s2) = S::dispatch(s);
        assert_eq!(p, Pick::Run(served));
        s = s2;
    }
    for t in 0..8u8 {
        if mask >> t & 1 == 1 {
            s = S::insert(s, t).1;
        }
    }
    s
}

/// The reference: the first ready thread scanning from `ptr`, and the next state.
fn model_dispatch(mask: u8, ptr: u8) -> (Pick, u8, u8) {
    for k in 0..8u8 {
        let t = (ptr + k) & 7;
        if mask >> t & 1 == 1 {
            return (Pick::Run(t), mask & !(1 << t), (t + 1) & 7);
        }
    }
    (Pick::Idle, mask, ptr)
}

#[test]
fn n_is_eight() {
    assert_eq!(S::N, 8);
}

#[test]
fn init_is_empty_and_idle() {
    assert_eq!(S::init(), state_of(0, 0));
    assert_eq!(S::dispatch(S::init()), (Pick::Idle, S::init()));
}

#[test]
fn rotation_is_fair_with_all_ready() {
    // All eight ready, pointer 0: eight dispatches serve 0..7 in order, then the set is empty.
    let mut s = state_of(0xFF, 0);
    for want in 0..8u8 {
        let (p, next) = S::dispatch(s);
        assert_eq!(p, Pick::Run(want));
        s = next;
    }
    assert_eq!(S::dispatch(s).0, Pick::Idle);
    assert_eq!(
        s,
        state_of(0, 0),
        "after serving thread 7 the pointer wrapped to 0"
    );
}

#[test]
fn rotation_starts_at_the_pointer() {
    for ptr in 0..8u8 {
        let mut s = state_of(0xFF, ptr);
        for k in 0..8u8 {
            let (p, next) = S::dispatch(s);
            assert_eq!(p, Pick::Run((ptr + k) & 7), "ptr {ptr}, dispatch {k}");
            s = next;
        }
    }
}

#[test]
fn a_served_thread_that_comes_back_waits_for_the_others() {
    // Threads 1, 3, 5 ready; serve 1; thread 1 becomes ready again at once.  The pointer is 2,
    // so 3 and 5 are served before 1 gets another turn.
    let mut s = state_of(0b0010_1010, 0);
    let mut order = Vec::new();
    for _ in 0..4 {
        let (p, next) = S::dispatch(s);
        let Pick::Run(t) = p else { panic!("idle") };
        order.push(t);
        s = next;
        if order.len() == 1 {
            s = S::insert(s, t).1;
        }
    }
    assert_eq!(order, vec![1, 3, 5, 1]);
}

#[test]
fn wraparound_picks_a_thread_below_the_pointer() {
    // Only thread 1 ready, pointer 6: the scan runs 6, 7, 0, 1.
    let (p, s2) = S::dispatch(state_of(0b0000_0010, 6));
    assert_eq!(p, Pick::Run(1));
    assert_eq!(s2, state_of(0, 2));
    // Pointer 7 and only thread 7 ready: picked at once, pointer wraps to 0.
    let (p, s2) = S::dispatch(state_of(0b1000_0000, 7));
    assert_eq!(p, Pick::Run(7));
    assert_eq!(s2, state_of(0, 0));
}

#[test]
fn dispatch_matches_the_reference_for_every_reachable_state() {
    for mask in 0..=255u8 {
        for ptr in 0..8u8 {
            let (p, s2) = S::dispatch(state_of(mask, ptr));
            let (wp, wmask, wptr) = model_dispatch(mask, ptr);
            assert_eq!(
                (p, s2),
                (wp, state_of(wmask, wptr)),
                "mask {mask:#010b} ptr {ptr}"
            );
        }
    }
}

#[test]
fn idle_leaves_the_state_alone() {
    for ptr in 0..8u8 {
        let s = state_of(0, ptr);
        assert_eq!(S::dispatch(s), (Pick::Idle, s));
    }
}

#[test]
fn insert_matches_the_reference_and_never_moves_the_pointer() {
    for mask in 0..=255u8 {
        for ptr in 0..8u8 {
            for t in 0..=255u8 {
                let (u, s2) = S::insert(state_of(mask, ptr), t);
                if t >= 8 {
                    assert_eq!((u, s2), (Upd::OutOfRange, state_of(mask, ptr)));
                } else if mask >> t & 1 == 1 {
                    assert_eq!((u, s2), (Upd::Unchanged, state_of(mask, ptr)));
                } else {
                    assert_eq!((u, s2), (Upd::Changed, state_of(mask | 1 << t, ptr)));
                }
            }
        }
    }
}

#[test]
fn remove_matches_the_reference_and_never_moves_the_pointer() {
    for mask in 0..=255u8 {
        for ptr in 0..8u8 {
            for t in 0..=255u8 {
                let (u, s2) = S::remove(state_of(mask, ptr), t);
                if t >= 8 {
                    assert_eq!((u, s2), (Upd::OutOfRange, state_of(mask, ptr)));
                } else if mask >> t & 1 == 0 {
                    assert_eq!((u, s2), (Upd::Unchanged, state_of(mask, ptr)));
                } else {
                    assert_eq!((u, s2), (Upd::Changed, state_of(mask & !(1 << t), ptr)));
                }
            }
        }
    }
}

/// A mutation sanity check (the claims of the evidence suite catch these as well): an `insert`
/// that reset the pointer would make this scenario serve thread 1 instead of thread 3, so the
/// reference comparison above and this scenario both fail for it.  Here the scenario is pinned
/// against the real `insert`, and the mutant is written out and shown to differ.
#[test]
fn an_insert_that_reset_the_pointer_would_be_caught() {
    let s = state_of(0b0000_1010, 2);
    let (_, real) = S::insert(s, 5);
    assert_eq!(S::dispatch(real).0, Pick::Run(3));
    // The mutant: the same insert, but the pointer goes back to 0.
    let (_, kept) = S::insert(state_of(0b0000_1010, 0), 5);
    assert_eq!(
        S::dispatch(kept).0,
        Pick::Run(1),
        "pointer 0 would serve thread 1"
    );
    assert_ne!(real, kept, "a reset pointer is a different state");
}

#[test]
fn draining_empties_the_set_in_at_most_eight_dispatches() {
    for mask in 0..=255u8 {
        for ptr in 0..8u8 {
            let mut s = state_of(mask, ptr);
            let mut served = 0u8;
            while let (Pick::Run(t), next) = S::dispatch(s) {
                assert_eq!(served >> t & 1, 0, "thread {t} served twice");
                served |= 1 << t;
                s = next;
            }
            assert_eq!(served, mask);
        }
    }
}

#[cfg(feature = "ghost")]
mod ghost_face {
    use super::*;
    use vos_sched_rr::ghost::{AbsW, CfgN, Image};
    use vos_sched_rr::*;

    #[test]
    fn the_wrappers_forward_to_the_production_operations() {
        assert_eq!(vos_ghost_init(), Image { mask: 0, ptr: 0 });
        assert_eq!(vos_ghost_cfg_n(), CfgN { v: 8 });
        for mask in 0..=255u8 {
            for ptr in 0..8u8 {
                let img = Image { mask, ptr };
                let s = state_of(mask, ptr);
                assert_eq!(vos_ghost_alpha_w(img), AbsW { w: mask });
                for t in [0u8, 3, 7, 8, 255] {
                    let (u, s2) = S::insert(s, t);
                    assert_eq!(vos_ghost_insert_out(img, t), u);
                    assert_eq!(image_of(s2), vos_ghost_insert_img(img, t));
                    let (u, s2) = S::remove(s, t);
                    assert_eq!(vos_ghost_remove_out(img, t), u);
                    assert_eq!(image_of(s2), vos_ghost_remove_img(img, t));
                }
                let (p, s2) = S::dispatch(s);
                assert_eq!(vos_ghost_dispatch_out(img), p);
                assert_eq!(image_of(s2), vos_ghost_dispatch_img(img));
            }
        }
    }

    /// The image of a reachable state, found by comparing with the states of `state_of`.
    fn image_of(s: Rr) -> Image {
        for mask in 0..=255u8 {
            for ptr in 0..8u8 {
                if state_of(mask, ptr) == s {
                    return Image { mask, ptr };
                }
            }
        }
        panic!("unreachable state");
    }

    /// A hostile image whose pointer is outside 0..8 is still answered totally: the pointer is
    /// reduced `& 7` before use, and an empty mask leaves the state alone.
    #[test]
    fn a_pointer_outside_the_range_is_reduced_not_trusted() {
        for ptr in 8..=255u8 {
            let img = Image {
                mask: 0b0001_0000,
                ptr,
            };
            assert_eq!(vos_ghost_dispatch_out(img), Pick::Run(4));
            assert_eq!(vos_ghost_dispatch_img(img), Image { mask: 0, ptr: 5 });
            let empty = Image { mask: 0, ptr };
            assert_eq!(vos_ghost_dispatch_out(empty), Pick::Idle);
            assert_eq!(vos_ghost_dispatch_img(empty), empty);
        }
    }
}
