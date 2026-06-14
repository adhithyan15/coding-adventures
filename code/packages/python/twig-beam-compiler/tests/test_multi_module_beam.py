"""Tests for TW04 Phase 4f — multi-module BEAM compilation.

These tests verify that ``compile_modules`` and ``run_modules`` correctly:

* Produce a ``MultiModuleBeamResult`` with one ``ModuleBeamCompileResult``
  per non-``host`` Twig module.
* Map module names to valid Erlang atom names (``"a/math"`` → ``"a_math"``).
* Emit cross-module ``IrOp.CALL IrLabel("a/math/add")`` in the entry
  module's IR so the BEAM backend lowers it to a remote ``call_ext``.
* Actually run on the real ``erl`` runtime when available, yielding the
  correct exit code (the Phase 4f acceptance criterion).

Structural tests (the majority) require no BEAM runtime — they only inspect
the compiled artefacts.  Runtime tests are gated behind ``@requires_beam``
which skips them when ``erl`` is not on PATH.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from compiler_ir import IrLabel, IrOp
from twig import resolve_modules

from twig_beam_compiler import erl_available
from twig_beam_compiler.compiler import (
    ModuleBeamCompileResult,
    MultiModuleBeamExecutionResult,
    MultiModuleBeamResult,
    compile_modules,
    module_name_to_beam_module,
    run_modules,
)

# ---------------------------------------------------------------------------
# Skip marker for tests that require real erl
# ---------------------------------------------------------------------------

requires_beam = pytest.mark.skipif(
    not erl_available(),
    reason="erl not on PATH",
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _resolve(entry: str, search_dir: Path) -> list:
    """Resolve entry module and its transitive imports from search_dir."""
    return resolve_modules(entry, search_paths=[search_dir])


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def two_module_dir(tmp_path: Path) -> Path:
    """Two-module acceptance-criterion program.

    Files::

        a/math.tw   — exports ``add`` and ``sub``
        user/hello.tw — imports ``a/math``, evaluates ``(a/math/add 17 25)``

    The result is 42, used as the ``erlang:halt(42)`` exit code.
    """
    (tmp_path / "a").mkdir()
    (tmp_path / "user").mkdir()
    (tmp_path / "a" / "math.tw").write_text(
        "(module a/math (export add sub))\n"
        "(define (add x y) (+ x y))\n"
        "(define (sub x y) (- x y))\n"
    )
    (tmp_path / "user" / "hello.tw").write_text(
        "(module user/hello (import a/math))\n"
        "(a/math/add 17 25)\n"
    )
    return tmp_path


@pytest.fixture()
def three_module_dir(tmp_path: Path) -> Path:
    """Three-module program: ``a/math``, ``b/utils``, ``user/main``.

    ``user/main`` calls both dep modules.  Final result = (5*2) + 5 = 15.
    """
    (tmp_path / "a").mkdir()
    (tmp_path / "b").mkdir()
    (tmp_path / "user").mkdir()
    (tmp_path / "a" / "math.tw").write_text(
        "(module a/math (export add))\n"
        "(define (add x y) (+ x y))\n"
    )
    (tmp_path / "b" / "utils.tw").write_text(
        "(module b/utils (export double))\n"
        "(define (double x) (* x 2))\n"
    )
    (tmp_path / "user" / "main.tw").write_text(
        "(module user/main (import a/math) (import b/utils))\n"
        "(a/math/add (b/utils/double 5) 5)\n"
    )
    return tmp_path


@pytest.fixture()
def single_dep_dir(tmp_path: Path) -> Path:
    """Simple dep module with one function and an entry calling it."""
    (tmp_path / "math").mkdir()
    (tmp_path / "entry").mkdir()
    (tmp_path / "math" / "ops.tw").write_text(
        "(module math/ops (export square))\n"
        "(define (square x) (* x x))\n"
    )
    (tmp_path / "entry" / "main.tw").write_text(
        "(module entry/main (import math/ops))\n"
        "(math/ops/square 7)\n"  # 49
    )
    return tmp_path


@pytest.fixture()
def recursive_dep_dir(tmp_path: Path) -> Path:
    """Dep module with a recursive function (factorial), called by entry."""
    (tmp_path / "a").mkdir()
    (tmp_path / "user").mkdir()
    (tmp_path / "a" / "math.tw").write_text(
        "(module a/math (export fact))\n"
        "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n"
    )
    (tmp_path / "user" / "main.tw").write_text(
        "(module user/main (import a/math))\n"
        "(a/math/fact 5)\n"  # 120
    )
    return tmp_path


@pytest.fixture()
def multi_call_dir(tmp_path: Path) -> Path:
    """Entry module calls the dep module function multiple times."""
    (tmp_path / "a").mkdir()
    (tmp_path / "user").mkdir()
    (tmp_path / "a" / "math.tw").write_text(
        "(module a/math (export add))\n"
        "(define (add x y) (+ x y))\n"
    )
    (tmp_path / "user" / "main.tw").write_text(
        "(module user/main (import a/math))\n"
        # (3+4) + (5+6) = 7 + 11 = 18
        "(a/math/add (a/math/add 3 4) (a/math/add 5 6))\n"
    )
    return tmp_path


# ---------------------------------------------------------------------------
# module_name_to_beam_module
# ---------------------------------------------------------------------------


class TestModuleNameToBeamModule:
    """``module_name_to_beam_module`` replaces ``/`` with ``_``."""

    def test_single_slash(self) -> None:
        assert module_name_to_beam_module("user/hello") == "user_hello"

    def test_a_math(self) -> None:
        assert module_name_to_beam_module("a/math") == "a_math"

    def test_double_slash(self) -> None:
        assert module_name_to_beam_module("a/b/c") == "a_b_c"

    def test_no_slash(self) -> None:
        assert module_name_to_beam_module("mymodule") == "mymodule"

    def test_stdlib_io(self) -> None:
        assert module_name_to_beam_module("stdlib/io") == "stdlib_io"

    def test_user_main(self) -> None:
        assert module_name_to_beam_module("user/main") == "user_main"

    def test_b_utils(self) -> None:
        assert module_name_to_beam_module("b/utils") == "b_utils"


# ---------------------------------------------------------------------------
# compile_modules — structural tests (no erl needed)
# ---------------------------------------------------------------------------


class TestCompileModulesStructure:
    """``compile_modules`` returns a correctly structured result."""

    def test_returns_multi_module_beam_result(
        self, two_module_dir: Path
    ) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        assert isinstance(result, MultiModuleBeamResult)

    def test_two_module_results(self, two_module_dir: Path) -> None:
        """Two real modules → two ``ModuleBeamCompileResult`` entries."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        # host is excluded; only a/math and user/hello
        assert len(result.module_results) == 2

    def test_entry_module_field(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        assert result.entry_module == "user/hello"

    def test_entry_beam_module_field(self, two_module_dir: Path) -> None:
        """``entry_beam_module`` is the Erlang atom for the entry module."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        assert result.entry_beam_module == "user_hello"

    def test_host_module_excluded(self, two_module_dir: Path) -> None:
        """The synthetic ``host`` module does not appear in module_results."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        names = [r.module_name for r in result.module_results]
        assert "host" not in names

    def test_module_results_are_frozen_dataclasses(
        self, two_module_dir: Path
    ) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        for mr in result.module_results:
            assert isinstance(mr, ModuleBeamCompileResult)

    def test_beam_bytes_are_nonempty(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        for mr in result.module_results:
            assert len(mr.beam_bytes) > 0

    def test_beam_files_start_with_for1_header(
        self, two_module_dir: Path
    ) -> None:
        """BEAM files always start with the ``FOR1`` chunk header."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        for mr in result.module_results:
            assert mr.beam_bytes[:4] == b"FOR1", (
                f"Module {mr.module_name!r} .beam doesn't start with FOR1"
            )

    def test_module_names_present(self, two_module_dir: Path) -> None:
        """Both ``a/math`` and ``user/hello`` appear in module_results."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        names = {r.module_name for r in result.module_results}
        assert "a/math" in names
        assert "user/hello" in names

    def test_beam_module_atoms(self, two_module_dir: Path) -> None:
        """``beam_module`` fields are correctly derived from module names."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        atoms = {r.beam_module for r in result.module_results}
        assert "a_math" in atoms
        assert "user_hello" in atoms

    def test_dep_module_exports_in_result(
        self, two_module_dir: Path
    ) -> None:
        """Dep module's ``exports`` tuple includes the declared exports."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        dep = next(r for r in result.module_results if r.module_name == "a/math")
        assert "add" in dep.exports
        assert "sub" in dep.exports

    def test_cross_module_call_in_entry_ir(
        self, two_module_dir: Path
    ) -> None:
        """Entry module IR contains ``IrOp.CALL IrLabel("a/math/add")``."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        entry_result = next(
            r for r in result.module_results if r.module_name == "user/hello"
        )
        cross_calls = [
            instr
            for instr in entry_result.ir.instructions
            if instr.opcode == IrOp.CALL
            and isinstance(instr.operands[0], IrLabel)
            and "/" in instr.operands[0].name
        ]
        assert len(cross_calls) >= 1
        assert cross_calls[0].operands[0].name == "a/math/add"

    def test_dep_module_ir_has_add_region(
        self, two_module_dir: Path
    ) -> None:
        """Dep module IR contains LABEL instructions for ``add`` and ``sub``."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        dep = next(r for r in result.module_results if r.module_name == "a/math")
        labels = [
            instr.operands[0].name
            for instr in dep.ir.instructions
            if instr.opcode == IrOp.LABEL
            and isinstance(instr.operands[0], IrLabel)
        ]
        assert "add" in labels
        assert "sub" in labels

    def test_three_module_results(self, three_module_dir: Path) -> None:
        """Three-module compile yields three ModuleBeamCompileResult entries."""
        modules = _resolve("user/main", three_module_dir)
        result = compile_modules(modules, "user/main")
        assert len(result.module_results) == 3

    def test_invalid_entry_module_raises(self, two_module_dir: Path) -> None:
        """Referencing a non-existent entry module raises ``ValueError``."""
        modules = _resolve("user/hello", two_module_dir)
        with pytest.raises(ValueError, match="entry_module"):
            compile_modules(modules, "nonexistent/mod")

    def test_beam_module_atom_for_three_modules(
        self, three_module_dir: Path
    ) -> None:
        """All three modules have correct ``beam_module`` atom names."""
        modules = _resolve("user/main", three_module_dir)
        result = compile_modules(modules, "user/main")
        atoms = {r.beam_module for r in result.module_results}
        assert "a_math" in atoms
        assert "b_utils" in atoms
        assert "user_main" in atoms

    def test_entry_beam_module_for_three_modules(
        self, three_module_dir: Path
    ) -> None:
        modules = _resolve("user/main", three_module_dir)
        result = compile_modules(modules, "user/main")
        assert result.entry_beam_module == "user_main"

    def test_dep_beam_bytes_contain_beam_magic(
        self, single_dep_dir: Path
    ) -> None:
        """Each dep module's .beam file contains the BEAM chunk magic."""
        modules = _resolve("entry/main", single_dep_dir)
        result = compile_modules(modules, "entry/main")
        dep = next(
            r for r in result.module_results if r.module_name == "math/ops"
        )
        # BEAM chunk ID appears near the start of FOR1-wrapped BEAM files.
        assert b"BEAM" in dep.beam_bytes[:20]

    def test_multi_call_cross_module_ir(self, multi_call_dir: Path) -> None:
        """Entry module with multiple cross-module calls has all in IR."""
        modules = _resolve("user/main", multi_call_dir)
        result = compile_modules(modules, "user/main")
        entry = next(
            r for r in result.module_results if r.module_name == "user/main"
        )
        cross_calls = [
            instr
            for instr in entry.ir.instructions
            if instr.opcode == IrOp.CALL
            and isinstance(instr.operands[0], IrLabel)
            and "/" in instr.operands[0].name
        ]
        # (a/math/add 3 4), (a/math/add 5 6), then outer (a/math/add ...)
        assert len(cross_calls) >= 2

    def test_result_is_immutable(self, two_module_dir: Path) -> None:
        """``MultiModuleBeamResult`` is a frozen dataclass."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        with pytest.raises((AttributeError, TypeError)):
            result.entry_module = "mutated"  # type: ignore[misc]

    def test_module_result_is_immutable(self, two_module_dir: Path) -> None:
        """``ModuleBeamCompileResult`` is a frozen dataclass."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        mr = result.module_results[0]
        with pytest.raises((AttributeError, TypeError)):
            mr.module_name = "mutated"  # type: ignore[misc]

    def test_module_results_is_tuple(self, two_module_dir: Path) -> None:
        """``module_results`` is a tuple, not a list."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        assert isinstance(result.module_results, tuple)

    def test_exports_is_tuple(self, two_module_dir: Path) -> None:
        """``ModuleBeamCompileResult.exports`` is a tuple."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, "user/hello")
        for mr in result.module_results:
            assert isinstance(mr.exports, tuple)

    def test_recursive_dep_compiles(self, recursive_dep_dir: Path) -> None:
        """A dep module with recursion compiles without error."""
        modules = _resolve("user/main", recursive_dep_dir)
        result = compile_modules(modules, "user/main")
        names = {r.module_name for r in result.module_results}
        assert "a/math" in names
        assert "user/main" in names


# ---------------------------------------------------------------------------
# run_modules — end-to-end on real erl
# ---------------------------------------------------------------------------


class TestRunModulesOnRealBeam:
    """Acceptance tests against the real ``erl`` runtime."""

    @requires_beam
    def test_two_module_add_returns_42(self, two_module_dir: Path) -> None:
        """The headline Phase 4f acceptance criterion.

        ``(a/math/add 17 25)`` in the entry module → exit code 42.
        """
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, "user/hello")
        assert result.returncode == 42, (
            f"Expected exit code 42, got {result.returncode}.\n"
            f"stderr: {result.stderr!r}"
        )

    @requires_beam
    def test_run_modules_returns_execution_result_type(
        self, two_module_dir: Path
    ) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, "user/hello")
        assert isinstance(result, MultiModuleBeamExecutionResult)

    @requires_beam
    def test_three_module_result_15(self, three_module_dir: Path) -> None:
        """Three-module program: ``(a/math/add (b/utils/double 5) 5)`` → 15."""
        modules = _resolve("user/main", three_module_dir)
        result = run_modules(modules, "user/main")
        assert result.returncode == 15, (
            f"Expected exit code 15, got {result.returncode}.\n"
            f"stderr: {result.stderr!r}"
        )

    @requires_beam
    def test_sub_from_dep_module(self, tmp_path: Path) -> None:
        """Dep module's ``sub`` function: 10 - 3 = 7."""
        (tmp_path / "a").mkdir()
        (tmp_path / "user").mkdir()
        (tmp_path / "a" / "math.tw").write_text(
            "(module a/math (export sub))\n"
            "(define (sub x y) (- x y))\n"
        )
        (tmp_path / "user" / "calc.tw").write_text(
            "(module user/calc (import a/math))\n"
            "(a/math/sub 10 3)\n"
        )
        modules = _resolve("user/calc", tmp_path)
        result = run_modules(modules, "user/calc")
        assert result.returncode == 7, result.stderr

    @requires_beam
    def test_recursive_fact_in_dep(self, recursive_dep_dir: Path) -> None:
        """Recursion inside a dep module: ``fact(5)`` = 120."""
        modules = _resolve("user/main", recursive_dep_dir)
        result = run_modules(modules, "user/main")
        assert result.returncode == 120, (
            f"Expected 120, got {result.returncode}.\n"
            f"stderr: {result.stderr!r}"
        )

    @requires_beam
    def test_multi_calls_same_dep(self, multi_call_dir: Path) -> None:
        """Entry calling dep function multiple times: (3+4)+(5+6) = 18."""
        modules = _resolve("user/main", multi_call_dir)
        result = run_modules(modules, "user/main")
        assert result.returncode == 18, (
            f"Expected 18, got {result.returncode}.\n"
            f"stderr: {result.stderr!r}"
        )

    @requires_beam
    def test_compilation_result_accessible(
        self, two_module_dir: Path
    ) -> None:
        """``compilation`` field on execution result is populated."""
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, "user/hello")
        assert isinstance(result.compilation, MultiModuleBeamResult)
        assert len(result.compilation.module_results) == 2
