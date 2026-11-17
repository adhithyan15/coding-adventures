//! # vos-sched-any: the most-general scheduler, for evidence only
//!
//! Backlog VOS-012a.  This is the **`-any`** implementation of the scheduler contract
//! of `vos-sched-contract` (VOS10): the scheduler that does *every* thing the safety
//! tier allows, so that a proof about the safety tier is a proof about all of them
//! (VOS01 section 3).
//!
//! ## READ THIS FIRST: evidence only, never shippable (SH16)
//!
//! A real scheduler is one particular, deterministic choice among the many the safety
//! tier permits.  This crate is *all* of the choices at once, selected by a hidden
//! register (the "oracle").  It exists so the verification can ask: "for **every**
//! oracle value, is the answer one the contract permits (sound), and does **every**
//! permitted answer occur for some oracle value (covering)?"  It is evidence that the
//! safety tier is neither too tight (no real scheduler fits) nor too loose; it is not
//! a scheduler to boot.  The name ends in `-any`, `vos.toml` says `kind = "any"`, and
//! no image may contain it.
//!
//! ## The idea
//!
//! The state is the ready set (one byte, bit `t` set when thread `t` is ready, as in
//! `vos-sched-fp-bitmap`) **plus** one more byte, `choice`: the oracle.
//!
//! ```text
//!   state = ( mask , choice )
//!
//!   insert(t)   mask | (1 << t)       choice untouched   (never reads the oracle)
//!   remove(t)   mask & !(1 << t)      choice untouched   (never reads the oracle)
//!   dispatch    PROPOSE  the thread the oracle names:      t = choice
//!               VALIDATE it:        t < 8 and bit t of mask is set
//!               DEFAULT to Idle:    otherwise
//!               and, when it ran thread t, clear bit t.    choice untouched
//! ```
//!
//! Worked example, `mask = 0b0100_1000` (threads 6 and 3 ready):
//!
//! ```text
//!   choice = 6    -> Run(6), mask becomes 0b0000_1000     (a ready thread: allowed)
//!   choice = 3    -> Run(3), mask becomes 0b0100_0000     (any ready thread, not just the top)
//!   choice = 5    -> Idle,   mask unchanged               (5 is not ready: validation fails)
//!   choice = 255  -> Idle,   mask unchanged               (255 is the "Idle" witness)
//! ```
//!
//! ## Sound and covering
//!
//! * **Sound**: `dispatch` answers `Run(t)` only for a ready `t < 8` and clears exactly
//!   that bit, or answers `Idle` and changes nothing.  That is `safe_dispatch`, the
//!   contract's relation.
//! * **Covering**: for every ready `t` the oracle value `t` produces `Run(t)`, and the
//!   oracle value 255 produces `Idle` whatever the mask.  So every output the relation
//!   permits is reachable.
//! * The oracle is **not** moved by any operation, so a claim about one dispatch is a
//!   claim about the oracle's value at that moment (`dispatch_img(s).choice == s.choice`).
//!
//! ## Where the oracle can be written
//!
//! In the shipped API the `choice` field is private and only `init` makes a state, with
//! `choice = 0`: the production surface of this crate is a scheduler that always
//! proposes thread 0.  The *only* code that can build a state with any other oracle
//! value is the [`ghost`] module (cargo feature `ghost`, the evidence build): its
//! wrappers take a public-field `Image` apart and put a `State` together, which is how
//! the harness quantifies over every oracle value (VOS02 section 4).
//!
//! ## Shape (VOS01 section 1.4)
//!
//! `no_std`, no `unsafe`, no static, no loop, no division and no panic path; every
//! shift goes through `wrapping_shl` (SH9) and `dispatch` validates `choice < 8` first.

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::arithmetic_side_effects)]

use vos_sched_contract::{Pick, Sched, Upd};

/// The scheduler state: the ready set and the oracle register.  Both fields are
/// private, so the only ways to make or change a `State` outside the evidence build
/// are the operations of [`AnySched`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    /// Bit `t` is set when thread `t` is ready.
    mask: u8,
    /// The oracle: the thread `dispatch` proposes; `255` (or any non-thread) proposes
    /// `Idle`.  Abstraction `alpha` forgets it.
    choice: u8,
}

/// The implementation marker: the type that carries the `Sched` impl.
pub struct AnySched;

// N is the width of the mask in bits.  A compile-time check, not a run-time one.
const _: () = assert!(AnySched::N == 8);

/// The mask with only bit `t` set, for `t < 8`.
///
/// `1 << t` for `t` in `0..8` is `1, 2, 4, ... 128`.  Callers have already ruled out
/// `t >= 8`, and `wrapping_shl` would in any case reduce the amount modulo the width
/// and never panic.
const fn bit_of(t: u8) -> u8 {
    1u8.wrapping_shl(t as u32)
}

impl Sched for AnySched {
    type State = State;

    const N: u8 = 8;

    /// No thread is ready, and the oracle starts at 0.
    fn init() -> State {
        State { mask: 0, choice: 0 }
    }

