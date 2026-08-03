//! Real garbage collection for [`crate::WasmExecutionContext::gc_heap`] — see
//! `code/specs/W04-wasm-gc.md` for the full design rationale.
//!
//! # Why not `gc-core`'s `FlatHeap`?
//!
//! `FlatHeap` is a raw-pointer collector (`alloc(n, kind) -> *mut u8`).
//! `gc_heap` is `Vec<Option<GcStruct>>`, indexed by literal position — a
//! `WasmValue::Ref(Some(handle))` **is** a `Vec` index, a WASM-spec-mandated
//! representation this collector cannot redesign. Removing entries by
//! shifting the `Vec` would silently invalidate every other live handle
//! pointing past the removed index (that's compaction — explicitly out of
//! scope, see the module doc in `lib.rs` and the spec). So this is a
//! **tombstone + free-list slot arena**: a dead slot becomes `None` (its
//! `GcStruct` — and the memory its `fields: Vec<WasmValue>` held — is
//! dropped, genuine reclamation) and its index joins [`GcState::free_list`]
//! for `struct.new` to reuse, rather than being physically removed.
//!
//! No generation tag is needed to guard against a reused slot aliasing a
//! stale handle (the usual reason a "generational arena" carries one):
//! mark-sweep's ordinary soundness argument — nothing left unmarked is
//! reachable, provided the root walk is exhaustive — already rules that out.
//! Every handle a WASM program can hold either sat in a location the mark
//! phase scans, or was freshly minted by `struct.new`; there is no way to
//! manufacture or retain a handle from outside that root set.
//!
//! # Precise scanning is free here
//!
//! Unlike `FlatHeap`, which traces raw untyped words and needs a registered
//! `HeapKind` field-offset map to know which words are pointers, a
//! `GcStruct` field is a tagged `WasmValue` — exactly `Ref(Some(_))`,
//! `Ref(None)`, or a numeric variant, self-describing with no schema needed.

use crate::{GcStruct, WasmExecutionContext, WasmValue, REF_NULL_SENTINEL, REF_TAG};
use gc_core::profile::GcCycleStats;
use gc_core::GcProfile;
use virtual_machine::{GenericVM, Value};

/// Object-count threshold a fresh [`GcState`] starts at (analogous to
/// `gc-core`'s `INITIAL_THRESHOLD`, but counting live objects rather than
/// bytes — this heap has no byte-size concept).
pub const INITIAL_THRESHOLD: usize = 1024;

/// Ceiling `adapt_threshold` will never grow past — load-bearing for safety,
/// not just tuning: without it a retention-heavy program could double the
/// threshold toward `usize::MAX`, making collection never fire again.
pub const MAX_THRESHOLD: usize = 1 << 24;

/// Per-call GC bookkeeping threaded alongside `gc_heap` on
/// [`WasmExecutionContext`] — reset fresh every call, exactly like the heap
/// itself (see the "Lifetime" section of `W04-wasm-gc.md`: `gc_heap` has no
/// cross-call continuity today, so neither does this).
#[derive(Debug, Clone)]
pub struct GcState {
    /// Reclaimed `gc_heap` indices, checked by `struct.new` before growing
    /// the arena.
    pub free_list: Vec<u32>,
    /// Live object count, tracked incrementally (incremented on
    /// `struct.new`, decremented per object during sweep) rather than
    /// recomputed by scanning `gc_heap` — the same "avoid an O(n) count on a
    /// hot path" reasoning behind `vm-core`'s `gc_object_count` field.
    pub live_count: usize,
    /// Collect when `live_count` reaches this; adapted after every cycle
    /// (see [`adapt_threshold`]).
    pub threshold: usize,
    /// Diagnostic history, reused as-is from `gc-core` (a pure numeric
    /// accumulator with no `FlatHeap`/pointer coupling) for consistency with
    /// the native-AOT and `vm-core` paths.
    pub profile: GcProfile,
}

impl Default for GcState {
    fn default() -> Self {
        GcState {
            free_list: Vec::new(),
            live_count: 0,
            threshold: INITIAL_THRESHOLD,
            profile: GcProfile::default(),
        }
    }
}

/// Whether the live count has reached the threshold — pure policy, mirrors
/// `gc-core::FlatHeap::should_collect`: it names no roots and runs no
/// collection itself.
fn should_collect(state: &GcState) -> bool {
    state.live_count >= state.threshold
}

