"""Tests for VMCOND00 Phase 2 — the ``throw`` opcode (Layer 2: Unwind Exceptions).

VMCOND00 Layer 2 adds one new IIR opcode to vm-core:

    throw  condition_reg
        Unwind the call stack looking for a matching entry in the static
        exception table of each frame.  On match, jump to the handler IP
        and place the condition object in the designated register.  If no
        match is found in the entire stack, raise UncaughtConditionError.

The static exception table lives on ``IIRFunction.exception_table`` as a
list of ``ExceptionTableEntry`` objects.  Each entry covers a half-open
instruction-index range ``[from_ip, to_ip)`` and specifies:

    handler_ip  — instruction index of the catch block's entry point
    type_id     — condition type to match ("*" = catch-all)
    val_reg     — register to receive the caught condition object

Coverage targets
----------------
- handle_throw: catch within same frame (catch-all and typed)
- handle_throw: throw propagates across frames (callee throws, caller catches)
- handle_throw: multiple frames, deepest match wins
- handle_throw: no handler → UncaughtConditionError
- handle_throw: range boundary: from_ip inclusive, to_ip exclusive
- handle_throw: type_id "*" matches any condition
- handle_throw: typed type_id matches by class name
- handle_throw: typed type_id does NOT match wrong class
- UncaughtConditionError: carries the original condition
- _throw_type_matches helper (indirectly via handle_throw)
"""

from __future__ import annotations

import pytest

from interpreter_ir import (
    CATCH_ALL,
    ExceptionTableEntry,
    IIRFunction,
    IIRInstr,
    IIRModule,
)

from vm_core import UncaughtConditionError, VMCore

# ---------------------------------------------------------------------------
# Helpers
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


def _i(op: str, dest: str | None = None, srcs: list | None = None,
       type_hint: str = "any") -> IIRInstr:
    return IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)


def _entry(from_ip: int, to_ip: int, handler_ip: int,
           type_id: str = CATCH_ALL, val_reg: str = "ex") -> ExceptionTableEntry:
    """Shorthand for building ExceptionTableEntry objects in tests."""
    return ExceptionTableEntry(
        from_ip=from_ip, to_ip=to_ip,
        handler_ip=handler_ip, type_id=type_id, val_reg=val_reg,
    )


# ---------------------------------------------------------------------------
# TestExceptionTableEntry — construction and field access
# ---------------------------------------------------------------------------


class TestExceptionTableEntry:
    """Test ExceptionTableEntry construction, immutability, and field access."""

    def test_basic_construction(self) -> None:
        """All five fields are stored correctly."""
        e = ExceptionTableEntry(
            from_ip=2, to_ip=6, handler_ip=10,
            type_id=CATCH_ALL, val_reg="ex",
        )
        assert e.from_ip == 2
        assert e.to_ip == 6
        assert e.handler_ip == 10
        assert e.type_id == CATCH_ALL
        assert e.val_reg == "ex"

    def test_catch_all_sentinel(self) -> None:
        """CATCH_ALL is the string '*'."""
        assert CATCH_ALL == "*"

    def test_frozen_dataclass(self) -> None:
        """ExceptionTableEntry is immutable after construction."""
        e = _entry(0, 5, 5)
        with pytest.raises(Exception):  # FrozenInstanceError
            e.from_ip = 99  # type: ignore[misc]

    def test_equality(self) -> None:
        """Two entries with identical fields are equal."""
        a = _entry(0, 5, 5)
        b = _entry(0, 5, 5)
        assert a == b

    def test_inequality(self) -> None:
        """Two entries with different fields are not equal."""
        a = _entry(0, 5, 5)
        b = _entry(0, 6, 5)
        assert a != b

    def test_typed_entry(self) -> None:
        """A non-catch-all entry stores its type_id."""
        e = ExceptionTableEntry(
            from_ip=0, to_ip=10, handler_ip=10,
            type_id="ValueError", val_reg="err",
        )
        assert e.type_id == "ValueError"
        assert e.val_reg == "err"


# ---------------------------------------------------------------------------
# TestThrowSameFrame — throw and catch in the same function
# ---------------------------------------------------------------------------


