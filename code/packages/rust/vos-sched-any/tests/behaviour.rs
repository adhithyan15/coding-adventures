//! Hand-written behaviour tests (the generated evidence suite is separate and needs the
//! `ghost` feature).
//!
//! Without the `ghost` feature a state is only ever made by `init` and the operations (its
//! fields are private), so the oracle is always 0 and the tests check the shipped surface:
//! always propose thread 0.  With `ghost` the oracle can be set, and the tests walk **all 256
//! masks times all 256 oracle values** against a plain reference written the obvious way,
//! for soundness (only what `safe_dispatch` permits) and coverage (everything it permits
//! occurs for some oracle value).

use vos_sched_any::{AnySched, State};
use vos_sched_contract::{Pick, Sched, Upd};

type S = AnySched;

/// The state whose ready set is exactly the bits of `mask`, oracle 0 (built by `insert`).
fn state_of(mask: u8) -> State {
    let mut s = S::init();
    for t in 0..8u8 {
        if mask >> t & 1 == 1 {
            let (u, next) = S::insert(s, t);
            assert_eq!(u, Upd::Changed);
            s = next;
        }
    }
    s
}

#[test]
fn n_is_eight() {
    assert_eq!(S::N, 8);
}

#[test]
fn init_is_empty_and_dispatches_idle() {
    assert_eq!(S::init(), state_of(0));
    assert_eq!(S::dispatch(S::init()), (Pick::Idle, S::init()));
}

#[test]
fn insert_and_remove_match_the_reference_for_every_state_and_id() {
    for mask in 0..=255u8 {
        for t in 0..=255u8 {
            let (u, s2) = S::insert(state_of(mask), t);
            if t >= 8 {
                assert_eq!((u, s2), (Upd::OutOfRange, state_of(mask)));
            } else if mask >> t & 1 == 1 {
                assert_eq!((u, s2), (Upd::Unchanged, state_of(mask)));
            } else {
                assert_eq!((u, s2), (Upd::Changed, state_of(mask | 1 << t)));
            }
            let (u, s2) = S::remove(state_of(mask), t);
            if t >= 8 {
                assert_eq!((u, s2), (Upd::OutOfRange, state_of(mask)));
            } else if mask >> t & 1 == 0 {
                assert_eq!((u, s2), (Upd::Unchanged, state_of(mask)));
            } else {
                assert_eq!((u, s2), (Upd::Changed, state_of(mask & !(1 << t))));
            }
        }
    }
}

