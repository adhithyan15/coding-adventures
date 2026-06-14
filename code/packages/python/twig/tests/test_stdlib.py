"""Tests for TW04 Phase 4g — bundled Twig standard library.

These tests verify:

1. ``stdlib_path()`` returns a ``Path`` that exists and contains the
   expected ``stdlib/io.tw``, ``stdlib/list.tw``, and ``stdlib/print.tw``
   source files.
2. ``resolve_modules("stdlib/io", search_paths=[])`` succeeds (stdlib is
   auto-included via ``include_stdlib=True`` default).
3. Each stdlib module exports the expected names.
4. ``include_stdlib=False`` disables the auto-injection so tests that need
   explicit control still work.
5. The compiler parses and extracts the stdlib modules without error
   (structural tests — no runtime needed).
"""

from __future__ import annotations

from pathlib import Path

import pytest

from twig import stdlib_path
from twig.ast_extract import extract_program
from twig.module_resolver import resolve_modules
from twig.parser import parse_twig

# ── stdlib_path() ─────────────────────────────────────────────────────────────


class TestStdlibPath:
    """``stdlib_path()`` locates the bundled stdlib root."""

    def test_returns_path_instance(self) -> None:
        assert isinstance(stdlib_path(), Path)

    def test_path_exists(self) -> None:
        assert stdlib_path().exists(), (
            f"stdlib root does not exist: {stdlib_path()}"
        )

    def test_path_is_directory(self) -> None:
        assert stdlib_path().is_dir()

    def test_stdlib_subdir_exists(self) -> None:
        """The ``stdlib/`` sub-directory must exist inside the root."""
        assert (stdlib_path() / "stdlib").is_dir()

    def test_io_tw_exists(self) -> None:
        assert (stdlib_path() / "stdlib" / "io.tw").is_file(), (
            "stdlib/io.tw missing from bundled stdlib"
        )

    def test_list_tw_exists(self) -> None:
        assert (stdlib_path() / "stdlib" / "list.tw").is_file(), (
            "stdlib/list.tw missing from bundled stdlib"
        )

    def test_print_tw_exists(self) -> None:
        assert (stdlib_path() / "stdlib" / "print.tw").is_file(), (
            "stdlib/print.tw missing from bundled stdlib"
        )


# ── Module resolution via auto-included stdlib ────────────────────────────────


class TestAutoIncludeStdlib:
    """With ``include_stdlib=True`` (the default), stdlib modules resolve
    without passing any explicit search paths."""

    def test_resolve_stdlib_io_no_search_paths(self) -> None:
        """``resolve_modules("stdlib/io", search_paths=[])`` succeeds."""
        modules = resolve_modules("stdlib/io", search_paths=[])
        names = [m.name for m in modules]
        assert "stdlib/io" in names

    def test_resolve_stdlib_list_no_search_paths(self) -> None:
        modules = resolve_modules("stdlib/list", search_paths=[])
        names = [m.name for m in modules]
        assert "stdlib/list" in names

    def test_resolve_stdlib_print_no_search_paths(self) -> None:
        """stdlib/print imports stdlib/io, so both must resolve."""
        modules = resolve_modules("stdlib/print", search_paths=[])
        names = [m.name for m in modules]
        assert "stdlib/print" in names
        assert "stdlib/io" in names

    def test_host_module_auto_resolved_alongside_stdlib(self) -> None:
        """Resolving stdlib/io also resolves the synthetic host module."""
        modules = resolve_modules("stdlib/io", search_paths=[])
        names = [m.name for m in modules]
        assert "host" in names

    def test_topological_order_host_before_stdlib_io(self) -> None:
        """``host`` must appear before ``stdlib/io`` (io imports host)."""
        modules = resolve_modules("stdlib/io", search_paths=[])
        names = [m.name for m in modules]
        assert names.index("host") < names.index("stdlib/io")

    def test_topological_order_stdlib_io_before_print(self) -> None:
        """``stdlib/io`` must appear before ``stdlib/print``."""
        modules = resolve_modules("stdlib/print", search_paths=[])
        names = [m.name for m in modules]
        assert names.index("stdlib/io") < names.index("stdlib/print")

    def test_include_stdlib_false_raises_for_stdlib_module(self) -> None:
        """Opting out of auto-stdlib with no other paths raises a config error.

        With ``include_stdlib=False`` and ``search_paths=[]`` there are
        no paths at all, so the resolver emits the "No module search
        paths configured" guard error rather than a "not found" error.
        """
        from twig.errors import TwigCompileError

        with pytest.raises(TwigCompileError, match="No module search paths"):
            resolve_modules(
                "stdlib/io",
                search_paths=[],
                include_stdlib=False,
            )

    def test_explicit_stdlib_path_works_with_include_stdlib_false(self) -> None:
        """Manual path injection with ``include_stdlib=False`` still works."""
        modules = resolve_modules(
            "stdlib/io",
            search_paths=[stdlib_path()],
            include_stdlib=False,
        )
        names = [m.name for m in modules]
        assert "stdlib/io" in names

    def test_user_module_can_import_stdlib_io(self, tmp_path: Path) -> None:
        """A user module that imports stdlib/io resolves correctly."""
        (tmp_path / "user").mkdir()
        (tmp_path / "user" / "hello.tw").write_text(
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n"
        )
        modules = resolve_modules("user/hello", search_paths=[tmp_path])
        names = [m.name for m in modules]
        assert "stdlib/io" in names
        assert "user/hello" in names
        # Order: host → stdlib/io → user/hello
        assert names.index("stdlib/io") < names.index("user/hello")


