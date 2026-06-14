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

from vm_core.debug import StepMode
from vm_core.errors import (
    FrameOverflowError,
    HandlerChainError,
    RestartChainError,
    UnboundExitTagError,
    UncaughtConditionError,
    UnknownOpcodeError,
    VMInterrupt,
)
from vm_core.exit_chain import ExitPointNode
from vm_core.frame import VMFrame
from vm_core.handler_chain import HandlerNode
from vm_core.restart_chain import RestartNode

if TYPE_CHECKING:
    from vm_core.core import VMCore


# ---------------------------------------------------------------------------
# Dispatch loop
# ---------------------------------------------------------------------------

def run_dispatch_loop(vm: VMCore) -> Any:
    """Execute instructions until the frame stack is empty.

    Returns the last value produced by a ``ret`` instruction in the root frame,
    or ``None`` if no ``ret`` was executed.

    If ``vm._tracer`` is not None, every dispatched instruction produces
    a ``VMTrace`` record — see :mod:`vm_core.tracer`.  The tracing path
    takes two extra ``list`` copies per instruction (register file
    before/after), so it is opt-in through ``VMCore.execute_traced``.
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
        ip_before = frame.ip
        frame.ip += 1

        # Snapshot state before dispatch when tracing is active.  We
        # capture the pre-observation count too so we can tell whether
        # the profiler produced a slot-delta during this instruction,
        # and the frame depth (since a ``ret`` will pop the frame off
        # the VM stack and we'd lose the original depth).
        # LANG06: fire debug hooks / check breakpoints / honour step mode.
        # This block is guarded by ``vm._debug_mode`` so it costs one
        # boolean check per instruction on the normal (non-debug) path.
        if vm._debug_mode:
            _check_debug_pause(vm, frame, instr, ip_before)

        # LANG18: record instruction execution for coverage analysis.
        # Guarded by ``vm._coverage_mode`` — one boolean check overhead.
        # Coverage and debug mode are orthogonal; both can be active at once.
        if vm._coverage_mode:
            fn_cov = vm._coverage.get(frame.fn.name)
            if fn_cov is None:
                fn_cov = set()
                vm._coverage[frame.fn.name] = fn_cov
            fn_cov.add(ip_before)

        tracing = vm._tracer is not None
        regs_before: list[Any] | None = None
        obs_count_before: int = 0
        depth_before: int = 0
        if tracing:
            regs_before = frame.registers.snapshot()
            obs_count_before = instr.observation_count
            depth_before = _compute_depth(vm, frame)

        try:
            result = _dispatch_one(vm, frame, instr)
        except Exception as exc:
            # LANG06: fire on_exception for unhandled errors so the debug
            # adapter can show a post-mortem stack trace before the VM exits.
            if vm._debug_mode and vm._debug_hooks is not None:
                try:
                    vm._debug_hooks.on_exception(frame, exc)
                except Exception:
                    pass  # adapter errors must never mask the original
            raise
        vm._metrics_instrs += 1

        if vm._profiler_enabled and instr.dest is not None and result is not None:
            vm._profiler.observe(instr, result)

        if tracing:
            slot_delta = []
            if instr.observation_count != obs_count_before and instr.observed_slot:
                slot_delta = [(ip_before, instr.observed_slot)]
            # The dispatch may have popped the frame (ret); in that case
            # the post-register snapshot should come from whatever frame
            # was active at dispatch time — ``frame`` is still a live
            # object even if it's no longer on the VM's frame stack.
            regs_after = frame.registers.snapshot()
            vm._tracer.observe(
                frame_depth=depth_before,
                fn_name=frame.fn.name,
                ip=ip_before,
                instr=instr,
                registers_before=regs_before or [],
                registers_after=regs_after,
                slot_delta=slot_delta,
            )

        if instr.op in {"ret", "ret_void"}:
            return_value = result

    return return_value


def _compute_depth(vm: VMCore, frame: VMFrame) -> int:
    """Compute a frame's depth by its position on the VM's frame stack.

    ``VMFrame`` does not carry a native ``depth`` field; recover it from
    the frame stack at trace time.  Returns ``0`` if the frame is no
    longer on the stack (e.g. because the dispatch just popped it) —
    the caller gets a stable value rather than -1.
    """
    for i, f in enumerate(vm._frames):
        if f is frame:
            return i
    return 0


# ---------------------------------------------------------------------------
# LANG06 debug helpers
# ---------------------------------------------------------------------------


def _check_debug_pause(
    vm: "VMCore", frame: VMFrame, instr: IIRInstr, ip: int
) -> None:
    """Decide whether the VM should pause before dispatching ``instr``.

    Called by the dispatch loop when ``vm._debug_mode`` is True.  This
    function checks three conditions in priority order:

    1. **Explicit pause request** (``vm._paused``): pause unconditionally.
    2. **Step mode**: pause based on the current step granularity.
    3. **Breakpoint**: pause if ``ip`` is registered for ``frame.fn.name``,
       subject to an optional condition expression.

    When a pause is warranted, ``vm._debug_hooks.on_instruction`` is called.
    After the hook returns, the adapter will have called one of the step or
    continue methods on the VM, which set ``_step_mode`` / ``_paused`` for
    the *next* iteration.  This function does not loop — it fires once per
    call.

    Conditional breakpoints
    -----------------------
    A condition string is a simple Python expression that is evaluated in a
    namespace containing the current named register values.  For example, the
    condition ``"a > 10"`` passes when the register named ``"a"`` holds a
    value greater than 10.  The evaluation uses :func:`eval` with a
    restricted globals dict (``__builtins__``  set to an empty dict) and the
    frame's name-to-value mapping as locals.

    If evaluation raises an exception (undefined name, type error, etc.), the
    breakpoint is treated as *not* triggered and execution continues.

    Parameters
    ----------
    vm:
        The running VMCore.
    frame:
        The current top frame.
    instr:
        The IIR instruction about to be dispatched.
    ip:
        The 0-based instruction index of ``instr`` within the function body
        (``frame.ip`` has already been advanced, so this is ``frame.ip - 1``).
    """
    hooks = vm._debug_hooks
    if hooks is None:
        return

    should_pause = False

    # --- 1. Explicit pause ---
    if vm._paused:
        should_pause = True

    # --- 2. Step mode ---
    elif vm._step_mode is StepMode.IN:
        should_pause = True

    elif vm._step_mode is StepMode.OVER:
        # Pause when we are at or above the depth where the step was requested.
        # len(vm._frames) is the current depth (frames already include the
        # current frame since we haven't dispatched yet).
        if len(vm._frames) <= vm._step_frame_depth:
            should_pause = True

    elif vm._step_mode is StepMode.OUT:
        # OUT is handled in handle_ret / handle_ret_void via on_return.
        # Here we do nothing — we let execution continue until a ret fires.
        pass

    # --- 3. Breakpoints ---
    if not should_pause:
        fn_bps = vm._breakpoints.get(frame.fn.name)
        if fn_bps is not None and ip in fn_bps:
            condition = fn_bps[ip]
            if condition is None:
                should_pause = True
            else:
                # Build a local namespace from the frame's named register values.
                local_ns: dict[str, Any] = {}
                for name, reg_idx in frame.name_to_reg.items():
                    if reg_idx < len(frame.registers):
                        local_ns[name] = frame.registers[reg_idx]
                try:
                    result = eval(condition, {"__builtins__": {}}, local_ns)  # noqa: S307
                    should_pause = bool(result)
                except Exception:
                    should_pause = False

    if should_pause:
        # Deliver the pause: clear step state, fire the hook.
        vm._paused = True
        vm._step_mode = None
        try:
            hooks.on_instruction(frame, instr)
        finally:
            # Always clear the paused flag after the hook returns so the
            # dispatch loop does not pause again on the very next instruction
            # unless the hook (or the adapter) called pause() / step_*() again.
            vm._paused = False


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
    # LANG06: fire on_return and handle StepMode.OUT.
    _fire_on_return(vm, frame, value)
    return value


def handle_ret_void(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    _pop_frame(vm, frame)
    # LANG06: fire on_return and handle StepMode.OUT.
    _fire_on_return(vm, frame, None)
    return None


def _fire_on_return(vm: "VMCore", frame: VMFrame, return_value: Any) -> None:
    """Fire ``on_return`` and handle ``StepMode.OUT``.

    Called after a frame is popped.  If the adapter was stepping out of the
    returned frame's depth, transition to a pause at the next instruction in
    the caller by setting ``_paused = True``.

    Parameters
    ----------
    vm:
        The running VMCore.
    frame:
        The frame that just returned (already off the stack).
    return_value:
        The value returned by the frame.
    """
    if not vm._debug_mode or vm._debug_hooks is None:
        return
    try:
        vm._debug_hooks.on_return(frame, return_value)
    except Exception:
        pass  # adapter errors must not abort execution

    # If we were in StepMode.OUT and the frame that returned was at or
    # below the depth where step_out was requested, transition to a pause
    # at the next instruction in the caller.
    if vm._step_mode is StepMode.OUT:
        # After the pop, len(vm._frames) is the caller's depth.
        # We requested step_out when the frame stack had _step_frame_depth
        # frames.  The frame that just returned was at that depth, so now
        # the stack is one shorter — we should pause.
        if len(vm._frames) < vm._step_frame_depth:
            vm._paused = True
            vm._step_mode = None


def _pop_frame(vm: VMCore, frame: VMFrame) -> VMFrame | None:
    """Pop the current frame; return the new top frame (caller), or None.

    Also cleans up any Layer 4 / Layer 5 chain nodes that were established in
    the frame being popped (or in deeper callees that did not clean up after
    themselves — a frontend code-generation bug, but we handle it gracefully).

    Cleanup rule: after the frame at depth ``d = len(vm._frames)`` pops, any
    restart or exit-point node with ``stack_depth >= d`` (or ``frame_depth >= d``
    for exit points) is no longer valid.  We strip those nodes before returning.
    """
    current_depth = len(vm._frames)  # depth BEFORE pop (includes the popped frame)

    # Clean up VMCOND00 Phase 4 restart-chain nodes established in this frame
    # or deeper frames (the latter is a frontend bug, but we handle it safely).
    while vm._restart_chain and vm._restart_chain[-1].stack_depth >= current_depth:
        vm._restart_chain.pop()

    # Same for exit-point chain nodes.
    while vm._exit_point_chain and (
        vm._exit_point_chain[-1].frame_depth >= current_depth
    ):
        vm._exit_point_chain.pop()

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

    # LANG06: fire on_call *before* the callee frame is pushed so that the
    # adapter sees the caller as the top-of-stack (consistent with how most
    # debuggers present "step into" — you are still in the caller when the
    # call event fires).
    if vm._debug_mode and vm._debug_hooks is not None:
        try:
            vm._debug_hooks.on_call(frame, callee)
        except Exception:
            pass  # adapter errors must not abort execution

    vm._frames.append(callee_frame)
    vm._metrics_frames += 1
    vm._fn_call_counts[fn_name] = vm._fn_call_counts.get(fn_name, 0) + 1

    # LANG06 StepMode.OUT tracking: if we were stepping out, record the new
    # depth so on_return in the callee fires appropriately.
    # (No extra work needed here — _step_frame_depth was set at the original
    # frame's depth, and on_return checks depth after pop.)

    return None  # result stored when callee executes ret


def handle_call_builtin(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    name = str(instr.srcs[0])
    args = [frame.resolve(s) for s in instr.srcs[1:]]
    result = vm._builtins.call(name, args)
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


# ---------------------------------------------------------------------------
# VMCOND00 Phase 1 — checked syscall + error branch
# ---------------------------------------------------------------------------
#
# handle_syscall_checked
# ----------------------
# Executes a SYSCALL00-numbered host syscall without trapping on errors.
# The handler looks up the syscall number (srcs[0]) in ``vm._syscall_table``,
# resolves the argument register (srcs[1]), and calls the implementation.
#
# The implementation must return a ``(value: int, error_code: int)`` tuple:
#   - value:      the success value (byte read, bytes written, fd, …).
#                 Stored in the val_dst register (srcs[2]).  Set to 0 on error.
#   - error_code: 0 on success, -1 on EOF, <-1 for negated errno.
#                 Stored in the err_dst register (srcs[3]).
#
# If the syscall number is not registered, the handler stores 0 in val_dst
# and EINVAL (−22) in err_dst — matching the C ABI in SYSCALL00 Section 4.
#
# handle_branch_err
# -----------------
# Conditional branch on a non-zero error register.  Behaves like
# ``handle_jmp_if_false`` (branch when condition is falsy) but with the
# polarity inverted: we branch when the error register is *non-zero* (i.e.,
# the syscall failed).  Falls through when err_reg == 0 (success).
#
# Branch statistics are NOT recorded for branch_err because the branch is
# not a general Boolean — it is a typed error-code check.  This keeps the
# branch profiler's data focused on algorithmic branches (if-else, loops)
# rather than syscall error paths.

_EINVAL: int = -22  # negated EINVAL — matches POSIX errno 22


def handle_syscall_checked(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Execute a numbered host syscall without trapping on errors.

    Operand layout in ``instr.srcs``:
      [0] n        — the SYSCALL00 canonical syscall number (immediate int)
      [1] arg_reg  — register name holding the single argument
      [2] val_dst  — register name to receive the success value (0 on error)
      [3] err_dst  — register name to receive the error code (0 = ok,
                     -1 = EOF, <-1 = negated errno)

    The handler never raises — all errors are reported via err_dst.
    """
    n = int(instr.srcs[0])
    arg = frame.resolve(instr.srcs[1])
    val_dst = str(instr.srcs[2])
    err_dst = str(instr.srcs[3])

    impl = vm._syscall_table.get(n)
    if impl is None:
        # Unknown syscall number → EINVAL, no value.
        frame.assign(val_dst, 0)
        frame.assign(err_dst, _EINVAL)
        return None

    try:
        value, error_code = impl(arg)
    except Exception:  # noqa: BLE001 — syscall impls must not propagate Python exc
        frame.assign(val_dst, 0)
        frame.assign(err_dst, _EINVAL)
        return None

    frame.assign(val_dst, value)
    frame.assign(err_dst, error_code)
    return None


