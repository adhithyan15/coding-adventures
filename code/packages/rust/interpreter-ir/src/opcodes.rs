//! Opcode category helpers and type-string constants for InterpreterIR.
//!
//! These functions let vm-core, jit-core, and IR passes classify instructions
//! without string-matching against long mnemonic lists.  Every set is a simple
//! `match` — no heap allocation, no `HashSet`, no lazy initialisation.  The
//! compiler inlines and optimises them to a jump table.
//!
//! # Type constants
//!
//! IIR uses *string* type hints rather than an enum so that language frontends
//! can introduce domain-specific types without modifying this crate.  The
//! constants below are the universally recognised ones:
//!
//! ```
//! use interpreter_ir::opcodes::{DYNAMIC_TYPE, POLYMORPHIC_TYPE};
//! assert_eq!(DYNAMIC_TYPE, "any");
//! assert_eq!(POLYMORPHIC_TYPE, "polymorphic");
//! ```
//!
//! # Opcode categories
//!
//! ```
//! use interpreter_ir::opcodes::is_arithmetic;
//! assert!(is_arithmetic("add"));
//! assert!(is_arithmetic("neg"));
//! assert!(!is_arithmetic("jmp"));
//! ```
//!
//! # Reference types (LANG16)
//!
//! Heap pointers are encoded as the string `"ref<T>"` where `T` is the
//! pointee type.  Examples:
//!
//! ```
//! use interpreter_ir::opcodes::{is_ref_type, unwrap_ref_type, make_ref_type};
//! assert!(is_ref_type("ref<u8>"));
//! assert_eq!(unwrap_ref_type("ref<u8>"), Some("u8".to_string()));
//! assert_eq!(make_ref_type("any"), "ref<any>");
//! ```

// ---------------------------------------------------------------------------
// Type-string constants
// ---------------------------------------------------------------------------

/// The dynamic (unknown) type used by untyped languages before profiling.
///
/// An instruction whose `type_hint == DYNAMIC_TYPE` will be observed by the
/// profiler; instructions with concrete types are skipped (zero overhead).
pub const DYNAMIC_TYPE: &str = "any";

/// Sentinel written by the profiler when a slot has seen multiple types.
///
/// A JIT that reads `observed_type == POLYMORPHIC_TYPE` should NOT specialise
/// — the value is too variable to fix at compile time.
pub const POLYMORPHIC_TYPE: &str = "polymorphic";

/// The type hint produced by `alloc_closure` instructions (LANG34).
///
/// A closure value is a heap-allocated record pairing a function name with
/// a vector of captured variable values.  The `"closure"` type is NOT in
/// `CONCRETE_TYPES` — it is not a scalar and the numeric `iir-to-*` backends
/// reject it (closures are not JVM primitives or WASM scalars).
///
/// ```
/// use interpreter_ir::opcodes::{CLOSURE_TYPE, is_closure_op};
/// assert_eq!(CLOSURE_TYPE, "closure");
/// assert!(is_closure_op("alloc_closure"));
/// assert!(is_closure_op("call_closure"));
/// ```
pub const CLOSURE_TYPE: &str = "closure";

/// The concrete types recognised by every LANG-pipeline backend.
///
/// Language frontends may use additional type strings; these are the ones
/// that all backends (WASM, JVM, Intel 4004, …) agree on.
pub const CONCRETE_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64",
    "i8", "i16", "i32", "i64",
    "f32", "f64",
    "bool", "str",
];

// ---------------------------------------------------------------------------
// Reference-type helpers (LANG16)
// ---------------------------------------------------------------------------
//
// Heap pointers use the "ref<T>" encoding so the rest of the type system stays
// unchanged.  A Lisp nil-terminated list might look like `ref<ref<any>>`;
// a boxed integer is `ref<u8>`.

const REF_PREFIX: &str = "ref<";
const REF_SUFFIX: &str = ">";

/// Return `true` if `type_hint` is a heap-reference type `"ref<T>"`.
pub fn is_ref_type(type_hint: &str) -> bool {
    type_hint.starts_with(REF_PREFIX) && type_hint.ends_with(REF_SUFFIX)
}

/// Return `Some(T)` for `"ref<T>"`, or `None` for non-reference types.
///
/// ```
/// use interpreter_ir::opcodes::unwrap_ref_type;
/// assert_eq!(unwrap_ref_type("ref<u8>"),       Some("u8".to_string()));
/// assert_eq!(unwrap_ref_type("ref<ref<any>>"), Some("ref<any>".to_string()));
/// assert_eq!(unwrap_ref_type("u8"),            None);
/// ```
pub fn unwrap_ref_type(type_hint: &str) -> Option<String> {
    if !is_ref_type(type_hint) {
        return None;
    }
    let inner = &type_hint[REF_PREFIX.len()..type_hint.len() - REF_SUFFIX.len()];
    Some(inner.to_string())
}

