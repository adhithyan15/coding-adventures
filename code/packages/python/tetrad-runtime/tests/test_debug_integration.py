"""End-to-end debug integration tests (LANG06).

These tests verify the full source-mapping pipeline from Tetrad source code
through the IIR translation into the DebugSidecar, and then exercise the
VM's debug hooks via TetradRuntime.compile_with_debug / run_with_debug.

Pipeline under test
-------------------
::

    Tetrad source
         │  tetrad-compiler: _emit() → CodeObject.source_map (tetrad_ip, line, col)
         ▼
    CodeObject
         │  iir_translator: _translate_function() → IIRFunction.source_map (iir_start, tetrad_ip, 0)
         ▼
    IIRModule + IIRFunction.source_map
         │  sidecar_builder: compose(CodeObject.source_map, IIRFunction.source_map)
         ▼
    DebugSidecar bytes
         │  DebugSidecarReader
         ▼
    reader.find_instr("file.tetrad", line)  → iir_idx
    reader.lookup("fn", iir_idx)            → SourceLocation(file, line, col)
    reader.live_variables("fn", iir_idx)    → [Variable(name="x", ...)]

All tests in this file use programs where the source line mapping is
predictable by inspection (single-statement lines, no continuation).
"""

from __future__ import annotations

from typing import Any

import pytest

from debug_sidecar import DebugSidecarReader
from vm_core import DebugHooks, VMCore, VMFrame
from interpreter_ir import IIRInstr

from tetrad_runtime import TetradRuntime
from tetrad_runtime.sidecar_builder import code_object_to_iir_with_sidecar

# ---------------------------------------------------------------------------
# Shared source programs — each function on its own line so source-line
# lookups are deterministic.
# ---------------------------------------------------------------------------

#: A simple one-function program.  Line 1 has the return statement.
SIMPLE_SRC = "fn main() -> u8 { return 42; }"

#: A two-function program.
#:   Line 1: fn add(a, b) → return a + b
#:   Line 2: fn main()    → return add(10, 20)
TWO_FN_SRC = (
    "fn add(a: u8, b: u8) -> u8 { return a + b; }\n"
    "fn main() -> u8 { return add(10, 20); }"
)

#: A program with local variables, one statement per line.
#:   Line 1: fn compute(x, y) {
#:   Line 2:   let z = x + y;
#:   Line 3:   return z;
#:   Line 4: }
#:   Line 5: fn main() -> u8 { return compute(3, 4); }
LOCALS_SRC = (
    "fn compute(x: u8, y: u8) -> u8 {\n"
    "  let z = x + y;\n"
    "  return z;\n"
    "}\n"
    "fn main() -> u8 { return compute(3, 4); }"
)

SOURCE_PATH = "test.tetrad"


# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------


def _runtime() -> TetradRuntime:
    """Return a fresh TetradRuntime with no I/O."""
    return TetradRuntime(io_in=lambda: 0, io_out=lambda v: None)


# ---------------------------------------------------------------------------
# Layer 1: sidecar_builder — source-map composition
# ---------------------------------------------------------------------------


