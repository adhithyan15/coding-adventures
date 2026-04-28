"""Tests for LANG06 debug hooks in vm-core.

Covers:
    - DebugHooks / StepMode are importable from vm_core
    - attach_debug_hooks / detach_debug_hooks / is_debug_mode
    - on_instruction fires when VM pauses (breakpoint or step)
    - on_instruction does NOT fire when running freely (zero overhead)
    - set_breakpoint / clear_breakpoint — unconditional
    - Conditional breakpoints — only pause when condition is truthy
    - StepMode.IN — pause at very next instruction
    - StepMode.OVER — pause at next instruction in same or outer frame
    - StepMode.OUT — pause at return site of current frame
    - call_stack() — correct snapshot during a pause
    - patch_function() — hot-swaps function body
    - on_call fires before entering callee, on_return fires after leaving
    - on_exception fires on an unhandled error during execution
    - Adapter errors inside hooks do not abort execution
"""

from __future__ import annotations

from typing import Any

import pytest
from interpreter_ir import IIRFunction, IIRInstr, IIRModule
from interpreter_ir.function import FunctionTypeStatus

from vm_core import DebugHooks, StepMode, VMCore, VMFrame
from vm_core.errors import UnknownOpcodeError


# ---------------------------------------------------------------------------
# Test helpers — tiny IIRModule builders
# ---------------------------------------------------------------------------

def _i(op: str, dest: str | None = None, srcs: list | None = None,
       type_hint: str = "any") -> IIRInstr:
    return IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)


def _fn(name: str, *instrs: IIRInstr, params: list[tuple[str, str]] | None = None,
        return_type: str = "any") -> IIRFunction:
    p = params or []
    return IIRFunction(
        name=name,
        params=p,
        return_type=return_type,
        instructions=list(instrs),
        register_count=max(8, len(p) + len(instrs) + 4),
    )


def _mod(*fns: IIRFunction) -> IIRModule:
    return IIRModule(name="test", functions=list(fns))


def _simple_program() -> IIRModule:
    """fn main(): return 42  (three instructions)."""
    main = _fn(
        "main",
        _i("const", "v", [42]),       # ip=0
        _i("ret",   None, ["v"]),     # ip=1
    )
    return _mod(main)


def _counter_program() -> IIRModule:
    """
    fn main():
        a = 1
        b = 2
        c = a + b
        return c
    Four instructions so we can set breakpoints at different IPs.
    """
    main = _fn(
        "main",
        _i("const", "a", [1]),          # ip=0
        _i("const", "b", [2]),          # ip=1
        _i("add",   "c", ["a", "b"]),   # ip=2
        _i("ret",   None, ["c"]),       # ip=3
    )
    return _mod(main)


def _two_fn_program() -> IIRModule:
    """
    fn add(x, y): return x + y
    fn main(): return add(10, 20)
    """
    add = _fn(
        "add",
        params=[("x", "any"), ("y", "any")],
        *[
            _i("add", "r", ["x", "y"]),
            _i("ret", None, ["r"]),
        ],
    )
    main = _fn(
        "main",
        _i("const", "p", [10]),
        _i("const", "q", [20]),
        _i("call",  "res", ["add", "p", "q"]),
        _i("ret",   None,  ["res"]),
    )
    return IIRModule(name="test", functions=[add, main])


# ---------------------------------------------------------------------------
# Recording adapter
# ---------------------------------------------------------------------------

