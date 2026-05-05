"""Tests for VMCOND00 Phase 4 — Layer 4 (Restarts) + Layer 5 (Non-Local Exits).

Layer 4 adds named restarts: callable continuations that a handler can find by
name and invoke without the signaling code needing to know the recovery strategy.

Layer 5 adds non-local exits: a dynamically-scoped tag that any code in its
dynamic extent can EXIT_TO, delivering a value and unwinding all three chains
(call stack, handler chain, restart chain) to the depth recorded at
ESTABLISH_EXIT time.

Acceptance criterion (from spec Phase 4 description):
    A restart named "use-value" established by outer code can be found by a
    handler in inner code, invoked with a substitute value, and execution
    resumes in the outer code at the post-ESTABLISH_EXIT instruction.

Test structure
--------------
Each test builds IIR instructions with the ``_i`` helper (shorthand for
IIRInstr) and assembles them into functions / modules.  The canonical
``vm.execute(module)`` API is used throughout.

``_fn`` — build an IIRFunction with a name, param list, and instructions.
``_i``  — build an IIRInstr (op, dest, srcs) with sensible defaults.
``_mod`` — build an IIRModule with a list of functions and an entry point.

Helper convention for observing restart/handler invocation side effects:
- A function that records its argument writes to I/O port 99 via ``io_out``.
- After execution: ``assert vm._io_ports.get(99) == expected_value``.
"""

from __future__ import annotations

import pytest
from interpreter_ir import IIRFunction, IIRInstr, IIRModule
from interpreter_ir.function import FunctionTypeStatus

from vm_core import (
    ExitPointNode,
    FrameOverflowError,
    RestartChainError,
    RestartNode,
    UnboundExitTagError,
    VMCore,
)

# ---------------------------------------------------------------------------
# IIR construction helpers
# ---------------------------------------------------------------------------

def _i(op: str, dest: str | None, srcs: list, type_hint: str = "any") -> IIRInstr:
    """Build an IIRInstr with the given op, dest, srcs, and optional type_hint."""
    return IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)


def _fn(
    name: str,
    params: list[tuple[str, str]],
    *instructions: IIRInstr,
) -> IIRFunction:
    """Build an IIRFunction from a name, params, and sequence of instructions."""
    return IIRFunction(
        name=name,
        params=params,
        return_type="any",
        instructions=list(instructions),
        type_status=FunctionTypeStatus.UNTYPED,
    )


def _mod(entry: str, *functions: IIRFunction) -> IIRModule:
    """Build an IIRModule with the given functions and entry point."""
    return IIRModule(name="test", functions=list(functions), entry_point=entry)


# ---------------------------------------------------------------------------
# TestPushPopRestart — basic push/pop mechanics
# ---------------------------------------------------------------------------


