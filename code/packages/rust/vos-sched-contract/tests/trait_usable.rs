//! The trait is implementable, and the contract's meaning holds for a plain Rust
//! scheduler (VOS-010).
//!
//! `vos-sched-contract` holds declarations only, so there is no function of its
//! own to cover. What can still go wrong is the declaration itself: a signature
//! nobody can implement, an associated constant that cannot be used as an array
//! bound, an enum whose derives are missing. This file implements the trait with
//! a three-line bitmap and a deliberately different one-cell-per-thread scan, and
//! checks, over **every** state and thread id at `N = 8`, the same safety and
//! policy statements that the Warrant lemmas and the evidence of the three schedulers state. (The Warrant-model
//! scan of the retired `vos-sched-contract-check` is gone; the 247 below is the figure it measured.) It is
//! a Rust-level cross-check of the claim text's meaning, not evidence about any
//! shipped scheduler (that is VOS-011).

use vos_sched_contract::{Pick, Sched, Upd};

/// Highest-priority-first, one byte of state.
struct Bitmap;

impl Sched for Bitmap {
    type State = u8;
    const N: u8 = 8;

    fn init() -> u8 {
        0
    }
    fn insert(s: u8, t: u8) -> (Upd, u8) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = 1u8 << t;
        if s & bit != 0 {
            (Upd::Unchanged, s)
        } else {
            (Upd::Changed, s | bit)
        }
    }
    fn remove(s: u8, t: u8) -> (Upd, u8) {
        if t >= Self::N {
            return (Upd::OutOfRange, s);
        }
        let bit = 1u8 << t;
        if s & bit == 0 {
            (Upd::Unchanged, s)
        } else {
            (Upd::Changed, s & !bit)
        }
    }
    fn dispatch(s: u8) -> (Pick, u8) {
        if s == 0 {
            (Pick::Idle, s)
        } else {
            let top = 7 - s.leading_zeros() as u8;
            (Pick::Run(top), s & !(1u8 << top))
        }
    }
}

/// Lowest-priority-first, one cell per thread: the safety tier only.
struct Scan;

impl Sched for Scan {
    type State = [bool; 8];
    const N: u8 = 8;

    fn init() -> [bool; 8] {
        [false; 8]
    }
    fn insert(mut s: [bool; 8], t: u8) -> (Upd, [bool; 8]) {
        match s.get_mut(usize::from(t)) {
            None => (Upd::OutOfRange, s),
            Some(cell) if *cell => (Upd::Unchanged, s),
            Some(cell) => {
                *cell = true;
                (Upd::Changed, s)
            }
        }
    }
    fn remove(mut s: [bool; 8], t: u8) -> (Upd, [bool; 8]) {
        match s.get_mut(usize::from(t)) {
            None => (Upd::OutOfRange, s),
            Some(cell) if !*cell => (Upd::Unchanged, s),
            Some(cell) => {
                *cell = false;
                (Upd::Changed, s)
            }
        }
    }
    fn dispatch(mut s: [bool; 8]) -> (Pick, [bool; 8]) {
        match s.iter().position(|&ready| ready) {
            None => (Pick::Idle, s),
            Some(i) => {
                s[i] = false;
                (Pick::Run(i as u8), s)
            }
        }
    }
}

/// The abstract state, as a mask, for each implementation.
trait Abs: Sched {
    fn mask(s: &Self::State) -> u8;
    fn build(mask: u8) -> Self::State;
}
impl Abs for Bitmap {
    fn mask(s: &u8) -> u8 {
        *s
    }
    fn build(mask: u8) -> u8 {
        mask
    }
}
impl Abs for Scan {
    fn mask(s: &[bool; 8]) -> u8 {
        s.iter()
            .enumerate()
            .fold(0, |m, (i, &r)| if r { m | (1 << i) } else { m })
    }
    fn build(mask: u8) -> [bool; 8] {
        core::array::from_fn(|i| mask & (1 << i) != 0)
    }
}