class RecordingAdapter(DebugHooks):
    """Debug adapter that records every event for test assertions.

    The adapter records (fn_name, ip) tuples for each on_instruction call
    so tests can assert which instructions triggered a pause.

    After each on_instruction call the adapter calls ``vm.step_in()`` by
    default (so execution advances to the next instruction instead of
    stalling forever).  Tests that need different behaviour should set
    ``self.next_action`` to a callable that accepts ``(vm, frame, instr)``.
    """

    def __init__(self, vm: VMCore) -> None:
        self.vm = vm
        self.instructions: list[tuple[str, int]] = []  # (fn_name, ip_before_dispatch)
        self.calls: list[tuple[str, str]] = []         # (caller_fn, callee_fn)
        self.returns: list[tuple[str, Any]] = []        # (fn_name, return_value)
        self.exceptions: list[tuple[str, Exception]] = []  # (fn_name, error)
        self.next_action: str = "step_in"  # "step_in", "step_over", "step_out", "continue"

    def on_instruction(self, frame: VMFrame, instr: IIRInstr) -> None:
        # ip has already been advanced; the paused instruction is at ip - 1.
        self.instructions.append((frame.fn.name, frame.ip - 1))
        if self.next_action == "step_in":
            self.vm.step_in()
        elif self.next_action == "step_over":
            self.vm.step_over()
        elif self.next_action == "step_out":
            self.vm.step_out()
        elif self.next_action == "continue":
            self.vm.continue_()

    def on_call(self, caller: VMFrame, callee: "IIRFunction") -> None:
        self.calls.append((caller.fn.name, callee.name))

    def on_return(self, frame: VMFrame, return_value: Any) -> None:
        self.returns.append((frame.fn.name, return_value))

    def on_exception(self, frame: VMFrame, error: Exception) -> None:
        self.exceptions.append((frame.fn.name, error))


# ---------------------------------------------------------------------------
# Tests: attach / detach / is_debug_mode
# ---------------------------------------------------------------------------

class TestAttachDetach:
    def test_is_debug_mode_false_by_default(self) -> None:
        vm = VMCore()
        assert vm.is_debug_mode() is False

    def test_attach_enables_debug_mode(self) -> None:
        vm = VMCore()
        hooks = DebugHooks()
        vm.attach_debug_hooks(hooks)
        assert vm.is_debug_mode() is True

    def test_detach_disables_debug_mode(self) -> None:
        vm = VMCore()
        vm.attach_debug_hooks(DebugHooks())
        vm.detach_debug_hooks()
        assert vm.is_debug_mode() is False

    def test_attach_replaces_previous_hooks(self) -> None:
        vm = VMCore()
        hooks1 = DebugHooks()
        hooks2 = DebugHooks()
        vm.attach_debug_hooks(hooks1)
        vm.attach_debug_hooks(hooks2)
        assert vm._debug_hooks is hooks2

    def test_default_hooks_are_noop(self) -> None:
        """Default DebugHooks methods should not raise."""
        hooks = DebugHooks()
        vm = VMCore()
        mod = _simple_program()
        frame = vm._frames  # empty before execute
        # Build a minimal frame manually for the noop test
        from vm_core.frame import VMFrame, RegisterFile
        fn = mod.get_function("main")
        assert fn is not None
        f = VMFrame.for_function(fn)
        instr = fn.instructions[0]
        # These should all silently succeed
        hooks.on_instruction(f, instr)
        hooks.on_call(f, fn)
        hooks.on_return(f, 42)
        hooks.on_exception(f, ValueError("test"))


# ---------------------------------------------------------------------------
# Tests: on_instruction fires only at pauses / breakpoints
# ---------------------------------------------------------------------------

class TestOnInstruction:
    def test_no_hooks_no_pause(self) -> None:
        """Without debug hooks attached, execution completes normally."""
        vm = VMCore()
        mod = _simple_program()
        result = vm.execute(mod)
        assert result == 42

    def test_hooks_attached_but_no_breakpoints(self) -> None:
        """Hooks attached but no breakpoints → on_instruction never called."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)
        mod = _simple_program()
        result = vm.execute(mod)
        assert result == 42
        assert adapter.instructions == []  # no pauses

    def test_breakpoint_fires_on_instruction(self) -> None:
        """Breakpoint at ip=0 → on_instruction fires once."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        adapter.next_action = "continue"  # resume after first pause
        vm.attach_debug_hooks(adapter)
        vm.set_breakpoint(0, "main")
        mod = _simple_program()
        result = vm.execute(mod)
        assert result == 42
        assert len(adapter.instructions) == 1
        assert adapter.instructions[0] == ("main", 0)

    def test_breakpoint_at_each_instruction(self) -> None:
        """Breakpoints at every ip → on_instruction fires for each."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        adapter.next_action = "step_in"  # step through all
        vm.attach_debug_hooks(adapter)
        # Set breakpoint at ip=0 only; then step_in will hit every instruction
        vm.set_breakpoint(0, "main")
        mod = _counter_program()  # 4 instructions
        result = vm.execute(mod)
        assert result == 3
        # step_in from ip=0 → pause at 0, 1, 2, 3 = 4 pauses
        assert len(adapter.instructions) == 4
        fired_ips = [ip for _, ip in adapter.instructions]
        assert fired_ips == [0, 1, 2, 3]

    def test_clear_breakpoint_stops_pause(self) -> None:
        """After clear_breakpoint, execution runs freely past that ip."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        adapter.next_action = "continue"
        vm.attach_debug_hooks(adapter)
        vm.set_breakpoint(0, "main")
        vm.clear_breakpoint(0, "main")
        mod = _simple_program()
        vm.execute(mod)
        assert adapter.instructions == []


