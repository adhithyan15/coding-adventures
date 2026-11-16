//! # `vos-sched-contract`: the scheduler interface (VOS-010, spec VOS10)
//!
//! This crate is a *contract package* (VOS02 section 2.2): it owns the Rust
//! trait every scheduler implements, and (beside this file, under `warrant/`)
//! the Warrant text that says what "implements" means. It contains no
//! scheduler. The scheduler that gets shipped is `vos-sched-fp-bitmap`
//! (VOS-011); the one that stands for *every* safe scheduler is
//! `vos-sched-any` (VOS-012).
//!
//! ## What a scheduler is, in one paragraph
//!
//! A scheduler remembers **which threads are ready to run** and, when asked,
//! chooses one of them. That is all. It does not know which thread is running
//! (the spine owns that), how long anybody has run (a later time contract
//! owns that), or what a thread *is* (the thread table owns that). Thread
//! number `t` has priority `t`: the thread table hands out ids in priority
//! order, so "unique static priorities" costs the scheduler nothing and a
//! higher id always wins. The abstract state is therefore a set of ids below
//! `N`; with `N = 8` it is eight booleans, small enough that every state of
//! every claim can be enumerated.
//!
//! ## The four operations
//!
//! Every operation has the by-value shape of VOS01 section 1.1,
//! `op(State, In) -> (Out, State)`: it is total (every input has a defined
//! answer), reports errors as values, never panics, and returns the new state
//! instead of mutating the old one. A `State` is therefore *moved* into each
//! call, so a stale copy is a compile error.
//!
//! ```text
//!   init()                      -> State              the empty ready set
//!   insert(s, t)   -> (Upd,  State)   thread t becomes ready
//!   remove(s, t)   -> (Upd,  State)   thread t stops being ready (blocked, exited, suspended)
//!   dispatch(s)    -> (Pick, State)   choose a ready thread AND take it out of the set
//! ```
//!
//! `dispatch` removes the thread it returns. That is the point of the
//! operation: the chosen thread is about to *run*, and a running thread is not
//! ready, so the scheduler and the spine agree by construction (the toy of
//! VOS01 section 5 needed a second call and a caller-side precondition to
//! say the same thing). To preempt or yield, the spine calls `insert` for the
//! running thread and then `dispatch`; there is no separate `yield` because
//! with unique priorities a yield *is* those two calls.
//!
//! ## What is promised, in two tiers (VOS01 section 2.4)
//!
//! * **Safety tier**, which every profile and every system theorem may use:
//!   `dispatch` returns `Idle` or a thread that was ready, and the new set is
//!   the old set minus exactly that thread. `Idle` is always allowed.
//! * **Policy tier**, which a profile may additionally *require*:
//!   `dispatch` returns the **highest-priority** ready thread, and `Idle` only
//!   for the empty set (a fixed-priority scheduler).
//!
//! A round-robin or earliest-deadline-first scheduler satisfies the safety tier
//! only. The exact claims live in `warrant/*.part`; `specs/VOS10` explains them.
//!
//! ## Shape rules
//!
//! The crate is class K, kind `contract` (`vos.toml`): `no_std`, no `unsafe`,
//! no dependency. It holds declarations only, so there is nothing here to
//! panic, loop or divide.

#![no_std]
#![forbid(unsafe_code)]

/// The outcome of `insert` and `remove`.
///
/// ```text
///   Changed      the ready set is different afterwards
///   Unchanged    it was already as asked (inserting a member, removing a non-member)
///   OutOfRange   the thread id is >= N; the state is returned exactly as given
/// ```
///
/// Because `Unchanged` is an ordinary answer, a caller may `remove` a thread
/// without knowing whether it is currently ready (a running thread is not).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Upd {
    /// The set changed.
    Changed,
    /// The set already had the requested membership.
    Unchanged,
    /// `t >= N`: the request named no thread; nothing changed.
    OutOfRange,
}

/// The outcome of `dispatch`.
///
/// ```text
///   Idle         no thread was chosen; the state is returned unchanged
///   Run(t)       thread t was ready; it is no longer in the set
/// ```
///
/// `Idle` is a first-class answer, not an error: an empty ready set must
/// return it, and the safety tier lets a scheduler return it for a non-empty
/// set too (the policy tier does not).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Nothing chosen.
    Idle,
    /// Run this thread next.
    Run(u8),
}

/// The scheduler interface. One associated type, one configuration constant,
/// and one associated function per operation, with no `self` (VOS01 section 1.2).
///
/// The spine calls these by UFCS through the profile, for example
/// `<P::Sched as Sched>::dispatch(k.sched)`, so a call site names the
/// *interface* and one `impl Profile` block names the implementation
/// (VOS02 section 5.1). `Profile` and `Kernel<P>` belong to the spine crate,
/// not here: a profile spans many interfaces, so no one contract crate can own
/// it (a correction to VOS01 section 1.2, recorded in VOS10 section 9).
pub trait Sched {
    /// The scheduler's whole mutable state, owned by one field of the kernel
    /// record. Implementations keep its fields private (VOS00 rule 2).
    ///
    /// The trait puts **no bound** on `State` (no `Copy`, `Clone` or `Default`):
    /// what a state may contain is a lexical matter for the shape check, not a
    /// trait bound. SH3 (no `static`, no interior mutability) and the privacy rule
    /// SH14 cover it; a by-value step needs nothing more.
    type State;

    /// The number of thread ids, `0..N`. A claim made at `N = 8` says nothing
    /// at another `N` (VOS00 section 9.3); the Warrant document pins the value
    /// with a configuration example.
    const N: u8;

    /// The initial state: no thread is ready.
    fn init() -> Self::State;

    /// Make thread `t` ready. `t >= N` answers [`Upd::OutOfRange`].
    fn insert(s: Self::State, t: u8) -> (Upd, Self::State);

    /// Take thread `t` out of the ready set, if it is there. `t >= N` answers
    /// [`Upd::OutOfRange`].
    fn remove(s: Self::State, t: u8) -> (Upd, Self::State);

    /// Choose the thread to run next and remove it from the ready set, or
    /// answer [`Pick::Idle`]. Defined for every state; there is no precondition.
    fn dispatch(s: Self::State) -> (Pick, Self::State);
}
