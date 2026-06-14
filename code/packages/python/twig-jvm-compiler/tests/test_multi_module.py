"""Tests for TW04 Phase 4d — multi-module JVM compilation.

These tests verify that ``compile_modules`` and ``run_modules`` correctly:

* Produce a ``MultiModuleResult`` with a shared ``TwigRuntime`` artifact
  and one ``ModuleCompileResult`` per non-``host`` module.
* Emit the entry module's artifact with a ``main()`` wrapper and all
  dependency modules without one.
* Emit ``IrOp.CALL IrLabel("a/math/add")`` for cross-module calls in
  the entry module's IR.
* Actually run on the real JVM when available, yielding the correct
  exit code (the acceptance criterion from the Phase 4d spec).
"""

from __future__ import annotations

from pathlib import Path

import pytest
from compiler_ir import IrLabel, IrOp
from twig import resolve_modules

from twig_jvm_compiler import (
    ModuleCompileResult,
    MultiModuleResult,
    compile_modules,
    java_available,
    module_name_to_jvm_class,
    run_modules,
)
from twig_jvm_compiler.compiler import MultiModuleExecutionResult

# ---------------------------------------------------------------------------
# Fixtures / shared helpers
# ---------------------------------------------------------------------------


@pytest.fixture()
def two_module_dir(tmp_path: Path) -> Path:
    """Write the two-module acceptance-criterion program to a temp directory.

    Files::

        a/math.tw — defines ``add`` (exported)
        user/hello.tw — imports ``a/math``, calls ``(a/math/add 17 25)``

    The result of ``(a/math/add 17 25)`` is 42.  The entry module writes
    that as a byte to stdout and exits normally.
    """
    (tmp_path / "a").mkdir()
    (tmp_path / "user").mkdir()

    (tmp_path / "a" / "math.tw").write_text("""
        (module a/math (export add sub))
        (define (add x y) (+ x y))
        (define (sub x y) (- x y))
    """)
    (tmp_path / "user" / "hello.tw").write_text("""
        (module user/hello (import a/math))
        (a/math/add 17 25)
    """)
    return tmp_path


def _resolve(entry: str, search_dir: Path):
    return resolve_modules(entry, search_paths=[search_dir])


# ---------------------------------------------------------------------------
# module_name_to_jvm_class
# ---------------------------------------------------------------------------


class TestModuleNameToJvmClass:
    def test_identity_single_slash(self) -> None:
        assert module_name_to_jvm_class("user/hello") == "user/hello"

    def test_identity_two_slashes(self) -> None:
        assert module_name_to_jvm_class("stdlib/io/print") == "stdlib/io/print"

    def test_identity_no_slash(self) -> None:
        assert module_name_to_jvm_class("mymodule") == "mymodule"


# ---------------------------------------------------------------------------
# compile_modules — structural tests (no java needed)
# ---------------------------------------------------------------------------