# ---------------------------------------------------------------------------
# Tests: conditional breakpoints
# ---------------------------------------------------------------------------

class TestConditionalBreakpoints:
    def test_condition_false_no_pause(self) -> None:
        """Condition 'a > 100' is never true → no pause."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)
        # ip=2: 'add "c" ["a", "b"]' — at this point a=1, b=2 are in registers
        vm.set_breakpoint(2, "main", condition="a > 100")
        mod = _counter_program()
        result = vm.execute(mod)
        assert result == 3
        assert adapter.instructions == []

    def test_condition_true_causes_pause(self) -> None:
        """Condition 'a == 1' is true → pause fires."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        adapter.next_action = "continue"
        vm.attach_debug_hooks(adapter)
        # At ip=2, a=1 and b=2 have been assigned
        vm.set_breakpoint(2, "main", condition="a == 1")
        mod = _counter_program()
        result = vm.execute(mod)
        assert result == 3
        assert len(adapter.instructions) == 1
        assert adapter.instructions[0] == ("main", 2)

    def test_invalid_condition_treated_as_false(self) -> None:
        """Condition that raises (undefined name) is treated as not triggered."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)
        vm.set_breakpoint(0, "main", condition="undefined_var > 0")
        mod = _simple_program()
        vm.execute(mod)
        assert adapter.instructions == []


# ---------------------------------------------------------------------------
# Tests: stepping
# ---------------------------------------------------------------------------

class TestStepIn:
    def test_step_in_pauses_at_every_instruction(self) -> None:
        """Starting from ip=0 breakpoint, step_in visits every instruction."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        adapter.next_action = "step_in"
        vm.attach_debug_hooks(adapter)
        vm.set_breakpoint(0, "main")
        mod = _counter_program()  # 4 instructions
        vm.execute(mod)
        assert len(adapter.instructions) == 4

    def test_step_in_enters_called_function(self) -> None:
        """step_in from main follows execution into the 'add' callee."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        adapter.next_action = "step_in"
        vm.attach_debug_hooks(adapter)
        vm.set_breakpoint(0, "main")
        mod = _two_fn_program()
        vm.execute(mod)
        fn_names = [fn for fn, _ in adapter.instructions]
        # Should have visited both main and add
        assert "add" in fn_names
        assert "main" in fn_names


class TestStepOver:
    def test_step_over_skips_callee_internals(self) -> None:
        """step_over from the 'call' instruction in main should not enter add."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)

        # Pause at ip=2 (the call instruction), then step_over
        pauses: list[tuple[str, int]] = []

        class StepOverAdapter(DebugHooks):
            def on_instruction(self2, frame: VMFrame, instr: IIRInstr) -> None:
                pauses.append((frame.fn.name, frame.ip - 1))
                vm.step_over()

        vm.attach_debug_hooks(StepOverAdapter())
        vm.set_breakpoint(2, "main")  # ip=2 is the 'call add' instruction
        mod = _two_fn_program()
        result = vm.execute(mod)
        assert result == 30
        # After stepping over the call we should be at ip=3 (ret) in main
        # — still in main, never in add
        fn_names = [fn for fn, _ in pauses]
        assert "add" not in fn_names
        assert all(fn == "main" for fn in fn_names)