/// Wrap `inner` as a reference type string.  Inverse of [`unwrap_ref_type`].
///
/// ```
/// use interpreter_ir::opcodes::make_ref_type;
/// assert_eq!(make_ref_type("u8"),  "ref<u8>");
/// assert_eq!(make_ref_type("any"), "ref<any>");
/// ```
pub fn make_ref_type(inner: &str) -> String {
    format!("{REF_PREFIX}{inner}{REF_SUFFIX}")
}

// ---------------------------------------------------------------------------
// Opcode category predicates
// ---------------------------------------------------------------------------
//
// Using plain `match` rather than `HashSet` means:
//   • Zero heap allocation
//   • LLVM can turn the match into a lookup table at -O2
//   • Every new opcode must be added to exactly one category (no silent gaps)

/// Integer and floating-point arithmetic.
pub fn is_arithmetic(op: &str) -> bool {
    matches!(op, "add" | "sub" | "mul" | "div" | "mod" | "neg")
}

/// Bitwise operations.
pub fn is_bitwise(op: &str) -> bool {
    matches!(op, "and" | "or" | "xor" | "not" | "shl" | "shr")
}

/// Comparison operations — all produce a `bool`.
pub fn is_cmp(op: &str) -> bool {
    matches!(
        op,
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

/// Conditional and unconditional branches.
pub fn is_branch(op: &str) -> bool {
    matches!(op, "jmp" | "jmp_if_true" | "jmp_if_false")
}

/// Control-flow terminators (labels count here because they delimit blocks).
pub fn is_control(op: &str) -> bool {
    matches!(op, "label" | "ret" | "ret_void")
}

/// Register and memory loads/stores.
pub fn is_memory(op: &str) -> bool {
    matches!(op, "load_reg" | "store_reg" | "load_mem" | "store_mem")
}

/// Module-level global variable operations (LANG32).
///
/// `global_load` reads a named global; `global_store` writes one.
/// The global name is always `srcs[0] = Operand::Str("name")` — a
/// compile-time string literal, NOT a register reference.
///
/// ```
/// use interpreter_ir::opcodes::is_global;
/// assert!(is_global("global_load"));
/// assert!(is_global("global_store"));
/// assert!(!is_global("load_reg"));
/// ```
pub fn is_global(op: &str) -> bool {
    matches!(op, "global_load" | "global_store")
}

/// Function calls.
pub fn is_call(op: &str) -> bool {
    matches!(op, "call" | "call_builtin")
}

/// I/O operations.
pub fn is_io(op: &str) -> bool {
    matches!(op, "io_in" | "io_out")
}

/// Closure allocation and application opcodes (LANG34).
///
/// | Opcode | Meaning |
/// |--------|---------|
/// | `alloc_closure` | Allocate a closure: `srcs[0] = Operand::Str(fn_name)`, `srcs[1..] = captures` |
/// | `call_closure`  | Invoke a closure: `srcs[0] = handle`, `srcs[1..] = user args` |
///
/// Both opcodes are value-producing (`dest` is always `Some`).
/// `alloc_closure` is also allocating (`may_alloc = true`).
///
/// ```
/// use interpreter_ir::opcodes::is_closure_op;
/// assert!(is_closure_op("alloc_closure"));
/// assert!(is_closure_op("call_closure"));
/// assert!(!is_closure_op("call_builtin"));
/// assert!(!is_closure_op("call"));
/// ```
pub fn is_closure_op(op: &str) -> bool {
    matches!(op, "alloc_closure" | "call_closure")
}

/// Type coercions and assertions.
pub fn is_coercion(op: &str) -> bool {
    matches!(op, "cast" | "type_assert")
}

/// Heap / GC operations (LANG16).
///
/// Programs that never allocate never emit these — GC overhead is zero.
/// Programs that do allocate use these seven opcodes to communicate
/// allocation intent and write-barrier points to vm-core's GC layer.
pub fn is_heap(op: &str) -> bool {
    matches!(
        op,
        "alloc"       // heap-allocate N bytes of kind K → ref<K>
        | "box"       // heap-allocate and store a value → ref<T>
        | "unbox"     // load from ref<T>; trap on null
        | "field_load"  // *(ref + offset)
        | "field_store" // *(ref + offset) = value; may emit write barrier
        | "is_null"   // (ref == NULL) → bool
        | "safepoint" // yield to GC if collection pending; may_alloc
    )
}

/// Return `true` if `op` produces a result value (has a non-`None` dest).
///
/// # Concurrency ops that produce a value
///
/// | Op | Dest type |
/// |----|-----------|
/// | `task_spawn` | `task<T>` |
/// | `task_current` | `task<void>` |
/// | `task_join` | T (parks) |
/// | `task_check_cancel` | `bool` |
/// | `group_new` | `task_group` |
/// | `group_spawn` | `task<T>` |
/// | `chan_new` | `channel<T>` |
/// | `chan_recv` | T (parks) |
/// | `chan_try_send` | `bool` |
/// | `chan_try_recv` | `option<T>` |
/// | `select_new` | `select_set` |
/// | `select_recv` | arm\_id (`u32`) |
/// | `select_send` | arm\_id (`u32`) |
/// | `select_join` | arm\_id (`u32`) |
/// | `select_timer` | arm\_id (`u32`) |
/// | `select_cancel` | arm\_id (`u32`) |
/// | `select_wait` | arm\_id (`u32`) (parks) |
/// | `select_default` | arm\_id (`u32`) |
pub fn is_value_producing(op: &str) -> bool {
    is_arithmetic(op)
        || is_bitwise(op)
        || is_cmp(op)
        || matches!(
            op,
            "const"
                | "load_reg"
                | "load_mem"
                | "call"
                | "call_builtin"
                | "io_in"
                | "cast"
                | "alloc"
                | "box"
                | "unbox"
                | "field_load"
                | "is_null"
                // Global variable read (LANG32)
                | "global_load"
                // Closure allocation and application (LANG34)
                | "alloc_closure"
                | "call_closure"
                // Concurrency ops that produce a dest value (LANG28)
                | "task_spawn"
                | "task_current"
                | "task_join"
                | "task_check_cancel"
                | "group_new"
                | "group_spawn"
                | "chan_new"
                | "chan_recv"
                | "chan_try_send"
                | "chan_try_recv"
                | "select_new"
                | "select_recv"
                | "select_send"
                | "select_join"
                | "select_timer"
                | "select_cancel"
                | "select_wait"
                | "select_default"
        )
}

/// Return `true` if `op` has side effects beyond producing a value.
///
/// An instruction with side effects must not be removed by dead-code
/// elimination even when its result is unused.
///
/// # Concurrency ops with side effects (LANG28)
///
/// | Op | Side effect |
/// |----|-------------|
/// | `task_yield` | cooperatively yields to the scheduler |
/// | `task_sleep` | parks until deadline |
/// | `task_cancel` | signals cancel token on a task |
/// | `task_detach` | detaches task from parent group |
/// | `group_join` | parks until all group tasks complete |
/// | `group_cancel` | cancels every running task in the group |
/// | `group_close` | prevents further spawns into the group |
/// | `chan_send` | delivers a value (may park if full) |
/// | `chan_close` | closes the send side of a channel |
pub fn has_side_effects(op: &str) -> bool {
    is_branch(op)
        || is_control(op)
        || matches!(
            op,
            "store_reg"
                | "store_mem"
                | "io_out"
                | "type_assert"
                | "field_store"
                | "safepoint"
                // Global variable write (LANG32)
                | "global_store"
                // Concurrency ops with side effects but no dest (LANG28)
                | "task_yield"
                | "task_sleep"
                | "task_cancel"
                | "task_detach"
                | "group_join"
                | "group_cancel"
                | "group_close"
                | "chan_send"
                | "chan_close"
        )
}

/// Return `true` if `op` may trigger a GC cycle.
///
/// Language frontends set `IIRInstr::may_alloc = true` for these opcodes
/// plus any `call` whose callee transitively allocates.
///
/// # Concurrency ops that allocate (LANG28)
///
/// Each of these creates a new heap-resident object:
///
/// | Op | Object allocated |
/// |----|-----------------|
/// | `task_spawn` | task control block |
/// | `group_new` | task-group descriptor |
/// | `group_spawn` | task control block (inside a group) |
/// | `chan_new` | bounded-channel buffer |
/// | `select_new` | select-set arm array |
pub fn is_allocating(op: &str) -> bool {
    matches!(
        op,
        "alloc" | "box" | "safepoint"
            // Closure allocation (LANG34)
            | "alloc_closure"
            // Concurrency allocators (LANG28)
            | "task_spawn"
            | "group_new"
            | "group_spawn"
            | "chan_new"
            | "select_new"
    )
}

// ---------------------------------------------------------------------------
// Concurrency opcodes (LANG28)
// ---------------------------------------------------------------------------
//
// These 27 opcodes implement the LANG28 cooperative-multitasking model.
// They are grouped into four families:
//
//   • Task    — spawn, join, yield, cancel, detach lightweight tasks
//   • Group   — task groups for structured concurrency
//   • Channel — typed message-passing queues (bounded MPSC/MPMC)
//   • Select  — reactive multi-arm waiting (like Go select / Erlang receive)
//
// ### "May park" semantics
//
// Many concurrency opcodes are marked `is_parking`.  A "parking" opcode
// suspends the current task and lets the scheduler run another task.  This
// has two implications:
//
// 1. A GC safepoint is implied — the GC may collect while the task is parked.
// 2. Any live ref<T> across a parking point must be treated as potentially
//    relocated by a moving GC (handled at a higher layer; IIR tracks which
//    instructions may park so that analysis passes can identify these points).
//
// ### Implementation note (Phase 28A — this file only)
//
// This commit adds the opcode taxonomy to the IIR layer.  No VM or backend
// implementation exists yet; that is the scope of LANG28B (vm-concurrency crate)
// and later phases.  Backends that encounter concurrency opcodes should return
// `UnsupportedOp` from their validator (as they do today for GC opcodes).

/// Return `true` if `op` is a **task** opcode.
///
/// Tasks are lightweight coroutines managed by the LANG28 cooperative scheduler.
///
/// | Mnemonic | Description |
/// |----------|-------------|
/// | `task_spawn` | Spawn a new task; src = callee + args; dest = `task<T>` |
/// | `task_current` | Get the current task's handle; dest = `task<void>` |
/// | `task_yield` | Cooperatively yield to the scheduler (**may park**) |
/// | `task_sleep` | Park until a deadline passes; src = deadline (**may park**) |
/// | `task_join` | Await a task's result; src = `task<T>`; dest = T (**may park**) |
/// | `task_cancel` | Request cancellation; src = `task<T>` + cancel\_token |
/// | `task_check_cancel` | Poll the current task's cancel flag; dest = bool |
/// | `task_detach` | Detach task from its parent group; src = `task<T>` |
pub fn is_task(op: &str) -> bool {
    matches!(
        op,
        "task_spawn"
            | "task_current"
            | "task_yield"
            | "task_sleep"
            | "task_join"
            | "task_cancel"
            | "task_check_cancel"
            | "task_detach"
    )
}

/// Return `true` if `op` is a **task-group** opcode.
///
/// Task groups implement structured concurrency: all spawned tasks must
/// complete (or be cancelled) before the group is closed.
///
/// | Mnemonic | Description |
/// |----------|-------------|
/// | `group_new` | Create an empty task group; dest = `task_group` |
/// | `group_spawn` | Spawn a task inside a group; src = group + fn + args; dest = `task<T>` |
/// | `group_join` | Wait for all tasks in the group to finish (**may park**) |
/// | `group_cancel` | Cancel all running tasks in the group |
/// | `group_close` | Close group to new spawns (all future spawn calls trap) |
pub fn is_task_group(op: &str) -> bool {
    matches!(
        op,
        "group_new" | "group_spawn" | "group_join" | "group_cancel" | "group_close"
    )
}

/// Return `true` if `op` is a **channel** opcode.
///
/// Channels are typed bounded queues for inter-task communication.
///
/// | Mnemonic | Description |
/// |----------|-------------|
/// | `chan_new` | Create a channel; src = capacity (0 = rendezvous); dest = `channel<T>` |
/// | `chan_send` | Send a value; src = channel + value (**may park** if full) |
/// | `chan_recv` | Receive a value; src = channel; dest = T (**may park** if empty) |
/// | `chan_try_send` | Non-blocking send; dest = bool (true = accepted) |
/// | `chan_try_recv` | Non-blocking receive; dest = `option<T>` (Some = value) |
/// | `chan_close` | Close the send side; subsequent recv returns `None` after drain |
pub fn is_channel(op: &str) -> bool {
    matches!(
        op,
        "chan_new" | "chan_send" | "chan_recv" | "chan_try_send" | "chan_try_recv" | "chan_close"
    )
}

/// Return `true` if `op` is a **select** opcode.
///
/// Select allows a task to wait on whichever of several events fires first.
/// This is analogous to Go's `select`, Erlang's `receive`, or Rust's `tokio::select!`.
///
/// A typical select sequence:
/// ```text
/// s = select_new()
/// arm0 = select_recv(s, ch_a)
/// arm1 = select_send(s, ch_b, value)
/// arm2 = select_timer(s, deadline)
/// fired = select_wait(s)    ; parks until one arm fires
/// ; pattern-match on fired == arm0, arm1, arm2 …
/// ```
///
/// | Mnemonic | Description |
/// |----------|-------------|
/// | `select_new` | Create an empty select set; dest = `select_set` |
/// | `select_recv` | Register a recv arm; src = select\_set + channel; dest = arm\_id |
/// | `select_send` | Register a send arm; src = select\_set + channel + value; dest = arm\_id |
/// | `select_join` | Register a task-join arm; src = select\_set + task; dest = arm\_id |
/// | `select_timer` | Register a timer arm; src = select\_set + deadline; dest = arm\_id |
/// | `select_cancel` | Register a cancel-check arm; src = select\_set + cancel\_token; dest = arm\_id |
/// | `select_wait` | Block until one arm fires; src = select\_set; dest = arm\_id (**may park**) |
/// | `select_default` | Add a no-wait arm (fires immediately if nothing else is ready) |
pub fn is_select(op: &str) -> bool {
    matches!(
        op,
        "select_new"
            | "select_recv"
            | "select_send"
            | "select_join"
            | "select_timer"
            | "select_cancel"
            | "select_wait"
            | "select_default"
    )
}

/// Return `true` if `op` is ANY concurrency opcode (task, group, channel, or select).
///
/// Equivalent to `is_task(op) || is_task_group(op) || is_channel(op) || is_select(op)`.
pub fn is_concurrency(op: &str) -> bool {
    is_task(op) || is_task_group(op) || is_channel(op) || is_select(op)
}

/// Return `true` if `op` may **park** the current task.
///
/// A parking opcode suspends execution and yields control to the scheduler.
/// This implies:
///
/// - A GC safepoint is active (live refs may be relocated by a moving GC).
/// - Any `cancel_token` owned by the current task may be signalled.
/// - The task may resume on a different OS thread (in the M:N scheduler).
///
/// Backends and analysis passes must treat any program point where
/// `is_parking(op)` is true as a potential stack-walk root.
pub fn is_parking(op: &str) -> bool {
    matches!(
        op,
        "task_yield"
            | "task_sleep"
            | "task_join"
            | "chan_send"
            | "chan_recv"
            | "group_join"
            | "select_wait"
    )
}

// ---------------------------------------------------------------------------
// Concurrency type-string helpers (LANG28)
// ---------------------------------------------------------------------------
//
// Concurrency types are encoded as structured strings, similar to `ref<T>`:
//
//   "task<T>"       — handle to a spawned task that produces T
//   "channel<T>"    — typed bounded queue carrying T
//   "task_group"    — an opaque task-group handle (no type parameter)
//   "select_set"    — a collection of select arms waiting to fire
//   "cancel_token"  — a token that can be passed to task_cancel / select_cancel
//   "deadline"      — an absolute point in time (used by task_sleep / select_timer)
//   "option<T>"     — returned by chan_try_recv when no value is available
//
// These type strings are recognised by `is_concurrency_type()` so that the
// IIR type checker and backend validators can reject them cleanly when they
// are not supported.

const TASK_PREFIX: &str = "task<";
const CHANNEL_PREFIX: &str = "channel<";
const OPTION_PREFIX: &str = "option<";

/// Return `true` if `type_hint` is a task type `"task<T>"`.
///
/// ```
/// use interpreter_ir::opcodes::is_task_type;
/// assert!(is_task_type("task<void>"));
/// assert!(is_task_type("task<i32>"));
/// assert!(!is_task_type("task_group"));   // bare group handle, not parameterised
/// ```
pub fn is_task_type(type_hint: &str) -> bool {
    type_hint.starts_with(TASK_PREFIX) && type_hint.ends_with('>')
}

/// Return `true` if `type_hint` is a channel type `"channel<T>"`.
///
/// ```
/// use interpreter_ir::opcodes::is_channel_type;
/// assert!(is_channel_type("channel<i32>"));
/// assert!(is_channel_type("channel<any>"));
/// assert!(!is_channel_type("chan_new"));
/// ```
pub fn is_channel_type(type_hint: &str) -> bool {
    type_hint.starts_with(CHANNEL_PREFIX) && type_hint.ends_with('>')
}

/// Return `true` if `type_hint` is an option type `"option<T>"`.
///
/// `chan_try_recv` returns `"option<T>"` where T is the channel's element type.
///
/// ```
/// use interpreter_ir::opcodes::is_option_type;
/// assert!(is_option_type("option<i32>"));
/// assert!(is_option_type("option<any>"));
/// ```
pub fn is_option_type(type_hint: &str) -> bool {
    type_hint.starts_with(OPTION_PREFIX) && type_hint.ends_with('>')
}

/// Return `true` if `type_hint` is any concurrency-specific type.
///
/// This covers: `task<T>`, `channel<T>`, `option<T>`, `task_group`,
/// `select_set`, `cancel_token`, `deadline`.
///
/// ```
/// use interpreter_ir::opcodes::is_concurrency_type;
/// assert!(is_concurrency_type("task<i32>"));
/// assert!(is_concurrency_type("channel<void>"));
/// assert!(is_concurrency_type("task_group"));
/// assert!(is_concurrency_type("select_set"));
/// assert!(is_concurrency_type("cancel_token"));
/// assert!(is_concurrency_type("deadline"));
/// assert!(is_concurrency_type("option<i32>"));
/// assert!(!is_concurrency_type("i32"));
/// assert!(!is_concurrency_type("ref<u8>"));
/// ```
pub fn is_concurrency_type(type_hint: &str) -> bool {
    is_task_type(type_hint)
        || is_channel_type(type_hint)
        || is_option_type(type_hint)
        || matches!(
            type_hint,
            "task_group" | "select_set" | "cancel_token" | "deadline"
        )
}

/// Construct a `"task<T>"` type string.
///
/// ```
/// use interpreter_ir::opcodes::make_task_type;
/// assert_eq!(make_task_type("i32"),  "task<i32>");
/// assert_eq!(make_task_type("void"), "task<void>");
/// ```
pub fn make_task_type(inner: &str) -> String {
    format!("{TASK_PREFIX}{inner}>")
}

/// Extract the element type from a `"task<T>"` string.
///
/// Returns `None` if `type_hint` is not a task type.
///
/// ```
/// use interpreter_ir::opcodes::unwrap_task_type;
/// assert_eq!(unwrap_task_type("task<i32>"),  Some("i32".to_string()));
/// assert_eq!(unwrap_task_type("task<void>"), Some("void".to_string()));
/// assert_eq!(unwrap_task_type("i32"),        None);
/// ```
pub fn unwrap_task_type(type_hint: &str) -> Option<String> {
    if !is_task_type(type_hint) {
        return None;
    }
    let inner = &type_hint[TASK_PREFIX.len()..type_hint.len() - 1];
    Some(inner.to_string())
}

/// Construct a `"channel<T>"` type string.
///
/// ```
/// use interpreter_ir::opcodes::make_channel_type;
/// assert_eq!(make_channel_type("i32"), "channel<i32>");
/// ```
pub fn make_channel_type(inner: &str) -> String {
    format!("{CHANNEL_PREFIX}{inner}>")
}

/// Extract the element type from a `"channel<T>"` string.
///
/// ```
/// use interpreter_ir::opcodes::unwrap_channel_type;
/// assert_eq!(unwrap_channel_type("channel<i32>"), Some("i32".to_string()));
/// assert_eq!(unwrap_channel_type("i32"), None);
/// ```
pub fn unwrap_channel_type(type_hint: &str) -> Option<String> {
    if !is_channel_type(type_hint) {
        return None;
    }
    let inner = &type_hint[CHANNEL_PREFIX.len()..type_hint.len() - 1];
    Some(inner.to_string())
}

/// Construct an `"option<T>"` type string.
///
/// ```
/// use interpreter_ir::opcodes::make_option_type;
/// assert_eq!(make_option_type("i32"), "option<i32>");
/// ```
pub fn make_option_type(inner: &str) -> String {
    format!("{OPTION_PREFIX}{inner}>")
}

/// Extract the element type from an `"option<T>"` string.
///
/// Returns `None` if `type_hint` is not an option type.
///
/// ```
/// use interpreter_ir::opcodes::unwrap_option_type;
/// assert_eq!(unwrap_option_type("option<i32>"),  Some("i32".to_string()));
/// assert_eq!(unwrap_option_type("option<any>"),  Some("any".to_string()));
/// assert_eq!(unwrap_option_type("i32"),           None);
/// ```
pub fn unwrap_option_type(type_hint: &str) -> Option<String> {
    if !is_option_type(type_hint) {
        return None;
    }
    let inner = &type_hint[OPTION_PREFIX.len()..type_hint.len() - 1];
    Some(inner.to_string())
}

/// Return `true` if `op` is a recognised IIR mnemonic.
///
/// Unknown mnemonics are rejected by the module validator.
pub fn is_known_op(op: &str) -> bool {
    op == "const"
        || is_arithmetic(op)
        || is_bitwise(op)
        || is_cmp(op)
        || is_branch(op)
        || is_control(op)
        || is_memory(op)
        || is_call(op)
        || is_io(op)
        || is_coercion(op)
        || is_heap(op)
        || is_global(op)
        || is_closure_op(op)
        || is_concurrency(op)
}

/// Return `true` if `type_hint` is a concrete (non-dynamic) type.
///
/// Concrete-type instructions are skipped by the profiler — their type is
/// already known at compile time.
pub fn is_concrete_type(type_hint: &str) -> bool {
    CONCRETE_TYPES.contains(&type_hint) || is_ref_type(type_hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_ops_recognised() {
        for op in &["add", "sub", "mul", "div", "mod", "neg"] {
            assert!(is_arithmetic(op), "{op}");
        }
        assert!(!is_arithmetic("jmp"));
    }

    #[test]
    fn cmp_ops_recognised() {
        for op in &["cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"] {
            assert!(is_cmp(op), "{op}");
        }
    }

    #[test]
    fn ref_type_round_trip() {
        assert!(is_ref_type("ref<u8>"));
        assert!(is_ref_type("ref<ref<any>>"));
        assert!(!is_ref_type("u8"));
        assert_eq!(unwrap_ref_type("ref<u8>"), Some("u8".to_string()));
        assert_eq!(unwrap_ref_type("ref<ref<any>>"), Some("ref<any>".to_string()));
        assert_eq!(unwrap_ref_type("u8"), None);
        assert_eq!(make_ref_type("u8"), "ref<u8>");
    }

    #[test]
    fn concrete_type_check() {
        for t in CONCRETE_TYPES {
            assert!(is_concrete_type(t), "{t}");
        }
        assert!(!is_concrete_type("any"));
        assert!(!is_concrete_type("polymorphic"));
        assert!(is_concrete_type("ref<u8>"));
    }

    #[test]
    fn is_known_op_covers_all_categories() {
        for op in &[
            "const", "add", "sub", "and", "cmp_eq", "jmp", "label", "ret",
            "load_reg", "call", "io_in", "cast", "alloc",
            // LANG32 global ops
            "global_load", "global_store",
            // LANG34 closure ops
            "alloc_closure", "call_closure",
        ] {
            assert!(is_known_op(op), "{op}");
        }
        assert!(!is_known_op("tetrad.move"));
    }

    // ── LANG34 closure opcode tests ───────────────────────────────────────────

    #[test]
    fn closure_type_constant() {
        assert_eq!(CLOSURE_TYPE, "closure");
        // "closure" is NOT a concrete scalar type — backends should reject it.
        assert!(!is_concrete_type(CLOSURE_TYPE));
        // "closure" is NOT in the polymorphic/dynamic sentinel set.
        assert_ne!(CLOSURE_TYPE, DYNAMIC_TYPE);
        assert_ne!(CLOSURE_TYPE, POLYMORPHIC_TYPE);
    }

    #[test]
    fn is_closure_op_recognised() {
        assert!(is_closure_op("alloc_closure"), "alloc_closure must be a closure op");
        assert!(is_closure_op("call_closure"), "call_closure must be a closure op");
        // Older call_builtin forms are NOT closure ops (they stay as call ops).
        assert!(!is_closure_op("call_builtin"));
        assert!(!is_closure_op("call"));
        assert!(!is_closure_op("const"));
    }

    #[test]
    fn closure_ops_are_known() {
        assert!(is_known_op("alloc_closure"));
        assert!(is_known_op("call_closure"));
    }

    #[test]
    fn alloc_closure_is_allocating() {
        assert!(is_allocating("alloc_closure"));
        // call_closure does not allocate — it invokes an already-allocated closure.
        assert!(!is_allocating("call_closure"));
    }

    #[test]
    fn closure_ops_are_value_producing() {
        assert!(is_value_producing("alloc_closure"));
        assert!(is_value_producing("call_closure"));
    }

    // ── LANG28 concurrency predicate tests ────────────────────────────────────

    #[test]
    fn task_ops_all_recognised() {
        let task_ops = [
            "task_spawn", "task_current", "task_yield", "task_sleep",
            "task_join", "task_cancel", "task_check_cancel", "task_detach",
        ];
        for op in &task_ops {
            assert!(is_task(op), "{op} should be a task op");
            assert!(is_concurrency(op), "{op} should be a concurrency op");
            assert!(is_known_op(op), "{op} should be a known op");
        }
        // non-task ops should not match
        assert!(!is_task("chan_new"));
        assert!(!is_task("group_new"));
        assert!(!is_task("select_new"));
        assert!(!is_task("add"));
    }

    #[test]
    fn task_group_ops_all_recognised() {
        let group_ops = ["group_new", "group_spawn", "group_join", "group_cancel", "group_close"];
        for op in &group_ops {
            assert!(is_task_group(op), "{op} should be a task-group op");
            assert!(is_concurrency(op), "{op} should be a concurrency op");
        }
        assert!(!is_task_group("task_spawn"));
        assert!(!is_task_group("chan_new"));
    }

    #[test]
    fn channel_ops_all_recognised() {
        let chan_ops = ["chan_new", "chan_send", "chan_recv", "chan_try_send", "chan_try_recv", "chan_close"];
        for op in &chan_ops {
            assert!(is_channel(op), "{op} should be a channel op");
            assert!(is_concurrency(op), "{op} should be a concurrency op");
        }
        assert!(!is_channel("task_spawn"));
        assert!(!is_channel("select_new"));
    }

    #[test]
    fn select_ops_all_recognised() {
        let sel_ops = [
            "select_new", "select_recv", "select_send", "select_join",
            "select_timer", "select_cancel", "select_wait", "select_default",
        ];
        for op in &sel_ops {
            assert!(is_select(op), "{op} should be a select op");
            assert!(is_concurrency(op), "{op} should be a concurrency op");
        }
        assert!(!is_select("chan_recv"));
        assert!(!is_select("task_join"));
    }

    #[test]
    fn parking_ops_are_a_strict_subset_of_concurrency() {
        let parking_ops = [
            "task_yield", "task_sleep", "task_join",
            "chan_send", "chan_recv",
            "group_join", "select_wait",
        ];
        for op in &parking_ops {
            assert!(is_parking(op), "{op} should be parking");
            assert!(is_concurrency(op), "{op} should also be concurrency");
        }
        // non-parking ops
        assert!(!is_parking("task_spawn"));
        assert!(!is_parking("chan_new"));
        assert!(!is_parking("add"));
    }

    #[test]
    fn concurrency_value_producing_ops() {
        // These ops produce a dest and must be in is_value_producing
        let vp = [
            "task_spawn", "task_current", "task_join", "task_check_cancel",
            "group_new", "group_spawn",
            "chan_new", "chan_recv", "chan_try_send", "chan_try_recv",
            "select_new", "select_recv", "select_send", "select_join",
            "select_timer", "select_cancel", "select_wait", "select_default",
        ];
        for op in &vp {
            assert!(is_value_producing(op), "{op} should be value-producing");
        }
        // These ops do NOT produce a value
        for op in &["task_yield", "task_sleep", "task_cancel", "task_detach",
                    "group_join", "group_cancel", "group_close",
                    "chan_send", "chan_close"] {
            assert!(!is_value_producing(op), "{op} should NOT be value-producing");
        }
    }

    #[test]
    fn concurrency_side_effecting_ops() {
        let se = [
            "task_yield", "task_sleep", "task_cancel", "task_detach",
            "group_join", "group_cancel", "group_close",
            "chan_send", "chan_close",
        ];
        for op in &se {
            assert!(has_side_effects(op), "{op} should have side effects");
        }
        // Value-producing concurrency ops are NOT side-effecting under the IIR model
        // (they may park but they produce a well-defined value and are not DCE-exempt
        //  solely for side-effect reasons — they would be removed if the result is dead)
        assert!(!has_side_effects("task_spawn"));
        assert!(!has_side_effects("chan_new"));
        assert!(!has_side_effects("select_new"));
    }

    #[test]
    fn concurrency_allocating_ops() {
        let alloc_ops = ["task_spawn", "group_new", "group_spawn", "chan_new", "select_new"];
        for op in &alloc_ops {
            assert!(is_allocating(op), "{op} should be allocating");
        }
        // Non-allocating concurrency ops
        assert!(!is_allocating("task_yield"));
        assert!(!is_allocating("chan_send"));
        assert!(!is_allocating("select_wait"));
    }

    // ── LANG28 concurrency type helpers ───────────────────────────────────────

    #[test]
    fn task_type_round_trip() {
        assert!(is_task_type("task<void>"));
        assert!(is_task_type("task<i32>"));
        assert!(is_task_type("task<any>"));
        assert!(!is_task_type("task_group"));   // bare handle, no angle brackets
        assert!(!is_task_type("i32"));
        assert!(!is_task_type("task<"));        // malformed (no closing >)

        assert_eq!(make_task_type("i32"),  "task<i32>");
        assert_eq!(make_task_type("void"), "task<void>");

        assert_eq!(unwrap_task_type("task<i32>"),  Some("i32".to_string()));
        assert_eq!(unwrap_task_type("task<void>"), Some("void".to_string()));
        assert_eq!(unwrap_task_type("task<any>"),  Some("any".to_string()));
        assert_eq!(unwrap_task_type("i32"),         None);
    }

    #[test]
    fn channel_type_round_trip() {
        assert!(is_channel_type("channel<i32>"));
        assert!(is_channel_type("channel<any>"));
        assert!(!is_channel_type("chan_new"));
        assert!(!is_channel_type("i32"));

        assert_eq!(make_channel_type("i32"), "channel<i32>");
        assert_eq!(make_channel_type("u8"),  "channel<u8>");

        assert_eq!(unwrap_channel_type("channel<i32>"), Some("i32".to_string()));
        assert_eq!(unwrap_channel_type("channel<any>"), Some("any".to_string()));
        assert_eq!(unwrap_channel_type("i32"),           None);
    }

    #[test]
    fn option_type_round_trip() {
        assert!(is_option_type("option<i32>"));
        assert!(is_option_type("option<any>"));
        assert!(!is_option_type("i32"));
        assert!(!is_option_type("option"));

        assert_eq!(make_option_type("i32"), "option<i32>");
        assert_eq!(make_option_type("u8"),  "option<u8>");

        assert_eq!(unwrap_option_type("option<i32>"),  Some("i32".to_string()));
        assert_eq!(unwrap_option_type("option<any>"),  Some("any".to_string()));
        assert_eq!(unwrap_option_type("i32"),           None);
    }

    #[test]
    fn is_concurrency_type_covers_all_variants() {
        // Parameterised types
        assert!(is_concurrency_type("task<i32>"));
        assert!(is_concurrency_type("task<void>"));
        assert!(is_concurrency_type("channel<i32>"));
        assert!(is_concurrency_type("channel<any>"));
        assert!(is_concurrency_type("option<i32>"));
        // Bare concurrency types
        assert!(is_concurrency_type("task_group"));
        assert!(is_concurrency_type("select_set"));
        assert!(is_concurrency_type("cancel_token"));
        assert!(is_concurrency_type("deadline"));
        // Non-concurrency types
        assert!(!is_concurrency_type("i32"));
        assert!(!is_concurrency_type("ref<u8>"));
        assert!(!is_concurrency_type("any"));
        assert!(!is_concurrency_type("bool"));
    }

    #[test]
    fn is_known_op_includes_concurrency() {
        for op in &[
            "task_spawn", "task_current", "task_yield", "task_sleep",
            "task_join", "task_cancel", "task_check_cancel", "task_detach",
            "group_new", "group_spawn", "group_join", "group_cancel", "group_close",
            "chan_new", "chan_send", "chan_recv", "chan_try_send", "chan_try_recv", "chan_close",
            "select_new", "select_recv", "select_send", "select_join",
            "select_timer", "select_cancel", "select_wait", "select_default",
        ] {
            assert!(is_known_op(op), "{op} must be in is_known_op");
        }
    }
}