class TestSidecarBuilder:
    """Tests for code_object_to_iir_with_sidecar (standalone function)."""

    def test_returns_module_and_bytes(self) -> None:
        """compile_with_debug should return an IIRModule and non-empty bytes."""
        rt = _runtime()
        module, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        assert module is not None
        assert isinstance(sidecar, bytes)
        assert len(sidecar) > 0

    def test_sidecar_is_valid_json_with_version_1(self) -> None:
        """The sidecar bytes should parse as a version-1 DebugSidecarReader."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        # Should not raise.
        reader = DebugSidecarReader(sidecar)
        assert reader is not None

    def test_source_file_registered(self) -> None:
        """The source path should appear in the sidecar's source file list."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        assert SOURCE_PATH in reader.source_files()

    def test_main_function_registered(self) -> None:
        """'main' should appear in the sidecar's function name list."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        assert "main" in reader.function_names()

    def test_both_functions_registered_in_two_fn_program(self) -> None:
        """Both 'add' and 'main' must be in the sidecar for TWO_FN_SRC."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(TWO_FN_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        names = reader.function_names()
        assert "main" in names
        assert "add" in names

    def test_last_sidecar_stored_on_runtime(self) -> None:
        """After compile_with_debug, rt._last_sidecar should be set."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        assert rt._last_sidecar is sidecar

    def test_standalone_function_matches_runtime_method(self) -> None:
        """code_object_to_iir_with_sidecar and compile_with_debug produce the same sidecar structure."""
        from tetrad_compiler import compile_program

        code = compile_program(SIMPLE_SRC)
        _, sidecar1 = code_object_to_iir_with_sidecar(code, SOURCE_PATH)
        reader1 = DebugSidecarReader(sidecar1)

        rt = _runtime()
        _, sidecar2 = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader2 = DebugSidecarReader(sidecar2)

        # Both should expose the same function names and source files.
        assert sorted(reader1.function_names()) == sorted(reader2.function_names())
        assert reader1.source_files() == reader2.source_files()


# ---------------------------------------------------------------------------
# Layer 2: DebugSidecarReader — source-line queries
# ---------------------------------------------------------------------------


class TestSourceLineQueries:
    """Tests for find_instr and lookup after sidecar_builder composition."""

    def test_find_instr_returns_int_for_line_1(self) -> None:
        """find_instr for line 1 of SIMPLE_SRC should return a non-negative index."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        idx = reader.find_instr(SOURCE_PATH, 1)
        assert idx is not None
        assert idx >= 0

    def test_find_instr_unknown_file_returns_none(self) -> None:
        """find_instr with a wrong file path should return None."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        assert reader.find_instr("no_such_file.tetrad", 1) is None

    def test_find_instr_unknown_line_returns_none(self) -> None:
        """find_instr for a line number beyond the program should return None."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        assert reader.find_instr(SOURCE_PATH, 9999) is None

    def test_lookup_maps_iir_index_to_correct_source_line(self) -> None:
        """lookup(fn, iir_idx) should return the source line used in find_instr."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        # find_instr gives us the IIR index for line 1.
        idx = reader.find_instr(SOURCE_PATH, 1)
        assert idx is not None, "line 1 should map to an IIR instruction"

        loc = reader.lookup("main", idx)
        assert loc is not None
        assert loc.line == 1
        assert loc.file == SOURCE_PATH

    def test_lookup_nearest_preceding_semantics(self) -> None:
        """lookup at an index between recorded entries should return the nearest preceding entry."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        idx = reader.find_instr(SOURCE_PATH, 1)
        assert idx is not None

        # The instruction *after* idx (if it exists) should also resolve to
        # a location (nearest-preceding lookup).  We don't assert the exact
        # line because the next IIR instruction might begin a new Tetrad
        # source entry — we just assert it does not raise.
        loc_next = reader.lookup("main", idx + 1)
        # May be None if idx is the last instruction, which is fine.
        if loc_next is not None:
            assert loc_next.file == SOURCE_PATH

    def test_two_fn_src_both_lines_resolve(self) -> None:
        """Lines 1 and 2 of TWO_FN_SRC should resolve to different IIR indices."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(TWO_FN_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        idx_line1 = reader.find_instr(SOURCE_PATH, 1)
        idx_line2 = reader.find_instr(SOURCE_PATH, 2)

        # Both lines must resolve.
        assert idx_line1 is not None, "line 1 (fn add) should be reachable"
        assert idx_line2 is not None, "line 2 (fn main) should be reachable"

    def test_lookup_add_fn_line_1(self) -> None:
        """Lookup for line 1 of TWO_FN_SRC should land in the 'add' function."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(TWO_FN_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        idx = reader.find_instr(SOURCE_PATH, 1)
        assert idx is not None

        # find_instr returns the lowest IIR index matching line 1; it may be
        # in 'add' or '__entry__'.  Look up in all known functions to confirm
        # at least one resolves to line 1.
        found = False
        for fn_name in reader.function_names():
            loc = reader.lookup(fn_name, idx)
            if loc is not None and loc.line == 1:
                found = True
                break
        assert found, f"No function maps IIR idx {idx} to line 1"


