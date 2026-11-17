//! # vos-sched-rr: a round-robin scheduler in two bytes
//!
//! The second real scheduler of the verified-OS program (backlog VOS-012b), and the first
//! **swap** (VOS00 section 8): it implements the same contract as `vos-sched-fp-bitmap`
//! (`vos-sched-contract`, VOS10) for eight threads, ids `0..8`, but with a different policy.
//! The bitmap scheduler always runs the highest-numbered ready thread; this one takes turns.
//!
//! ## The idea
//!
//! The state is two bytes.  `mask` is the ready set exactly as in the bitmap scheduler (bit `t`
//! set when thread `t` is ready); `ptr` is the **rotation pointer**, the thread id where the next
//! scan starts:
//!
//! ```text
//!   mask = 0100_1010  threads 6, 3 and 1 are ready
//!   ptr  = 2          the next scan starts at thread 2
//!
//!   scan order:  2  3  4  5  6  7  0  1        (wrapping at 8)
//!   ready:          3        6        .  1
//!   first ready thread in scan order: 3
//! ```
//!
//! * `insert(t)` sets bit `t`, `remove(t)` clears it.  Neither reads or moves `ptr`.
//! * `dispatch` scans from `ptr`, picks the first ready thread `t`, **clears its bit** (dispatch
//!   removes the thread it chooses, VOS10 section 3.2) and moves the pointer to `(t + 1) & 7`,
//!   one past the thread just served.  So the thread that just ran is the *last* one the next
//!   scan reaches: nobody is served twice while another is waiting.
//!
//! ## What the contract can and cannot see
//!
//! The abstraction `alpha` forgets `ptr`: two states with the same mask but different pointers are
//! the same abstract ready set.  That is why this crate carries the **representation** tier
//! (VOS02 section 2.7) and not the canonical one: the reachable states are the image of
//! `rep(m, h) = Impl(<mask of m>, h)` over every ready set `m` and every history code `h = ptr`.
//! It carries no `policy` tier either: round-robin is not highest-priority-first, so a profile
//! that needs that guarantee cannot use this scheduler.
//!
//! ## The loop-free scan
//!
//! The scan over eight slots is three steps and no loop (VOS03 SH1):
//!
//! 1. **Rotate** the mask right by `p` so the pointer's thread becomes bit 0 and the scan order
//!    becomes bit order: `r = (mask >> p) | (mask << (8 - p))`.  When `p = 0` the left shift
//!    is by 8, which `wrapping_shl` reduces to 0, giving `mask | mask = mask`: still right.
//! 2. **Lowest set bit** of `r` is the *offset* of the first ready thread from the pointer.  The
//!    lowest set bit is isolated with `r & -r` (`isolate_lowest_one`) and located with the same three-stage `clz8` the
//!    bitmap scheduler uses: `offset = 7 - clz8(r & -r)`.
//! 3. The thread is `t = (p + offset) & 7`.
//!
//! Worked example, `mask = 0100_1010`, `p = 2`:
//!
//! ```text
//!   mask >> 2          = 0001_0010
//!   mask << 6 (wrap)   = 1000_0000      (only bit 1 of the mask survives the shift, to bit 7)
//!   r                  = 1001_0010      bit 0 is thread 2, bit 1 is thread 3, ...
//!   r & -r             = 0000_0010      lowest set bit is bit 1
//!   offset             = 7 - clz8(0000_0010) = 7 - 6 = 1
//!   t                  = (2 + 1) & 7 = 3
//! ```
//!
//! ## Shape (VOS01 section 1.4)
//!
//! `no_std`, no `unsafe`, no static, no loop, no division, no panic path.  Every operation is
//! total: a thread id outside `0..8` is answered `Upd::OutOfRange` with the state returned
//! unchanged, an empty mask is answered `Pick::Idle` with the state unchanged, and a pointer
//! outside `0..8` (not reachable from `init`, but a hostile state may carry one) is reduced
//! `& 7` before use.  Every shift amount is passed through `wrapping_shl`/`wrapping_shr`, so it
//! cannot overflow in a debug build and wrap in a release build (SH9).
//!
//! ## The ghost face
//!
//! As in the bitmap scheduler: the evidence generator builds states from a record with public
//! fields, so the `ghost` module (compiled only with the cargo feature `ghost`) holds a
//! public-field mirror `Image` and forwarding wrappers.  The code the evidence runs is the code
//! that ships (VOS02 section 4).

