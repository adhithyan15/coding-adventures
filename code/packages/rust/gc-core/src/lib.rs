//! # gc-core — LANG16: Adaptive GC Adapter for the LANG VM Pipeline
//!
//! `gc-core` wires the standalone [`garbage-collector`] crate into the LANG
//! pipeline (LANG01–LANG15).  It adds:
//!
//! - **[`HeapRef`]** — an opaque, typed reference to a managed heap object
//!   (the runtime representation of `ref<T>` in InterpreterIR).
//! - **[`kind`]** — [`HeapKind`] layout descriptors and a [`KindRegistry`]
//!   so the GC can trace object graphs without RTTI.
//! - **[`profile`]** — [`GcProfile`] and [`GcCycleStats`] that accumulate
//!   per-cycle metrics: allocation rate, survival ratio, pause time,
//!   fragmentation.
//! - **[`policy`]** — a [`GcPolicy`] trait with a [`DefaultPolicy`] and an
//!   [`AdaptivePolicy`] that inspects the profile and recommends switching
//!   GC algorithms when the current one is not well-suited to the workload.
//! - **[`adapter`]** — [`GcAdapter`] bridging any `GarbageCollector`
//!   implementation with profiling instrumentation.
//! - **[`root_set`]** — [`RootSet`] for collecting live heap roots before
//!   each GC cycle.
//! - **[`write_barrier`]** — [`WriteBarrier`] trait with [`NoOpBarrier`]
//!   (mark-and-sweep) and [`CardTableBarrier`] stub (generational).
//! - **[`GcCore`]** — the top-level facade that `vm-core` (LANG02) holds and
//!   calls on every safepoint.
//!
//! ## Adaptive GC selection
//!
//! One of the most important design decisions in this crate is the explicit
//! separation of **algorithm** (how to collect) from **policy** (when to
//! switch algorithms).  Different GC algorithms have fundamentally different
//! performance characteristics:
//!
//! | Algorithm     | Strength                                  | Weakness                      |
//! |---------------|-------------------------------------------|-------------------------------|
//! | Mark-and-sweep | Handles cycles; simple                   | Pause grows with heap size    |
//! | Generational  | Cheap minor GC for short-lived objects    | Cross-generation write barrier|
//! | Compacting    | Eliminates fragmentation; cache-friendly  | Must update all pointers      |
//! | Incremental   | Bounded per-allocation pause              | Write barrier on every store  |
//!
//! `GcProfile` records the signals (survival ratio, pause time, fragmentation)
//! that distinguish these regimes.  [`AdaptivePolicy`] reads those signals and
//! emits [`PolicyDecision::SuggestSwitch`] when a different algorithm would
//! be more suitable.  [`GcCore`] records the advisory and — once more
//! algorithms are implemented — can carry out the switch mid-execution.
//!
//! ## IIR opcodes introduced by LANG16
//!
//! The following opcodes are added to InterpreterIR (LANG01) and handled by
//! the `vm-core` dispatch loop using `GcCore`:
//!
//! | Opcode        | Operands          | Effect                              |
//! |---------------|-------------------|-------------------------------------|
//! | `alloc`       | (size, kind)      | `dest = GcCore::alloc(...)`         |
//! | `box`         | (value,)          | box a primitive on the heap         |
//! | `unbox`       | (ref,)            | dereference a ref; trap if null     |
//! | `field_load`  | (ref, offset)     | read a field from a heap object     |
//! | `field_store` | (ref, offset, v)  | write a field; call write barrier   |
//! | `is_null`     | (ref,)            | `dest = ref == null`                |
//! | `safepoint`   | ()                | `GcCore::force_collect(roots)`      |
//!
//! ## Quick start
//!
//! ```
//! use gc_core::GcCore;
//! use gc_core::root_set::RootSet;
//! use gc_core::kind::HeapKind;
//! use garbage_collector::Symbol;
//!
//! // 1. Create the GcCore with the default mark-and-sweep algorithm.
//! let mut gc = GcCore::with_mark_and_sweep();
//!
//! // 2. Register a layout descriptor for Symbol objects.
//! let sym_kind = gc.register_kind(HeapKind {
//!     kind_id: 0,
//!     size: 32,
//!     field_offsets: vec![],
//!     type_name: "Symbol".to_string(),
//!     finalizer: false,
//! });
//!
//! // 3. Allocate objects during VM execution.
//! let r1 = gc.alloc(Box::new(Symbol::new("foo")), sym_kind);
//! let r2 = gc.alloc(Box::new(Symbol::new("bar")), sym_kind);
//! assert_eq!(gc.heap_size(), 2);
//!
//! // 4. Collect, keeping only r1 as a live root.
//! let mut roots = RootSet::new();
//! roots.add_ref(r1);
//! let stats = gc.force_collect(&roots);
//! assert_eq!(stats.freed, 1);
//! assert_eq!(gc.heap_size(), 1);
//!
//! // 5. Inspect the profiling data.
//! let profile = gc.profile();
//! assert_eq!(profile.total_allocations, 2);
//! assert_eq!(profile.total_collections, 1);
//! ```

pub mod adapter;
pub mod gc_core;
pub mod heap_ref;
pub mod kind;
pub mod policy;
pub mod profile;
pub mod root_set;
pub mod write_barrier;

// Top-level re-exports for the most commonly used types.
pub use adapter::GcAdapter;
pub use gc_core::{GcCore, PolicyAdvisory};
pub use heap_ref::HeapRef;
pub use kind::{HeapKind, KindRegistry};
pub use policy::{AdaptivePolicy, DefaultPolicy, GcAlgorithm, GcPolicy, PolicyDecision};
pub use profile::{GcCycleStats, GcProfile};
pub use root_set::RootSet;
pub use write_barrier::{CardTableBarrier, NoOpBarrier, WriteBarrier};
