"""Tests for LANG18 VM coverage mode.

Coverage mode records which IIR instruction indices are reached during
execution, keyed by function name.  It is entirely opt-in (disabled by
default), adds a single boolean guard per instruction, and is independent
of the LANG06 debug hooks.

Test strategy
-------------
- Default state: coverage is off, no data collected.
- After ``enable_coverage()`` + ``execute()``: data reflects executed instructions.
- Multiple runs accumulate data unless ``reset_coverage()`` is called.
- ``reset_coverage()`` clears data and disables coverage mode.
- ``coverage_data()`` returns immutable ``frozenset`` values.
- Coverage and debug mode can be active simultaneously without interference.
- ``disable_coverage()`` stops collecting; data is preserved.
- Unreached instructions (skipped by a branch) do NOT appear in data.
"""

from __future__ import annotations

from typing import Any

import pytest
from interpreter_ir import IIRFunction, IIRInstr, IIRModule

from vm_core import DebugHooks, VMCore


# ---------------------------------------------------------------------------
# Helpers — mirror the factory pattern from test_debug_hooks.py
# ---------------------------------------------------------------------------

def _i(op: str, dest: str | None, srcs: list[Any] | None = None,
       type_hint: str = "any") -> IIRInstr:
    return IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)


def _fn(name: str, *instrs: IIRInstr,
        params: list[tuple[str, str]] | None = None) -> IIRFunction:
    p = params or []
    return IIRFunction(
        name=name,
        params=p,
        return_type="any",
        instructions=list(instrs),
        register_count=max(8, len(p) + len(instrs) + 4),
    )


def _mod(*fns: IIRFunction) -> IIRModule:
    return IIRModule(name="test", functions=list(fns))


def _simple_module() -> IIRModule:
    """fn main(): const v=1; const w=2; return v  — 3 instructions."""
    main = _fn(
        "main",
        _i("const", "v", [1]),        # ip=0
        _i("const", "w", [2]),        # ip=1
        _i("ret",   None, ["v"]),     # ip=2
    )
    return _mod(main)


def _two_fn_module() -> IIRModule:
    """main calls helper.

    helper: const r=99; ret r      — ips 0,1
    main:   const a=5; call helper → b; ret b  — ips 0,1,2
    """
    helper = _fn(
        "helper",
        _i("const", "r", [99]),      # ip=0
        _i("ret",   None, ["r"]),    # ip=1
    )
    main = _fn(
        "main",
        _i("const", "a", [5]),                  # ip=0
        _i("call",  "b", ["helper"]),            # ip=1
        _i("ret",   None, ["b"]),                # ip=2
    )
    return IIRModule(name="test", functions=[helper, main], entry_point="main")


def _branch_module() -> IIRModule:
    """Conditional branch that is always taken (skips ip=2).

    main:
        const v=1               # ip=0
        jmp_if_true v, "done"   # ip=1  → always jumps to ip=3 (label "done")
        const v=99              # ip=2  (never reached)
        label "done"            # ip=3
        ret v                   # ip=4
    """
    main = _fn(
        "main",
        _i("const",        "v",  [1]),           # ip=0
        _i("jmp_if_true",  None, ["v", "done"]), # ip=1 — jumps to "done"
        _i("const",        "v",  [99]),          # ip=2 — never reached
        _i("label",        None, ["done"]),      # ip=3
        _i("ret",          None, ["v"]),         # ip=4
    )
    return _mod(main)


# ---------------------------------------------------------------------------
# Default state
# ---------------------------------------------------------------------------

class TestCoverageDefaultState:
    """Coverage mode is off by default; no data is collected."""

    def test_coverage_mode_off_by_default(self) -> None:
        vm = VMCore()
        assert not vm.is_coverage_mode()

    def test_no_data_before_enable(self) -> None:
        vm = VMCore()
        vm.execute(_simple_module())
        assert vm.coverage_data() == {}

    def test_coverage_data_empty_on_fresh_vm(self) -> None:
        vm = VMCore()
        assert vm.coverage_data() == {}


# ---------------------------------------------------------------------------
# Basic coverage collection
# ---------------------------------------------------------------------------