#![no_std]
#![forbid(unsafe_code)]
#![deny(clippy::arithmetic_side_effects)]

use vos_sched_contract::{Pick, Sched, Upd};

/// The scheduler state: the ready set and the rotation pointer.  The fields are private, so the
/// only ways to make or change an `Rr` are the operations of [`RoundRobin`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rr {
    mask: u8,
    ptr: u8,
}

/// The implementation marker: the type that carries the `Sched` impl.
pub struct RoundRobin;

// N is the width of the mask in bits.  A compile-time check, not a run-time one.
const _: () = assert!(RoundRobin::N == 8);

/// Leading zeros of a byte as a three-stage binary search (the same routine as in
/// `vos-sched-fp-bitmap`; see there for the stage-by-stage example).
///
/// Each stage asks "are the top `k` bits all zero?"; if so the count grows by `k` and the byte
/// is shifted left by `k`.  `x == 0` gives 8.  Every value is at most 8 and the sums are
/// `wrapping_add`, so nothing can overflow (SH9).
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

/// The mask with only bit `t` set, for `t < 8` (callers answer `OutOfRange` first; for any
/// other `t`, `wrapping_shl` reduces the amount modulo the width and never panics).
const fn bit_of(t: u8) -> u8 {
    1u8.wrapping_shl(t as u32)
}

/// The mask rotated right by `p`, `p` in `0..8`: bit `i` of the result is bit `(p + i) & 7`
/// of the mask.  For `p = 0` the left shift amount is `8`, reduced to `0` by `wrapping_shl`,
/// and `mask | mask = mask`.
const fn rotr8(mask: u8, p: u8) -> u8 {
    mask.wrapping_shr(p as u32) | mask.wrapping_shl(8u8.wrapping_sub(p) as u32)
}

impl Sched for RoundRobin {
    type State = Rr;

    const N: u8 = 8;

    /// No thread is ready, and the scan starts at thread 0.
    fn init() -> Rr {
        Rr { mask: 0, ptr: 0 }
    }

    /// Thread `t` becomes ready.  Already ready: `Unchanged`.  `t >= 8`: `OutOfRange`, state
    /// untouched.  The pointer is never moved.
    fn insert(s: Rr, t: u8) -> (Upd, Rr) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = bit_of(t);
        if s.mask & bit != 0 {
            (Upd::Unchanged, s)
        } else {
            (
                Upd::Changed,
                Rr {
                    mask: s.mask | bit,
                    ptr: s.ptr,
                },
            )
        }
    }

    /// Thread `t` stops being ready.  Not ready: `Unchanged`.  `t >= 8`: `OutOfRange`, state
    /// untouched.  The pointer is never moved.
    fn remove(s: Rr, t: u8) -> (Upd, Rr) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = bit_of(t);
        if s.mask & bit == 0 {
            (Upd::Unchanged, s)
        } else {
            (
                Upd::Changed,
                Rr {
                    mask: s.mask & !bit,
                    ptr: s.ptr,
                },
            )
        }
    }

    /// Choose the first ready thread at or after the pointer (wrapping at 8), remove it and
    /// move the pointer one past it; `Idle` and no change when none is ready.
    fn dispatch(s: Rr) -> (Pick, Rr) {
        if s.mask == 0 {
            return (Pick::Idle, s);
        }
        let p = s.ptr & 7;
        let r = rotr8(s.mask, p);
        // `r != 0` because the mask is non-zero and a rotation keeps its bits.
        // `isolate_lowest_one` is `r & -r`: only the lowest set bit survives, and its index
        // is `7 - clz8`.
        let low = r.isolate_lowest_one();
        let offset = 7u8.wrapping_sub(clz8(low));
        let t = p.wrapping_add(offset) & 7;
        (
            Pick::Run(t),
            Rr {
                mask: s.mask & !bit_of(t),
                ptr: t.wrapping_add(1) & 7,
            },
        )
    }
}

