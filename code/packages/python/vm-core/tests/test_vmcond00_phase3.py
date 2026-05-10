"""Tests for VMCOND00 Phase 3 — Layer 3 Dynamic Handlers.

VMCOND00 Layer 3 adds a non-unwinding condition handler chain to vm-core.
Five new IIR opcodes implement the full protocol:

    push_handler  type_id, fn_reg
        Push a new handler onto ``vm._handler_chain``.  ``type_id`` is an
        immediate string (``"*"`` for catch-all or a Python type name).
        ``fn_reg`` is a register holding the name of the IIR handler function.

    pop_handler
        Pop the most recently pushed handler.  Raises ``HandlerChainError``
        on underflow.

    signal  condition_reg
        Walk the handler chain most-recent → oldest.  On the first match,
        invoke the handler non-unwinding (push a frame on top of the caller,
        resume after return).  If no handler matches, continue silently
        (no-op).

    error  condition_reg
        Like ``signal`` but:
          1. Checks the Layer 2 static exception table first — if the current
             IP is inside a guarded range that covers this condition, delegate
             to the Layer 2 unwind path (so Layer 2 wins over Layer 3).
          2. If no handler matches anywhere, raises ``UncaughtConditionError``
             instead of continuing silently.

    warn  condition_reg
        Like ``signal`` but, if no handler matches, emits a ``[vm-core WARN]``
        line to ``sys.stderr`` and continues.  Never aborts.

Non-unwinding invocation protocol
----------------------------------
When a signaling opcode finds a matching handler:

  1. A fresh ``VMFrame`` is pushed for the handler function.
  2. The condition is copied into the handler's register 0 (its first param).
  3. The dispatch loop runs inside the handler.
  4. When the handler executes ``ret``, ``handle_ret`` pops the handler frame
     and the dispatch loop resumes in the *original* frame — at the
     instruction *after* the signaling opcode (because ``frame.ip`` was
     already incremented before the signal handler ran).

Coverage targets
----------------
- HandlerNode: construction and field access
- HandlerChainError: VMError subclass, message
- push_handler: populates vm._handler_chain
- pop_handler: drains vm._handler_chain; underflow → HandlerChainError
- signal: no handler → no-op; catch-all match; type-specific match
- signal: type-mismatch → no-op
- signal: handler receives the correct condition value
- signal: execution continues at the instruction AFTER signal
- error: no handler → UncaughtConditionError
- error: catch-all Layer 3 handler invoked
- error: Layer 2 exception table takes priority over Layer 3 chain
- error: Layer 3 used when Layer 2 entry does not cover the IP
- warn: no handler → stderr message, execution continues
- warn: handler matched → invoked, no stderr
- cross-frame: handler pushed in outer frame, signal raised in inner callee
- LIFO handler order: most-recently-pushed handler wins
"""

from __future__ import annotations

import io
import sys

import pytest
from interpreter_ir import (
    CATCH_ALL,
    ExceptionTableEntry,
    IIRFunction,
    IIRInstr,
    IIRModule,
)

from vm_core import (
    HandlerChainError,
    HandlerNode,
    UncaughtConditionError,
    VMCore,
    VMError,
)

# ---------------------------------------------------------------------------
# Helpers (mirroring test_vmcond00_phase2.py conventions)
# ---------------------------------------------------------------------------


def _fn(
    name: str,
    params: list[tuple[str, str]],
    *instrs: IIRInstr,
    return_type: str = "any",
    exception_table: list[ExceptionTableEntry] | None = None,
) -> IIRFunction:
    """Build an IIRFunction with auto-computed register_count."""
    fn = IIRFunction(
        name=name,
        params=params,
        return_type=return_type,
        instructions=list(instrs),
        register_count=max(8, len(params) + len(instrs)),
    )
    if exception_table is not None:
        fn.exception_table = exception_table
    return fn


def _mod(*fns: IIRFunction) -> IIRModule:
    return IIRModule(name="test", functions=list(fns))