class TestThrowSameFrame:
    """throw caught within the same function frame (no cross-frame unwinding)."""

    def _run(self, cond_value: object, type_id: str = CATCH_ALL) -> object:
        """Run a single-function throw/catch program; return the caught value."""
        #
        # IIR program:
        #   main:
        #     const cond, <cond_value>    ; ip=0
        #     throw cond                  ; ip=1  ← inside [1, 2)
        #     const result, 999           ; ip=2  (unreachable)
        #     ret result                  ; ip=3  (unreachable)
        #   label catch_block             ; ip=4
        #     ret ex                      ; ip=5  ← handler returns caught value
        #
        # Exception table: [from_ip=1, to_ip=2, handler_ip=4, type_id=*, val_reg=ex]
        #
        prog = _fn(
            "main",
            [],
            _i("const", "cond", [cond_value]),   # ip=0
            _i("throw", None, ["cond"]),           # ip=1
            _i("const", "result", [999]),          # ip=2 (unreachable)
            _i("ret", None, ["result"]),            # ip=3 (unreachable)
            _i("label", None, ["catch_block"]),    # ip=4
            _i("ret", None, ["ex"]),               # ip=5
            exception_table=[_entry(1, 2, 4, type_id=type_id)],
        )
        vm = VMCore()
        return vm.execute(_mod(prog))

    def test_catch_all_catches_integer(self) -> None:
        """A catch-all ('*') entry catches a thrown integer."""
        result = self._run(42)
        assert result == 42

    def test_catch_all_catches_string(self) -> None:
        """A catch-all entry catches a thrown string."""
        result = self._run("oops")
        assert result == "oops"

    def test_catch_all_catches_none(self) -> None:
        """A catch-all entry catches None (a valid condition object)."""
        result = self._run(None)
        assert result is None

    def test_typed_catch_matches_correct_type(self) -> None:
        """type_id='int' catches an integer condition."""
        result = self._run(7, type_id="int")
        assert result == 7

    def test_typed_catch_does_not_match_wrong_type(self) -> None:
        """type_id='str' does NOT catch an integer — raises UncaughtConditionError."""
        with pytest.raises(UncaughtConditionError):
            self._run(7, type_id="str")

    def test_condition_placed_in_val_reg(self) -> None:
        """The thrown value is placed in the val_reg register (here 'ex')."""
        result = self._run("sentinel_value")
        assert result == "sentinel_value"

    def test_code_after_throw_is_not_executed(self) -> None:
        """Instructions between throw and the handler are skipped."""
        result = self._run(1)
        # If the code after throw (const result, 999) ran, we'd get 999.
        assert result != 999


# ---------------------------------------------------------------------------
# TestThrowRangeBoundaries — from_ip inclusive, to_ip exclusive
# ---------------------------------------------------------------------------