#[test]
fn the_shipped_oracle_is_zero_so_dispatch_runs_thread_zero_when_ready() {
    for mask in 0..=255u8 {
        let (p, s2) = S::dispatch(state_of(mask));
        if mask & 1 == 1 {
            assert_eq!((p, s2), (Pick::Run(0), state_of(mask & !1)));
        } else {
            assert_eq!((p, s2), (Pick::Idle, state_of(mask)));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The soundness and coverage checks, as plain functions over a dispatch function, so they can be
// run on the real dispatch AND on deliberately broken ones (a mutation sanity check: a check
// that no mutant can fail is not a check).
// ---------------------------------------------------------------------------------------------

/// A dispatch under test: `(mask, oracle) -> (pick, new mask, new oracle)`.
type Dispatch = fn(u8, u8) -> (Pick, u8, u8);

/// What the safety tier permits for a mask: `Idle` and each ready thread (the contract's
/// `safe_dispatch`, written as plain Rust).
fn permitted(mask: u8) -> Vec<Pick> {
    let mut v = vec![Pick::Idle];
    v.extend((0..8u8).filter(|t| mask >> t & 1 == 1).map(Pick::Run));
    v
}

/// The mask after the permitted pick `p`: the chosen thread leaves.
fn after(mask: u8, p: Pick) -> u8 {
    match p {
        Pick::Idle => mask,
        Pick::Run(t) => mask & !(1 << t),
    }
}

/// Sound: for every mask and oracle value the output is permitted, the chosen thread (and only
/// it) leaves, and the oracle is not moved.
fn is_sound(d: Dispatch) -> bool {
    (0..=255u8).all(|mask| {
        (0..=255u8).all(|c| {
            let (p, m2, c2) = d(mask, c);
            permitted(mask).contains(&p) && m2 == after(mask, p) && c2 == c
        })
    })
}

/// Covering: every permitted output occurs for some oracle value (the witnesses of the
/// contract: oracle `t` for `Run(t)`, 255 for `Idle`).
fn is_covering(d: Dispatch) -> bool {
    (0..=255u8).all(|mask| {
        permitted(mask).into_iter().all(|want| {
            let witness = match want {
                Pick::Idle => 255,
                Pick::Run(t) => t,
            };
            d(mask, witness).0 == want
        })
    })
}

fn real(mask: u8, c: u8) -> (Pick, u8, u8) {
    // The reference, written the obvious way: propose `c`, validate, default to Idle.  (A state
    // whose oracle is not 0 can only be made by the ghost face, so the plain tests compare the
    // shipped dispatch against this only for oracle 0, and the ghost tests for every oracle.)
    if c < 8 && mask >> c & 1 == 1 {
        (Pick::Run(c), mask & !(1 << c), c)
    } else {
        (Pick::Idle, mask, c)
    }
}

/// Ignores the oracle: always proposes the top ready thread.
fn oracle_ignoring(mask: u8, c: u8) -> (Pick, u8, u8) {
    match (0..8u8).rev().find(|t| mask >> t & 1 == 1) {
        Some(t) => (Pick::Run(t), mask & !(1 << t), c),
        None => (Pick::Idle, mask, c),
    }
}

/// Skips validation: runs whatever thread the oracle names below 8, ready or not.
fn non_member(mask: u8, c: u8) -> (Pick, u8, u8) {
    if c < 8 {
        (Pick::Run(c), mask & !(1 << c), c)
    } else {
        (Pick::Idle, mask, c)
    }
}

/// Moves the oracle.
fn moves_the_oracle(mask: u8, c: u8) -> (Pick, u8, u8) {
    let (p, m, _) = real(mask, c);
    (p, m, c.wrapping_add(1))
}

#[test]
fn the_checks_accept_the_reference_dispatch_and_reject_each_mutant() {
    assert!(is_sound(real) && is_covering(real));
    // An oracle-ignoring dispatch is sound (it only ever picks a ready thread) but not covering.
    assert!(is_sound(oracle_ignoring));
    assert!(!is_covering(oracle_ignoring));
    // A dispatch that returns a non-member is not sound (it is covering: the witnesses still work).
    assert!(!is_sound(non_member));
    assert!(is_covering(non_member));
    assert!(!is_sound(moves_the_oracle));
}

#[cfg(feature = "ghost")]
mod ghost_face {
    use super::*;
    use vos_sched_any::ghost::{AbsW, CfgN, Image};
    use vos_sched_any::*;

    #[test]
    fn the_wrappers_forward_and_the_oracle_is_writable_only_here() {
        assert_eq!(vos_ghost_init(), Image { mask: 0, choice: 0 });
        assert_eq!(vos_ghost_cfg_n(), CfgN { v: 8 });
        for mask in [0u8, 1, 72, 255] {
            for choice in [0u8, 3, 7, 8, 255] {
                let img = Image { mask, choice };
                assert_eq!(vos_ghost_alpha_w(img), AbsW { w: mask });
                // alpha forgets the oracle: two images with the same mask are one abstract state.
                assert_eq!(
                    vos_ghost_alpha_w(img),
                    vos_ghost_alpha_w(Image { mask, choice: 1 })
                );
                for t in [0u8, 3, 7, 8, 255] {
                    let (u, _) = S::insert(state_of(mask), t);
                    assert_eq!(vos_ghost_insert_out(img, t), u);
                    let (u, _) = S::remove(state_of(mask), t);
                    assert_eq!(vos_ghost_remove_out(img, t), u);
                    // insert and remove never read or move the oracle.
                    assert_eq!(vos_ghost_insert_img(img, t).choice, choice);
                    assert_eq!(vos_ghost_remove_img(img, t).choice, choice);
                }
            }
        }
    }

    /// The production `dispatch`, driven through the ghost face (the only way to set the oracle).
    fn ghost_dispatch(mask: u8, choice: u8) -> (Pick, u8, u8) {
        let img = Image { mask, choice };
        let n = vos_ghost_dispatch_img(img);
        (vos_ghost_dispatch_out(img), n.mask, n.choice)
    }

    #[test]
    fn the_production_dispatch_is_sound_and_covering_for_every_mask_and_oracle() {
        assert!(is_sound(ghost_dispatch));
        assert!(is_covering(ghost_dispatch));
    }

    #[test]
    fn the_hand_written_reference_agrees_with_the_production_dispatch() {
        for mask in 0..=255u8 {
            for c in 0..=255u8 {
                assert_eq!(
                    ghost_dispatch(mask, c),
                    real(mask, c),
                    "mask {mask} oracle {c}"
                );
            }
        }
    }
}
