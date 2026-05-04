"""TW04 Phase 4g — stdlib/io structural tests on BEAM.

These tests verify that Twig programs importing ``stdlib/io`` from the
bundled stdlib resolve and structurally compile for BEAM, and document
the known limitation of the BEAM multi-module + host-call gap.

Known limitation (Phase 4f gap)
---------------------------------
The BEAM multi-module compiler (Phase 4f) does not yet support
``host/write-byte`` / ``host/read-byte`` / ``host/exit`` in multi-module
builds.  In single-module mode (``run_source``), these map to inline
BEAM instructions (e.g. ``io:fwrite``).  In multi-module mode, the
cross-module IR treats every name with an interior ``/`` as a BEAM
remote call, including ``host/write-byte``, which generates a
``call_ext`` to a non-existent ``host`` module at runtime.

Resolving this requires either:
* A BEAM shim module named ``host`` that re-exports the syscall operations
* Per-backend detection of the synthetic ``host`` module and special-casing
  it in the IR lowering stage

Until that is addressed, end-to-end runtime tests for stdlib/io on BEAM
multi-module are xfail (expected to fail) rather than skipped, so CI
catches regressions if the fix lands.  Structural tests (resolution,
compilation, IR inspection) run unconditionally and must always pass.

Tests that require a live ``erl`` binary skip cleanly when it is not on PATH.
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
# Runtime tests — xfail due to Phase 4f host-call gap
# ---------------------------------------------------------------------------


@requires_erl
@pytest.mark.xfail(
    reason=(
        "BEAM multi-module + host/write-byte not yet supported: the Phase 4f "
        "lowering emits call_ext to a 'host' BEAM module that does not exist. "
        "The stdlib/io runtime tests will pass once the BEAM host-shim or "
        "per-module host-call lowering is implemented."
    ),
    strict=False,  # allow unexpected passes if the fix lands
)
class TestStdlibIoRuntimeBeam:
    """End-to-end runtime tests for stdlib/io on BEAM.

    These are xfail (expected to fail) because the Phase 4f BEAM
    multi-module compiler does not yet support ``host/write-byte`` in
    multi-module builds.  They are kept as xfail rather than skipped so
    that CI catches the moment the fix lands.
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
