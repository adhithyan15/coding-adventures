//! # vos-sched-fp-bitmap: a fixed-priority scheduler in one byte
//!
//! The first real component of the verified-OS program (backlog VOS-011).  It
//! implements the scheduler contract of `vos-sched-contract` (VOS10) for eight
//! threads, ids `0..8`, where **a larger id is a higher priority**.
//!
//! ## The idea
//!
//! The ready set is one byte.  Bit `t` is set when thread `t` is ready:
//!
//! ```text
//!   bit:     7   6   5   4   3   2   1   0
//!   thread:  7   6   5   4   3   2   1   0
//!   mask:    0   1   0   0   1   0   0   0     threads 6 and 3 are ready
//! ```
//!
//! * `insert(t)`  sets bit `t`:        `mask | (1 << t)`
//! * `remove(t)`  clears bit `t`:      `mask & !(1 << t)`
//! * `dispatch`   finds the **highest** set bit, which is the highest-priority
//!   ready thread, and clears it (dispatch removes the thread it chooses, VOS10
//!   section 3.2).  The highest set bit of `0100_1000` is bit 6.
//!
//! "Highest set bit" is `7 - clz(mask)`, where `clz` counts the leading zero
//! bits.  For `0100_1000` there is one leading zero, so the answer is `7 - 1 = 6`.
//!
//! ## Why a hand-written leading-zero count
//!
//! `u8::leading_zeros` is one instruction on most machines, but on RV32I (the
//! image target, VOS01 section 5.3) there is no count-leading-zeros instruction
//! and the compiler may emit a call into a runtime routine that no contract
//! covers.  The routine below is three fixed steps, a binary search with no loop,
//! no branch and no division, so its cost is visible in the source and the step
//! bound of the contract (`steps <= 160`) is easy to believe.
//!
//! ## Shape (VOS01 section 1.4)
//!
//! The crate is `no_std`, has no `unsafe`, no static, no loop, no division and no
//! panic path.  Every operation is total: a thread id outside `0..8` is answered
//! `Upd::OutOfRange` and the state is returned unchanged.  Every shift amount is
//! passed through `wrapping_shl`, so it cannot overflow in a debug build and wrap
//! in a release build (SH9).
//!
//! ## The ghost face
//!
//! The evidence generator builds states from a record with **public** fields, but
//! the shipped `Bitmap` keeps its field private (VOS00 rule 2: only this crate
//! can make a state).  The `ghost` module, compiled only with the cargo feature
//! `ghost`, holds a public-field mirror (`Image`) and thin forwarding wrappers.
//! Each wrapper takes the image apart, makes the one UFCS call the spine would
//! make into the production code, and puts the result back together: the code the
//! evidence runs **is** the code that ships (VOS02 section 4).

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::arithmetic_side_effects)]

use vos_sched_contract::{Pick, Sched, Upd};

/// The scheduler state: the ready set as a bitmap.  The field is private, so the
/// only ways to make or change a `Bitmap` are the operations of [`FpBitmap`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    mask: u8,
}

/// The implementation marker: the type that carries the `Sched` impl.
pub struct FpBitmap;

// N is the width of the mask in bits.  A compile-time check, not a run-time one.
const _: () = assert!(FpBitmap::N == 8);

/// Leading zeros of a byte as a three-stage binary search.
///
/// Each stage asks "are the top `k` bits all zero?".  If so the answer gets `k`
/// larger and the byte is shifted left by `k`, bringing the next candidate bits
/// to the top.  Worked example, `x = 0b0000_1010`:
///
/// ```text
///   stage 1  top 4 bits zero?  yes   count 4   x << 4 = 1010_0000
///   stage 2  top 2 bits zero?  no    count 4   x       = 1010_0000
///   stage 3  top 1 bit  zero?  no    count 4   x       = 1010_0000
///   x is not zero, so no extra count            result 4
/// ```
///
/// The last term handles `x == 0`: all three stages fire (4 + 2 + 1 = 7) and
/// `x` is still zero, so the answer is 8, the full width.  The `as u8` casts turn
/// a `bool` into 0 or 1, so each stage is a comparison and a shift, never a branch.
///
/// Every value is at most 8 and the sums are `wrapping_add`, so nothing can
/// overflow (SH9); the shift amounts `e4 << 2`, `e2 << 1`, `e1` are 0, 4, 2, 1.
const fn clz8(x: u8) -> u8 {
    let e4 = (x >> 4 == 0) as u8;
    let x = x.wrapping_shl((e4 << 2) as u32);
    let e2 = (x >> 6 == 0) as u8;
    let x = x.wrapping_shl((e2 << 1) as u32);
    let e1 = (x >> 7 == 0) as u8;
    let x = x.wrapping_shl(e1 as u32);
    (e4 << 2)
        .wrapping_add(e2 << 1)
        .wrapping_add(e1)
        .wrapping_add((x == 0) as u8)
}