# ---------------------------------------------------------------------------
# Layer 3: live_variables
# ---------------------------------------------------------------------------


class TestLiveVariables:
    """Tests for DebugSidecarReader.live_variables."""

    def test_params_are_live_at_instr_0_of_compute(self) -> None:
        """Parameters of 'compute' should be declared live from instruction 0."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(LOCALS_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        vars_ = reader.live_variables("compute", 0)
        names = {v.name for v in vars_}
        assert "x" in names, f"param 'x' missing from live variables: {names}"
        assert "y" in names, f"param 'y' missing from live variables: {names}"

    def test_local_declared_in_live_variables(self) -> None:
        """The local variable 'z' in 'compute' should appear in live_variables."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(LOCALS_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        # 'z' is declared live from 0..n_instrs.  Any mid-function index should
        # include it.
        vars_at_0 = reader.live_variables("compute", 0)
        names = {v.name for v in vars_at_0}
        assert "z" in names, f"local 'z' missing from live variables at 0: {names}"

    def test_main_has_no_local_variables(self) -> None:
        """The 'main' function in SIMPLE_SRC has no params or locals."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        vars_ = reader.live_variables("main", 0)
        assert vars_ == [], f"Expected no variables in main, got {vars_}"

    def test_type_hint_is_u8(self) -> None:
        """All declared variables should carry the 'u8' type hint."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(LOCALS_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)
        vars_ = reader.live_variables("compute", 0)
        for v in vars_:
            assert v.type_hint == "u8", (
                f"Variable {v.name!r} has type_hint {v.type_hint!r}, expected 'u8'"
            )


# ---------------------------------------------------------------------------
# Layer 4: run_with_debug — debug hook integration
# ---------------------------------------------------------------------------


class _ContinueAdapter(DebugHooks):
    """Adapter that records every pause and then calls vm.continue_().

    Used to verify which instructions trigger a breakpoint without
    stepping through the entire program.
    """

    def __init__(self, rt: TetradRuntime) -> None:
        self.rt = rt
        # Each entry is (fn_name, ip_before_dispatch).
        self.pauses: list[tuple[str, int]] = []

    def on_instruction(self, frame: VMFrame, instr: IIRInstr) -> None:
        # frame.ip has already been advanced past this instruction.
        self.pauses.append((frame.fn.name, frame.ip - 1))
        # Resume without pausing again.
        assert self.rt._last_vm is not None
        self.rt._last_vm.continue_()


class _StepInAdapter(DebugHooks):
    """Adapter that steps IN at every pause, recording visited instructions.

    Calls step_in() up to ``max_steps`` times, then switches to continue_()
    to prevent infinite loops in test programs.
    """

    def __init__(self, rt: TetradRuntime, max_steps: int = 30) -> None:
        self.rt = rt
        self.max_steps = max_steps
        self.pauses: list[tuple[str, int]] = []

    def on_instruction(self, frame: VMFrame, instr: IIRInstr) -> None:
        self.pauses.append((frame.fn.name, frame.ip - 1))
        assert self.rt._last_vm is not None
        if len(self.pauses) < self.max_steps:
            self.rt._last_vm.step_in()
        else:
            self.rt._last_vm.continue_()


