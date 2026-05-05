"""Opcode category frozensets and type-string helpers for InterpreterIR.

These let vm-core, jit-core, and IR passes classify instructions
without comparing against long lists of mnemonic strings.

Usage::

    from interpreter_ir.opcodes import ARITHMETIC_OPS, CMP_OPS, is_ref_type
    if instr.op in ARITHMETIC_OPS:
        ...
    if is_ref_type(instr.type_hint):
        ...
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Type constants
# ---------------------------------------------------------------------------

CONCRETE_TYPES: frozenset[str] = frozenset(
    {"u8", "u16", "u32", "u64", "bool", "str"}
)

# The "unknown" type used by dynamically typed languages before profiling.
DYNAMIC_TYPE = "any"

# Sentinel placed by the vm-core profiler when multiple types are observed.
POLYMORPHIC_TYPE = "polymorphic"

# ---------------------------------------------------------------------------
# Reference-type encoding (LANG16)
# ---------------------------------------------------------------------------
#
# Heap pointers are typed as ``"ref<T>"`` where ``T`` is the pointee type.
# Examples::
#
#     "ref<u8>"        → pointer to a heap-allocated u8 (a "boxed" byte)
#     "ref<any>"       → pointer to a heap object of unknown shape
#                        (Lisp's default — every cons cell is ref<any>)
#     "ref<ref<any>>"  → pointer to a pointer (e.g. a cons cell's `cdr` slot)
#
# The string-form encoding keeps the rest of the type system unchanged.
# Anything that needs to recognise ref types calls :func:`is_ref_type` or
# :func:`unwrap_ref_type`.
# ---------------------------------------------------------------------------

REF_PREFIX = "ref<"
REF_SUFFIX = ">"


def is_ref_type(type_hint: str) -> bool:
    """Return True if ``type_hint`` is a heap-reference type ``ref<T>``."""
    return type_hint.startswith(REF_PREFIX) and type_hint.endswith(REF_SUFFIX)


def unwrap_ref_type(type_hint: str) -> str | None:
    """Return ``T`` for ``ref<T>``, else ``None``.

    Examples::

        unwrap_ref_type("ref<u8>")        == "u8"
        unwrap_ref_type("ref<ref<any>>")  == "ref<any>"
        unwrap_ref_type("u8")             is None
    """
    if not is_ref_type(type_hint):
        return None
    return type_hint[len(REF_PREFIX):-len(REF_SUFFIX)]


def make_ref_type(inner: str) -> str:
    """Wrap ``inner`` as a reference type.  Inverse of :func:`unwrap_ref_type`."""
    return f"{REF_PREFIX}{inner}{REF_SUFFIX}"


# ---------------------------------------------------------------------------
# Opcode categories
# ---------------------------------------------------------------------------

ARITHMETIC_OPS: frozenset[str] = frozenset(
    {"add", "sub", "mul", "div", "mod", "neg"}
)

BITWISE_OPS: frozenset[str] = frozenset(
    {"and", "or", "xor", "not", "shl", "shr"}
)

CMP_OPS: frozenset[str] = frozenset(
    {"cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"}
)

BRANCH_OPS: frozenset[str] = frozenset(
    {"jmp", "jmp_if_true", "jmp_if_false", "branch_err"}
)

CONTROL_OPS: frozenset[str] = frozenset(
    {"label", "ret", "ret_void"}
)

MEMORY_OPS: frozenset[str] = frozenset(
    {"load_reg", "store_reg", "load_mem", "store_mem"}
)

CALL_OPS: frozenset[str] = frozenset(
    {"call", "call_builtin"}
)

IO_OPS: frozenset[str] = frozenset(
    {"io_in", "io_out"}
)

COERCION_OPS: frozenset[str] = frozenset(
    {"cast", "type_assert"}
)

# ---------------------------------------------------------------------------
# Heap / GC opcodes (LANG16)
# ---------------------------------------------------------------------------
#
# Programs that don't allocate ignore this entire set — these opcodes
# never appear in their IIR and the GC subsystem is never wired in.
# Programs that DO allocate use these seven opcodes to talk to the
# heap / GC; everything richer (weak refs, finalizers, generational
# barriers) builds on top in library code.
# ---------------------------------------------------------------------------

HEAP_OPS: frozenset[str] = frozenset(
    {
        "alloc",        # dest = heap-alloc(size, kind);   may_alloc
        "box",          # dest = heap-alloc-and-store(value);  may_alloc
        "unbox",        # dest = *ref;  trap on null
        "field_load",   # dest = *(ref + offset)
        "field_store",  # *(ref + offset) = value;  may emit write barrier
        "is_null",      # dest = (ref == NULL)
        "safepoint",    # yield to GC if collection pending; may_alloc
    }
)

# All ops that produce a value (have a non-None dest).
_VALUE_EXTRA: frozenset[str] = frozenset(
    {"const", "load_reg", "load_mem", "call", "call_builtin", "io_in", "cast"}
)
_HEAP_VALUE_OPS: frozenset[str] = frozenset(
    {"alloc", "box", "unbox", "field_load", "is_null"}
)
VALUE_OPS: frozenset[str] = (
    ARITHMETIC_OPS | BITWISE_OPS | CMP_OPS | _VALUE_EXTRA | _HEAP_VALUE_OPS
)

# ---------------------------------------------------------------------------
# VMCOND00 Phase 1 — checked syscall + error branch
# ---------------------------------------------------------------------------
#
# ``syscall_checked`` invokes a SYSCALL00 host syscall by number and stores
# both the success value and an error code into named registers.  It never
# traps — errors are surfaced as a non-zero error register.
#
# ``branch_err`` is the companion branch: it jumps to a label when the error
# register (populated by syscall_checked) is non-zero, and falls through on
# success.  It lives in BRANCH_OPS so that live-variable analysis and
# control-flow passes treat it as a conditional branch.
#
# IIR operand conventions:
#
#   syscall_checked:
#     srcs = [n (immediate int), arg_reg, val_dst, err_dst]
#     dest = None   (both output registers are named in srcs)
#
#   branch_err:
#     srcs = [err_reg, label_str]
#     dest = None
#
# The distinction from ``jmp_if_false`` / ``jmp_if_true`` is semantic:
# ``branch_err`` explicitly documents that it consumes an *error code*, not
# a Boolean condition.  Backends can exploit this typing to route to
# exception-handling machinery rather than a plain conditional jump.
#
# Note: ``syscall_checked`` is in SIDE_EFFECT_OPS (it performs I/O) and NOT
# in VALUE_OPS (it has two output slots in srcs, not a single dest).
SYSCALL_CHECKED_OPS: frozenset[str] = frozenset({"syscall_checked"})

# ---------------------------------------------------------------------------
# VMCOND00 Phase 2 — unwind exceptions (THROW)
# ---------------------------------------------------------------------------
#
# ``throw`` unwinds the call stack searching for a matching entry in the
# function's static exception table.  It is fundamentally different from the
# BRANCH_OPS (which are intra-function conditional jumps) and from CONTROL_OPS
# (which are unconditional or function-return control flow).
#
# ``throw`` is not in BRANCH_OPS because it may pop multiple frames — it is
# inter-frame control flow, not intra-function branching.
# ``throw`` is not in CONTROL_OPS because it does not return from the current
# function in the same way ``ret`` / ``ret_void`` do.
# ``throw`` is not in VALUE_OPS because it produces no value in a dest
# register (the caught condition is written to the *handler's* val_reg, which
# is described in the exception table entry, not in the throw instruction).
#
# IIR operand convention:
#
#   throw:
#     srcs = [condition_reg]   — register holding the condition object to throw
#     dest = None
#
# The VM THROW handler:
#   1. Reads condition = frame.resolve(srcs[0])
#   2. Searches vm._frames from innermost to outermost:
#        for entry in frame.fn.exception_table:
#            if entry.from_ip <= (frame.ip - 1) < entry.to_ip:
#                if matches(condition, entry.type_id):
#                    frame.ip = entry.handler_ip
#                    frame.assign(entry.val_reg, condition)
#                    return
#        vm._frames.pop()  # no match — propagate to caller
#   3. If the stack is exhausted, raises UncaughtConditionError.
THROW_OPS: frozenset[str] = frozenset({"throw"})

# ---------------------------------------------------------------------------
# VMCOND00 Phase 3 — dynamic handlers (Layer 3)
# ---------------------------------------------------------------------------
#
# Five opcodes implementing the Layer 3 condition-handler protocol.  Layer 3
# is non-unwinding: when a matching handler is found by SIGNAL/ERROR/WARN, the
# VM pushes a handler invocation frame ON TOP of the current call stack without
# disturbing any of the frames below it.  The call stack beneath the handler is
# fully intact; code in those frames can still expose restarts (Layer 4) that
# the handler can invoke.
#
# IIR operand conventions:
#
#   push_handler:
#     dest = None
#     srcs = [type_id_str, fn_reg]
#     type_id_str — immediate string: "*" (catch-all) or type name to match.
#     fn_reg      — register name holding the handler callable (string key
#                   into the IIR module's function table, or a closure value
#                   in Phase 4).
#
#   pop_handler:
#     dest = None
#     srcs = []
#     Pops the most recently pushed handler.  Must be paired with a
#     PUSH_HANDLER in the same function on every control-flow path that
#     exits the guarded region.
#
#   signal:
#     dest = None
#     srcs = [condition_reg]
#     Walk the handler chain; call first match non-unwinding.  If no match,
#     continue execution (signal is always a no-op when unhandled).
#
#   error:
#     dest = None
#     srcs = [condition_reg]
#     Walk the handler chain like SIGNAL.  If no match, raise
#     UncaughtConditionError (the unhandled condition aborts the thread).
#     If a Layer 2 exception table entry also covers the current IP and type,
#     that takes priority — the error degrades to a THROW.
#
#   warn:
#     dest = None
#     srcs = [condition_reg]
#     Walk the handler chain like SIGNAL.  If no match, emit the condition's
#     repr to stderr and continue execution (WARN never aborts).
#
# Set membership:
#   - IN  SIDE_EFFECT_OPS: all five modify global state (the handler chain or
#     stderr) and may invoke arbitrary code.
#   - IN  ALL_OPS.
#   - NOT in VALUE_OPS: none produce a dest register value.
#   - NOT in BRANCH_OPS: the handler invocation is not a static intra-function
#     conditional jump; it is a dynamic call whose target is on the chain.
#   - NOT in CONTROL_OPS: they don't return from the current function.
HANDLER_OPS: frozenset[str] = frozenset(
    {"push_handler", "pop_handler", "signal", "error", "warn"}
)

# All ops that have side effects beyond producing a value.
SIDE_EFFECT_OPS: frozenset[str] = (
    BRANCH_OPS
    | CONTROL_OPS
    | SYSCALL_CHECKED_OPS
    | THROW_OPS
    | HANDLER_OPS
    | frozenset({"store_reg", "store_mem", "io_out", "type_assert"})
    | frozenset({"field_store", "safepoint"})
)

# Heap ops whose execution may trigger a GC cycle (collection happens
# at safepoints, and these are the safepoints).  Frontends should set
# ``IIRInstr.may_alloc=True`` for every emission of these opcodes plus
# any ``call`` whose callee transitively allocates.
ALLOCATING_OPS: frozenset[str] = frozenset({"alloc", "box", "safepoint"})

# All known opcodes.
ALL_OPS: frozenset[str] = (
    ARITHMETIC_OPS
    | BITWISE_OPS
    | CMP_OPS
    | BRANCH_OPS
    | CONTROL_OPS
    | MEMORY_OPS
    | CALL_OPS
    | IO_OPS
    | COERCION_OPS
    | HEAP_OPS
    | SYSCALL_CHECKED_OPS
    | THROW_OPS
    | HANDLER_OPS
    | frozenset({"const"})
)