    /// Thread `t` becomes ready.  Already ready: `Unchanged`.  `t >= 8`:
    /// `OutOfRange`, state untouched.  The oracle is carried over by `..s`: this
    /// operation never reads it.
    fn insert(s: State, t: u8) -> (Upd, State) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = bit_of(t);
        if s.mask & bit != 0 {
            (Upd::Unchanged, s)
        } else {
            (
                Upd::Changed,
                State {
                    mask: s.mask | bit,
                    ..s
                },
            )
        }
    }

    /// Thread `t` stops being ready.  Not ready: `Unchanged`.  `t >= 8`:
    /// `OutOfRange`, state untouched.  `!bit` has every bit set except `t`.
    fn remove(s: State, t: u8) -> (Upd, State) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = bit_of(t);
        if s.mask & bit == 0 {
            (Upd::Unchanged, s)
        } else {
            (
                Upd::Changed,
                State {
                    mask: s.mask & !bit,
                    ..s
                },
            )
        }
    }

    /// Propose the oracle's thread, validate it, default to `Idle`; running a thread
    /// removes it from the ready set.  The oracle is unchanged.
    fn dispatch(s: State) -> (Pick, State) {
        let t = s.choice;
        if t < Self::N && s.mask & bit_of(t) != 0 {
            (
                Pick::Run(t),
                State {
                    mask: s.mask & !bit_of(t),
                    ..s
                },
            )
        } else {
            (Pick::Idle, s)
        }
    }
}

/// The evidence-only face of the crate (SH13, SH17, SH18; VOS02 section 4).
///
/// This module, and only this module, can write the oracle: [`Image`] has a public
/// `choice`, and the wrappers build a production [`State`] from it.
#[cfg(feature = "ghost")]
pub mod ghost {
    use super::*;

    /// The state with its fields public: exactly the fields of [`State`], same names,
    /// types and order.  A field added to `State` and forgotten here breaks the build
    /// of every wrapper below (E0027 and E0063).
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Image {
        pub mask: u8,
        pub choice: u8,
    }

    /// The abstraction `alpha`, as a one-field record of an integer: bit `i` is
    /// "thread `i` is ready" (VOS02 section 4.4).  It drops the oracle.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct AbsW {
        pub w: u8,
    }

    /// The configuration constant `N`, as a one-field record of an integer.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct CfgN {
        pub v: u8,
    }

    /// `init`, as an image.
    pub fn vos_ghost_init() -> Image {
        let State { mask, choice } = <AnySched as Sched>::init();
        Image { mask, choice }
    }

    /// `insert`, first projection: the answer.
    pub fn vos_ghost_insert_out(s: Image, t: u8) -> Upd {
        let (o, State { mask: _, choice: _ }) = <AnySched as Sched>::insert(
            State {
                mask: s.mask,
                choice: s.choice,
            },
            t,
        );
        o
    }

    /// `insert`, second projection: the new state.
    pub fn vos_ghost_insert_img(s: Image, t: u8) -> Image {
        let (_o, State { mask, choice }) = <AnySched as Sched>::insert(
            State {
                mask: s.mask,
                choice: s.choice,
            },
            t,
        );
        Image { mask, choice }
    }

    /// `remove`, first projection: the answer.
    pub fn vos_ghost_remove_out(s: Image, t: u8) -> Upd {
        let (o, State { mask: _, choice: _ }) = <AnySched as Sched>::remove(
            State {
                mask: s.mask,
                choice: s.choice,
            },
            t,
        );
        o
    }

    /// `remove`, second projection: the new state.
    pub fn vos_ghost_remove_img(s: Image, t: u8) -> Image {
        let (_o, State { mask, choice }) = <AnySched as Sched>::remove(
            State {
                mask: s.mask,
                choice: s.choice,
            },
            t,
        );
        Image { mask, choice }
    }

    /// `dispatch`, first projection: the choice.
    pub fn vos_ghost_dispatch_out(s: Image) -> Pick {
        let (o, State { mask: _, choice: _ }) = <AnySched as Sched>::dispatch(State {
            mask: s.mask,
            choice: s.choice,
        });
        o
    }

    /// `dispatch`, second projection: the new state.
    pub fn vos_ghost_dispatch_img(s: Image) -> Image {
        let (_o, State { mask, choice }) = <AnySched as Sched>::dispatch(State {
            mask: s.mask,
            choice: s.choice,
        });
        Image { mask, choice }
    }

    /// The abstraction: the mask read as "bit i is thread i"; the oracle is dropped.
    pub fn vos_ghost_alpha_w(s: Image) -> AbsW {
        let Image { mask, choice: _ } = s;
        AbsW { w: mask }
    }

    /// The configuration constant, read from the trait.
    pub fn vos_ghost_cfg_n() -> CfgN {
        CfgN {
            v: <AnySched as Sched>::N,
        }
    }
}

#[cfg(feature = "ghost")]
pub use ghost::{
    vos_ghost_alpha_w, vos_ghost_cfg_n, vos_ghost_dispatch_img, vos_ghost_dispatch_out,
    vos_ghost_init, vos_ghost_insert_img, vos_ghost_insert_out, vos_ghost_remove_img,
    vos_ghost_remove_out,
};