/// Re-tune the threshold after a cycle, given the live count *before* it —
/// the identical heuristic `gc-core::FlatHeap::adapt_threshold` uses (ported
/// verbatim, just in units of objects instead of bytes): more than half the
/// pre-cycle live set survived → double (capped at [`MAX_THRESHOLD`]);
/// otherwise → halve (floored at [`INITIAL_THRESHOLD`]).
fn adapt_threshold(state: &mut GcState, prev_live: usize) {
    if state.live_count > prev_live / 2 {
        state.threshold = (state.threshold * 2).min(MAX_THRESHOLD);
    } else {
        state.threshold = (state.threshold / 2).max(INITIAL_THRESHOLD);
    }
}

/// Push every `Ref(Some(_))` found in `values` onto the mark worklist.
fn push_roots_from_values(values: &[WasmValue], work: &mut Vec<u32>) {
    for v in values {
        if let WasmValue::Ref(Some(h)) = v {
            work.push(*h);
        }
    }
}

/// Push every live reference held on the interpreter's operand stack, using
/// the same `REF_TAG`/`REF_NULL_SENTINEL` convention `to_typed`/`from_typed`
/// already establish for round-tripping a [`WasmValue::Ref`] through the
/// generic typed stack.
fn push_roots_from_stack(vm: &GenericVM, work: &mut Vec<u32>) {
    for tv in &vm.typed_stack {
        if tv.value_type == REF_TAG {
            if let Value::Int(h) = tv.value {
                if h != REF_NULL_SENTINEL {
                    work.push(h as u32);
                }
            }
        }
    }
}

/// Collect the current, root-reachable set of `gc_heap` indices —
/// transitive and cycle-safe: a worklist walk with a `marked` visited set,
/// so a cycle (`struct.set` can point one live object's field at another
/// that points back) just re-adds an already-`true` index and no-ops.
fn mark(vm: &GenericVM, ctx: &WasmExecutionContext) -> Vec<bool> {
    let heap_len = ctx.gc_heap.len();
    let mut marked = vec![false; heap_len];
    let mut work: Vec<u32> = Vec::new();

    push_roots_from_values(&ctx.globals, &mut work);
    push_roots_from_values(&ctx.typed_locals, &mut work);
    for frame in &ctx.saved_frames {
        push_roots_from_values(&frame.locals, &mut work);
    }
    push_roots_from_stack(vm, &mut work);

    while let Some(h) = work.pop() {
        let idx = h as usize;
        if idx >= heap_len || marked[idx] {
            continue;
        }
        marked[idx] = true;
        if let Some(obj) = &ctx.gc_heap[idx] {
            push_roots_from_values(&obj.fields, &mut work);
        }
    }

    marked
}

/// Reclaim every unmarked, currently-live slot: drop its `GcStruct` (freeing
/// the field-vector memory it held), tombstone it to `None`, and push its
/// index onto the free list for `struct.new` to reuse. Returns the number
/// of objects freed.
fn sweep(ctx: &mut WasmExecutionContext, marked: &[bool]) -> usize {
    // Disjoint field borrows off one `&mut WasmExecutionContext` — sound
    // because `gc_heap` and `gc_state` are different fields, not aliases.
    let WasmExecutionContext { gc_heap, gc_state, .. } = ctx;
    let mut freed = 0usize;
    for i in 0..gc_heap.len() {
        if !marked[i] && gc_heap[i].is_some() {
            gc_heap[i] = None;
            gc_state.free_list.push(i as u32);
            freed += 1;
        }
    }
    gc_state.live_count -= freed;
    freed
}

/// Collect if `ctx.gc_state`'s threshold says a collection is due — the
/// entry point `execute_branch` (every taken `br`/`br_if`/`br_table`, i.e.
/// every loop back-edge) and the internal `call_function` helper (every
/// `call`/`call_indirect`, nested or not) both call on every dispatch,
/// mirroring the existing "safepoints at back-edges and calls" convention
/// (see `W04-wasm-gc.md` §4) rather than a per-instruction counter — which
/// would need threading a budget through the generic, WASM-agnostic
/// `virtual-machine` crate's dispatch loop, coupling it to this crate's GC.
pub(crate) fn maybe_collect(vm: &GenericVM, ctx: &mut WasmExecutionContext) {
    if !should_collect(&ctx.gc_state) {
        return;
    }
    let live_before = ctx.gc_state.live_count;
    let marked = mark(vm, ctx);
    let freed = sweep(ctx, &marked);

    ctx.gc_state.profile.record_cycle(&GcCycleStats {
        freed,
        survived: ctx.gc_state.live_count,
        pause_ns: 0,
        heap_size_before: live_before,
        heap_size_after: ctx.gc_state.live_count,
    });
    adapt_threshold(&mut ctx.gc_state, live_before);
}