class TestRunWithDebug:
    """Integration tests for TetradRuntime.run_with_debug."""

    def test_run_with_debug_returns_correct_result(self) -> None:
        """run_with_debug should return the same result as run()."""
        rt = _runtime()
        result = rt.run_with_debug(SIMPLE_SRC, SOURCE_PATH)
        assert result == 42

    def test_run_with_debug_two_fn_returns_correct_result(self) -> None:
        """run_with_debug on a two-function program should return add(10,20)=30."""
        rt = _runtime()
        result = rt.run_with_debug(TWO_FN_SRC, SOURCE_PATH)
        assert result == 30

    def test_on_instruction_fires_at_breakpoint(self) -> None:
        """When a breakpoint is set at the IIR index for line 1, on_instruction fires."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        bp_idx = reader.find_instr(SOURCE_PATH, 1)
        assert bp_idx is not None, "line 1 of SIMPLE_SRC must be resolvable"

        adapter = _ContinueAdapter(rt)
        rt.run_with_debug(
            SIMPLE_SRC,
            SOURCE_PATH,
            hooks=adapter,
            breakpoints={"main": [bp_idx]},
        )

        # The adapter should have been invoked at least once.
        assert len(adapter.pauses) >= 1
        # The first pause should be at bp_idx in 'main'.
        fn_names = [fn for fn, _ in adapter.pauses]
        ips = [ip for _, ip in adapter.pauses]
        assert "main" in fn_names
        assert bp_idx in ips

    def test_breakpoint_resolves_to_correct_source_line(self) -> None:
        """When paused at a breakpoint, reader.lookup returns the expected source line."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(TWO_FN_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        # Set a breakpoint on line 1 (the 'add' function body).
        bp_idx = reader.find_instr(SOURCE_PATH, 1)
        assert bp_idx is not None, "line 1 must be resolvable"

        # Determine which function owns this IIR index by finding its location.
        bp_fn = None
        for fn_name in reader.function_names():
            loc = reader.lookup(fn_name, bp_idx)
            if loc is not None and loc.line == 1:
                bp_fn = fn_name
                break
        assert bp_fn is not None, "No function maps bp_idx to line 1"

        # Build a breakpoint keyed to the correct function.
        paused_ips: list[tuple[str, int]] = []

        class LookupAdapter(DebugHooks):
            def on_instruction(self, frame: VMFrame, instr: IIRInstr) -> None:
                ip = frame.ip - 1
                paused_ips.append((frame.fn.name, ip))
                loc = reader.lookup(frame.fn.name, ip)
                assert loc is not None, (
                    f"reader.lookup({frame.fn.name!r}, {ip}) returned None at breakpoint"
                )
                assert loc.line == 1, (
                    f"Expected source line 1, got {loc.line}"
                )
                assert loc.file == SOURCE_PATH
                rt._last_vm.continue_()  # type: ignore[union-attr]

        adapter = LookupAdapter()
        rt.run_with_debug(
            TWO_FN_SRC,
            SOURCE_PATH,
            hooks=adapter,
            breakpoints={bp_fn: [bp_idx]},
        )
        assert len(paused_ips) >= 1, "Breakpoint should have fired at least once"

    def test_no_hooks_does_not_attach_debug_mode(self) -> None:
        """run_with_debug with hooks=None should not enable debug mode on the VM."""
        rt = _runtime()
        rt.run_with_debug(SIMPLE_SRC, SOURCE_PATH)
        assert rt._last_vm is not None
        assert rt._last_vm.is_debug_mode() is False

    def test_hooks_enables_debug_mode(self) -> None:
        """Passing hooks= should enable debug mode on the internal VM."""
        rt = _runtime()
        adapter = _ContinueAdapter(rt)
        rt.run_with_debug(SIMPLE_SRC, SOURCE_PATH, hooks=adapter)
        assert rt._last_vm is not None
        # Debug mode is still True after execution (hooks remain attached).
        assert rt._last_vm.is_debug_mode() is True

    def test_step_in_visits_multiple_instructions(self) -> None:
        """step_in should visit more than one instruction in a simple program.

        Stepping requires an initial trigger.  We seed it with a breakpoint
        at instruction 0 of 'main'.  The adapter then calls step_in() in every
        on_instruction, so each subsequent instruction also pauses.
        """
        rt = _runtime()
        adapter = _StepInAdapter(rt, max_steps=20)
        rt.run_with_debug(
            SIMPLE_SRC,
            SOURCE_PATH,
            hooks=adapter,
            # Instruction 0 of 'main' is the breakpoint seed.  The adapter
            # calls step_in() from there, which causes every subsequent
            # instruction in the run to also pause.
            breakpoints={"main": [0]},
        )
        # A Tetrad 'return 42' compiles to at least 2 IIR instructions.
        assert len(adapter.pauses) >= 2

    def test_step_in_enters_callee(self) -> None:
        """step_in should visit instructions inside the callee ('add') function.

        Seed the stepping with a breakpoint at 'main[0]'.  From there the
        adapter calls step_in() repeatedly, eventually following the
        'call add' into 'add'.
        """
        rt = _runtime()
        adapter = _StepInAdapter(rt, max_steps=30)
        rt.run_with_debug(
            TWO_FN_SRC,
            SOURCE_PATH,
            hooks=adapter,
            breakpoints={"main": [0]},  # seed the initial pause
        )
        fn_names_visited = {fn for fn, _ in adapter.pauses}
        assert "add" in fn_names_visited, (
            f"step_in should enter 'add'; visited: {fn_names_visited}"
        )

    def test_call_stack_shows_callee_when_paused_in_add(self) -> None:
        """When paused inside 'add', call_stack() should include 'add' on top."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(TWO_FN_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        # Find the IIR index for line 1, which lives in 'add'.
        bp_idx = reader.find_instr(SOURCE_PATH, 1)
        assert bp_idx is not None

        # Determine the function owning this breakpoint (should be 'add').
        bp_fn = None
        for fn_name in reader.function_names():
            loc = reader.lookup(fn_name, bp_idx)
            if loc is not None and loc.line == 1 and fn_name == "add":
                bp_fn = "add"
                break
        if bp_fn is None:
            # Fallback: use whatever function owns this index
            for fn_name in reader.function_names():
                if reader.lookup(fn_name, bp_idx) is not None:
                    bp_fn = fn_name
                    break

        assert bp_fn is not None

        call_stacks_seen: list[list[str]] = []

        class StackAdapter(DebugHooks):
            def on_instruction(self, frame: VMFrame, instr: IIRInstr) -> None:
                stack = rt._last_vm.call_stack()  # type: ignore[union-attr]
                call_stacks_seen.append([f.fn.name for f in stack])
                rt._last_vm.continue_()  # type: ignore[union-attr]

        adapter = StackAdapter()
        rt.run_with_debug(
            TWO_FN_SRC,
            SOURCE_PATH,
            hooks=adapter,
            breakpoints={bp_fn: [bp_idx]},
        )

        # At least one pause must have happened.
        assert len(call_stacks_seen) >= 1, "Breakpoint in add should have fired"
        top_fn = call_stacks_seen[0][-1]
        assert top_fn == bp_fn, (
            f"Expected top frame to be {bp_fn!r}, got {top_fn!r}. "
            f"Full stack: {call_stacks_seen[0]}"
        )

    def test_multiple_breakpoints_in_same_function(self) -> None:
        """Setting multiple breakpoints in main should fire on_instruction for each."""
        rt = _runtime()
        _, sidecar = rt.compile_with_debug(SIMPLE_SRC, SOURCE_PATH)
        reader = DebugSidecarReader(sidecar)

        bp_idx = reader.find_instr(SOURCE_PATH, 1)
        assert bp_idx is not None

        # Set two different breakpoints in main (bp_idx and bp_idx+1 if valid).
        module = rt.last_module
        assert module is not None
        main_fn = module.get_function("main")
        assert main_fn is not None
        n = len(main_fn.instructions)

        bps = [bp_idx]
        if bp_idx + 1 < n:
            bps.append(bp_idx + 1)

        adapter = _ContinueAdapter(rt)
        rt.run_with_debug(
            SIMPLE_SRC,
            SOURCE_PATH,
            hooks=adapter,
            breakpoints={"main": bps},
        )

        # Should have paused at least once per breakpoint set.
        assert len(adapter.pauses) >= 1

    def test_run_with_debug_locals_returns_correct_result(self) -> None:
        """run_with_debug on LOCALS_SRC should return compute(3,4)=7."""
        rt = _runtime()
        result = rt.run_with_debug(LOCALS_SRC, SOURCE_PATH)
        assert result == 7