class TestThrowRangeBoundaries:
    """Verify the half-open [from_ip, to_ip) semantics of guarded ranges."""

    def _run_at_ip(self, throw_at_ip: int, entry_from: int, entry_to: int) -> object:
        """Build a program where throw lands at throw_at_ip; return result."""
        #
        # IIR program structure:
        #   ip 0: const cond, 99
        #   ip 1: nop (label placeholder for padding)
        #   ip 2: nop
        #   ip 3: throw cond    ← only used if throw_at_ip == 3
        #   ... (we use const/ret to arrive at the right ip)
        #
        # Actually, easiest approach: build a program that is entirely
        # pad instructions + one throw at a specific index.
        #
        # ip 0: const cond, 99
        # ip 1-N: const pad, 0   (enough to position the throw)
        # ip throw_at_ip: throw cond
        # ip throw_at_ip+1: const ok, 0 / ret ok   (fall-through — no throw)
        # ip catch_ip: label catch / ret cond (handler)
        #
        # But this gets complex. Simpler: just test the two critical
        # boundary positions explicitly.
        #
        #   from_ip=2, to_ip=4: entries at 2,3 are guarded; 1,4 are not.
        #
        instructions = [
            _i("const", "cond", [99]),    # ip=0
            _i("const", "pad", [0]),      # ip=1
            _i("const", "pad2", [0]),     # ip=2
            _i("const", "pad3", [0]),     # ip=3
            _i("const", "pad4", [0]),     # ip=4
        ]
        # Insert throw at throw_at_ip, shifting everything after it.
        throw_instr = _i("throw", None, ["cond"])
        instructions.insert(throw_at_ip, throw_instr)
        # Now handler_ip = throw_at_ip + 1 (label) + 1 (original instructions
        # after insertion).  This is getting complicated — just use a direct
        # approach: ret success if throw is NOT caught, otherwise ret handler_val.
        #
        # Simplest: build the program inline rather than parameterising.
        raise NotImplementedError("use explicit tests below instead")

    def test_from_ip_is_inclusive(self) -> None:
        """A throw AT from_ip (inclusive) IS caught by the entry."""
        #
        #   ip=0: const cond, 55
        #   ip=1: throw cond          ← throw at ip=1, entry covers [1, 3)
        #   ip=2: const ok, 0         (unreachable)
        #   ip=3: ret ok              (unreachable)
        #   ip=4: label catch
        #   ip=5: ret ex
        #
        prog = _fn(
            "main", [],
            _i("const", "cond", [55]),     # ip=0
            _i("throw", None, ["cond"]),   # ip=1 — AT from_ip=1 → caught
            _i("const", "ok", [0]),        # ip=2
            _i("ret", None, ["ok"]),       # ip=3
            _i("label", None, ["catch"]),  # ip=4
            _i("ret", None, ["ex"]),       # ip=5
            exception_table=[_entry(1, 3, 4)],  # covers [1, 3)
        )
        vm = VMCore()
        result = vm.execute(_mod(prog))
        assert result == 55

    def test_to_ip_is_exclusive(self) -> None:
        """A throw AT to_ip (exclusive) is NOT caught by the entry."""
        #
        #   ip=0: const cond, 55
        #   ip=1: const skip, 0       (guarded range covers only [1, 2))
        #   ip=2: throw cond          ← throw at ip=2, entry covers [1, 2) → NOT caught
        #   ip=3: const ok, 0         (unreachable if caught, reachable if not)
        #   ip=4: ret ok              (unreachable)
        #   ip=5: label catch
        #   ip=6: ret ex              (handler — should NOT be reached)
        #
        prog = _fn(
            "main", [],
            _i("const", "cond", [55]),     # ip=0
            _i("const", "skip", [0]),      # ip=1 (inside guarded range)
            _i("throw", None, ["cond"]),   # ip=2 — AT to_ip=2 → NOT caught
            _i("const", "ok", [0]),        # ip=3
            _i("ret", None, ["ok"]),       # ip=4
            _i("label", None, ["catch"]),  # ip=5
            _i("ret", None, ["ex"]),       # ip=6
            exception_table=[_entry(1, 2, 5)],  # covers only [1, 2)
        )
        vm = VMCore()
        with pytest.raises(UncaughtConditionError) as exc_info:
            vm.execute(_mod(prog))
        assert exc_info.value.condition == 55

    def test_throw_before_from_ip_not_caught(self) -> None:
        """A throw before from_ip is not covered by the entry."""
        #
        #   ip=0: const cond, 77
        #   ip=1: throw cond          ← throw at ip=1, entry covers [2, 4) → NOT caught
        #   ip=2: const ok, 0
        #   ip=3: ret ok
        #   ip=4: label catch
        #   ip=5: ret ex
        #
        prog = _fn(
            "main", [],
            _i("const", "cond", [77]),     # ip=0
            _i("throw", None, ["cond"]),   # ip=1 — before from_ip=2 → NOT caught
            _i("const", "ok", [0]),        # ip=2
            _i("ret", None, ["ok"]),       # ip=3
            _i("label", None, ["catch"]),  # ip=4
            _i("ret", None, ["ex"]),       # ip=5
            exception_table=[_entry(2, 4, 4)],  # covers [2, 4)
        )
        vm = VMCore()
        with pytest.raises(UncaughtConditionError) as exc_info:
            vm.execute(_mod(prog))
        assert exc_info.value.condition == 77


# ---------------------------------------------------------------------------
# TestThrowAcrossFrames — cross-frame unwinding
# ---------------------------------------------------------------------------