/// Allocate a `GcStruct`, reusing a tombstoned slot from `gc_state.free_list`
/// before growing `gc_heap` — the free-list half of the slot-arena design.
/// Returns the new object's handle.
pub(crate) fn alloc(ctx: &mut WasmExecutionContext, obj: GcStruct) -> u32 {
    let WasmExecutionContext { gc_heap, gc_state, .. } = ctx;
    gc_state.live_count += 1;
    if let Some(idx) = gc_state.free_list.pop() {
        gc_heap[idx as usize] = Some(obj);
        idx
    } else {
        let handle = gc_heap.len() as u32;
        gc_heap.push(Some(obj));
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SavedFrame, WasmExecutionContext};
    use std::collections::HashMap;

    fn obj(fields: Vec<WasmValue>) -> GcStruct {
        GcStruct { type_idx: 0, fields }
    }

    /// A bare `WasmExecutionContext` with every non-heap field trivially
    /// empty — enough to unit-test `alloc`/`mark`/`sweep` directly without
    /// the full raw-bytecode harness `lib.rs`'s own tests use.
    fn empty_ctx() -> WasmExecutionContext {
        WasmExecutionContext {
            memory: None,
            tables: Vec::new(),
            globals: Vec::new(),
            global_types: Vec::new(),
            func_types: Vec::new(),
            func_bodies: Vec::new(),
            host_functions: Vec::new(),
            typed_locals: Vec::new(),
            label_stack: Vec::new(),
            control_flow_map: HashMap::new(),
            saved_frames: Vec::new(),
            returned: false,
            br_table_targets: Vec::new(),
            gc_ops: Vec::new(),
            gc_heap: Vec::new(),
            struct_field_counts: Vec::new(),
            gc_state: GcState::default(),
        }
    }

    #[test]
    fn should_collect_fires_at_threshold() {
        let mut state = GcState { threshold: 10, ..GcState::default() };
        state.live_count = 9;
        assert!(!should_collect(&state));
        state.live_count = 10;
        assert!(should_collect(&state));
    }

    #[test]
    fn adapt_threshold_doubles_when_retention_high() {
        let mut state = GcState { threshold: 1000, ..GcState::default() };
        state.live_count = 600; // > half of 1000 survived
        adapt_threshold(&mut state, 1000);
        assert_eq!(state.threshold, 2000);
    }

    #[test]
    fn adapt_threshold_halves_when_garbage_heavy() {
        // 4096, well above INITIAL_THRESHOLD's floor, so halving is actually
        // observable rather than immediately clamped back up to the floor.
        let mut state = GcState { threshold: 4096, ..GcState::default() };
        state.live_count = 400; // <= half of 4096 survived
        adapt_threshold(&mut state, 4096);
        assert_eq!(state.threshold, 2048);
    }

    #[test]
    fn adapt_threshold_respects_floor_and_ceiling() {
        let mut low = GcState { threshold: INITIAL_THRESHOLD, ..GcState::default() };
        adapt_threshold(&mut low, 100); // low.live_count (0) <= 50 -> halve
        assert_eq!(low.threshold, INITIAL_THRESHOLD, "never drops below the floor");

        let mut high = GcState { threshold: MAX_THRESHOLD, ..GcState::default() };
        high.live_count = MAX_THRESHOLD; // > half survived -> double
        adapt_threshold(&mut high, MAX_THRESHOLD);
        assert_eq!(high.threshold, MAX_THRESHOLD, "never exceeds the ceiling");
    }

    #[test]
    fn alloc_reuses_free_list_before_growing() {
        let mut ctx = empty_ctx();

        let a = alloc(&mut ctx, obj(vec![WasmValue::I32(1)]));
        let b = alloc(&mut ctx, obj(vec![WasmValue::I32(2)]));
        assert_eq!(ctx.gc_heap.len(), 2);

        ctx.gc_state.free_list.push(a);
        ctx.gc_state.live_count -= 1; // simulate a's collection
        let c = alloc(&mut ctx, obj(vec![WasmValue::I32(3)]));
        assert_eq!(c, a, "reused the freed slot instead of growing");
        assert_eq!(ctx.gc_heap.len(), 2, "arena did not grow");
        assert_eq!(
            ctx.gc_heap[b as usize].as_ref().unwrap().fields[0],
            WasmValue::I32(2),
            "b's slot untouched by a's reuse"
        );
    }

    #[test]
    fn mark_finds_transitively_reachable_objects_through_a_field_chain() {
        let mut ctx = empty_ctx();
        let vm = GenericVM::new();

        // c <- b <- a, only `a` is a root (a global).
        let c = alloc(&mut ctx, obj(vec![WasmValue::I32(42)]));
        let b = alloc(&mut ctx, obj(vec![WasmValue::Ref(Some(c))]));
        let a = alloc(&mut ctx, obj(vec![WasmValue::Ref(Some(b))]));
        ctx.globals.push(WasmValue::Ref(Some(a)));

        let marked = mark(&vm, &ctx);
        assert!(marked[a as usize] && marked[b as usize] && marked[c as usize]);
    }

    #[test]
    fn maybe_collect_reclaims_unreachable_and_preserves_reachable_chain() {
        let mut ctx = empty_ctx();
        ctx.gc_state.threshold = 1; // force collection eligibility
        let vm = GenericVM::new();

        let c = alloc(&mut ctx, obj(vec![WasmValue::I32(42)]));
        let b = alloc(&mut ctx, obj(vec![WasmValue::Ref(Some(c))]));
        let a = alloc(&mut ctx, obj(vec![WasmValue::Ref(Some(b))]));
        ctx.globals.push(WasmValue::Ref(Some(a)));
        // Pure garbage: unreachable from anything.
        let garbage = alloc(&mut ctx, obj(vec![WasmValue::I32(999)]));

        maybe_collect(&vm, &mut ctx);

        assert_eq!(ctx.gc_state.live_count, 3, "a, b, c survive; garbage is reclaimed");
        assert!(ctx.gc_heap[a as usize].is_some());
        assert!(ctx.gc_heap[b as usize].is_some());
        assert!(ctx.gc_heap[c as usize].is_some());
        assert!(ctx.gc_heap[garbage as usize].is_none(), "unreachable object tombstoned");
        assert_eq!(ctx.gc_state.free_list, vec![garbage], "its slot is ready for reuse");
        assert_eq!(ctx.gc_state.profile.total_collections, 1);
        assert_eq!(ctx.gc_state.profile.total_freed, 1);
    }

    /// Two objects whose fields point at each other, reachable from nothing —
    /// proves the worklist mark is genuinely cycle-safe (a naive unguarded
    /// recursive mark would loop forever) and that an unreachable cycle is
    /// still collected, not kept alive just because it references itself.
    #[test]
    fn cyclic_but_unreachable_objects_are_both_collected() {
        let mut ctx = empty_ctx();
        ctx.gc_state.threshold = 1;
        let vm = GenericVM::new();

        // Allocate both first (fields default to I32(0)), then wire the cycle
        // via struct.set-equivalent direct field mutation.
        let x = alloc(&mut ctx, obj(vec![WasmValue::I32(0)]));
        let y = alloc(&mut ctx, obj(vec![WasmValue::Ref(Some(x))]));
        ctx.gc_heap[x as usize].as_mut().unwrap().fields[0] = WasmValue::Ref(Some(y));
        // No root anywhere points at x or y.

        maybe_collect(&vm, &mut ctx);

        assert_eq!(ctx.gc_state.live_count, 0, "the cycle is unreachable, so both members are freed");
        assert!(ctx.gc_heap[x as usize].is_none());
        assert!(ctx.gc_heap[y as usize].is_none());
    }

    /// A live root sitting only in a *suspended caller's* frame (not the
    /// active frame) must still be found — the one root source with no
    /// equivalent test anywhere else in this codebase's GC work.
    #[test]
    fn mark_finds_roots_in_saved_caller_frames() {
        let mut ctx = empty_ctx();
        let vm = GenericVM::new();

        let kept = alloc(&mut ctx, obj(vec![WasmValue::I32(7)]));
        // The active frame's locals don't reference it...
        ctx.typed_locals = vec![WasmValue::I32(0)];
        // ...but a paused caller, one level up the call stack, does.
        ctx.saved_frames.push(SavedFrame {
            locals: vec![WasmValue::Ref(Some(kept))],
            label_stack: Vec::new(),
            stack_height: 0,
            control_flow_map: HashMap::new(),
            return_pc: 0,
            return_arity: 0,
            br_table_targets: Vec::new(),
            gc_ops: Vec::new(),
        });

        let marked = mark(&vm, &ctx);
        assert!(marked[kept as usize], "a saved caller frame's local is a live root");
    }
}