# ── stdlib/io export names ────────────────────────────────────────────────────


class TestStdlibIoExports:
    """``stdlib/io`` declares the expected export surface."""

    @pytest.fixture()
    def io_module(self):
        modules = resolve_modules("stdlib/io", search_paths=[])
        return next(m for m in modules if m.name == "stdlib/io")

    def test_module_declaration_present(self, io_module) -> None:
        assert io_module.program.module is not None

    def test_exports_print_int(self, io_module) -> None:
        assert "print-int" in io_module.program.module.exports

    def test_exports_println(self, io_module) -> None:
        assert "println" in io_module.program.module.exports

    def test_exports_newline(self, io_module) -> None:
        assert "newline" in io_module.program.module.exports

    def test_exports_print_bool(self, io_module) -> None:
        assert "print-bool" in io_module.program.module.exports

    def test_imports_host(self, io_module) -> None:
        assert "host" in io_module.program.module.imports

    def test_source_path_points_to_io_tw(self, io_module) -> None:
        assert io_module.source_path is not None
        assert io_module.source_path.name == "io.tw"

    def test_source_path_is_file(self, io_module) -> None:
        assert io_module.source_path.is_file()


# ── stdlib/list export names ──────────────────────────────────────────────────


class TestStdlibListExports:
    """``stdlib/list`` declares the expected export surface."""

    @pytest.fixture()
    def list_module(self):
        modules = resolve_modules("stdlib/list", search_paths=[])
        return next(m for m in modules if m.name == "stdlib/list")

    def test_module_declaration_present(self, list_module) -> None:
        assert list_module.program.module is not None

    def test_exports_length(self, list_module) -> None:
        assert "length" in list_module.program.module.exports

    def test_exports_reverse(self, list_module) -> None:
        assert "reverse" in list_module.program.module.exports

    def test_exports_map(self, list_module) -> None:
        assert "map" in list_module.program.module.exports

    def test_exports_filter(self, list_module) -> None:
        assert "filter" in list_module.program.module.exports

    def test_exports_fold(self, list_module) -> None:
        assert "fold" in list_module.program.module.exports

    def test_source_path_points_to_list_tw(self, list_module) -> None:
        assert list_module.source_path is not None
        assert list_module.source_path.name == "list.tw"


# ── stdlib/print export names ─────────────────────────────────────────────────


class TestStdlibPrintExports:
    """``stdlib/print`` declares the expected export surface."""

    @pytest.fixture()
    def print_module(self):
        modules = resolve_modules("stdlib/print", search_paths=[])
        return next(m for m in modules if m.name == "stdlib/print")

    def test_module_declaration_present(self, print_module) -> None:
        assert print_module.program.module is not None

    def test_exports_print(self, print_module) -> None:
        assert "print" in print_module.program.module.exports

    def test_imports_host(self, print_module) -> None:
        assert "host" in print_module.program.module.imports

    def test_imports_stdlib_io(self, print_module) -> None:
        assert "stdlib/io" in print_module.program.module.imports

    def test_source_path_points_to_print_tw(self, print_module) -> None:
        assert print_module.source_path is not None
        assert print_module.source_path.name == "print.tw"


# ── Compilation (parse + extract) — no runtime needed ────────────────────────


class TestStdlibParseAndExtract:
    """The stdlib source files parse and extract without errors."""

    def _parse_extract(self, name: str):
        path = stdlib_path() / "stdlib" / f"{name}.tw"
        source = path.read_text()
        return extract_program(parse_twig(source))

    def test_io_tw_parses(self) -> None:
        prog = self._parse_extract("io")
        assert prog.module is not None
        assert prog.module.name == "stdlib/io"

    def test_list_tw_parses(self) -> None:
        prog = self._parse_extract("list")
        assert prog.module is not None
        assert prog.module.name == "stdlib/list"

    def test_print_tw_parses(self) -> None:
        prog = self._parse_extract("print")
        assert prog.module is not None
        assert prog.module.name == "stdlib/print"

    def test_io_tw_has_multiple_forms(self) -> None:
        """io.tw defines at least 4 functions (print-digits + 3 exports)."""
        prog = self._parse_extract("io")
        # forms = list of top-level expressions (excluding the module decl)
        assert len(prog.forms) >= 4

    def test_list_tw_has_multiple_forms(self) -> None:
        """list.tw defines at least 5 functions."""
        prog = self._parse_extract("list")
        assert len(prog.forms) >= 5

    def test_print_tw_has_at_least_one_form(self) -> None:
        prog = self._parse_extract("print")
        assert len(prog.forms) >= 1