class TestThrowAcrossFrames:
    """throw propagates across call frames; catch in a caller."""

    def test_callee_throws_caller_catches(self) -> None:
        """callee throws, caller has an exception table entry that catches it."""
        #
        # thrower():
        #   ip=0: const cond, "error!"
        #   ip=1: throw cond
        #   ip=2: ret_void            (unreachable)
        #
        # main():
        #   ip=0: call thrower         ← inside [0, 1), catches the throw
        #   ip=1: const ok, 0          (unreachable)
        #   ip=2: ret ok               (unreachable)
        #   ip=3: label handler
        #   ip=4: ret ex               ← handler returns the caught condition
        #
        thrower = _fn(
            "thrower", [],
            _i("const", "cond", ["error!"]),
            _i("throw", None, ["cond"]),
            _i("ret_void"),
        )
        caller = _fn(
            "main", [],
            _i("call", "dummy", ["thrower"]),   # ip=0
            _i("const", "ok", [0]),              # ip=1
            _i("ret", None, ["ok"]),             # ip=2
            _i("label", None, ["handler"]),      # ip=3
            _i("ret", None, ["ex"]),             # ip=4
            exception_table=[_entry(0, 1, 3)],  # covers the call at ip=0
        )
        vm = VMCore()
        result = vm.execute(_mod(thrower, caller), fn="main")
        assert result == "error!"

    def test_callee_frames_popped_on_catch(self) -> None:
        """After catch, callee frames are gone — not dangling on the stack."""
        thrown = []

        def inspector(args: list) -> int:
            # If called, frames haven't been properly popped
            return 0

        thrower = _fn(
            "thrower", [],
            _i("const", "cond", [99]),
            _i("throw", None, ["cond"]),
            _i("ret_void"),
        )
        caller = _fn(
            "main", [],
            _i("call", "dummy", ["thrower"]),   # ip=0
            _i("ret", None, ["ex"]),             # ip=1 (unreachable)
            _i("label", None, ["h"]),            # ip=2
            _i("ret", None, ["ex"]),             # ip=3
            exception_table=[_entry(0, 1, 2)],
        )
        vm = VMCore()
        result = vm.execute(_mod(thrower, caller), fn="main")
        assert result == 99
        # If we successfully caught and returned, frames were properly unwound.

    def test_three_level_propagation(self) -> None:
        """Throw propagates through two intermediate frames to the outermost caller."""
        #
        # inner(): throw
        # middle(): call inner (no exception table)
        # outer(): call middle (exception table catches it)
        #
        inner = _fn(
            "inner", [],
            _i("const", "cond", ["deep"]),
            _i("throw", None, ["cond"]),
            _i("ret_void"),
        )
        middle = _fn(
            "middle", [],
            _i("call", "dummy", ["inner"]),
            _i("ret_void"),
        )
        outer = _fn(
            "outer", [],
            _i("call", "dummy", ["middle"]),   # ip=0
            _i("const", "ok", [0]),             # ip=1
            _i("ret", None, ["ok"]),            # ip=2
            _i("label", None, ["h"]),           # ip=3
            _i("ret", None, ["ex"]),            # ip=4
            exception_table=[_entry(0, 1, 3)],
        )
        vm = VMCore()
        result = vm.execute(_mod(inner, middle, outer), fn="outer")
        assert result == "deep"

    def test_no_handler_raises_uncaught(self) -> None:
        """No exception table anywhere → UncaughtConditionError."""
        thrower = _fn(
            "thrower", [],
            _i("const", "cond", ["gone"]),
            _i("throw", None, ["cond"]),
            _i("ret_void"),
        )
        caller = _fn(
            "main", [],
            _i("call", "dummy", ["thrower"]),
            _i("ret_void"),
        )
        vm = VMCore()
        with pytest.raises(UncaughtConditionError) as exc_info:
            vm.execute(_mod(thrower, caller), fn="main")
        assert exc_info.value.condition == "gone"

    def test_inner_handler_takes_priority_over_outer(self) -> None:
        """The innermost matching entry wins, not an outer one."""
        #
        # inner(): throw in guarded range
        # outer(): call inner — ALSO has an exception table entry
        #
        # The inner function's entry should match first.
        #
        inner = _fn(
            "inner", [],
            _i("const", "cond", [42]),             # ip=0
            _i("throw", None, ["cond"]),            # ip=1  ← inside [1, 2)
            _i("const", "ok", [0]),                 # ip=2
            _i("ret", None, ["ok"]),                # ip=3
            _i("label", None, ["inner_h"]),         # ip=4
            _i("const", "caught_inner", [1]),       # ip=5
            _i("ret", None, ["caught_inner"]),      # ip=6
            exception_table=[_entry(1, 2, 4)],      # inner catches first
        )
        outer = _fn(
            "outer", [],
            _i("call", "dummy", ["inner"]),         # ip=0
            _i("ret", None, ["dummy"]),             # ip=1
            _i("label", None, ["outer_h"]),         # ip=2
            _i("const", "caught_outer", [2]),       # ip=3
            _i("ret", None, ["caught_outer"]),      # ip=4
            exception_table=[_entry(0, 1, 2)],      # outer also has a handler
        )
        vm = VMCore()
        result = vm.execute(_mod(inner, outer), fn="outer")
        # inner caught it → returns 1 (not 2 from outer handler)
        assert result == 1


