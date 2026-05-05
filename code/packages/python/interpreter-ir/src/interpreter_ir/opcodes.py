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

# All ops that have side effects beyond producing a value.
SIDE_EFFECT_OPS: frozenset[str] = (
    BRANCH_OPS
    | CONTROL_OPS
    | SYSCALL_CHECKED_OPS
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
    | frozenset({"const"})
)