/// The mask with only bit `t` set, for `t < 8`.
///
/// `1 << t` for `t` in `0..8` is the powers of two `1, 2, 4, ... 128`.  The
/// callers have already answered `OutOfRange` for `t >= 8`, and `wrapping_shl`
/// would in any case reduce the amount modulo the width, never panic.
const fn bit_of(t: u8) -> u8 {
    1u8.wrapping_shl(t as u32)
}

impl Sched for FpBitmap {
    type State = Bitmap;

    const N: u8 = 8;

    /// No thread is ready: the empty set is the zero byte.
    fn init() -> Bitmap {
        Bitmap { mask: 0 }
    }

    /// Thread `t` becomes ready.  Already ready: `Unchanged`.  `t >= 8`:
    /// `OutOfRange`, state untouched.
    fn insert(s: Bitmap, t: u8) -> (Upd, Bitmap) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = bit_of(t);
        if s.mask & bit != 0 {
            (Upd::Unchanged, s)
        } else {
            (Upd::Changed, Bitmap { mask: s.mask | bit })
        }
    }

    /// Thread `t` stops being ready.  Not ready: `Unchanged`.  `t >= 8`:
    /// `OutOfRange`, state untouched.  `!bit` has every bit set except `t`, so
    /// the `&` clears exactly that one.
    fn remove(s: Bitmap, t: u8) -> (Upd, Bitmap) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = bit_of(t);
        if s.mask & bit == 0 {
            (Upd::Unchanged, s)
        } else {
            (
                Upd::Changed,
                Bitmap {
                    mask: s.mask & !bit,
                },
            )
        }
    }

    /// Choose the highest-priority ready thread and remove it; `Idle` when none
    /// is ready.  The index of the highest set bit of a non-zero byte is
    /// `7 - clz8(mask)`; clearing it is the same `& !bit` as in `remove`.
    fn dispatch(s: Bitmap) -> (Pick, Bitmap) {
        if s.mask == 0 {
            return (Pick::Idle, s);
        }
        let top = 7u8.wrapping_sub(clz8(s.mask));
        (
            Pick::Run(top),
            Bitmap {
                mask: s.mask & !bit_of(top),
            },
        )
    }
}

/// The evidence-only face of the crate (SH13, SH17, SH18; VOS02 section 4).
#[cfg(feature = "ghost")]
pub mod ghost {
    use super::*;

    /// The state with its field public: exactly the fields of [`Bitmap`], same
    /// names, types and order.  A field added to `Bitmap` and forgotten here
    /// breaks the build of every wrapper below (E0027 and E0063).
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Image {
        pub mask: u8,
    }

    /// The abstraction `alpha`, as a one-field record of an integer: bit `i` is
    /// "thread `i` is ready" (VOS02 section 4.4, interim vocabulary).
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
        let Bitmap { mask } = <FpBitmap as Sched>::init();
        Image { mask }
    }

    /// `insert`, first projection: the answer.
    pub fn vos_ghost_insert_out(s: Image, t: u8) -> Upd {
        let (o, Bitmap { mask: _ }) = <FpBitmap as Sched>::insert(Bitmap { mask: s.mask }, t);
        o
    }

    /// `insert`, second projection: the new state.
    pub fn vos_ghost_insert_img(s: Image, t: u8) -> Image {
        let (_o, Bitmap { mask }) = <FpBitmap as Sched>::insert(Bitmap { mask: s.mask }, t);
        Image { mask }
    }

    /// `remove`, first projection: the answer.
    pub fn vos_ghost_remove_out(s: Image, t: u8) -> Upd {
        let (o, Bitmap { mask: _ }) = <FpBitmap as Sched>::remove(Bitmap { mask: s.mask }, t);
        o
    }

    /// `remove`, second projection: the new state.
    pub fn vos_ghost_remove_img(s: Image, t: u8) -> Image {
        let (_o, Bitmap { mask }) = <FpBitmap as Sched>::remove(Bitmap { mask: s.mask }, t);
        Image { mask }
    }

    /// `dispatch`, first projection: the choice.
    pub fn vos_ghost_dispatch_out(s: Image) -> Pick {
        let (o, Bitmap { mask: _ }) = <FpBitmap as Sched>::dispatch(Bitmap { mask: s.mask });
        o
    }

    /// `dispatch`, second projection: the new state.
    pub fn vos_ghost_dispatch_img(s: Image) -> Image {
        let (_o, Bitmap { mask }) = <FpBitmap as Sched>::dispatch(Bitmap { mask: s.mask });
        Image { mask }
    }

    /// The abstraction: the mask itself, read as "bit i is thread i".
    pub fn vos_ghost_alpha_w(s: Image) -> AbsW {
        let Image { mask } = s;
        AbsW { w: mask }
    }

    /// The configuration constant, read from the trait.
    pub fn vos_ghost_cfg_n() -> CfgN {
        CfgN {
            v: <FpBitmap as Sched>::N,
        }
    }
}

#[cfg(feature = "ghost")]
pub use ghost::{
    vos_ghost_alpha_w, vos_ghost_cfg_n, vos_ghost_dispatch_img, vos_ghost_dispatch_out,
    vos_ghost_init, vos_ghost_insert_img, vos_ghost_insert_out, vos_ghost_remove_img,
    vos_ghost_remove_out,
};