class TestPushPopRestart:
    """push_restart and pop_restart: basic chain management."""

    def test_push_restart_appends_to_chain(self) -> None:
        """push_restart adds a RestartNode to vm._restart_chain."""
        # IIR: const restart_fn_name = "my_restart"
        #      push_restart "use-value", restart_fn_name
        #      const z = 0; ret z
        fn = _fn("main", [],
            _i("const", "fn_name", ["my_restart"]),
            _i("push_restart", None, ["use-value", "fn_name"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        # After normal execution the chain is empty (cleaned up by ret)
        assert vm._restart_chain == []

    def test_push_restart_chain_during_execution(self) -> None:
        """A restart on the chain during execution has correct fields."""
        # We'll inspect the chain from inside a handler invocation.
        # Use a simpler approach: check via compute_restarts.
        fn = _fn("main", [],
            _i("const", "fn_name", ["my_restart"]),
            _i("push_restart", None, ["use-value", "fn_name"]),
            _i("compute_restarts", "all", []),
            _i("const", "port", [98]),
            _i("io_out", None, ["port", "all"]),
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        restarts = vm._io_ports.get(98)
        assert isinstance(restarts, list)
        assert len(restarts) == 1
        node = restarts[0]
        assert isinstance(node, RestartNode)
        assert node.name == "use-value"
        assert node.restart_fn == "my_restart"

    def test_pop_restart_empty_chain_raises(self) -> None:
        """pop_restart on empty chain raises RestartChainError."""
        fn = _fn("main", [],
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        with pytest.raises(RestartChainError, match="pop_restart"):
            vm.execute(_mod("main", fn))

    def test_push_restart_non_string_fn_raises(self) -> None:
        """push_restart rejects non-string restart_fn at push time (not deferred)."""
        # Store an integer (not a string) in the fn register and push_restart.
        # The validation happens at push time now — RestartChainError is raised
        # before the node ever enters the chain.
        fn = _fn("main", [],
            _i("const", "fn_name", [42]),          # integer, not a function name
            _i("push_restart", None, ["bad", "fn_name"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        with pytest.raises(RestartChainError, match="restart_fn must be a str"):
            vm.execute(_mod("main", fn))

    def test_push_pop_balanced(self) -> None:
        """Balanced push/pop leaves empty chain."""
        fn = _fn("main", [],
            _i("const", "fn_name", ["r"]),
            _i("push_restart", None, ["a", "fn_name"]),
            _i("push_restart", None, ["b", "fn_name"]),
            _i("pop_restart", None, []),
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        assert vm._restart_chain == []

    def test_multiple_restarts_ordering(self) -> None:
        """compute_restarts returns outermost first (matching list order)."""
        fn = _fn("main", [],
            _i("const", "f1", ["r1"]),
            _i("const", "f2", ["r2"]),
            _i("push_restart", None, ["alpha", "f1"]),
            _i("push_restart", None, ["beta", "f2"]),
            _i("compute_restarts", "all", []),
            _i("const", "port", [98]),
            _i("io_out", None, ["port", "all"]),
            _i("pop_restart", None, []),
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        restarts = vm._io_ports.get(98)
        assert isinstance(restarts, list)
        assert len(restarts) == 2
        # List order: outermost (alpha) first, innermost (beta) last.
        assert restarts[0].name == "alpha"
        assert restarts[1].name == "beta"


# ---------------------------------------------------------------------------
# TestFindRestart — find_restart opcode
# ---------------------------------------------------------------------------


class TestFindRestart:
    """find_restart: searches the restart chain newest-first."""

    def test_find_existing_restart(self) -> None:
        """find_restart returns the matching RestartNode."""
        fn = _fn("main", [],
            _i("const", "fn_name", ["my_restart"]),
            _i("push_restart", None, ["use-value", "fn_name"]),
            _i("find_restart", "handle", ["use-value"]),
            _i("const", "port", [97]),
            _i("io_out", None, ["port", "handle"]),
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        handle = vm._io_ports.get(97)
        assert isinstance(handle, RestartNode)
        assert handle.name == "use-value"

    def test_find_nonexistent_restart_returns_none(self) -> None:
        """find_restart returns None when no restart matches."""
        fn = _fn("main", [],
            _i("find_restart", "handle", ["nonexistent"]),
            _i("const", "port", [97]),
            _i("io_out", None, ["port", "handle"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        assert vm._io_ports.get(97) is None

    def test_find_restart_innermost_shadows_outer(self) -> None:
        """When two restarts share a name, find_restart returns the innermost."""
        fn = _fn("main", [],
            _i("const", "outer_fn", ["outer_restart"]),
            _i("const", "inner_fn", ["inner_restart"]),
            _i("push_restart", None, ["use-value", "outer_fn"]),
            _i("push_restart", None, ["use-value", "inner_fn"]),
            _i("find_restart", "handle", ["use-value"]),
            _i("const", "port", [97]),
            _i("io_out", None, ["port", "handle"]),
            _i("pop_restart", None, []),
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        handle = vm._io_ports.get(97)
        # The inner restart (push second) should be found.
        assert isinstance(handle, RestartNode)
        assert handle.restart_fn == "inner_restart"

    def test_find_restart_no_dest(self) -> None:
        """find_restart with dest=None does not crash."""
        fn = _fn("main", [],
            _i("const", "fn_name", ["r"]),
            _i("push_restart", None, ["x", "fn_name"]),
            _i("find_restart", None, ["x"]),   # dest=None — result discarded
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))  # Must not raise


# ---------------------------------------------------------------------------
# TestComputeRestarts — compute_restarts opcode
# ---------------------------------------------------------------------------


class TestComputeRestarts:
    """compute_restarts: collect all active restart handles."""

    def test_empty_chain_returns_empty_list(self) -> None:
        """compute_restarts on empty chain returns []."""
        fn = _fn("main", [],
            _i("compute_restarts", "all", []),
            _i("const", "port", [96]),
            _i("io_out", None, ["port", "all"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        assert vm._io_ports.get(96) == []

    def test_compute_restarts_no_dest(self) -> None:
        """compute_restarts with dest=None does not crash."""
        fn = _fn("main", [],
            _i("compute_restarts", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))


# ---------------------------------------------------------------------------
# TestInvokeRestart — invoke_restart opcode
# ---------------------------------------------------------------------------


def _restart_that_captures_arg(fn_name: str = "capturing_restart") -> IIRFunction:
    """Build a restart function that writes its arg to I/O port 99."""
    return _fn(fn_name, [("arg", "any")],
        _i("const", "port", [99]),
        _i("io_out", None, ["port", "arg"]),
        _i("const", "z", [0]),
        _i("ret", None, ["z"]),
    )


class TestInvokeRestart:
    """invoke_restart: call the restart function."""

    def test_invoke_restart_calls_function(self) -> None:
        """invoke_restart invokes the restart function with the argument."""
        restart_fn = _restart_that_captures_arg("my_restart")
        main_fn = _fn("main", [],
            _i("const", "fn_name", ["my_restart"]),
            _i("push_restart", None, ["use-value", "fn_name"]),
            _i("find_restart", "handle", ["use-value"]),
            _i("const", "arg", [42]),
            _i("invoke_restart", "result", ["handle", "arg"]),
            _i("pop_restart", None, []),
            _i("const", "port", [100]),
            _i("io_out", None, ["port", "result"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", main_fn, restart_fn))
        # The restart wrote 42 to port 99.
        assert vm._io_ports.get(99) == 42

    def test_invoke_restart_return_value_in_dest(self) -> None:
        """The restart's return value ends up in invoke_restart's dest register."""
        # Build a restart that returns a specific value.
        restart_fn = _fn("returning_restart", [("arg", "any")],
            _i("const", "r", [777]),
            _i("ret", None, ["r"]),
        )
        main_fn = _fn("main", [],
            _i("const", "fn_name", ["returning_restart"]),
            _i("push_restart", None, ["r", "fn_name"]),
            _i("find_restart", "handle", ["r"]),
            _i("const", "arg", [0]),
            _i("invoke_restart", "result", ["handle", "arg"]),
            _i("pop_restart", None, []),
            _i("const", "port", [100]),
            _i("io_out", None, ["port", "result"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", main_fn, restart_fn))
        assert vm._io_ports.get(100) == 777

    def test_invoke_restart_none_handle_raises(self) -> None:
        """invoke_restart with a None handle raises RestartChainError."""
        main_fn = _fn("main", [],
            _i("find_restart", "handle", ["no-such-restart"]),  # → None
            _i("const", "arg", [0]),
            _i("invoke_restart", "result", ["handle", "arg"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        with pytest.raises(RestartChainError, match="None"):
            vm.execute(_mod("main", main_fn))

    def test_invoke_restart_invalid_handle_type_raises(self) -> None:
        """invoke_restart with non-RestartNode handle raises RestartChainError."""
        main_fn = _fn("main", [],
            _i("const", "handle", [123]),  # integer, not a RestartNode
            _i("const", "arg", [0]),
            _i("invoke_restart", "result", ["handle", "arg"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        with pytest.raises(RestartChainError, match="RestartNode"):
            vm.execute(_mod("main", main_fn))

    def test_invoke_restart_non_string_fn_raises(self) -> None:
        """invoke_restart where restart_fn is not a string raises RestartChainError."""
        vm = VMCore()
        # Seed the restart chain directly with an invalid restart_fn (42, not a str).
        # execute() does not clear _restart_chain, so this node will be live during
        # the run.  find_restart will return it; invoke_restart must reject it.
        vm._restart_chain.append(RestartNode(name="bad", restart_fn=42, stack_depth=1))
        invoke_fn = _fn("main", [],
            _i("find_restart", "handle", ["bad"]),
            _i("const", "arg", [0]),
            _i("invoke_restart", "result", ["handle", "arg"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        with pytest.raises(RestartChainError, match="restart_fn must be a str"):
            vm.execute(_mod("main", invoke_fn))

    def test_invoke_restart_unknown_function_raises(self) -> None:
        """invoke_restart referencing unknown IIR function raises RestartChainError."""
        main_fn = _fn("main", [],
            _i("const", "fn_name", ["does_not_exist"]),
            _i("push_restart", None, ["use-value", "fn_name"]),
            _i("find_restart", "handle", ["use-value"]),
            _i("const", "arg", [0]),
            _i("invoke_restart", "result", ["handle", "arg"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        with pytest.raises(RestartChainError, match="does_not_exist"):
            vm.execute(_mod("main", main_fn))

    def test_invoke_restart_frame_overflow_raises(self) -> None:
        """invoke_restart that would exceed max_frames raises FrameOverflowError."""
        restart_fn = _fn("my_restart", [("a", "any")],
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main_fn = _fn("main", [],
            _i("const", "fn_name", ["my_restart"]),
            _i("push_restart", None, ["use-value", "fn_name"]),
            _i("find_restart", "handle", ["use-value"]),
            _i("const", "arg", [0]),
            _i("invoke_restart", "result", ["handle", "arg"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore(max_frames=1)  # 1 frame = root only; no room for restart frame
        with pytest.raises(FrameOverflowError):
            vm.execute(_mod("main", main_fn, restart_fn))


# ---------------------------------------------------------------------------
# TestEstablishExitAndExitTo — Layer 5 exit-point opcodes
# ---------------------------------------------------------------------------


class TestEstablishExitAndExitTo:
    """establish_exit and exit_to: non-local exit protocol."""

    def test_exit_to_delivers_value_to_result_reg(self) -> None:
        """exit_to stores the value in the exit point's result_reg."""
        # IIR:
        #   establish_exit "done", "result", "after"
        #   const val = 999
        #   exit_to "done", val
        #   label "after"           ; resume here after exit
        #   const port = 95
        #   io_out port, result
        #   ret result
        fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("const", "val", [999]),
            _i("exit_to", None, ["done", "val"]),
            _i("label", None, ["after"]),
            _i("const", "port", [95]),
            _i("io_out", None, ["port", "result"]),
            _i("ret", None, ["result"]),
        )
        vm = VMCore()
        ret = vm.execute(_mod("main", fn))
        assert ret == 999
        assert vm._io_ports.get(95) == 999

    def test_normal_fallthrough_skips_exit_to(self) -> None:
        """Without exit_to, fallthrough leaves result_reg at its default (0)."""
        # IIR:
        #   establish_exit "done", "result", "after"
        #   ; no exit_to — fall through naturally
        #   label "after"
        #   ret result
        fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("label", None, ["after"]),
            _i("ret", None, ["result"]),
        )
        vm = VMCore()
        ret = vm.execute(_mod("main", fn))
        # result was never written by exit_to → default register value is 0
        assert ret == 0

    def test_exit_to_unknown_tag_raises(self) -> None:
        """exit_to with no matching exit point raises UnboundExitTagError."""
        fn = _fn("main", [],
            _i("const", "val", [1]),
            _i("exit_to", None, ["no-such-tag", "val"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        with pytest.raises(UnboundExitTagError) as exc_info:
            vm.execute(_mod("main", fn))
        assert exc_info.value.tag == "no-such-tag"

    def test_exit_to_finds_innermost_tag(self) -> None:
        """When two exit points share a tag, exit_to uses the innermost."""
        fn = _fn("main", [],
            _i("establish_exit", None, ["done", "outer_r", "outer_after"]),
            _i("establish_exit", None, ["done", "inner_r", "inner_after"]),
            _i("const", "val", [55]),
            _i("exit_to", None, ["done", "val"]),
            _i("label", None, ["inner_after"]),
            _i("label", None, ["outer_after"]),
            # Which result register got 55 tells us which exit point was matched.
            _i("const", "port_inner", [90]),
            _i("io_out", None, ["port_inner", "inner_r"]),
            _i("const", "port_outer", [91]),
            _i("io_out", None, ["port_outer", "outer_r"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        # Innermost "done" exit point gets the value.
        assert vm._io_ports.get(90) == 55
        # Outermost "done" exit point was never triggered.
        assert vm._io_ports.get(91) == 0  # default register value

    def test_exit_to_across_call_frames(self) -> None:
        """exit_to from inside a called function unwinds frames correctly."""
        # Outer function establishes exit "done", then calls inner_fn.
        # inner_fn calls exit_to "done".  Stack unwinds to outer frame.
        #
        # outer:
        #   establish_exit "done", "result", "after"
        #   call inner_fn
        #   label "after"
        #   ret result
        #
        # inner_fn:
        #   const val = 42
        #   exit_to "done", val
        #   (never reaches here)
        #   const z = 0
        #   ret z
        outer_fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("call", "ignored", ["inner_fn"]),
            _i("label", None, ["after"]),
            _i("ret", None, ["result"]),
        )
        inner_fn = _fn("inner_fn", [],
            _i("const", "val", [42]),
            _i("exit_to", None, ["done", "val"]),
            # The following are never reached:
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        ret = vm.execute(_mod("main", outer_fn, inner_fn))
        assert ret == 42

    def test_exit_to_unwinds_handler_chain(self) -> None:
        """exit_to removes handler chain nodes established above frame_depth."""
        # outer: establish_exit "done", result, after
        # outer: push_handler "*", my_handler
        # outer: call inner_fn
        # inner_fn: push_handler "*", my_handler
        # inner_fn: exit_to "done", 1
        # (after, outer): pop_handler  ← this should NOT run (exited)
        # We check that handler chain is cleaned up after exit_to.
        #
        # Simpler: after exit_to lands in outer, outer pops handler and checks.
        # We'll check vm._handler_chain is empty after execution.
        handler_fn = _fn("my_handler", [("c", "any")],
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        inner_fn = _fn("inner_fn", [],
            _i("const", "fn", ["my_handler"]),
            _i("push_handler", None, ["*", "fn"]),
            _i("const", "val", [1]),
            _i("exit_to", None, ["done", "val"]),
            # Never reached:
            _i("pop_handler", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        outer_fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("call", "ignored", ["inner_fn"]),
            _i("label", None, ["after"]),
            _i("ret", None, ["result"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", outer_fn, inner_fn, handler_fn))
        # All handler chain nodes established inside inner_fn were unwound.
        assert vm._handler_chain == []

    def test_exit_to_unwinds_restart_chain(self) -> None:
        """exit_to removes restart chain nodes established above frame_depth."""
        inner_fn = _fn("inner_fn", [],
            _i("const", "fn", ["some_restart"]),
            _i("push_restart", None, ["use-value", "fn"]),
            _i("const", "val", [7]),
            _i("exit_to", None, ["done", "val"]),
            # Never reached:
            _i("pop_restart", None, []),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        some_restart_fn = _fn("some_restart", [("a", "any")],
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        outer_fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("call", "ignored", ["inner_fn"]),
            _i("label", None, ["after"]),
            _i("ret", None, ["result"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", outer_fn, inner_fn, some_restart_fn))
        # Restart chain nodes from inner_fn were removed by exit_to.
        assert vm._restart_chain == []

    def test_establish_exit_unknown_label_raises(self) -> None:
        """establish_exit with a non-existent label raises KeyError."""
        fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "no_such_label"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        with pytest.raises(KeyError, match="no_such_label"):
            vm.execute(_mod("main", fn))

    def test_exit_point_cleaned_up_on_normal_ret(self) -> None:
        """Exit-point nodes established in a function are removed when it returns."""
        # Establish an exit point but let execution fall through normally.
        fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("label", None, ["after"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        # The exit point should have been cleaned up by _pop_frame.
        assert vm._exit_point_chain == []

    def test_restart_cleaned_up_on_normal_ret(self) -> None:
        """Restart nodes not explicitly popped are removed when the frame returns."""
        # Push a restart but deliberately do NOT pop it (frontend bug simulation).
        fn = _fn("main", [],
            _i("const", "fn_name", ["r"]),
            _i("push_restart", None, ["use-value", "fn_name"]),
            # Intentionally skip pop_restart to test _pop_frame cleanup.
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        vm = VMCore()
        vm.execute(_mod("main", fn))
        # _pop_frame should have cleaned this up.
        assert vm._restart_chain == []


# ---------------------------------------------------------------------------
# TestResetClearsPhase4Chains — reset() must clear new chains
# ---------------------------------------------------------------------------


class TestResetClearsPhase4Chains:
    """reset() clears all Phase 4 runtime chains between executions."""

    def test_reset_clears_restart_chain(self) -> None:
        """vm.reset() empties vm._restart_chain."""
        vm = VMCore()
        vm._restart_chain.append(
            RestartNode(name="x", restart_fn="fn", stack_depth=1)
        )
        vm.reset()
        assert vm._restart_chain == []

    def test_reset_clears_exit_point_chain(self) -> None:
        """vm.reset() empties vm._exit_point_chain."""
        vm = VMCore()
        vm._exit_point_chain.append(
            ExitPointNode(tag="done", result_reg="r", resume_ip=0, frame_depth=1)
        )
        vm.reset()
        assert vm._exit_point_chain == []


# ---------------------------------------------------------------------------
# TestExitPointNodeAndRestartNode — dataclass tests
# ---------------------------------------------------------------------------


class TestRestartNodeDataclass:
    """Unit tests for RestartNode as a standalone dataclass."""

    def test_construction(self) -> None:
        node = RestartNode(name="use-value", restart_fn="my_fn", stack_depth=3)
        assert node.name == "use-value"
        assert node.restart_fn == "my_fn"
        assert node.stack_depth == 3

    def test_equality(self) -> None:
        a = RestartNode(name="x", restart_fn="f", stack_depth=1)
        b = RestartNode(name="x", restart_fn="f", stack_depth=1)
        assert a == b

    def test_inequality(self) -> None:
        a = RestartNode(name="x", restart_fn="f", stack_depth=1)
        b = RestartNode(name="y", restart_fn="f", stack_depth=1)
        assert a != b


class TestExitPointNodeDataclass:
    """Unit tests for ExitPointNode as a standalone dataclass."""

    def test_construction(self) -> None:
        node = ExitPointNode(tag="done", result_reg="r", resume_ip=10, frame_depth=2)
        assert node.tag == "done"
        assert node.result_reg == "r"
        assert node.resume_ip == 10
        assert node.frame_depth == 2

    def test_construction_none_result_reg(self) -> None:
        """result_reg may be None (exit value discarded)."""
        node = ExitPointNode(tag="abort", result_reg=None, resume_ip=5, frame_depth=1)
        assert node.result_reg is None

    def test_equality(self) -> None:
        a = ExitPointNode(tag="done", result_reg="r", resume_ip=10, frame_depth=2)
        b = ExitPointNode(tag="done", result_reg="r", resume_ip=10, frame_depth=2)
        assert a == b

    def test_inequality(self) -> None:
        a = ExitPointNode(tag="done", result_reg="r", resume_ip=10, frame_depth=2)
        b = ExitPointNode(tag="abort", result_reg="r", resume_ip=10, frame_depth=2)
        assert a != b


# ---------------------------------------------------------------------------
# TestUnboundExitTagError — error class
# ---------------------------------------------------------------------------


class TestUnboundExitTagError:
    """Unit tests for UnboundExitTagError."""

    def test_tag_attribute(self) -> None:
        err = UnboundExitTagError("my-tag")
        assert err.tag == "my-tag"

    def test_message_contains_tag(self) -> None:
        err = UnboundExitTagError("some-tag")
        assert "some-tag" in str(err)


# ---------------------------------------------------------------------------
# TestRestartChainError — error class
# ---------------------------------------------------------------------------


class TestRestartChainError:
    """Unit tests for RestartChainError."""

    def test_is_vm_error(self) -> None:
        from vm_core.errors import VMError
        err = RestartChainError("underflow")
        assert isinstance(err, VMError)

    def test_message(self) -> None:
        err = RestartChainError("custom message")
        assert "custom message" in str(err)


# ---------------------------------------------------------------------------
# TestPhase4AcceptanceTest — full integration (spec acceptance criterion)
# ---------------------------------------------------------------------------


class TestPhase4AcceptanceTest:
    """Spec Phase 4 acceptance criterion: restart + exit_to round-trip.

    "A restart named 'use-value' established by outer code can be found by
    a handler in inner code, invoked with a substitute value, and execution
    resumes in the outer code at the post-ESTABLISH_EXIT instruction."

    Program structure:

      outer (main):
        1. establish_exit "done", "result", "after"
        2. push_restart "use-value", "use_value_impl"
        3. push_handler "*", "my_handler"
        4. call inner_fn
        5. pop_handler                      # never reached (exit_to fires)
        6. pop_restart                      # never reached
        7. label "after"                    # EXIT_TO resumes here
        8. ret result                       # returns 42

      inner_fn:
        1. const port = 98
        2. const cond = "error-condition"
        3. signal cond                      # fires my_handler non-unwinding
        4. const z = 0
        5. ret z

      my_handler (receives condition in params[0]):
        1. find_restart "use-value" → handle
        2. const sub_val = 42
        3. invoke_restart handle, sub_val   # calls use_value_impl(42)
        4. (invoke_restart pushes use_value_impl frame)

      use_value_impl (receives arg in params[0]):
        1. exit_to "done", arg              # delivers 42 to outer "result"
        (stack unwinds to outer frame; outer's ip = "after" label index)
    """

    def test_full_restart_exit_to_roundtrip(self) -> None:
        use_value_impl = _fn("use_value_impl", [("arg", "any")],
            _i("exit_to", None, ["done", "arg"]),
            # The following are unreachable (exit_to always transfers):
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )

        my_handler = _fn("my_handler", [("cond", "any")],
            _i("find_restart", "handle", ["use-value"]),
            _i("const", "sub_val", [42]),
            _i("invoke_restart", "ignored", ["handle", "sub_val"]),
            # invoke_restart pushed use_value_impl's frame; when it
            # executes exit_to, the stack unwinds past this handler frame.
            # These instructions are unreachable:
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )

        inner_fn = _fn("inner_fn", [],
            _i("const", "cond", ["simulated-error"]),
            _i("signal", None, ["cond"]),   # fires my_handler
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )

        main_fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("const", "restart_fn", ["use_value_impl"]),
            _i("push_restart", None, ["use-value", "restart_fn"]),
            _i("const", "handler_fn", ["my_handler"]),
            _i("push_handler", None, ["*", "handler_fn"]),
            _i("call", "ignored", ["inner_fn"]),
            # These instructions are unreachable after exit_to fires:
            _i("pop_handler", None, []),
            _i("pop_restart", None, []),
            _i("label", None, ["after"]),
            _i("ret", None, ["result"]),
        )

        vm = VMCore()
        ret = vm.execute(_mod("main", main_fn, inner_fn, my_handler, use_value_impl))
        assert ret == 42

    def test_restart_not_needed_no_exit_to(self) -> None:
        """If no signal fires the handler, execution completes normally."""
        use_value_impl = _fn("use_value_impl", [("arg", "any")],
            _i("exit_to", None, ["done", "arg"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main_fn = _fn("main", [],
            _i("establish_exit", None, ["done", "result", "after"]),
            _i("const", "restart_fn", ["use_value_impl"]),
            _i("push_restart", None, ["use-value", "restart_fn"]),
            # No signal, no call to inner code — just fall through.
            _i("pop_restart", None, []),
            _i("label", None, ["after"]),
            _i("const", "result", [99]),   # set result manually for this path
            _i("ret", None, ["result"]),
        )
        vm = VMCore()
        ret = vm.execute(_mod("main", main_fn, use_value_impl))
        assert ret == 99