/// The evidence-only face of the crate (SH13, SH17, SH18; VOS02 section 4).
#[cfg(feature = "ghost")]
pub mod ghost {
    use super::*;

    /// The state with its fields public: exactly the fields of [`Rr`], same names, types and
    /// order.  A field added to `Rr` and forgotten here breaks the build of every wrapper below.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Image {
        pub mask: u8,
        pub ptr: u8,
    }

    /// The abstraction `alpha`, as a one-field record of an integer: bit `i` is "thread `i`
    /// is ready".  The pointer is forgotten (VOS02 section 2.7).
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
        let Rr { mask, ptr } = <RoundRobin as Sched>::init();
        Image { mask, ptr }
    }

    /// `insert`, first projection: the answer.
    pub fn vos_ghost_insert_out(s: Image, t: u8) -> Upd {
        let (o, Rr { mask: _, ptr: _ }) = <RoundRobin as Sched>::insert(
            Rr {
                mask: s.mask,
                ptr: s.ptr,
            },
            t,
        );
        o
    }

    /// `insert`, second projection: the new state.
    pub fn vos_ghost_insert_img(s: Image, t: u8) -> Image {
        let (_o, Rr { mask, ptr }) = <RoundRobin as Sched>::insert(
            Rr {
                mask: s.mask,
                ptr: s.ptr,
            },
            t,
        );
        Image { mask, ptr }
    }

    /// `remove`, first projection: the answer.
    pub fn vos_ghost_remove_out(s: Image, t: u8) -> Upd {
        let (o, Rr { mask: _, ptr: _ }) = <RoundRobin as Sched>::remove(
            Rr {
                mask: s.mask,
                ptr: s.ptr,
            },
            t,
        );
        o
    }

    /// `remove`, second projection: the new state.
    pub fn vos_ghost_remove_img(s: Image, t: u8) -> Image {
        let (_o, Rr { mask, ptr }) = <RoundRobin as Sched>::remove(
            Rr {
                mask: s.mask,
                ptr: s.ptr,
            },
            t,
        );
        Image { mask, ptr }
    }

    /// `dispatch`, first projection: the choice.
    pub fn vos_ghost_dispatch_out(s: Image) -> Pick {
        let (o, Rr { mask: _, ptr: _ }) = <RoundRobin as Sched>::dispatch(Rr {
            mask: s.mask,
            ptr: s.ptr,
        });
        o
    }

    /// `dispatch`, second projection: the new state.
    pub fn vos_ghost_dispatch_img(s: Image) -> Image {
        let (_o, Rr { mask, ptr }) = <RoundRobin as Sched>::dispatch(Rr {
            mask: s.mask,
            ptr: s.ptr,
        });
        Image { mask, ptr }
    }

    /// The abstraction: the mask, read as "bit i is thread i"; the pointer is dropped.
    pub fn vos_ghost_alpha_w(s: Image) -> AbsW {
        let Image { mask, ptr: _ } = s;
        AbsW { w: mask }
    }

    /// The configuration constant, read from the trait.
    pub fn vos_ghost_cfg_n() -> CfgN {
        CfgN {
            v: <RoundRobin as Sched>::N,
        }
    }
}

#[cfg(feature = "ghost")]
pub use ghost::{
    vos_ghost_alpha_w, vos_ghost_cfg_n, vos_ghost_dispatch_img, vos_ghost_dispatch_out,
    vos_ghost_init, vos_ghost_insert_img, vos_ghost_insert_out, vos_ghost_remove_img,
    vos_ghost_remove_out,
};