# ---------------------------------------------------------------------------
# TestUncaughtConditionError — error type and payload
# ---------------------------------------------------------------------------


class TestUncaughtConditionError:
    """Test UncaughtConditionError properties."""

    def test_condition_attribute_preserved(self) -> None:
        """UncaughtConditionError.condition holds the original thrown value."""
        cond = object()
        prog = _fn(
            "main", [],
            _i("const", "cond", [cond]),
            _i("throw", None, ["cond"]),
            _i("ret_void"),
        )
        vm = VMCore()
        with pytest.raises(UncaughtConditionError) as exc_info:
            vm.execute(_mod(prog))
        assert exc_info.value.condition is cond

    def test_str_representation(self) -> None:
        """str(UncaughtConditionError) mentions the condition."""
        err = UncaughtConditionError("oops")
        assert "oops" in str(err)

    def test_is_subclass_of_vmerror(self) -> None:
        """UncaughtConditionError inherits from VMError."""
        from vm_core import VMError
        assert issubclass(UncaughtConditionError, VMError)

    def test_throw_from_root_frame_uncaught(self) -> None:
        """Throw from root frame with no exception table raises UncaughtConditionError."""
        prog = _fn(
            "main", [],
            _i("const", "cond", ["root-level"]),
            _i("throw", None, ["cond"]),
            _i("ret_void"),
        )
        vm = VMCore()
        with pytest.raises(UncaughtConditionError) as exc_info:
            vm.execute(_mod(prog))
        assert exc_info.value.condition == "root-level"


# ---------------------------------------------------------------------------
# TestThrowTypeMatching — type_id semantics
# ---------------------------------------------------------------------------


class TestThrowTypeMatching:
    """Verify type_id matching rules for exception table entries."""

    def _run_typed(self, condition: object, type_id: str) -> bool:
        """Return True if the throw was caught (False if UncaughtConditionError)."""
        prog = _fn(
            "main", [],
            _i("const", "cond", [condition]),     # ip=0
            _i("throw", None, ["cond"]),           # ip=1
            _i("const", "missed", [0]),            # ip=2 (unreachable if caught)
            _i("ret", None, ["missed"]),           # ip=3
            _i("label", None, ["h"]),              # ip=4
            _i("const", "caught", [1]),            # ip=5
            _i("ret", None, ["caught"]),           # ip=6
            exception_table=[_entry(1, 2, 4, type_id=type_id)],
        )
        vm = VMCore()
        try:
            result = vm.execute(_mod(prog))
            return result == 1  # caught path
        except UncaughtConditionError:
            return False

    def test_catch_all_matches_int(self) -> None:
        assert self._run_typed(42, CATCH_ALL) is True

    def test_catch_all_matches_str(self) -> None:
        assert self._run_typed("err", CATCH_ALL) is True

    def test_catch_all_matches_list(self) -> None:
        assert self._run_typed([1, 2], CATCH_ALL) is True

    def test_catch_all_matches_none(self) -> None:
        # None has type NoneType
        assert self._run_typed(None, CATCH_ALL) is True

    def test_type_name_matches_int(self) -> None:
        """type(42).__name__ == 'int' → matches type_id='int'."""
        assert self._run_typed(42, "int") is True

    def test_type_name_matches_str(self) -> None:
        assert self._run_typed("x", "str") is True

    def test_type_name_mismatch(self) -> None:
        """type_id='str' does NOT match an int condition."""
        assert self._run_typed(42, "str") is False

    def test_type_name_matches_custom_class(self) -> None:
        """Custom class condition matched by its __name__."""
        class MyError:  # noqa: N801
            pass
        cond = MyError()
        assert self._run_typed(cond, "MyError") is True

    def test_type_name_mismatch_custom_class(self) -> None:
        """Wrong type_id for custom class is not caught."""
        class MyError:  # noqa: N801
            pass
        cond = MyError()
        assert self._run_typed(cond, "OtherError") is False
