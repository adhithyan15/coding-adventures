"""TW04 Phase 4g — stdlib/io tests on BEAM (structural + runtime).

These tests verify that Twig programs importing ``stdlib/io`` from the
bundled stdlib resolve, structurally compile, and execute correctly on the
real ``erl`` runtime.

Phase 4f host-call gap — fixed
---------------------------------
The BEAM multi-module compiler previously emitted ``call_ext`` to a
non-existent ``host`` module for every ``host/write-byte`` call, causing
a runtime error.  The fix (shipped in ir-to-beam 0.6.0 and
twig-beam-compiler 0.9.0):

* ``twig-beam-compiler`` emits ``IrOp.SYSCALL IrImmediate(1)`` (instead
  of ``IrOp.CALL IrLabel("host/write-byte")``) for every ``host/*`` call.
* ``ir-to-beam`` lowers ``IrOp.SYSCALL 1`` to
  ``io:put_chars([Byte])`` directly in the generated BEAM code.

Structural tests (resolution, compilation, IR inspection) run
unconditionally.  Runtime tests skip cleanly when ``erl`` is not on PATH.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from twig import resolve_modules, stdlib_path

from twig_beam_compiler import erl_available
from twig_beam_compiler.compiler import (
    ModuleBeamCompileResult,
    MultiModuleBeamResult,
    compile_modules,
)

requires_erl = pytest.mark.skipif(
    not erl_available(),
    reason="erl not on PATH",
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_module(tmp_path: Path, rel: str, contents: str) -> Path:
    path = tmp_path / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents)
    return path


# ---------------------------------------------------------------------------
# Module resolution (no runtime)
# ---------------------------------------------------------------------------


class TestStdlibIoResolutionBeam:
    """stdlib/io resolves correctly for BEAM multi-module builds."""

    def test_stdlib_io_resolves(self, tmp_path: Path) -> None:
        """A user module importing stdlib/io resolves with stdlib auto-included."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        names = [m.name for m in modules]
        assert "stdlib/io" in names

    def test_stdlib_io_before_user_module(self, tmp_path: Path) -> None:
        """``stdlib/io`` appears before the user module (deps-first order)."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        names = [m.name for m in modules]
        assert names.index("stdlib/io") < names.index("user/hello")

    def test_host_resolved_before_stdlib_io(self, tmp_path: Path) -> None:
        """``host`` is resolved before ``stdlib/io`` (stdlib/io imports host)."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        names = [m.name for m in modules]
        assert names.index("host") < names.index("stdlib/io")

    def test_stdlib_path_exists(self) -> None:
        assert stdlib_path().exists()
        assert (stdlib_path() / "stdlib" / "io.tw").is_file()


# ---------------------------------------------------------------------------
# Structural compilation (no runtime) — BEAM
# ---------------------------------------------------------------------------


class TestStdlibIoCompileStructuralBeam:
    """stdlib/io structurally compiles for BEAM (no erl needed)."""

    def test_compile_modules_returns_multi_module_result(
        self, tmp_path: Path
    ) -> None:
        """``compile_modules`` with stdlib/io returns a MultiModuleBeamResult."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        result = compile_modules(modules, entry_module="user/hello")
        assert isinstance(result, MultiModuleBeamResult)

    def test_compile_includes_stdlib_io_module(self, tmp_path: Path) -> None:
        """Compiled result includes a ModuleBeamCompileResult for stdlib/io."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        result = compile_modules(modules, entry_module="user/hello")
        names = [mr.module_name for mr in result.module_results]
        assert "stdlib/io" in names

    def test_host_module_excluded_from_results(self, tmp_path: Path) -> None:
        """The synthetic host module is excluded from BEAM compile results."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        result = compile_modules(modules, entry_module="user/hello")
        names = [mr.module_name for mr in result.module_results]
        assert "host" not in names

    def test_stdlib_io_result_is_module_beam_compile_result(
        self, tmp_path: Path
    ) -> None:
        """stdlib/io compile result is a ModuleBeamCompileResult."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        result = compile_modules(modules, entry_module="user/hello")
        io_result = next(
            mr for mr in result.module_results if mr.module_name == "stdlib/io"
        )
        assert isinstance(io_result, ModuleBeamCompileResult)

    def test_stdlib_io_beam_bytes_nonempty(self, tmp_path: Path) -> None:
        """stdlib/io produces non-empty .beam bytes."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        result = compile_modules(modules, entry_module="user/hello")
        io_result = next(
            mr for mr in result.module_results if mr.module_name == "stdlib/io"
        )
        assert len(io_result.beam_bytes) > 0

    def test_user_hello_is_entry_module(self, tmp_path: Path) -> None:
        """The entry module is correctly identified in the result."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        result = compile_modules(modules, entry_module="user/hello")
        assert result.entry_module == "user/hello"


# ---------------------------------------------------------------------------
# Runtime tests — TW04 Phase 4f host-call gap fixed
# ---------------------------------------------------------------------------


@requires_erl
class TestStdlibIoRuntimeBeam:
    """End-to-end runtime tests for stdlib/io on BEAM.

    The Phase 4f BEAM host-call gap is now fixed: ``host/write-byte`` is
    emitted as ``IrOp.SYSCALL 1`` by the twig-beam-compiler and lowered to
    ``io:put_chars([Byte])`` by the BEAM backend instead of a ``call_ext``
    to a non-existent ``host`` module.  These tests are only skipped when
    ``erl`` is not on PATH.
    """

    def _run(self, entry: str, search_dir: Path):
        from twig_beam_compiler.compiler import run_modules
        modules = resolve_modules(entry, search_paths=[search_dir])
        return run_modules(modules, entry_module=entry)

    def test_println_42(self, tmp_path: Path) -> None:
        """``(stdlib/io/println 42)`` writes "42" to stdout."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        result = self._run("user/hello", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout.strip() == b"42"

    def test_println_sum_17_25(self, tmp_path: Path) -> None:
        """``(stdlib/io/println (+ 17 25))`` writes "42"."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println (+ 17 25))\n",
        )
        result = self._run("user/hello", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout.strip() == b"42"

    def test_println_twice(self, tmp_path: Path) -> None:
        """Calling println twice produces two lines."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n"
            "(stdlib/io/println (+ 17 25))\n",
        )
        result = self._run("user/hello", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout.strip() == b"42\n42"