class TestStepOut:
    def test_step_out_returns_to_caller(self) -> None:
        """step_out from inside 'add' should pause in main after the call."""
        vm = VMCore()

        pauses: list[tuple[str, int]] = []
        step_phase: list[str] = ["bp"]  # track what we do at each pause

        class StepOutAdapter(DebugHooks):
            def on_instruction(self2, frame: VMFrame, instr: IIRInstr) -> None:
                pauses.append((frame.fn.name, frame.ip - 1))
                if step_phase[0] == "bp":
                    # First pause: we're inside 'add'; step out
                    step_phase[0] = "out"
                    vm.step_out()
                else:
                    # Second pause: we're back in 'main'; continue
                    vm.continue_()

        vm.attach_debug_hooks(StepOutAdapter())
        # Break at the first instruction inside 'add' (ip=0)
        vm.set_breakpoint(0, "add")
        mod = _two_fn_program()
        result = vm.execute(mod)
        assert result == 30
        # Pause 1: inside 'add'; Pause 2: back in 'main' after the call returns
        assert len(pauses) == 2
        assert pauses[0][0] == "add"
        assert pauses[1][0] == "main"


# ---------------------------------------------------------------------------
# Tests: call_stack
# ---------------------------------------------------------------------------

class TestCallStack:
    def test_call_stack_returns_correct_frames(self) -> None:
        """call_stack() during a pause in 'add' shows both frames."""
        vm = VMCore()
        captured_stack: list[list] = []

        class StackAdapter(DebugHooks):
            def on_instruction(self2, frame: VMFrame, instr: IIRInstr) -> None:
                captured_stack.append(vm.call_stack())
                vm.continue_()

        vm.attach_debug_hooks(StackAdapter())
        vm.set_breakpoint(0, "add")
        mod = _two_fn_program()
        vm.execute(mod)

        assert len(captured_stack) == 1
        stack = captured_stack[0]
        # Stack has at least two frames: __entry__ and 'add' (via main → add)
        fn_names = [f.fn.name for f in stack]
        assert "add" in fn_names

    def test_call_stack_is_copy(self) -> None:
        """Modifying the returned list does not affect the VM's frame stack."""
        vm = VMCore()

        class ModifyAdapter(DebugHooks):
            def on_instruction(self2, frame: VMFrame, instr: IIRInstr) -> None:
                stack = vm.call_stack()
                stack.clear()  # mutate the copy
                vm.continue_()

        vm.attach_debug_hooks(ModifyAdapter())
        vm.set_breakpoint(0, "main")
        mod = _simple_program()
        # Should not raise — the VM's internal frame stack is not affected
        result = vm.execute(mod)
        assert result == 42


# ---------------------------------------------------------------------------
# Tests: patch_function
# ---------------------------------------------------------------------------

class TestPatchFunction:
    def test_patch_function_replaces_body(self) -> None:
        """After patch_function, the new body runs for subsequent calls."""
        vm = VMCore()

        # Original: main returns 42; we will patch it to return 99
        new_main = _fn(
            "main",
            _i("const", "v", [99]),
            _i("ret",   None, ["v"]),
        )

        patched = False

        class PatchAdapter(DebugHooks):
            def on_instruction(self2, frame: VMFrame, instr: IIRInstr) -> None:
                nonlocal patched
                if not patched:
                    vm.patch_function("main", new_main)
                    patched = True
                vm.continue_()

        vm.attach_debug_hooks(PatchAdapter())
        vm.set_breakpoint(0, "main")
        mod = _simple_program()
        # The patch happens mid-execution of the original main; the current
        # frame is still running the old body.  The new body takes effect
        # for future calls.  This test just verifies no crash occurs.
        vm.execute(mod)
        assert patched

    def test_patch_function_raises_for_unknown_fn(self) -> None:
        """patch_function raises KeyError for a non-existent function."""
        vm = VMCore()
        mod = _simple_program()
        vm.execute(mod)  # load a module

        new_fn = _fn("nonexistent", _i("ret_void"))
        # Must call execute first to load module
        vm2 = VMCore()

        class PatchAdapter(DebugHooks):
            def on_instruction(self2, frame: VMFrame, instr: IIRInstr) -> None:
                with pytest.raises(KeyError):
                    vm2.patch_function("nonexistent", new_fn)
                vm2.continue_()

        vm2.attach_debug_hooks(PatchAdapter())
        vm2.set_breakpoint(0, "main")
        vm2.execute(mod)