def _i(
    op: str,
    dest: str | None = None,
    srcs: list | None = None,
    type_hint: str = "any",
) -> IIRInstr:
    return IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)


def _entry(
    from_ip: int, to_ip: int, handler_ip: int,
    type_id: str = CATCH_ALL, val_reg: str = "ex",
) -> ExceptionTableEntry:
    return ExceptionTableEntry(
        from_ip=from_ip, to_ip=to_ip,
        handler_ip=handler_ip, type_id=type_id, val_reg=val_reg,
    )


# ---------------------------------------------------------------------------
# TestHandlerNode — dataclass construction
# ---------------------------------------------------------------------------


class TestHandlerNode:
    """HandlerNode construction and field access."""

    def test_basic_construction(self) -> None:
        """All three fields are stored correctly."""
        node = HandlerNode(condition_type="*", handler_fn="my_fn", stack_depth=2)
        assert node.condition_type == "*"
        assert node.handler_fn == "my_fn"
        assert node.stack_depth == 2

    def test_catch_all_condition_type(self) -> None:
        """Catch-all uses the '*' sentinel (same as CATCH_ALL)."""
        node = HandlerNode(condition_type=CATCH_ALL, handler_fn="h", stack_depth=0)
        assert node.condition_type == "*"

    def test_typed_condition_type(self) -> None:
        """Type-specific handlers store the type name as a string."""
        node = HandlerNode(condition_type="ValueError", handler_fn="h", stack_depth=1)
        assert node.condition_type == "ValueError"

    def test_handler_fn_is_arbitrary_object(self) -> None:
        """handler_fn typed as 'object' — a callable can be stored (Phase 4)."""
        def my_closure(c: object) -> None:
            pass
        node = HandlerNode(condition_type="*", handler_fn=my_closure, stack_depth=0)
        assert node.handler_fn is my_closure

    def test_equality(self) -> None:
        """Two HandlerNodes with identical fields compare equal."""
        a = HandlerNode(condition_type="*", handler_fn="h", stack_depth=3)
        b = HandlerNode(condition_type="*", handler_fn="h", stack_depth=3)
        assert a == b

    def test_inequality_different_type(self) -> None:
        """Nodes with different condition_type are not equal."""
        a = HandlerNode(condition_type="*", handler_fn="h", stack_depth=0)
        b = HandlerNode(condition_type="int", handler_fn="h", stack_depth=0)
        assert a != b


# ---------------------------------------------------------------------------
# TestHandlerChainError — error type hierarchy
# ---------------------------------------------------------------------------


class TestHandlerChainError:
    """HandlerChainError is a VMError raised on pop_handler underflow."""

    def test_is_vm_error(self) -> None:
        """HandlerChainError inherits from VMError."""
        err = HandlerChainError("underflow")
        assert isinstance(err, VMError)

    def test_message_preserved(self) -> None:
        """The message is accessible via str()."""
        err = HandlerChainError("pop_handler on empty chain")
        assert "pop_handler" in str(err)

    def test_is_exception(self) -> None:
        """HandlerChainError can be raised and caught."""
        with pytest.raises(HandlerChainError):
            raise HandlerChainError("test")


# ---------------------------------------------------------------------------
# TestInvokeHandlerValidation — security guards in _invoke_handler_nonunwinding
# ---------------------------------------------------------------------------