def handle_branch_err(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Branch to a label when an error register is non-zero.

    Operand layout in ``instr.srcs``:
      [0] err_reg — name of the error-code register (from syscall_checked)
      [1] label   — IR label to jump to when err_reg != 0

    Falls through (does not branch) when err_reg == 0 (success).

    Unlike ``jmp_if_true`` / ``jmp_if_false``, branch_err does not record
    branch statistics — it is a typed error-code test, not an algorithmic
    conditional.
    """
    err = frame.resolve(instr.srcs[0])
    if err != 0:
        target_ip = frame.fn.label_index(str(instr.srcs[1]))
        frame.ip = target_ip
    return None


# ---------------------------------------------------------------------------
# VMCOND00 Phase 2 — handle_throw
# ---------------------------------------------------------------------------
#
# Implements the ``throw`` opcode: Layer 2 (Unwind Exceptions) in the VMCOND00
# condition-system spec.
#
# Algorithm
# ---------
# 1. Read condition = frame.resolve(srcs[0]).
# 2. Walk the call stack from the innermost frame outward (vm._frames[-1] to
#    vm._frames[0]).  For each frame:
#      a. Compute the IP of the instruction that caused the throw/propagation.
#         Because the dispatch loop advances frame.ip BEFORE calling a handler,
#         the "current instruction" is always at frame.ip - 1:
#           - For the innermost frame: the throw instruction itself.
#           - For caller frames: the call instruction whose callee threw.
#      b. Walk the frame's function's exception_table (in order — the frontend
#         is responsible for listing entries innermost-first within overlapping
#         ranges).
#      c. If an entry's [from_ip, to_ip) range covers the throw IP and the
#         type_id matches, the handler wins:
#           - Set frame.ip = entry.handler_ip
#           - frame.assign(entry.val_reg, condition)
#           - Return — the dispatch loop continues in the handler frame.
#      d. If no entry matches, pop the frame and move to the caller.
# 3. If the stack is exhausted without finding a handler, raise
#    UncaughtConditionError(condition).
#
# Type matching (Phase 2)
# -----------------------
# Two patterns are recognised:
#   "*"   → catch-all (CATCH_ALL sentinel from exception_table module)
#   str   → exact match against type(condition).__name__
#
# Phase 3 will replace exact-name matching with a proper subtype walk once the
# condition type hierarchy is defined.
#
# Note on frame.ip timing
# -----------------------
# The dispatch loop structure is:
#
#     instr = frame.fn.instructions[frame.ip]
#     frame.ip += 1                     ← advanced HERE
#     result = _dispatch_one(vm, frame, instr)
#
# So when handle_throw is called, frame.ip already points to the instruction
# AFTER the throw.  The throw itself is at frame.ip - 1.  For caller frames,
# the call instruction that triggered the callee frame push is also at
# frame.ip - 1 (the call instruction's ip was advanced before the callee frame
# was pushed — see handle_call).


def _throw_type_matches(condition: object, type_id: str) -> bool:
    """Return True if ``condition`` matches the exception table ``type_id``.

    Phase 2 matching rules:
    - ``"*"``   → matches everything (CATCH_ALL).
    - Any other string → matches when ``type(condition).__name__ == type_id``.

    Phase 3 will extend this to a full subtype walk.
    """
    if type_id == "*":
        return True
    return type(condition).__name__ == type_id


def handle_throw(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Unwind the call stack searching for a matching exception table handler.

    Operand layout in ``instr.srcs``:
      [0] condition_reg — register holding the condition object to throw.

    The handler walks the static exception table of every active frame,
    innermost first, checking the guarded range ``[from_ip, to_ip)`` and
    the condition type.  On the first match it sets the frame's instruction
    pointer to the handler and assigns the condition to the handler's register.

    If no matching entry is found anywhere in the call stack,
    :exc:`vm_core.errors.UncaughtConditionError` is raised.
    """
    condition = frame.resolve(instr.srcs[0])

    # Walk frames from innermost to outermost.  vm._frames is a list with the
    # innermost (most recently pushed) frame at the END (index -1).
    while vm._frames:
        search_frame = vm._frames[-1]
        # The instruction that caused the throw/propagation is one step behind
        # the already-advanced ip pointer (see dispatch loop structure above).
        throw_ip = search_frame.ip - 1

        for entry in search_frame.fn.exception_table:
            in_range = entry.from_ip <= throw_ip < entry.to_ip
            if in_range and _throw_type_matches(condition, entry.type_id):
                # Handler found — jump into it and assign the condition.
                search_frame.ip = entry.handler_ip
                search_frame.assign(entry.val_reg, condition)
                return  # dispatch loop continues in search_frame

        # No match in this frame — pop it and search the caller.
        vm._frames.pop()

    # Stack exhausted — the condition propagated past the top frame.
    raise UncaughtConditionError(condition)


# ---------------------------------------------------------------------------
# VMCOND00 Phase 3 — dynamic handler chain (Layer 3)
# ---------------------------------------------------------------------------
#
# Overview
# --------
# Layer 3 adds a *non-unwinding* handler mechanism: when a condition is
# signaled (SIGNAL / ERROR / WARN), the VM searches a per-instance linked
# list (``vm._handler_chain``) for the most recently pushed handler whose
# ``condition_type`` matches the condition.  If found, the handler function
# is *called* — pushed as a new frame on the call stack WITHOUT removing any
# existing frames.  When the handler returns normally, execution resumes at
# the instruction immediately after the signaling opcode.
#
# Key contrast with Layer 2 (THROW):
#   - THROW unwinds the call stack searching the STATIC exception table.
#   - SIGNAL/ERROR/WARN search the DYNAMIC handler chain and never unwind
#     the call stack on their own.
#
# The non-unwinding call works naturally in our dispatch loop model:
#   1. `signal` advances ip (pre-increment) then calls handle_signal.
#   2. handle_signal pushes the handler frame onto vm._frames.
#   3. The dispatch loop continues in the handler frame.
#   4. When the handler executes `ret`, handle_ret pops the handler frame.
#   5. The dispatch loop now runs the signal frame, whose ip already points
#      to the instruction AFTER signal.  Execution resumes correctly.
#
# Operand conventions
# -------------------
# push_handler:  srcs[0] = type_id (immediate string), srcs[1] = fn_reg
# pop_handler:   srcs = []
# signal:        srcs[0] = condition_reg
# error:         srcs[0] = condition_reg
# warn:          srcs[0] = condition_reg


def _handler_type_matches(condition: object, condition_type: str) -> bool:
    """Return True if ``condition`` matches the handler's ``condition_type``.

    Matching semantics mirror :func:`_throw_type_matches` (Phase 2):
    - ``"*"``  → catch-all, always matches.
    - Other string → exact ``type(condition).__name__`` equality.
    Phase 4 will replace this with a proper subtype walk via the condition
    type registry.
    """
    if condition_type == "*":
        return True
    return type(condition).__name__ == condition_type


def _invoke_handler_nonunwinding(
    vm: VMCore, node: HandlerNode, condition: object
) -> None:
    """Push the handler function as a new frame, non-unwinding.

    The current frame (the one that issued signal/error/warn) stays on the
    stack below the handler frame.  When the handler returns, handle_ret pops
    the handler frame and the dispatch loop resumes in the original frame at
    its pre-incremented ip — i.e. at the instruction *after* the signaling
    opcode.

    Phase 3 constraint: ``node.handler_fn`` must be a string (the name of a
    callable in ``vm._module``).  Phase 4 will add closure support.
    """
    # Validate Phase 3 constraint: handler_fn must be a str (IIR fn name).
    # A non-string value indicates a code-generation bug in the frontend.
    if not isinstance(node.handler_fn, str):
        raise HandlerChainError(
            f"handler_fn must be a str (IIR function name); "
            f"got {type(node.handler_fn).__name__!r}"
        )
    handler_name: str = node.handler_fn
    # Guard against the handler function being absent from the module.  An
    # unguarded None here would produce an opaque AttributeError rather than a
    # clean VMError that the caller can catch.
    if vm._module is None:  # pragma: no cover — execute() always sets _module first
        raise HandlerChainError("handler chain invocation with no module loaded")
    handler_fn = vm._module.get_function(handler_name)
    if handler_fn is None:
        raise HandlerChainError(
            f"handler chain references unknown function {handler_name!r}"
        )
    # Guard against unbounded frame-stack growth.  A guest program that loops
    # on signal/error/warn with a matching handler could grow vm._frames past
    # the configured limit, bypassing the FrameOverflowError that handle_call
    # enforces.  Apply the same check here so the safety contract is consistent.
    if len(vm._frames) >= vm._max_frames:
        raise FrameOverflowError(
            f"call stack depth {vm._max_frames} exceeded "
            f"invoking handler {handler_name!r}"
        )
    # Push handler frame with return_dest=None: SIGNAL does not use the
    # handler's return value, so we discard it.
    handler_frame = VMFrame.for_function(handler_fn, return_dest=None)
    # Pass the condition as the first argument (parameter 0) when the handler
    # declares at least one parameter.  Mirror the argument-copy loop used by
    # handle_call (dispatch.py lines ~665-668).
    if handler_fn.params:
        handler_frame.registers[0] = condition
    vm._frames.append(handler_frame)


def handle_push_handler(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Push a new handler onto the handler chain (``push_handler``).

    Operand layout in ``instr.srcs``:
      [0] type_id — immediate string: ``"*"`` (catch-all) or a type name.
      [1] fn_reg  — register name holding the handler callable (a string
                    naming an IIR function in this module, Phase 3).

    Example IIR::

        push_handler "*", my_handler_reg

    After this instruction, the most recently pushed handler on
    ``vm._handler_chain`` covers all conditions (``"*"``) and calls the
    function whose name is in ``my_handler_reg``.
    """
    type_id: str = instr.srcs[0]  # immediate — used directly
    handler_fn = frame.resolve(instr.srcs[1])  # register → callable name
    node = HandlerNode(
        condition_type=type_id,
        handler_fn=handler_fn,
        stack_depth=len(vm._frames),
    )
    vm._handler_chain.append(node)


def handle_pop_handler(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Pop the most recently pushed handler (``pop_handler``).

    Raises :exc:`~vm_core.errors.HandlerChainError` on underflow (i.e.
    ``pop_handler`` with no matching ``push_handler`` on the chain).  This
    indicates a frontend code-generation bug — PUSH/POP must be paired on
    every control-flow path that exits a guarded region.
    """
    if not vm._handler_chain:
        raise HandlerChainError(
            "pop_handler on an empty handler chain — "
            "push_handler / pop_handler are unbalanced"
        )
    vm._handler_chain.pop()


def handle_signal(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Signal a condition, invoking a matching handler non-unwinding.

    Operand layout:
      srcs[0] = condition_reg — holds the condition object to signal.

    Walks ``vm._handler_chain`` from the most recently pushed handler to the
    oldest.  On the first match, calls the handler function as a new frame on
    top of the existing call stack (non-unwinding).  Execution resumes after
    this instruction when the handler returns.

    If no handler matches, ``signal`` is a **no-op** — execution continues
    silently at the next instruction.  This is intentional: signaling a
    condition that nobody handles is not an error at Layer 3; use ``error``
    if you need the abort-on-unhandled behaviour.
    """
    condition = frame.resolve(instr.srcs[0])
    # Search from END (most recently pushed) to START (oldest).
    for node in reversed(vm._handler_chain):
        if _handler_type_matches(condition, node.condition_type):
            _invoke_handler_nonunwinding(vm, node, condition)
            return
    # No matching handler — SIGNAL is a no-op.


def handle_error(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Raise an error condition, aborting if unhandled.

    Operand layout:
      srcs[0] = condition_reg — holds the condition object.

    Two-phase dispatch:
    1. **Layer 2 check first.**  If the current instruction's IP is inside a
       guarded range in ``frame.fn.exception_table`` AND that entry's
       ``type_id`` matches the condition, delegate to ``handle_throw`` to
       execute the Layer 2 unwind path.  This ensures that Layer 2 static
       exception handlers take priority over Layer 3 dynamic handlers when
       both are in scope.
    2. **Layer 3 handler chain.**  Walk ``vm._handler_chain`` exactly as
       ``handle_signal`` does.  On match, call the handler non-unwinding.
    3. **Abort.**  If neither phase finds a handler, raise
       :exc:`~vm_core.errors.UncaughtConditionError`.
    """
    condition = frame.resolve(instr.srcs[0])

    # Phase 1: check Layer 2 static exception table.
    throw_ip = frame.ip - 1
    for entry in frame.fn.exception_table:
        in_range = entry.from_ip <= throw_ip < entry.to_ip
        if in_range and _throw_type_matches(condition, entry.type_id):
            # Delegate to the Layer 2 THROW algorithm.
            frame.ip = entry.handler_ip
            frame.assign(entry.val_reg, condition)
            return

    # Phase 2: walk the Layer 3 handler chain.
    for node in reversed(vm._handler_chain):
        if _handler_type_matches(condition, node.condition_type):
            _invoke_handler_nonunwinding(vm, node, condition)
            return

    # Phase 3: no handler anywhere — abort.
    raise UncaughtConditionError(condition)


def handle_warn(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Warn about a condition; emit to stderr if unhandled.

    Operand layout:
      srcs[0] = condition_reg — holds the condition object.

    Walks the handler chain exactly as ``handle_signal``.  On match, calls
    the handler non-unwinding.

    If no handler matches, emits a warning line to ``sys.stderr`` and
    continues execution.  ``warn`` **never** aborts — it is the safe
    "something unusual happened, but we can keep going" signal.
    """
    import sys  # local import — warn is uncommon; keep top-level imports lean

    condition = frame.resolve(instr.srcs[0])

    # Walk handler chain.
    for node in reversed(vm._handler_chain):
        if _handler_type_matches(condition, node.condition_type):
            _invoke_handler_nonunwinding(vm, node, condition)
            return

    # No handler — emit warning to stderr, continue.
    # Use the same hardened repr strategy as UncaughtConditionError to guard
    # against guest objects whose __repr__ raises or returns unbounded strings.
    try:
        cond_repr = repr(condition)
        if len(cond_repr) > 200:
            cond_repr = cond_repr[:200] + "…"
    except Exception:  # noqa: BLE001
        try:
            cond_repr = f"<{type(condition).__name__} (repr failed)>"
        except Exception:  # noqa: BLE001
            cond_repr = "<unknown (repr and type name failed)>"
    print(f"[vm-core WARN] {cond_repr}", file=sys.stderr)


# ---------------------------------------------------------------------------
# VMCOND00 Phase 4 — restart chain (Layer 4) and exit-point chain (Layer 5)
# ---------------------------------------------------------------------------
#
# Overview
# --------
# Layer 4 adds *named restarts*: callable continuations that a handler (Layer 3)
# can find by name and invoke without the signaling code needing to know which
# recovery strategy will be chosen.  The restart chain is structured identically
# to the handler chain: a list of nodes appended at the end, searched from the
# end (innermost first).
#
# Layer 5 adds *non-local exits*: a dynamically scoped tag any code in its
# dynamic extent can call EXIT_TO on, which unwinds the call stack, handler
# chain, and restart chain to the depth recorded at ESTABLISH_EXIT time.
#
# Operand conventions
# -------------------
# push_restart:    srcs[0] = name (immediate string), srcs[1] = fn_reg
# pop_restart:     srcs = []
# find_restart:    srcs[0] = name (immediate string); dest = result_reg
# invoke_restart:  srcs[0] = handle_reg, srcs[1] = arg_reg; dest = result_reg
# compute_restarts: srcs = []; dest = result_reg
# establish_exit:  srcs[0] = tag (imm), srcs[1] = result_reg_name (imm),
#                  srcs[2] = after_label (imm); dest = None
# exit_to:         srcs[0] = tag (imm), srcs[1] = val_reg; dest = None


def _unwind_to_depth(vm: VMCore, target_depth: int) -> None:
    """Unwind vm._frames, vm._handler_chain, and vm._restart_chain to target_depth.

    After this call:
    - ``len(vm._frames) == target_depth`` (frames above are popped).
    - Handler chain and restart chain nodes with ``stack_depth > target_depth``
      are removed (they were established in now-gone frames).

    Used by EXIT_TO to perform the stack-unwinding step.

    Parameters
    ----------
    vm:
        The running VMCore.
    target_depth:
        The desired call stack depth after unwinding.  Must be ≥ 0.

    Notes
    -----
    The exit-point chain is NOT cleaned up here — the EXIT_TO handler manages
    it directly (it truncates the chain to just before the matched node and
    then calls this function).
    """
    # Guard: a negative target_depth would silently behave like 0 (loop always
    # runs because len() >= 0 > negative).  Make the invariant explicit so
    # frontend bugs producing negative frame_depths surface immediately.
    assert target_depth >= 0, (  # noqa: S101 — invariant check, not user input
        f"_unwind_to_depth: target_depth must be >= 0, got {target_depth!r}"
    )
    # Unwind call frames.  We don't fire debug hooks or on_return here because
    # the unwind is an exceptional control transfer, not a structured return.
    while len(vm._frames) > target_depth:
        vm._frames.pop()

    # Remove handler chain nodes established in now-gone frames.  The chain is
    # a list with the oldest node at index 0 and the newest at the end; nodes
    # with stack_depth > target_depth were pushed in frames we just popped.
    while vm._handler_chain and vm._handler_chain[-1].stack_depth > target_depth:
        vm._handler_chain.pop()

    # Same for the restart chain.
    while vm._restart_chain and vm._restart_chain[-1].stack_depth > target_depth:
        vm._restart_chain.pop()


def handle_push_restart(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Push a named restart onto the restart chain (``push_restart``).

    Operand layout in ``instr.srcs``:
      [0] name  — immediate string: the restart's symbolic name.
      [1] fn_reg — register holding the restart callable (a string naming an
                   IIR function in this module, Phase 4).

    After this instruction the most recently pushed node on ``vm._restart_chain``
    has ``name`` equal to the immediate and ``restart_fn`` equal to whatever
    value was in ``fn_reg``.

    Example IIR::

        push_restart "use-value", my_restart_reg
    """
    name: str = instr.srcs[0]       # immediate — restart's symbolic name
    restart_fn = frame.resolve(instr.srcs[1])  # register → callable name

    # Validate restart_fn at push time so the chain invariant holds for its
    # entire lifetime, not only when invoke_restart is called.  A non-string
    # value indicates a frontend code-generation bug and is rejected here
    # rather than silently storing a garbage node that would fail later.
    if not isinstance(restart_fn, str):
        raise RestartChainError(
            f"push_restart: restart_fn must be a str (IIR function name); "
            f"got {type(restart_fn).__name__!r}"
        )

    node = RestartNode(
        name=name,
        restart_fn=restart_fn,
        stack_depth=len(vm._frames),
    )
    vm._restart_chain.append(node)


def handle_pop_restart(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Pop the most recently pushed restart (``pop_restart``).

    Raises :exc:`~vm_core.errors.RestartChainError` on underflow (i.e.
    ``pop_restart`` with no matching ``push_restart`` on the chain).
    """
    if not vm._restart_chain:
        raise RestartChainError(
            "pop_restart on an empty restart chain — "
            "push_restart / pop_restart are unbalanced"
        )
    vm._restart_chain.pop()


def handle_find_restart(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    """Search the restart chain for a named restart (``find_restart``).

    Operand layout:
      srcs[0] = name — immediate string: the restart name to search for.
      dest    = result_reg (the register to write the result into).

    Walks ``vm._restart_chain`` from the most recently pushed (innermost) to
    the oldest (outermost), returning the first :class:`RestartNode` whose
    ``name`` matches.  If no match is found, writes ``None`` into dest.

    The returned value is a :class:`RestartNode` object that can be passed
    directly to ``invoke_restart``.

    Example IIR::

        find_restart handle, "use-value"   # handle ← RestartNode or None
        invoke_restart result, handle, arg  # call the restart with arg
    """
    name: str = instr.srcs[0]  # immediate — name to search for
    result: RestartNode | None = None
    for node in reversed(vm._restart_chain):
        if node.name == name:
            result = node
            break
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_invoke_restart(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Invoke a restart by its handle (``invoke_restart``).

    Operand layout:
      srcs[0] = handle_reg — register holding a RestartNode (from find_restart).
      srcs[1] = arg_reg    — register holding the argument to pass.
      dest                 — register to receive the restart's return value
                             (may be None if the caller discards the result).

    The restart function is called as a new VM frame, exactly like a regular
    ``call`` instruction.  If the restart returns normally, ``handle_ret``
    writes the return value into this frame's ``dest`` register (via
    ``return_dest``).  If the restart calls ``exit_to``, the call stack will
    be unwound by that opcode instead.

    Raises :exc:`~vm_core.errors.RestartChainError` if:
    - ``handle_reg`` holds ``None`` (FIND_RESTART returned nil and caller
      did not check before invoking).
    - ``handle_reg`` holds a value that is not a :class:`RestartNode`.
    - The restart function name is not found in the current module.

    Raises :exc:`~vm_core.errors.FrameOverflowError` if the call stack
    would exceed ``vm._max_frames``.
    """
    handle = frame.resolve(instr.srcs[0])
    arg = frame.resolve(instr.srcs[1])

    if handle is None:
        raise RestartChainError(
            "invoke_restart: handle is None — did find_restart return NIL?"
        )
    if not isinstance(handle, RestartNode):
        raise RestartChainError(
            f"invoke_restart: expected a RestartNode handle; "
            f"got {type(handle).__name__!r}"
        )

    restart_fn_name = handle.restart_fn
    if not isinstance(restart_fn_name, str):
        raise RestartChainError(
            f"invoke_restart: restart_fn must be a str (IIR function name); "
            f"got {type(restart_fn_name).__name__!r}"
        )

    if vm._module is None:  # pragma: no cover — execute() always sets _module
        raise RestartChainError("invoke_restart: no module loaded")

    # Look up the restart function in the current module.
    #
    # Security note: the function name is a guest-provided string with no
    # allowlist or visibility qualifier.  Any function in the module can be
    # invoked as a restart.  This is acceptable for Phase 4 because the
    # bytecode comes from a trusted front-end compiler.  If the VM is ever
    # exposed to bytecode from untrusted sources, add a visibility mechanism
    # to IIRFunction (e.g. is_restart_handler: bool) and assert it here.
    restart_fn = vm._module.get_function(restart_fn_name)
    if restart_fn is None:
        raise RestartChainError(
            f"invoke_restart: unknown restart function {restart_fn_name!r}"
        )

    if len(vm._frames) >= vm._max_frames:
        raise FrameOverflowError(
            f"call stack depth {vm._max_frames} exceeded "
            f"invoking restart {restart_fn_name!r}"
        )

    # Resolve the dest name to an integer register index in the caller frame,
    # exactly as handle_call does.  This is needed because VMFrame.for_function
    # stores return_dest as an int and handle_ret writes via
    # caller_frame.registers[frame.return_dest].
    ret_reg: int | None = None
    if instr.dest:
        if instr.dest not in frame.name_to_reg:
            ret_reg = len(frame.name_to_reg)
            frame.name_to_reg[instr.dest] = ret_reg
        else:
            ret_reg = frame.name_to_reg[instr.dest]

    # Push the restart function as a new frame, passing arg as first parameter.
    new_frame = VMFrame.for_function(restart_fn, return_dest=ret_reg)
    if restart_fn.params:
        new_frame.registers[0] = arg
    vm._frames.append(new_frame)
    vm._metrics_frames += 1


def handle_compute_restarts(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> Any:
    """Collect all active restart handles into a list (``compute_restarts``).

    Operand layout:
      dest = result_reg — register to receive the list of RestartNode handles.

    Returns a plain Python list of all :class:`RestartNode` objects currently
    on ``vm._restart_chain``, from outermost (oldest) to innermost (newest).
    This ordering is conventional; callers that want innermost-first can
    reverse the list.

    Intended for debuggers and REPLs that present restart choices to the user.
    In production code, ``find_restart`` + ``invoke_restart`` is more efficient
    since it short-circuits on the first match.

    Example IIR::

        compute_restarts all_restarts
        # all_restarts is now a Python list of RestartNode objects
    """
    result = list(vm._restart_chain)  # copy: outermost first
    if instr.dest:
        frame.assign(instr.dest, result)
    return result


def handle_establish_exit(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Push an exit point onto the exit-point chain (``establish_exit``).

    Operand layout in ``instr.srcs``:
      [0] tag           — immediate string: the exit-point tag.
      [1] result_reg    — immediate string: name of the register in the
                          *current* frame that will receive the exit value
                          when ``exit_to`` fires.
      [2] after_label   — immediate string: label name to resolve to the
                          ``resume_ip`` (the IP to jump to when exit_to fires).

    On normal execution (no ``exit_to`` fires) the exit point node remains on
    the chain until the enclosing frame returns; ``handle_ret`` / ``handle_ret_void``
    clean it up automatically to avoid memory leaks.

    Raises ``KeyError`` if ``after_label`` is not defined in the current function.

    Example IIR::

        establish_exit "done", "result", "after_block"
        ...  ; guarded body — can call exit_to "done", value
        label "after_block"
        ...  ; execution resumes here whether via exit_to or natural fallthrough
    """
    tag: str = instr.srcs[0]           # immediate — exit-point tag
    result_reg_name: str = instr.srcs[1]  # immediate — register name in this frame
    after_label: str = instr.srcs[2]   # immediate — label to resolve to IP

    # Resolve the label to an instruction index now (at establish_exit time)
    # so EXIT_TO can set frame.ip directly without a second lookup.
    resume_ip = frame.fn.label_index(after_label)  # raises KeyError if missing

    # Pre-initialise the result register to 0 in this frame's name_to_reg so
    # that a normal fallthrough (where EXIT_TO never fires) can still execute
    # ``ret result_reg`` without hitting UndefinedVariableError.  The default
    # of 0 is intentional — the spec says the result register is unspecified if
    # EXIT_TO never fires; 0 is a safe, deterministic sentinel.
    if result_reg_name not in frame.name_to_reg:
        frame.assign(result_reg_name, 0)

    node = ExitPointNode(
        tag=tag,
        result_reg=result_reg_name,
        resume_ip=resume_ip,
        frame_depth=len(vm._frames),
    )
    vm._exit_point_chain.append(node)


def handle_exit_to(vm: VMCore, frame: VMFrame, instr: IIRInstr) -> None:
    """Perform a non-local exit to an exit point (``exit_to``).

    Operand layout:
      srcs[0] = tag     — immediate string: the exit-point tag to find.
      srcs[1] = val_reg — register holding the value to deliver.

    Algorithm:
      1. Read ``val = registers[val_reg]``.
      2. Search ``vm._exit_point_chain`` from the most recently pushed (END)
         to the oldest (START) for a node whose ``tag == tag``.
      3. Truncate ``vm._exit_point_chain`` to just before the matched node
         (discarding the matched node and all more recently pushed nodes).
      4. Call ``_unwind_to_depth(vm, node.frame_depth)`` to unwind the call
         stack, handler chain, and restart chain to the target depth.
      5. If ``node.result_reg`` is set and ``vm._frames`` is non-empty, assign
         ``val`` into ``vm._frames[-1].registers[node.result_reg]``.
      6. Set ``vm._frames[-1].ip = node.resume_ip``.

    Raises :exc:`~vm_core.errors.UnboundExitTagError` if no matching exit
    point exists anywhere on the chain.

    Example IIR::

        exit_to "done", return_val
    """
    tag: str = instr.srcs[0]          # immediate — tag to search for
    val = frame.resolve(instr.srcs[1])  # register → exit value

    # Search from the most recently pushed (END) to find the innermost match.
    # We need both the index (to truncate the chain) and the node itself.
    matched_node: ExitPointNode | None = None
    matched_idx: int = -1
    for i in range(len(vm._exit_point_chain) - 1, -1, -1):
        node = vm._exit_point_chain[i]
        if node.tag == tag:
            matched_node = node
            matched_idx = i
            break

    if matched_node is None:
        raise UnboundExitTagError(tag)

    # Truncate exit-point chain: keep only nodes strictly before matched_idx.
    # This removes the matched node AND all more recently pushed nodes (those
    # with indices > matched_idx, established in frames we are about to unwind).
    vm._exit_point_chain = vm._exit_point_chain[:matched_idx]

    # Unwind call stack, handler chain, and restart chain to target depth.
    _unwind_to_depth(vm, matched_node.frame_depth)

    # Deliver the exit value into the target frame's result register.
    # Use an explicit ``is not None`` check rather than truthiness so that
    # an empty-string register name (a frontend bug, but a possible one)
    # does not silently suppress value delivery.
    if matched_node.result_reg is not None and vm._frames:
        vm._frames[-1].assign(matched_node.result_reg, val)

    # Jump to the resume instruction in the target frame.
    if vm._frames:
        vm._frames[-1].ip = matched_node.resume_ip


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
    # VMCOND00 Phase 1 — checked syscalls and error-code branches.
    "syscall_checked": handle_syscall_checked,
    "branch_err": handle_branch_err,
    # VMCOND00 Phase 2 — stack-unwinding exception dispatch.
    "throw": handle_throw,
    # VMCOND00 Phase 3 — dynamic handler chain (non-unwinding).
    "push_handler": handle_push_handler,
    "pop_handler": handle_pop_handler,
    "signal": handle_signal,
    "error": handle_error,
    "warn": handle_warn,
    # VMCOND00 Phase 4 — restart chain (Layer 4).
    "push_restart": handle_push_restart,
    "pop_restart": handle_pop_restart,
    "find_restart": handle_find_restart,
    "invoke_restart": handle_invoke_restart,
    "compute_restarts": handle_compute_restarts,
    # VMCOND00 Phase 4 — exit-point chain (Layer 5).
    "establish_exit": handle_establish_exit,
    "exit_to": handle_exit_to,
}