# ---------------------------------------------------------------------------
# Tests: on_call and on_return
# ---------------------------------------------------------------------------

class TestOnCallOnReturn:
    def test_on_call_fires_for_every_call(self) -> None:
        """on_call fires once per function invocation."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)
        mod = _two_fn_program()
        vm.execute(mod)
        # main calls add once
        callee_names = [callee for _, callee in adapter.calls]
        assert "add" in callee_names

    def test_on_return_fires_for_every_ret(self) -> None:
        """on_return fires once per function return."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)
        mod = _two_fn_program()
        result = vm.execute(mod)
        assert result == 30
        returned_fns = [fn for fn, _ in adapter.returns]
        assert "add" in returned_fns
        assert "main" in returned_fns

    def test_on_return_value_is_correct(self) -> None:
        """on_return receives the actual return value."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)
        mod = _simple_program()
        vm.execute(mod)
        # main returns 42
        main_returns = [v for fn, v in adapter.returns if fn == "main"]
        assert 42 in main_returns


# ---------------------------------------------------------------------------
# Tests: on_exception
# ---------------------------------------------------------------------------

class TestOnException:
    def test_on_exception_fires_on_unknown_opcode(self) -> None:
        """on_exception fires when the dispatch loop raises."""
        vm = VMCore()
        adapter = RecordingAdapter(vm)
        vm.attach_debug_hooks(adapter)

        bad_fn = _fn("main", _i("totally_unknown_opcode"))
        mod = _mod(bad_fn)
        with pytest.raises(UnknownOpcodeError):
            vm.execute(mod)

        assert len(adapter.exceptions) == 1
        fn_name, error = adapter.exceptions[0]
        assert fn_name == "main"
        assert isinstance(error, UnknownOpcodeError)

    def test_adapter_error_in_on_exception_does_not_mask(self) -> None:
        """If on_exception itself raises, the original error is still propagated."""
        vm = VMCore()

        class BrokenAdapter(DebugHooks):
            def on_exception(self2, frame: VMFrame, error: Exception) -> None:
                raise RuntimeError("adapter broken")

        vm.attach_debug_hooks(BrokenAdapter())
        bad_fn = _fn("main", _i("bad_op"))
        mod = _mod(bad_fn)
        with pytest.raises(UnknownOpcodeError):
            vm.execute(mod)


# ---------------------------------------------------------------------------
# Tests: adapter errors inside hooks do not abort execution
# ---------------------------------------------------------------------------

class TestAdapterRobustness:
    def test_adapter_error_in_on_call_does_not_abort(self) -> None:
        """If on_call raises, execution continues normally."""
        vm = VMCore()

        class BrokenCallAdapter(DebugHooks):
            def on_call(self2, caller: VMFrame, callee: "IIRFunction") -> None:
                raise RuntimeError("on_call broken")

        vm.attach_debug_hooks(BrokenCallAdapter())
        mod = _two_fn_program()
        # Should not raise — adapter errors are swallowed
        result = vm.execute(mod)
        assert result == 30

    def test_adapter_error_in_on_return_does_not_abort(self) -> None:
        """If on_return raises, execution continues normally."""
        vm = VMCore()

        class BrokenReturnAdapter(DebugHooks):
            def on_return(self2, frame: VMFrame, return_value: Any) -> None:
                raise RuntimeError("on_return broken")

        vm.attach_debug_hooks(BrokenReturnAdapter())
        mod = _two_fn_program()
        result = vm.execute(mod)
        assert result == 30