class TestCoverageCollection:
    """After enable_coverage() + execute(), data reflects executed instructions."""

    def test_enable_sets_flag(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        assert vm.is_coverage_mode()

    def test_covered_indices_present_after_run(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        data = vm.coverage_data()
        assert "main" in data
        # All 3 instructions are hit.
        assert data["main"] == frozenset({0, 1, 2})

    def test_all_three_instructions_covered(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        covered = vm.coverage_data()["main"]
        assert 0 in covered
        assert 1 in covered
        assert 2 in covered

    def test_coverage_data_values_are_frozensets(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        for ips in vm.coverage_data().values():
            assert isinstance(ips, frozenset)

    def test_coverage_data_is_snapshot_not_live_view(self) -> None:
        """Mutating the returned dict does not affect internal state."""
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        snapshot = vm.coverage_data()
        snapshot.pop("main", None)           # mutate the snapshot
        assert "main" in vm.coverage_data()  # live data unchanged

    def test_two_functions_both_covered(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_two_fn_module())
        data = vm.coverage_data()
        assert "main" in data
        assert "helper" in data

    def test_helper_coverage_matches_instruction_count(self) -> None:
        """helper has 2 instructions (ip 0 and 1); both must be recorded."""
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_two_fn_module())
        assert vm.coverage_data()["helper"] == frozenset({0, 1})


# ---------------------------------------------------------------------------
# Branch coverage — unreached branch is not in data
# ---------------------------------------------------------------------------

class TestBranchCoverage:
    """Unreached instructions must not appear in coverage data."""

    def test_always_true_branch_skips_ip2(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_branch_module())
        data = vm.coverage_data()["main"]
        assert 2 not in data

    def test_always_true_branch_reaches_ip0_ip1_ip3_ip4(self) -> None:
        """ip=0 (const), ip=1 (branch), ip=3 (label), ip=4 (ret) must all be hit."""
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_branch_module())
        data = vm.coverage_data()["main"]
        assert {0, 1, 3, 4} <= data


# ---------------------------------------------------------------------------
# Accumulation across multiple runs
# ---------------------------------------------------------------------------

class TestCoverageAccumulation:
    """Multiple runs accumulate data; reset_coverage clears it."""

    def test_second_run_does_not_erase_first(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        module = _simple_module()
        vm.execute(module)
        first = vm.coverage_data()["main"]
        vm.execute(module)
        second = vm.coverage_data()["main"]
        assert first == second  # same instructions covered

    def test_reset_coverage_clears_data(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        assert "main" in vm.coverage_data()
        vm.reset_coverage()
        assert vm.coverage_data() == {}

    def test_reset_coverage_disables_mode(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        vm.reset_coverage()
        assert not vm.is_coverage_mode()

    def test_after_reset_no_data_without_re_enable(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        module = _simple_module()
        vm.execute(module)
        vm.reset_coverage()
        vm.execute(module)          # coverage mode is now OFF
        assert vm.coverage_data() == {}


# ---------------------------------------------------------------------------
# disable_coverage
# ---------------------------------------------------------------------------

class TestDisableCoverage:
    """disable_coverage stops collection; existing data is preserved."""

    def test_disable_preserves_data(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        before = vm.coverage_data()
        vm.disable_coverage()
        assert vm.coverage_data() == before

    def test_disable_stops_collection(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.execute(_simple_module())
        vm.disable_coverage()
        vm.reset_coverage()         # clear, but mode is already off
        vm.execute(_simple_module())
        assert vm.coverage_data() == {}

    def test_disable_is_idempotent(self) -> None:
        vm = VMCore()
        vm.disable_coverage()       # called on already-off VM
        assert not vm.is_coverage_mode()

    def test_enable_is_idempotent(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        vm.enable_coverage()        # double-enable is safe
        assert vm.is_coverage_mode()


# ---------------------------------------------------------------------------
# Coverage + debug mode co-existence
# ---------------------------------------------------------------------------

class TestCoverageAndDebugMode:
    """Coverage and debug mode can both be active simultaneously."""

    def test_both_flags_can_be_true(self) -> None:
        class _NullHooks(DebugHooks):
            pass

        vm = VMCore()
        vm.attach_debug_hooks(_NullHooks())
        vm.enable_coverage()
        assert vm.is_debug_mode()
        assert vm.is_coverage_mode()

    def test_coverage_collected_while_debug_hooks_attached(self) -> None:
        """Coverage still works when debug hooks fire on_instruction."""
        vm = VMCore()

        class _StepAllHooks(DebugHooks):
            def on_instruction(self, frame, instr) -> None:  # type: ignore[override]
                vm.step_in()

        vm.attach_debug_hooks(_StepAllHooks())
        vm.enable_coverage()
        # Seed a breakpoint at ip=0 so on_instruction fires on the first instr.
        vm.set_breakpoint(0, "main")
        vm.execute(_simple_module())

        data = vm.coverage_data()
        assert "main" in data
        # All instructions still counted even with debug hooks active.
        assert data["main"] == frozenset({0, 1, 2})

    def test_debug_mode_off_does_not_prevent_coverage(self) -> None:
        vm = VMCore()
        vm.enable_coverage()
        assert not vm.is_debug_mode()
        vm.execute(_simple_module())
        assert "main" in vm.coverage_data()
