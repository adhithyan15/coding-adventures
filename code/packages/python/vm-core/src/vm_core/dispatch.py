"""Dispatch loop and standard opcode handlers for vm-core.

The dispatch loop is a tight ``while`` cycle:

1. Peek at the current frame's instruction
2. Look up the handler in the opcode table
3. Call the handler — it reads from ``frame.registers`` / ``frame.name_to_reg``
   and writes results back
4. If profiling is enabled, observe the result
5. Advance ``frame.ip``

Handlers are plain Python functions with the signature::

    def handle_XXX(vm: "VMCore", frame: VMFrame, instr: IIRInstr) -> Any:
        ...

They return the value stored in ``instr.dest`` (or None for void ops).

Standard opcodes
----------------
The standard opcode table covers every mnemonic in ``interpreter_ir.ALL_OPS``
plus ``"const"``.  Languages may override individual handlers by passing a
``opcodes`` dict to VMCore — entries in that dict shadow the standard table.

u8_wrap
-------
When ``VMCore`` is constructed with ``u8_wrap=True`` (Tetrad mode), every
arithmetic result is masked with ``& 0xFF`` before being stored.  This
mask is applied by the arithmetic handlers, not the dispatch loop itself.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from interpreter_ir import IIRInstr

from vm_core.errors import FrameOverflowError, UnknownOpcodeError, VMInterrupt
from vm_core.frame import VMFrame

if TYPE_CHECKING:
    from vm_core.core import VMCore


# ---------------------------------------------------------------------------
# Dispatch loop
# ---------------------------------------------------------------------------

def run_dispatch_loop(vm: VMCore) -> Any:
    """Execute instructions until the frame stack is empty.

    Returns the last value produced by a ``ret`` instruction in the root frame,
    or ``None`` if no ``ret`` was executed.
    """
    return_value: Any = None

    while vm._frames:
        frame = vm._frames[-1]

        if frame.ip >= len(frame.fn.instructions):
            vm._frames.pop()
            continue

        if vm._interrupted:
            vm._interrupted = False
            raise VMInterrupt("execution interrupted")

        instr = frame.fn.instructions[frame.ip]
        frame.ip += 1

        result = _dispatch_one(vm, frame, instr)
        vm._metrics_instrs += 1

        if vm._profiler_enabled and instr.dest is not None and result is not None:
            vm._profiler.observe(instr, result)

        if instr.op in {"ret", "ret_void"}:
            return_value = result

    return return_value


def _dispatch_one(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    handler = vm._opcode_table.get(instr.op)
    if handler is None:
        raise UnknownOpcodeError(
            f"no handler for opcode {instr.op!r} in function {frame.fn.name!r}"
        )
    return handler(vm, frame, instr)


# ---------------------------------------------------------------------------
# Standard opcode handlers
# ---------------------------------------------------------------------------

def _wrap(vm: VMCore, v: int) -> int:
    return v & 0xFF if vm._u8_wrap else v


# --- Constant ---

def handle_const(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    value = instr.srcs[0]
    if instr.dest:
        frame.assign(instr.dest, value)
    return value


# --- Arithmetic ---

def handle_add(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = _wrap(vm, a + b)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_sub(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = _wrap(vm, a - b)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_mul(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = _wrap(vm, a * b)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_div(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = _wrap(vm, a // b)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_mod(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = _wrap(vm, a % b)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_neg(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    result = _wrap(vm, -a)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


# --- Bitwise ---

def handle_and(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = a & b
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_or(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = a | b
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_xor(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    b = frame.resolve(instr.srcs[1])
    result = a ^ b
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_not(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    result = ~a
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_shl(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    n = frame.resolve(instr.srcs[1])
    result = a << n
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_shr(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    a = frame.resolve(instr.srcs[0])
    n = frame.resolve(instr.srcs[1])
    result = a >> n
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


# --- Comparisons ---

def handle_cmp_eq(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> bool:
    r = frame.resolve(instr.srcs[0]) == frame.resolve(instr.srcs[1])
    if instr.dest:
        frame.assign(instr.dest, r)
    return r


def handle_cmp_ne(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> bool:
    r = frame.resolve(instr.srcs[0]) != frame.resolve(instr.srcs[1])
    if instr.dest:
        frame.assign(instr.dest, r)
    return r


def handle_cmp_lt(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> bool:
    r = frame.resolve(instr.srcs[0]) < frame.resolve(instr.srcs[1])
    if instr.dest:
        frame.assign(instr.dest, r)
    return r


def handle_cmp_le(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> bool:
    r = frame.resolve(instr.srcs[0]) <= frame.resolve(instr.srcs[1])
    if instr.dest:
        frame.assign(instr.dest, r)
    return r


def handle_cmp_gt(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> bool:
    r = frame.resolve(instr.srcs[0]) > frame.resolve(instr.srcs[1])
    if instr.dest:
        frame.assign(instr.dest, r)
    return r


def handle_cmp_ge(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> bool:
    r = frame.resolve(instr.srcs[0]) >= frame.resolve(instr.srcs[1])
    if instr.dest:
        frame.assign(instr.dest, r)
    return r


# --- Control flow ---

def handle_label(_vm: VMCore, _frame: VMFrame, _instr: IIRInstr) -> None:
    """Labels are no-ops at runtime; they only exist for branch resolution."""
    return None


def handle_jmp(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    # ``frame.ip`` has already been advanced past this instruction by the
    # dispatch loop, so the *source* index of this jump is ip - 1.  We use
    # it for back-edge detection (a back-edge is any jump whose target
    # index is strictly less than the source index).
    source_ip = frame.ip - 1
    target_ip = frame.fn.label_index(str(instr.srcs[0]))
    if target_ip < source_ip:
        _bump_loop(vm, frame.fn.name, source_ip)
    frame.ip = target_ip
    return None


def handle_jmp_if_true(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    source_ip = frame.ip - 1
    cond = frame.resolve(instr.srcs[0])
    taken = bool(cond)
    _bump_branch(vm, frame.fn.name, source_ip, taken)
    if taken:
        target_ip = frame.fn.label_index(str(instr.srcs[1]))
        if target_ip < source_ip:
            _bump_loop(vm, frame.fn.name, source_ip)
        frame.ip = target_ip
    return None


def handle_jmp_if_false(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    source_ip = frame.ip - 1
    cond = frame.resolve(instr.srcs[0])
    # For a jmp_if_false, "taken" means the branch was followed, i.e.
    # the condition was false.  We record it in the same (taken vs
    # not-taken) shape so consumers get a uniform view regardless of
    # which conditional opcode was used.
    taken = not bool(cond)
    _bump_branch(vm, frame.fn.name, source_ip, taken)
    if taken:
        target_ip = frame.fn.label_index(str(instr.srcs[1]))
        if target_ip < source_ip:
            _bump_loop(vm, frame.fn.name, source_ip)
        frame.ip = target_ip
    return None


# ---------------------------------------------------------------------------
# Branch / loop counter helpers
# ---------------------------------------------------------------------------
#
# These mutate the live dicts on ``VMCore``.  They are private to this
# module — external callers read the deep-copied snapshot via
# ``VMCore.metrics()`` or the typed accessors ``VMCore.branch_profile``
# and ``VMCore.loop_iterations``.
# ---------------------------------------------------------------------------


def _bump_branch(vm: VMCore, fn_name: str, source_ip: int, taken: bool) -> None:
    """Record one conditional-branch observation on the live metrics dicts."""
    from vm_core.metrics import BranchStats
    fn_stats = vm._branch_stats.setdefault(fn_name, {})
    stats = fn_stats.get(source_ip)
    if stats is None:
        stats = BranchStats()
        fn_stats[source_ip] = stats
    if taken:
        stats.taken_count += 1
    else:
        stats.not_taken_count += 1


def _bump_loop(vm: VMCore, fn_name: str, source_ip: int) -> None:
    """Record one back-edge traversal on the live metrics dicts."""
    fn_loops = vm._loop_back_edges.setdefault(fn_name, {})
    fn_loops[source_ip] = fn_loops.get(source_ip, 0) + 1


def handle_ret(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    value = frame.resolve(instr.srcs[0]) if instr.srcs else None
    caller_frame = _pop_frame(vm, frame)
    if caller_frame is not None and frame.return_dest is not None:
        caller_frame.registers[frame.return_dest] = value
        if instr.dest and frame.return_dest < len(frame.registers):
            pass  # dest is in caller
    return value


def handle_ret_void(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    _pop_frame(vm, frame)
    return None


def _pop_frame(vm: VMCore, frame: VMFrame) -> VMFrame | None:
    """Pop the current frame; return the new top frame (caller), or None."""
    vm._frames.pop()
    return vm._frames[-1] if vm._frames else None


# --- Memory / registers ---

def handle_load_reg(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    idx = int(frame.resolve(instr.srcs[0]))
    value = frame.registers[idx]
    if instr.dest:
        frame.assign(instr.dest, value)
    return value


def handle_store_reg(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    idx = int(frame.resolve(instr.srcs[0]))
    value = frame.resolve(instr.srcs[1])
    frame.registers[idx] = value
    return None


def handle_load_mem(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    addr = int(frame.resolve(instr.srcs[0]))
    value = vm._memory.get(addr, 0)
    if instr.dest:
        frame.assign(instr.dest, value)
    return value


def handle_store_mem(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    addr = int(frame.resolve(instr.srcs[0]))
    value = frame.resolve(instr.srcs[1])
    vm._memory[addr] = value
    return None


# --- Function calls ---

def handle_call(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    fn_name = str(instr.srcs[0])

    # JIT handler takes priority.
    jit_handler = vm._jit_handlers.get(fn_name)
    if jit_handler is not None:
        args = [frame.resolve(s) for s in instr.srcs[1:]]
        result = jit_handler(args)
        vm._metrics_jit_hits += 1
        if instr.dest:
            frame.assign(instr.dest, result)
        return result

    # Interpreter path.
    if vm._module is None or (callee := vm._module.get_function(fn_name)) is None:
        raise UnknownOpcodeError(f"function {fn_name!r} not found in module")

    if len(vm._frames) >= vm._max_frames:
        raise FrameOverflowError(
            f"call stack depth {vm._max_frames} exceeded calling {fn_name!r}"
        )

    # Allocate return-value register in caller.
    ret_reg: int | None = None
    if instr.dest:
        if instr.dest not in frame.name_to_reg:
            ret_reg = len(frame.name_to_reg)
            frame.name_to_reg[instr.dest] = ret_reg
        else:
            ret_reg = frame.name_to_reg[instr.dest]

    callee_frame = VMFrame.for_function(callee, return_dest=ret_reg)

    # Copy arguments into callee registers (params 0..N-1).
    args = instr.srcs[1:]
    for i, arg_src in enumerate(args[: len(callee.params)]):
        callee_frame.registers[i] = frame.resolve(arg_src)

    vm._frames.append(callee_frame)
    vm._metrics_frames += 1
    vm._fn_call_counts[fn_name] = vm._fn_call_counts.get(fn_name, 0) + 1
    return None  # result stored when callee executes ret


def handle_call_builtin(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    name = str(instr.srcs[0])
    args = [frame.resolve(s) for s in instr.srcs[1:]]
    result = vm._builtins.call(name, args)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


# --- I/O ---

def handle_io_in(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    port = int(frame.resolve(instr.srcs[0]))
    value = vm._io_ports.get(port, 0)
    if instr.dest:
        frame.assign(instr.dest, value)
    return value


def handle_io_out(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    port = int(frame.resolve(instr.srcs[0]))
    value = frame.resolve(instr.srcs[1])
    vm._io_ports[port] = value
    return None


# --- Coercions ---

def handle_cast(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    value = frame.resolve(instr.srcs[0])
    target = str(instr.srcs[1]) if len(instr.srcs) > 1 else instr.type_hint
    if target in {"u8", "u16", "u32", "u64"}:
        value = int(value)
        if target == "u8":
            value &= 0xFF
        elif target == "u16":
            value &= 0xFFFF
        elif target == "u32":
            value &= 0xFFFFFFFF
    elif target == "bool":
        value = bool(value)
    elif target == "str":
        value = str(value)
    if instr.dest:
        frame.assign(instr.dest, value)
    return value


def handle_type_assert(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Runtime type assertion — raises VMError if the value is the wrong type."""
    from vm_core.errors import VMError
    value = frame.resolve(instr.srcs[0])
    expected = str(instr.srcs[1]) if len(instr.srcs) > 1 else instr.type_hint
    actual = _runtime_type(value)
    if actual != expected:
        raise VMError(
            f"type_assert failed in {frame.fn.name!r}: "
            f"expected {expected!r}, got {actual!r} ({value!r})"
        )
    return None


