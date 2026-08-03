//! # gc-core — the shared GC engine for the LANG pipeline
//!
//! `gc-core` is [`FlatHeap`]: a real, malloc-backed mark-sweep /
//! generational / compacting / incremental collector, plus the supporting
//! machinery every consumer needs around it:
//!
//! - **[`HeapRef`]** — an opaque, typed reference to a managed heap object
//!   (the runtime representation of `ref<T>` in InterpreterIR).
//! - **[`kind`]** — [`HeapKind`] layout descriptors and a [`KindRegistry`]
//!   so the GC can trace object graphs without RTTI.
//! - **[`profile`]** — [`GcProfile`] and [`GcCycleStats`] that accumulate
//!   per-cycle metrics: allocation rate, survival ratio, pause time,
//!   fragmentation.
//! - **[`policy`]** — a [`GcPolicy`] trait with a [`DefaultPolicy`] and an
//!   [`AdaptivePolicy`] that reads a [`GcProfile`] snapshot and recommends an
//!   algorithm switch; [`FlatHeap::should_compact`] uses it to decide when an
//!   automatic collection should also relocate objects.
//! - **[`stackmap_builder`]** — the producer side of the precise-root
//!   stack-map format, driven by native code generators.
//!
//! Two consumers link against the *same* [`FlatHeap`] engine rather than each
//! carrying their own collector:
//!
//! - **Native-AOT / LLVM / WASM** backends link through the C ABI
//!   (`gc-core-capi`), which owns a process-global `FlatHeap` and exposes it
//!   as `extern "C"` entry points (`__twig_gc_alloc`, `__gc_safepoint`, ...).
//! - **`vm-core`** (the bytecode interpreter) depends on this crate directly
//!   as a Rust library — no C ABI needed — allocating GC-managed objects
//!   through its own `FlatHeap` instance and rooting them precisely from its
//!   own register/global/local storage (see `vm-core`'s `Value::HeapRef` and
//!   its `safepoint` opcode).
//!
//! An earlier design (`GcCore`/`GcAdapter` over a separate, synthetic-address
//! `garbage-collector` crate) explored a `HashMap`-based heap aimed at
//! interpreters specifically. It was never wired into any real consumer —
//! `vm-core` never depended on it — and has been removed in favor of
//! `vm-core` sharing [`FlatHeap`] directly, the same way the native-AOT
//! backends do.

pub mod flat_heap;
pub mod heap_ref;
pub mod kind;
pub mod policy;
pub mod profile;
/// Producer side of the precise-root stack-map format: the helper a native code
/// generator drives while lowering a function to emit its per-safepoint records.
/// See [`stackmap_builder`].
pub mod stackmap_builder;

// Top-level re-exports for the most commonly used types.
pub use flat_heap::{frame_root_slots, FlatHeap, StackMapRecord, StackMapTable};
pub use heap_ref::HeapRef;
pub use kind::{HeapKind, KindRegistry};
pub use policy::{AdaptivePolicy, DefaultPolicy, GcAlgorithm, GcPolicy, PolicyDecision};
pub use profile::{GcCycleStats, GcProfile};
pub use stackmap_builder::StackMapBuilder;
