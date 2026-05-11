//! # vm-concurrency — LANG28B single-thread cooperative scheduler
//!
//! This crate implements the cooperative-multitasking scheduler for the LANG VM.
//! It is the primary runtime for the 27 concurrency opcodes defined in
//! `interpreter-ir` v0.3.0 (LANG28A).
//!
//! ## Architecture
//!
//! ```text
//! vm-core  ──calls──►  vm-concurrency
//!                          │
//!                          ├── Scheduler (task table, ready queue, timers)
//!                          ├── ChannelEntry (bounded message queues)
//!                          ├── GroupEntry (structured concurrency scopes)
//!                          ├── SelectSetEntry (multi-arm reactive waiting)
//!                          └── CancelTokenEntry (cooperative cancellation)
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use vm_concurrency::{Scheduler, GroupPolicy};
//!
//! let mut sched = Scheduler::deterministic(42);
//!
//! // Spawn two tasks.
//! let t1 = sched.task_spawn().unwrap();
//! let t2 = sched.task_spawn().unwrap();
//!
//! // The scheduler picks tasks in spawn order (deterministic).
//! let next = sched.pick_next().unwrap();
//! sched.set_current(next);
//! assert_eq!(sched.task_current(), Some(t1));
//! ```
//!
//! ## Deterministic mode
//!
//! For testing, use `Scheduler::deterministic(seed)`:
//! - tasks run in spawn order (FIFO);
//! - the clock starts at `Deadline::ZERO` and only advances via `advance_clock`;
//! - select tie-breaking uses the seed.
//!
//! ## Non-goals for LANG28B
//!
//! M:N worker pools, GC root enumeration across task stacks, debugger integration,
//! native event backends, JIT/AOT stack maps, and host-VM backends are planned
//! for later LANG28 phases.  Backends that encounter concurrency opcodes should
//! continue to return `UnsupportedOp` from their validator.

// ── Modules ───────────────────────────────────────────────────────────────────

pub mod cancel;
pub mod channel;
pub mod error;
pub mod group;
pub mod scheduler;
pub mod select;
pub mod task;
pub mod types;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::ConcurrencyError;
pub use scheduler::Scheduler;
pub use types::{
    ArmId, CancelTokenId, ChannelId, Deadline, GroupId, GroupPolicy,
    ParkReason, RecvResult, SelectArmKind, SelectResult, SelectSetId,
    SelectStatus, SendResult, TaskHandle, TaskState, Value,
};