def _runtime_type(value: Any) -> str:
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        if 0 <= value <= 255:
            return "u8"
        if 0 <= value <= 65535:
            return "u16"
        if 0 <= value <= 0xFFFF_FFFF:
            return "u32"
        return "u64"
    if isinstance(value, float):
        return "f64"
    if isinstance(value, str):
        return "str"
    return "any"


# ---------------------------------------------------------------------------
# Standard opcode table
# ---------------------------------------------------------------------------

STANDARD_OPCODES: dict[str, Any] = {
    "const": handle_const,
    "add": handle_add,
    "sub": handle_sub,
    "mul": handle_mul,
    "div": handle_div,
    "mod": handle_mod,
    "neg": handle_neg,
    "and": handle_and,
    "or": handle_or,
    "xor": handle_xor,
    "not": handle_not,
    "shl": handle_shl,
    "shr": handle_shr,
    "cmp_eq": handle_cmp_eq,
    "cmp_ne": handle_cmp_ne,
    "cmp_lt": handle_cmp_lt,
    "cmp_le": handle_cmp_le,
    "cmp_gt": handle_cmp_gt,
    "cmp_ge": handle_cmp_ge,
    "label": handle_label,
    "jmp": handle_jmp,
    "jmp_if_true": handle_jmp_if_true,
    "jmp_if_false": handle_jmp_if_false,
    "ret": handle_ret,
    "ret_void": handle_ret_void,
    "load_reg": handle_load_reg,
    "store_reg": handle_store_reg,
    "load_mem": handle_load_mem,
    "store_mem": handle_store_mem,
    "call": handle_call,
    "call_builtin": handle_call_builtin,
    "io_in": handle_io_in,
    "io_out": handle_io_out,
    "cast": handle_cast,
    "type_assert": handle_type_assert,
}