/// `safe_dispatch` of VOS10 section 5.1, over masks.
fn safe_dispatch(m: u8, o: Pick, m2: u8) -> bool {
    match o {
        Pick::Idle => m2 == m,
        Pick::Run(t) => t < 8 && m & (1 << t) != 0 && m2 == m & !(1 << t),
    }
}

/// `policy_dispatch`: the maximum member, `Idle` only for the empty set.
fn policy_dispatch(m: u8, o: Pick, m2: u8) -> bool {
    let top = if m == 0 {
        Pick::Idle
    } else {
        Pick::Run(7 - m.leading_zeros() as u8)
    };
    o == top && safe_dispatch(m, o, m2)
}

/// The abstract `insert` and `remove` of the contract block.
fn abs_insert(m: u8, t: u8) -> (Upd, u8) {
    if t >= 8 {
        (Upd::OutOfRange, m)
    } else if m & (1 << t) != 0 {
        (Upd::Unchanged, m)
    } else {
        (Upd::Changed, m | (1 << t))
    }
}
fn abs_remove(m: u8, t: u8) -> (Upd, u8) {
    if t >= 8 {
        (Upd::OutOfRange, m)
    } else if m & (1 << t) != 0 {
        (Upd::Changed, m & !(1 << t))
    } else {
        (Upd::Unchanged, m)
    }
}

/// The safety obligations over every abstract state and every id (65,536 cases).
fn check_safety<S: Abs>() {
    assert_eq!(S::N, 8);
    assert_eq!(S::mask(&S::init()), 0);
    for mask in 0..=255u8 {
        for t in 0..=255u8 {
            let (u, s) = S::insert(S::build(mask), t);
            assert_eq!((u, S::mask(&s)), abs_insert(mask, t));
            let (u, s) = S::remove(S::build(mask), t);
            assert_eq!((u, S::mask(&s)), abs_remove(mask, t));
        }
        let (o, s) = S::dispatch(S::build(mask));
        assert!(safe_dispatch(mask, o, S::mask(&s)), "mask {mask:#010b}");
    }
}

#[test]
fn the_bitmap_carries_safety_and_policy() {
    check_safety::<Bitmap>();
    for mask in 0..=255u8 {
        let (o, s) = Bitmap::dispatch(mask);
        assert!(policy_dispatch(mask, o, s), "mask {mask:#010b}");
    }
}

#[test]
fn the_scan_carries_safety_and_fails_policy_on_exactly_247_states() {
    check_safety::<Scan>();
    let failing = (0..=255u8)
        .filter(|&mask| {
            let (o, s) = Scan::dispatch(Scan::build(mask));
            !policy_dispatch(mask, o, Scan::mask(&s))
        })
        .count();
    // 256 states, minus the empty set and the 8 singletons, where lowest and highest agree
    assert_eq!(failing, 247);
}

#[test]
fn the_enums_are_plain_copyable_values() {
    let u = Upd::Changed;
    let copy = u;
    assert_eq!(u, copy);
    assert_ne!(Upd::Unchanged, Upd::OutOfRange);
    let p = Pick::Run(3);
    assert_eq!(p, p);
    assert_ne!(Pick::Idle, Pick::Run(0));
    assert_eq!(format!("{p:?} {u:?}"), "Run(3) Changed");
}

#[test]
fn out_of_range_ids_change_nothing_and_dispatch_never_panics() {
    for t in 8..=255u8 {
        assert_eq!(Bitmap::insert(0b1010, t), (Upd::OutOfRange, 0b1010));
        assert_eq!(Bitmap::remove(0b1010, t), (Upd::OutOfRange, 0b1010));
    }
    let mut s = Bitmap::init();
    for t in 0..8 {
        s = Bitmap::insert(s, t).1;
    }
    // draining returns every thread once, highest first, then Idle forever
    for expect in (0..8).rev() {
        let (o, next) = Bitmap::dispatch(s);
        assert_eq!(o, Pick::Run(expect));
        s = next;
    }
    assert_eq!(Bitmap::dispatch(s), (Pick::Idle, 0));
}
