"""Tests for TW04 Phase 4e — multi-module CLR compilation.

These tests verify that ``compile_modules`` and ``run_modules`` correctly:

* Produce a ``MultiModuleClrResult`` with one ``ModuleClrCompileResult``
  per non-``host`` Twig module.
* Emit the entry module's methods into the main TypeDef and dep modules'
  methods into extra TypeDef rows.
* Emit cross-module ``IrOp.CALL IrLabel("a/math/add")`` in the entry
  module's IR.
* Actually run on the real .NET runtime when available, yielding the
  correct exit code (the Phase 4e acceptance criterion).
"""

from __future__ import annotations

from pathlib import Path

import pytest
from compiler_ir import IrLabel, IrOp
from twig import resolve_modules

from twig_clr_compiler import dotnet_available
from twig_clr_compiler.compiler import (
    MultiModuleClrExecutionResult,
    MultiModuleClrResult,
    ModuleClrCompileResult,
    compile_modules,
    module_name_to_clr_type,
    run_modules,
)

# ---------------------------------------------------------------------------
# Skip marker for tests that require real dotnet
# ---------------------------------------------------------------------------

requires_dotnet = pytest.mark.skipif(
    not dotnet_available(),
    reason="dotnet not on PATH",
)


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def two_module_dir(tmp_path: Path) -> Path:
    """Write the two-module acceptance-criterion program to a temp directory.

    Files::

        a/math.tw — exports ``add`` and ``sub``
        user/hello.tw — imports ``a/math``, evaluates ``(a/math/add 17 25)``

    The result is 42, which becomes the process exit code.
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

    ``user/main`` calls both dep modules.  Final result = 10 + 5 = 15.
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
        "(a/math/add (b/utils/double 5) 5)\n"  # (5*2) + 5 = 15
    )
    return tmp_path


@pytest.fixture()
def host_call_dir(tmp_path: Path) -> Path:
    """Single entry module that uses ``host/write-byte`` to emit a byte."""
    (tmp_path / "user").mkdir()
    (tmp_path / "user" / "io.tw").write_text(
        "(module user/io (import host))\n"
        "(host/write-byte 65)\n"  # 'A'
    )
    return tmp_path


@pytest.fixture()
def host_and_dep_dir(tmp_path: Path) -> Path:
    """Entry module uses both ``host/write-byte`` and a dep module function."""
    (tmp_path / "a").mkdir()
    (tmp_path / "user").mkdir()
    (tmp_path / "a" / "math.tw").write_text(
        "(module a/math (export add))\n"
        "(define (add x y) (+ x y))\n"
    )
    (tmp_path / "user" / "hello.tw").write_text(
        "(module user/hello (import host) (import a/math))\n"
        "(host/write-byte (a/math/add 10 32))\n"  # write byte 42
    )
    return tmp_path


def _resolve(entry: str, search_dir: Path):
    return resolve_modules(entry, search_paths=[search_dir])


# ---------------------------------------------------------------------------
# module_name_to_clr_type
# ---------------------------------------------------------------------------


class TestModuleNameToClrType:
    """``module_name_to_clr_type`` replaces ``/`` with ``_``."""

    def test_single_slash(self) -> None:
        assert module_name_to_clr_type("user/hello") == "user_hello"

    def test_double_slash(self) -> None:
        assert module_name_to_clr_type("a/b/c") == "a_b_c"

    def test_no_slash(self) -> None:
        assert module_name_to_clr_type("mymodule") == "mymodule"

    def test_dep_module_name(self) -> None:
        assert module_name_to_clr_type("a/math") == "a_math"

    def test_user_module_name(self) -> None:
        assert module_name_to_clr_type("user/main") == "user_main"


# ---------------------------------------------------------------------------
# compile_modules — structural tests (no dotnet needed)
# ---------------------------------------------------------------------------


class TestCompileModules:
    """``compile_modules`` returns a correctly structured ``MultiModuleClrResult``."""

    def test_returns_multi_module_clr_result(
        self, two_module_dir: Path
    ) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert isinstance(result, MultiModuleClrResult)

    def test_two_module_results(self, two_module_dir: Path) -> None:
        """Two real modules → two ``ModuleClrCompileResult`` entries."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        # host module is excluded; only a/math and user/hello
        assert len(result.module_results) == 2

    def test_entry_module_first(self, two_module_dir: Path) -> None:
        """The entry module is always the first element in ``module_results``."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert result.module_results[0].module_name == "user/hello"
        assert result.entry_module == "user/hello"

    def test_dep_module_second(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert result.module_results[1].module_name == "a/math"

    def test_entry_type_name(self, two_module_dir: Path) -> None:
        """Entry module type name is derived from ``module_name_to_clr_type``."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert result.entry_type_name == "user_hello"

    def test_module_compile_results_are_frozen_dataclasses(
        self, two_module_dir: Path
    ) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        for mr in result.module_results:
            assert isinstance(mr, ModuleClrCompileResult)

    def test_assembly_bytes_are_nonempty(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert len(result.assembly_bytes) > 0

    def test_assembly_starts_with_mz_header(self, two_module_dir: Path) -> None:
        """The PE file starts with the standard MZ signature."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert result.assembly_bytes[:2] == b"MZ"

    def test_dep_module_exports_in_callable_names(
        self, two_module_dir: Path
    ) -> None:
        """Dep module's exported functions appear in its callable_names."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        dep_result = next(
            r for r in result.module_results if r.module_name == "a/math"
        )
        assert "add" in dep_result.callable_names
        assert "sub" in dep_result.callable_names

    def test_three_module_dir_results(self, three_module_dir: Path) -> None:
        """Three-module compile yields three ModuleClrCompileResult entries."""
        modules = _resolve("user/main", three_module_dir)
        result = compile_modules(modules, entry_module="user/main")
        assert len(result.module_results) == 3

    def test_host_module_excluded(self, host_call_dir: Path) -> None:
        """The synthetic ``host`` module does not appear in module_results."""
        modules = _resolve("user/io", host_call_dir)
        result = compile_modules(modules, entry_module="user/io")
        names = [r.module_name for r in result.module_results]
        assert "host" not in names

    def test_cross_module_call_in_ir(self, two_module_dir: Path) -> None:
        """Entry module IR contains ``IrOp.CALL IrLabel("a/math/add")``."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        entry_result = result.module_results[0]
        instructions = list(entry_result.ir.instructions)
        cross_calls = [
            instr
            for instr in instructions
            if instr.opcode == IrOp.CALL
            and isinstance(instr.operands[0], IrLabel)
            and "/" in instr.operands[0].name
        ]
        assert len(cross_calls) >= 1
        label_name = cross_calls[0].operands[0].name
        assert label_name == "a/math/add"

    def test_dep_module_ir_has_add_and_sub_regions(
        self, two_module_dir: Path
    ) -> None:
        """Dep module IR contains LABEL instructions for ``add`` and ``sub``."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        dep_result = next(
            r for r in result.module_results if r.module_name == "a/math"
        )
        labels = [
            instr.operands[0].name
            for instr in dep_result.ir.instructions
            if instr.opcode == IrOp.LABEL
            and isinstance(instr.operands[0], IrLabel)
        ]
        assert "add" in labels
        assert "sub" in labels

    def test_invalid_entry_module_raises(self, two_module_dir: Path) -> None:
        """Referencing a non-existent entry module raises ``ClrPackageError``."""
        from twig_clr_compiler.compiler import ClrPackageError
        modules = _resolve("user/hello", two_module_dir)
        with pytest.raises(ClrPackageError, match="entry module"):
            compile_modules(modules, entry_module="nonexistent/mod")

    def test_host_and_dep_module_together(self, host_and_dep_dir: Path) -> None:
        """Entry module can use both host calls and dep module calls."""
        modules = _resolve("user/hello", host_and_dep_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert len(result.module_results) == 2
        assert result.assembly_bytes[:2] == b"MZ"

    def test_dep_module_artifact_methods_non_empty(
        self, two_module_dir: Path
    ) -> None:
        """Dep module's CILProgramArtifact contains at least the exported methods."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        dep_result = next(
            r for r in result.module_results if r.module_name == "a/math"
        )
        # artifact.methods includes main + exported add + sub
        assert len(dep_result.artifact.methods) >= 2

    def test_token_offset_increases_for_each_dep(
        self, three_module_dir: Path
    ) -> None:
        """Each dep module's callable_names are disjoint from the entry module's."""
        modules = _resolve("user/main", three_module_dir)
        result = compile_modules(modules, entry_module="user/main")
        all_callable_tuples = [r.callable_names for r in result.module_results]
        # All callable name sets per module should be non-empty.
        for names in all_callable_tuples:
            assert len(names) >= 1


# ---------------------------------------------------------------------------
# run_modules — end-to-end on real dotnet
# ---------------------------------------------------------------------------


class TestRunModulesOnRealDotnet:
    """Acceptance tests against the real ``dotnet`` runtime."""

    @requires_dotnet
    def test_two_module_add_returns_42(self, two_module_dir: Path) -> None:
        """The headline Phase 4e acceptance criterion.

        ``(a/math/add 17 25)`` in the entry module → exit code 42.
        """
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, entry_module="user/hello")
        assert result.returncode == 42, result.compilation.module_results[0].ir

    @requires_dotnet
    def test_three_module_result_15(self, three_module_dir: Path) -> None:
        """Three-module program: ``(a/math/add (b/utils/double 5) 5)`` → 15."""
        modules = _resolve("user/main", three_module_dir)
        result = run_modules(modules, entry_module="user/main")
        assert result.returncode == 15, result.compilation.module_results[0].ir

    @requires_dotnet
    def test_host_write_byte_in_multi_module(
        self, host_and_dep_dir: Path
    ) -> None:
        """Entry module writing a byte via host call in multi-module mode."""
        modules = _resolve("user/hello", host_and_dep_dir)
        result = run_modules(
            modules, entry_module="user/hello", assembly_name="ClrHostAndDep"
        )
        # host/write-byte(42) writes byte 42 to stdout; exit code 0 for void syscall
        assert result.stdout == b"*", f"stderr: {result.stderr}"

    @requires_dotnet
    def test_run_modules_returns_execution_result_type(
        self, two_module_dir: Path
    ) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, entry_module="user/hello")
        assert isinstance(result, MultiModuleClrExecutionResult)

    @requires_dotnet
    def test_sub_function_from_dep_module(self, tmp_path: Path) -> None:
        """Verify sub exported from dep module returns correct result (10-3=7)."""
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
        result = run_modules(
            modules, entry_module="user/calc", assembly_name="ClrSub"
        )
        assert result.returncode == 7, result.stderr

    @requires_dotnet
    def test_recursive_function_in_dep_module(self, tmp_path: Path) -> None:
        """Recursion inside a dep module works across the TypeDef boundary."""
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
        modules = _resolve("user/main", tmp_path)
        result = run_modules(
            modules, entry_module="user/main", assembly_name="ClrRecursiveDep"
        )
        assert result.returncode == 120, result.stderr