class TestCompileModules:
    """compile_modules returns a correctly structured MultiModuleResult."""

    def test_returns_multi_module_result(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert isinstance(result, MultiModuleResult)

    def test_runtime_artifact_present(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert result.runtime_artifact is not None
        assert result.runtime_artifact.class_bytes[:4] == b"\xca\xfe\xba\xbe"

    def test_runtime_artifact_class_name(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        # class_name uses dots (canonical Java); binary name uses slashes
        assert "TwigRuntime" in result.runtime_artifact.class_name

    def test_two_module_results_returned(self, two_module_dir: Path) -> None:
        """``host`` is excluded; ``a/math`` and ``user/hello`` both compile."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        module_names = [m.module_name for m in result.modules]
        assert "a/math" in module_names
        assert "user/hello" in module_names
        # host must NOT appear — it has no class
        assert "host" not in module_names

    def test_entry_class_name(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        assert result.entry_class_name == "user/hello"

    def test_module_compile_result_types(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        for m in result.modules:
            assert isinstance(m, ModuleCompileResult)

    def test_entry_module_flagged_as_entry(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        entry_results = [m for m in result.modules if m.module_name == "user/hello"]
        assert len(entry_results) == 1
        assert entry_results[0].is_entry is True

    def test_dep_module_not_flagged_as_entry(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        dep_results = [m for m in result.modules if m.module_name == "a/math"]
        assert len(dep_results) == 1
        assert dep_results[0].is_entry is False

    def test_topological_order_preserved(self, two_module_dir: Path) -> None:
        """Dependencies must appear before importers."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        names = [m.module_name for m in result.modules]
        assert names.index("a/math") < names.index("user/hello")

    def test_invalid_entry_module_raises(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        with pytest.raises(ValueError, match="not found"):
            compile_modules(modules, entry_module="does/not/exist")


# ---------------------------------------------------------------------------
# Entry module has main(); dep module does not
# ---------------------------------------------------------------------------


class TestMainWrapperPresence:
    """Entry module includes main(); dependency module must not."""

    def test_entry_module_has_main_wrapper(self, two_module_dir: Path) -> None:
        """The entry module's class bytes must contain a ``main`` method
        so the JVM can invoke it as an entry point."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        entry = next(m for m in result.modules if m.module_name == "user/hello")
        # The constant pool must contain the UTF-8 string "main"
        assert b"main" in entry.artifact.class_bytes, (
            "entry module's class bytes must contain 'main'"
        )

    def test_dep_module_has_no_main_wrapper(self, two_module_dir: Path) -> None:
        """The dependency module's class bytes must NOT contain a ``main``
        wrapper (it would be dead code and confuses the JVM launcher)."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        dep = next(m for m in result.modules if m.module_name == "a/math")
        # The string "main" should NOT appear as a method name in the
        # constant pool.  (It may appear as part of "([Ljava/lang/String;)V"
        # in a full descriptor, so we check the methods table instead by
        # checking for the main descriptor.)
        assert b"([Ljava/lang/String;)V" not in dep.artifact.class_bytes, (
            "dep module's class bytes must NOT contain a main() descriptor"
        )


# ---------------------------------------------------------------------------
# Cross-module CALL in IR
# ---------------------------------------------------------------------------


class TestCrossModuleCallInIr:
    """The entry module's IrProgram must contain IrOp.CALL with a
    qualified label like 'a/math/add'."""

    def test_cross_module_call_in_ir(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        entry = next(m for m in result.modules if m.module_name == "user/hello")

        call_labels = [
            instr.operands[0]
            for instr in entry.ir_program.instructions
            if instr.opcode == IrOp.CALL
        ]
        qualified_labels = [
            lbl.name for lbl in call_labels
            if isinstance(lbl, IrLabel) and "/" in lbl.name
        ]
        assert qualified_labels, (
            "entry module IR should contain at least one cross-module CALL"
        )
        assert "a/math/add" in qualified_labels

    def test_dep_module_ir_has_local_calls_only(self, two_module_dir: Path) -> None:
        """The dependency module (a/math) only has local function calls."""
        modules = _resolve("user/hello", two_module_dir)
        result = compile_modules(modules, entry_module="user/hello")
        dep = next(m for m in result.modules if m.module_name == "a/math")

        cross_module_calls = [
            instr
            for instr in dep.ir_program.instructions
            if instr.opcode == IrOp.CALL
            and isinstance(instr.operands[0], IrLabel)
            and "/" in instr.operands[0].name
        ]
        assert cross_module_calls == [], (
            "a/math should not call any cross-module functions"
        )


# ---------------------------------------------------------------------------
# run_modules — real JVM test
# ---------------------------------------------------------------------------


@pytest.mark.skipif(
    not java_available(),
    reason="java not on PATH — skipping real-JVM multi-module test",
)
class TestRunModulesOnRealJava:
    """End-to-end test: the two-module program runs on real java and
    exits with code 0 (SYSCALL 1 writes the byte 42 to stdout)."""

    def test_run_modules_exit_code_zero(self, two_module_dir: Path) -> None:
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, entry_module="user/hello")
        assert isinstance(result, MultiModuleExecutionResult)
        assert result.exit_code == 0, (
            f"Expected exit code 0, got {result.exit_code}.\n"
            f"stdout={result.stdout!r}\n"
            f"stderr={result.stderr!r}"
        )

    def test_run_modules_stdout_is_byte_42(self, two_module_dir: Path) -> None:
        """17 + 25 = 42.  The entry module's _start writes that as a byte
        to stdout via SYSCALL 1, so stdout should be b'\\x2a' (= 42)."""
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, entry_module="user/hello")
        assert result.stdout == bytes([42]), (
            f"Expected stdout = b'\\x2a' (42), got {result.stdout!r}"
        )

    def test_run_modules_acceptance_criterion(self, two_module_dir: Path) -> None:
        """This test is the spec's acceptance criterion verbatim."""
        modules = _resolve("user/hello", two_module_dir)
        result = run_modules(modules, entry_module="user/hello")
        assert result.exit_code == 0

    def test_sub_function_also_callable(self, tmp_path: Path) -> None:
        """Regression: exporting two functions (add + sub) must both compile."""
        (tmp_path / "a").mkdir()
        (tmp_path / "user").mkdir()
        (tmp_path / "a" / "math.tw").write_text("""
            (module a/math (export add sub))
            (define (add x y) (+ x y))
            (define (sub x y) (- x y))
        """)
        (tmp_path / "user" / "hello.tw").write_text("""
            (module user/hello (import a/math))
            (a/math/sub 30 5)
        """)
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        result = run_modules(modules, entry_module="user/hello")
        assert result.exit_code == 0
        # 30 - 5 = 25 = 0x19
        assert result.stdout == bytes([25])