class TestInvokeHandlerValidation:
    """Security: _invoke_handler_nonunwinding validates handler_fn before use.

    These tests exercise the two validation guards added by the security
    review:
      1. handler_fn must be a str (not an arbitrary object).
      2. The named function must exist in the module.

    Both paths raise HandlerChainError rather than leaking an opaque
    AttributeError or TypeError to the caller.
    """

    def test_non_string_handler_fn_raises_handler_chain_error(self) -> None:
        """If handler_fn is not a str, HandlerChainError is raised.

        Simulates a frontend code-gen bug where an integer ends up in the
        handler-fn register instead of a function name string.  The VM must
        reject it with a clean VMError, not crash with AttributeError.
        """
        from vm_core.handler_chain import HandlerNode

        # Build a minimal module and seed the chain with a bad node directly.
        noop = _fn(
            "main", [],
            _i("const", "cond", [1]),
            _i("signal", None, ["cond"]),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        # Inject a handler with a non-string handler_fn (integer) before executing.
        # We do this by running the VM with a push_handler that stores an integer
        # in the fn register — but since IIR `const` stores it as-is, this is
        # achievable via a const instruction with an integer value.
        #
        # Simpler approach: directly seed _handler_chain before calling execute()
        # by running a trivial module to initialize the VM, then injecting.
        vm._module = _mod(noop)
        vm._handler_chain = [
            HandlerNode(condition_type="*", handler_fn=42, stack_depth=0)
        ]
        # The signal instruction will find the handler and try to invoke it.
        # Since handler_fn=42 (not str), HandlerChainError should be raised.
        with pytest.raises(HandlerChainError, match="handler_fn must be a str"):
            vm.execute(_mod(noop))

    def test_unknown_function_name_raises_handler_chain_error(self) -> None:
        """If handler_fn names a function not in the module, HandlerChainError is raised.

        This guards against dangling handler references (e.g., module was
        reloaded without the handler function, or a typo in the function name).
        """
        from vm_core.handler_chain import HandlerNode

        main = _fn(
            "main", [],
            _i("const", "cond", [1]),
            _i("signal", None, ["cond"]),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        vm._module = _mod(main)
        vm._handler_chain = [
            HandlerNode(
                condition_type="*",
                handler_fn="nonexistent_function",
                stack_depth=0,
            )
        ]
        with pytest.raises(HandlerChainError, match="nonexistent_function"):
            vm.execute(_mod(main))

    def test_frame_overflow_raises_frame_overflow_error(self) -> None:
        """Invoking a handler when the frame stack is at max_frames raises FrameOverflowError.

        This ensures that a guest program cannot bypass the max_frames DoS guard
        by using signal/error/warn with a matching handler in a recursive pattern.
        """
        from vm_core import FrameOverflowError
        from vm_core.handler_chain import HandlerNode

        noop_handler = _fn(
            "noop_handler", [("cond", "any")],
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main = _fn(
            "main", [],
            _i("const", "cond", [1]),
            _i("signal", None, ["cond"]),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore(max_frames=1)  # set limit to 1 frame (just main)
        # Inject a handler node so signal will try to invoke it.
        vm._module = _mod(main, noop_handler)
        vm._handler_chain = [
            HandlerNode(
                condition_type="*",
                handler_fn="noop_handler",
                stack_depth=0,
            )
        ]
        # With max_frames=1, the main frame fills the limit; pushing a handler
        # frame should raise FrameOverflowError.
        with pytest.raises(FrameOverflowError):
            vm.execute(_mod(main, noop_handler))


# ---------------------------------------------------------------------------
# TestPushPopHandler — handler chain mechanics
# ---------------------------------------------------------------------------


class TestPushPopHandler:
    """push_handler / pop_handler manage vm._handler_chain correctly."""

    def test_push_adds_node_to_chain(self) -> None:
        """After push_handler, one HandlerNode is on vm._handler_chain."""
        #
        #   ip=0: const hfn, "noop_handler"
        #   ip=1: push_handler *, hfn
        #   ip=2: pop_handler
        #   ip=3: const ok, 1
        #   ip=4: ret ok
        #
        # We capture the VM's handler chain state by observing side effects.
        # The cleanest check is to run push_handler then pop_handler and verify
        # no crash (and the chain is empty at the end).
        noop = _fn(
            "noop_handler", [("cond", "any")],
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main = _fn(
            "main", [],
            _i("const", "hfn", ["noop_handler"]),   # ip=0
            _i("push_handler", None, ["*", "hfn"]),  # ip=1
            _i("pop_handler", None, []),              # ip=2
            _i("const", "ok", [1]),                  # ip=3
            _i("ret", None, ["ok"]),                 # ip=4
        )
        vm = VMCore()
        result = vm.execute(_mod(main, noop))
        assert result == 1
        # Chain is empty after pop — no leak.
        assert vm._handler_chain == []

    def test_multiple_push_pop_balanced(self) -> None:
        """Multiple balanced push/pop leaves an empty chain."""
        noop = _fn(
            "noop_handler", [("cond", "any")],
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main = _fn(
            "main", [],
            _i("const", "hfn", ["noop_handler"]),   # ip=0
            _i("push_handler", None, ["*", "hfn"]),  # ip=1
            _i("push_handler", None, ["int", "hfn"]), # ip=2
            _i("pop_handler", None, []),              # ip=3
            _i("pop_handler", None, []),              # ip=4
            _i("const", "ok", [2]),                  # ip=5
            _i("ret", None, ["ok"]),                 # ip=6
        )
        vm = VMCore()
        result = vm.execute(_mod(main, noop))
        assert result == 2
        assert vm._handler_chain == []

    def test_pop_handler_underflow_raises(self) -> None:
        """pop_handler with an empty chain raises HandlerChainError."""
        main = _fn(
            "main", [],
            _i("pop_handler", None, []),  # ip=0 — chain is empty → underflow
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        with pytest.raises(HandlerChainError):
            vm.execute(_mod(main))

    def test_pop_handler_underflow_message(self) -> None:
        """HandlerChainError message mentions pop_handler and the imbalance."""
        main = _fn(
            "main", [],
            _i("pop_handler", None, []),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        with pytest.raises(HandlerChainError, match="pop_handler"):
            vm.execute(_mod(main))


# ---------------------------------------------------------------------------
# TestSignalOpcode — signal semantics
# ---------------------------------------------------------------------------


class TestSignalOpcode:
    """Tests for the ``signal`` opcode."""

    # -----------------------------------------------------------------------
    # Helper: a handler that writes the condition to io port 99.
    # After execution, check vm._io_ports[99] == expected_condition.
    # -----------------------------------------------------------------------

    @staticmethod
    def _handler_writes_port() -> IIRFunction:
        """Handler that stores its condition argument in io port 99."""
        return _fn(
            "capturing_handler", [("cond", "any")],
            _i("const", "port", [99]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )

    def test_signal_no_handler_is_noop(self) -> None:
        """signal with no handler on the chain continues silently."""
        #
        #   ip=0: const cond, 42
        #   ip=1: signal cond          ← no handler → no-op
        #   ip=2: const ok, 99
        #   ip=3: ret ok
        #
        main = _fn(
            "main", [],
            _i("const", "cond", [42]),         # ip=0
            _i("signal", None, ["cond"]),       # ip=1 — no handler → continue
            _i("const", "ok", [99]),            # ip=2
            _i("ret", None, ["ok"]),            # ip=3
        )
        vm = VMCore()
        result = vm.execute(_mod(main))
        assert result == 99

    def test_signal_catch_all_handler_invoked(self) -> None:
        """signal invokes a catch-all ('*') handler."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [77]),                       # ip=0
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["*", "hfn"]),           # ip=2
            _i("signal", None, ["cond"]),                    # ip=3 → handler runs
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [1]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
        )
        vm = VMCore()
        result = vm.execute(_mod(main, handler))
        # Execution continued after signal.
        assert result == 1
        # Handler ran and stored the condition in port 99.
        assert vm._io_ports.get(99) == 77

    def test_signal_execution_resumes_after_signal_instr(self) -> None:
        """After signal + handler return, ip resumes at the next instruction."""
        noop_handler = _fn(
            "noop_handler", [("cond", "any")],
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main = _fn(
            "main", [],
            _i("const", "cond", [1]),                        # ip=0
            _i("const", "hfn", ["noop_handler"]),            # ip=1
            _i("push_handler", None, ["*", "hfn"]),           # ip=2
            _i("signal", None, ["cond"]),                    # ip=3
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "after", [55]),                      # ip=5 ← must reach here
            _i("ret", None, ["after"]),                      # ip=6
        )
        vm = VMCore()
        result = vm.execute(_mod(main, noop_handler))
        assert result == 55

    def test_signal_handler_receives_condition(self) -> None:
        """The condition object is passed as the first argument to the handler."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [12345]),                    # ip=0
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["*", "hfn"]),           # ip=2
            _i("signal", None, ["cond"]),                    # ip=3
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [0]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
        )
        vm = VMCore()
        vm.execute(_mod(main, handler))
        assert vm._io_ports.get(99) == 12345

    def test_signal_type_match_exact(self) -> None:
        """A typed handler is invoked when the condition's type name matches."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [7]),                        # ip=0 — cond is int
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["int", "hfn"]),         # ip=2 — type="int"
            _i("signal", None, ["cond"]),                    # ip=3 — "int" matches
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [0]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
        )
        vm = VMCore()
        vm.execute(_mod(main, handler))
        assert vm._io_ports.get(99) == 7

    def test_signal_type_mismatch_is_noop(self) -> None:
        """signal does NOT invoke a typed handler if the type name doesn't match."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [7]),                        # ip=0 — cond is int
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["str", "hfn"]),         # ip=2 type="str" ≠ "int"
            _i("signal", None, ["cond"]),                    # ip=3 — no match → no-op
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [9]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
        )
        vm = VMCore()
        result = vm.execute(_mod(main, handler))
        assert result == 9
        # Handler did NOT run.
        assert vm._io_ports.get(99) is None

    def test_signal_string_condition_matched_by_type(self) -> None:
        """A handler for 'str' matches when the condition is a str."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", ["hello"]),                  # ip=0 — cond is str
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["str", "hfn"]),         # ip=2
            _i("signal", None, ["cond"]),                    # ip=3
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [0]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
        )
        vm = VMCore()
        vm.execute(_mod(main, handler))
        assert vm._io_ports.get(99) == "hello"


# ---------------------------------------------------------------------------
# TestErrorOpcode — error semantics
# ---------------------------------------------------------------------------


class TestErrorOpcode:
    """Tests for the ``error`` opcode."""

    @staticmethod
    def _handler_writes_port() -> IIRFunction:
        """Handler that stores its condition argument in io port 99."""
        return _fn(
            "capturing_handler", [("cond", "any")],
            _i("const", "port", [99]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )

    def test_error_no_handler_raises_uncaught(self) -> None:
        """error with no handler anywhere raises UncaughtConditionError."""
        main = _fn(
            "main", [],
            _i("const", "cond", [99]),
            _i("error", None, ["cond"]),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        with pytest.raises(UncaughtConditionError) as exc_info:
            vm.execute(_mod(main))
        assert exc_info.value.condition == 99

    def test_error_layer3_handler_invoked(self) -> None:
        """error invokes a matching Layer 3 handler."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [88]),                       # ip=0
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["*", "hfn"]),           # ip=2
            _i("error", None, ["cond"]),                     # ip=3
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [0]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
        )
        vm = VMCore()
        result = vm.execute(_mod(main, handler))
        assert result == 0
        assert vm._io_ports.get(99) == 88

    def test_error_layer2_priority_over_layer3(self) -> None:
        """Layer 2 exception table takes priority over Layer 3 handler chain.

        When error fires and the current IP is inside a Layer 2 guarded range
        that covers the condition type, the Layer 2 unwind path runs — the
        Layer 3 handler is NOT invoked.

        Program layout (exception_table covers [3, 4)):

            ip=0: const cond, 55
            ip=1: const hfn, "capturing_handler"
            ip=2: push_handler *, hfn          (Layer 3 registered)
            ip=3: error cond                   ← inside [3,4) → Layer 2 wins
            ip=4: pop_handler                  (unreachable — Layer 2 unwinds)
            ip=5: const unreachable, 0
            ip=6: ret unreachable
            ip=7: label l2_catch
            ip=8: ret ex                       (Layer 2 handler: return the condition)

        Expected: Layer 2 catch runs (returns 55), Layer 3 handler NOT invoked
                  (io port 99 remains unset).
        """
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [55]),                       # ip=0
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["*", "hfn"]),           # ip=2
            _i("error", None, ["cond"]),                     # ip=3 ← in [3,4)
            _i("pop_handler", None, []),                      # ip=4 (unreachable)
            _i("const", "unreachable", [0]),                 # ip=5 (unreachable)
            _i("ret", None, ["unreachable"]),                # ip=6 (unreachable)
            _i("label", None, ["l2_catch"]),                 # ip=7
            _i("ret", None, ["ex"]),                         # ip=8
            exception_table=[_entry(3, 4, 7, "*", "ex")],
        )
        vm = VMCore()
        result = vm.execute(_mod(main, handler))
        # Layer 2 catch returned the condition (55).
        assert result == 55
        # Layer 3 handler NOT invoked.
        assert vm._io_ports.get(99) is None

    def test_error_layer2_not_triggered_outside_range(self) -> None:
        """Layer 3 is used when the error falls outside the Layer 2 range.

        The exception table covers ip=2 only ([2, 3)).  The error fires at ip=3
        — outside the range — so Layer 3 handler handles it.

            ip=0: const cond, 33
            ip=1: const ok, 0        (inside [2,3) range? no — ip 1 not in range)
            ip=2: const skip, 0      (ip=2: inside [2,3) — but not the error instr)
            ip=3: error cond         ← ip=3, exception_table covers [2,3) → NO match
            ip=4: pop_handler
            ip=5: const ok, 0
            ip=6: ret ok

        Layer 3 handler fires; Layer 2 skipped.
        """
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [33]),                       # ip=0
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["*", "hfn"]),           # ip=2
            _i("error", None, ["cond"]),                     # ip=3 — outside [2,3)
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [0]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
            _i("label", None, ["l2_catch"]),                 # ip=7
            _i("ret", None, ["ex"]),                         # ip=8
            exception_table=[_entry(2, 3, 7, "*", "ex")],  # covers only [2,3)
        )
        vm = VMCore()
        vm.execute(_mod(main, handler))
        # Layer 3 handler ran.
        assert vm._io_ports.get(99) == 33

    def test_error_typed_handler_no_match_raises(self) -> None:
        """error raises UncaughtConditionError when the typed handler does not match."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [7]),                        # ip=0 — int
            _i("const", "hfn", ["capturing_handler"]),       # ip=1
            _i("push_handler", None, ["str", "hfn"]),         # ip=2 — "str" ≠ "int"
            _i("error", None, ["cond"]),                     # ip=3 — no match
            _i("pop_handler", None, []),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        with pytest.raises(UncaughtConditionError):
            vm.execute(_mod(main, handler))


# ---------------------------------------------------------------------------
# TestWarnOpcode — warn semantics
# ---------------------------------------------------------------------------


class TestWarnOpcode:
    """Tests for the ``warn`` opcode."""

    @staticmethod
    def _handler_writes_port() -> IIRFunction:
        return _fn(
            "capturing_handler", [("cond", "any")],
            _i("const", "port", [99]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )

    def test_warn_no_handler_emits_to_stderr(self) -> None:
        """warn with no handler writes a [vm-core WARN] line to stderr."""
        main = _fn(
            "main", [],
            _i("const", "cond", ["something unusual"]),    # ip=0
            _i("warn", None, ["cond"]),                   # ip=1 → stderr
            _i("const", "ok", [5]),                       # ip=2
            _i("ret", None, ["ok"]),                      # ip=3
        )
        vm = VMCore()
        captured = io.StringIO()
        result = None
        # Capture stderr output.
        saved_stderr = sys.stderr
        sys.stderr = captured
        try:
            result = vm.execute(_mod(main))
        finally:
            sys.stderr = saved_stderr
        # Execution continued and returned normally.
        assert result == 5
        # The warning was emitted to stderr.
        output = captured.getvalue()
        assert "[vm-core WARN]" in output
        assert "something unusual" in output

    def test_warn_no_handler_continues_execution(self) -> None:
        """warn does not abort — execution resumes after the warn instruction."""
        main = _fn(
            "main", [],
            _i("const", "cond", [0]),
            _i("warn", None, ["cond"]),
            _i("const", "sentinel", [42]),
            _i("ret", None, ["sentinel"]),
        )
        vm = VMCore()
        # Suppress stderr noise in tests.
        sys.stderr = io.StringIO()
        try:
            result = vm.execute(_mod(main))
        finally:
            sys.stderr = sys.__stderr__
        assert result == 42

    def test_warn_handler_invoked_no_stderr(self) -> None:
        """When a handler matches warn, it is invoked and no stderr output occurs."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [66]),
            _i("const", "hfn", ["capturing_handler"]),
            _i("push_handler", None, ["*", "hfn"]),
            _i("warn", None, ["cond"]),
            _i("pop_handler", None, []),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        captured = io.StringIO()
        saved_stderr = sys.stderr
        sys.stderr = captured
        try:
            result = vm.execute(_mod(main, handler))
        finally:
            sys.stderr = saved_stderr
        assert result == 0
        # Handler ran.
        assert vm._io_ports.get(99) == 66
        # No stderr output when a handler matched.
        assert captured.getvalue() == ""

    def test_warn_type_mismatch_emits_stderr(self) -> None:
        """warn with a typed handler that doesn't match still emits to stderr."""
        handler = self._handler_writes_port()
        main = _fn(
            "main", [],
            _i("const", "cond", [7]),                      # int
            _i("const", "hfn", ["capturing_handler"]),
            _i("push_handler", None, ["str", "hfn"]),       # type="str" ≠ "int"
            _i("warn", None, ["cond"]),                    # no match → stderr
            _i("pop_handler", None, []),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        captured = io.StringIO()
        saved_stderr = sys.stderr
        sys.stderr = captured
        try:
            result = vm.execute(_mod(main, handler))
        finally:
            sys.stderr = saved_stderr
        assert result == 0
        assert "[vm-core WARN]" in captured.getvalue()
        # Handler did NOT run.
        assert vm._io_ports.get(99) is None


# ---------------------------------------------------------------------------
# TestCrossFrameHandler — handler chain visible across call frames
# ---------------------------------------------------------------------------


class TestCrossFrameHandler:
    """Handler pushed in an outer frame is visible when an inner callee signals."""

    def test_signal_in_callee_caught_by_outer_handler(self) -> None:
        """A handler pushed in main() intercepts a signal fired in an inner call.

        Call graph:  main → inner → signal
        Handler:     registered in main before calling inner.

        Program:
            main:
              ip=0: const hfn, "capturing_handler"
              ip=1: push_handler *, hfn
              ip=2: const dummy, 0
              ip=3: call "inner", dummy   ← callee signals
              ip=4: pop_handler
              ip=5: const ok, 0
              ip=6: ret ok

            inner (no params needed, but uses dummy arg):
              ip=0: const cond, 999
              ip=1: signal cond           ← handler is in caller's chain
              ip=2: const z, 0
              ip=3: ret z

            capturing_handler:
              ... writes cond to port 99 ...
        """
        handler = _fn(
            "capturing_handler", [("cond", "any")],
            _i("const", "port", [99]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        inner = _fn(
            "inner", [("dummy", "any")],
            _i("const", "cond", [999]),                      # ip=0
            _i("signal", None, ["cond"]),                   # ip=1
            _i("const", "z", [0]),                          # ip=2
            _i("ret", None, ["z"]),                         # ip=3
        )
        main = _fn(
            "main", [],
            _i("const", "hfn", ["capturing_handler"]),       # ip=0
            _i("push_handler", None, ["*", "hfn"]),           # ip=1
            _i("const", "dummy", [0]),                       # ip=2
            _i("call", "unused", ["inner", "dummy"]),        # ip=3
            _i("pop_handler", None, []),                      # ip=4
            _i("const", "ok", [0]),                          # ip=5
            _i("ret", None, ["ok"]),                         # ip=6
        )
        vm = VMCore()
        result = vm.execute(_mod(main, inner, handler))
        assert result == 0
        # Handler ran in response to inner's signal.
        assert vm._io_ports.get(99) == 999

    def test_error_in_callee_caught_by_outer_handler(self) -> None:
        """A Layer 3 error in an inner call is handled by the outer handler chain."""
        handler = _fn(
            "capturing_handler", [("cond", "any")],
            _i("const", "port", [99]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        inner = _fn(
            "inner", [("dummy", "any")],
            _i("const", "cond", [444]),
            _i("error", None, ["cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main = _fn(
            "main", [],
            _i("const", "hfn", ["capturing_handler"]),
            _i("push_handler", None, ["*", "hfn"]),
            _i("const", "dummy", [0]),
            _i("call", "unused", ["inner", "dummy"]),
            _i("pop_handler", None, []),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        result = vm.execute(_mod(main, inner, handler))
        assert result == 0
        assert vm._io_ports.get(99) == 444


# ---------------------------------------------------------------------------
# TestHandlerChainLIFO — most-recently-pushed handler wins
# ---------------------------------------------------------------------------


class TestHandlerChainLIFO:
    """The handler chain is LIFO: the most recently pushed handler wins."""

    def test_innermost_handler_wins(self) -> None:
        """When two handlers are pushed, the most recent one intercepts signal.

        Both handlers are catch-all ('*'), but write to different io ports:
        - outer_handler writes to port 10
        - inner_handler writes to port 20

        Since inner was pushed last, it should intercept the signal.
        """
        outer = _fn(
            "outer_handler", [("cond", "any")],
            _i("const", "port", [10]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        inner = _fn(
            "inner_handler", [("cond", "any")],
            _i("const", "port", [20]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main = _fn(
            "main", [],
            _i("const", "cond", [111]),
            _i("const", "outer_fn", ["outer_handler"]),
            _i("push_handler", None, ["*", "outer_fn"]),   # pushed first (older)
            _i("const", "inner_fn", ["inner_handler"]),
            _i("push_handler", None, ["*", "inner_fn"]),   # pushed second (newer)
            _i("signal", None, ["cond"]),                 # inner handler wins
            _i("pop_handler", None, []),
            _i("pop_handler", None, []),
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        vm.execute(_mod(main, outer, inner))
        # Inner handler (port 20) ran.
        assert vm._io_ports.get(20) == 111
        # Outer handler (port 10) did NOT run.
        assert vm._io_ports.get(10) is None

    def test_pop_restores_outer_handler(self) -> None:
        """After popping the inner handler, the outer handler is active again."""
        outer = _fn(
            "outer_handler", [("cond", "any")],
            _i("const", "port", [10]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        inner = _fn(
            "inner_handler", [("cond", "any")],
            _i("const", "port", [20]),
            _i("io_out", None, ["port", "cond"]),
            _i("const", "z", [0]),
            _i("ret", None, ["z"]),
        )
        main = _fn(
            "main", [],
            _i("const", "cond", [7]),
            _i("const", "outer_fn", ["outer_handler"]),
            _i("push_handler", None, ["*", "outer_fn"]),
            _i("const", "inner_fn", ["inner_handler"]),
            _i("push_handler", None, ["*", "inner_fn"]),
            _i("pop_handler", None, []),                   # pop inner
            _i("signal", None, ["cond"]),                 # outer now active
            _i("pop_handler", None, []),                   # pop outer
            _i("const", "ok", [0]),
            _i("ret", None, ["ok"]),
        )
        vm = VMCore()
        vm.execute(_mod(main, outer, inner))
        # Outer handler (port 10) ran this time.
        assert vm._io_ports.get(10) == 7
        # Inner handler (port 20) did NOT run (was already popped).
        assert vm._io_ports.get(20) is None
