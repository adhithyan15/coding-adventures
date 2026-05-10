"""Tests for ``twig.module_resolver`` (TW04 Phase 4b).

The resolver walks the import graph and returns every reachable
module's parsed AST in topological order — deepest dependencies
first, entry module last.  These tests cover the four classes of
behaviour the spec calls out:

1. **Happy path** — single module, transitive imports, the
   synthetic ``host`` module, multiple search paths.
2. **Cycle detection** — two-node and longer cycles produce
   precise path messages.
3. **Missing imports** — entry or transitive module not found
   on disk.
4. **Path / name mismatches** — file at ``stdlib/io.tw``
   declaring ``(module foo)`` is rejected.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from twig.errors import TwigCompileError
from twig.module_resolver import (
    HOST_EXPORTS,
    HOST_MODULE_NAME,
    ResolvedModule,
    resolve_modules,
)


def _write(root: Path, rel: str, contents: str) -> Path:
    """Create ``root/<rel>`` with the given contents and return its path."""
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents)
    return path


# ── Happy path ────────────────────────────────────────────────────────────


def test_single_module_with_no_imports(tmp_path: Path) -> None:
    """A module with no imports resolves to a single-element list."""
    _write(tmp_path, "lone.tw", "(module lone (export f))\n(define f 42)\n")
    result = resolve_modules("lone", search_paths=[tmp_path])
    assert len(result) == 1
    assert result[0].name == "lone"
    assert result[0].source_path is not None
    assert result[0].program.module is not None
    assert result[0].program.module.exports == ["f"]


def test_module_importing_another_module(tmp_path: Path) -> None:
    """``a`` imports ``b``; both appear in topological order
    (``b`` before ``a``).
    """
    _write(tmp_path, "a.tw", "(module a (import b))\n(define x 1)\n")
    _write(tmp_path, "b.tw", "(module b (export y))\n(define y 2)\n")
    result = resolve_modules("a", search_paths=[tmp_path])
    names = [r.name for r in result]
    assert names == ["b", "a"], (
        "topological order should put dependencies before importers"
    )


def test_transitive_imports_chain(tmp_path: Path) -> None:
    """``a`` → ``b`` → ``c``: result is ``[c, b, a]``."""
    _write(tmp_path, "a.tw", "(module a (import b))\n(define x 1)\n")
    _write(tmp_path, "b.tw", "(module b (import c) (export f))\n(define f 2)\n")
    _write(tmp_path, "c.tw", "(module c (export g))\n(define g 3)\n")
    result = resolve_modules("a", search_paths=[tmp_path])
    assert [r.name for r in result] == ["c", "b", "a"]


def test_diamond_dependency(tmp_path: Path) -> None:
    """``a`` imports ``b`` and ``c``; both import ``d``.

    Diamond shape — ``d`` should appear exactly once and before
    both ``b`` and ``c``, which both appear before ``a``.
    """
    _write(tmp_path, "a.tw", "(module a (import b c))\n(define x 1)\n")
    _write(tmp_path, "b.tw", "(module b (import d) (export f))\n(define f 2)\n")
    _write(tmp_path, "c.tw", "(module c (import d) (export g))\n(define g 3)\n")
    _write(tmp_path, "d.tw", "(module d (export h))\n(define h 4)\n")
    result = resolve_modules("a", search_paths=[tmp_path])
    names = [r.name for r in result]
    assert names.count("d") == 1, "shared dependency must not be duplicated"
    assert names.index("d") < names.index("b")
    assert names.index("d") < names.index("c")
    assert names.index("b") < names.index("a")
    assert names.index("c") < names.index("a")


def test_nested_module_path(tmp_path: Path) -> None:
    """Slash-separated module names map to nested directories.

    ``user/compiler/lexer`` lives at
    ``<search_path>/user/compiler/lexer.tw``.
    """
    _write(
        tmp_path,
        "user/compiler/lexer.tw",
        "(module user/compiler/lexer (export tokenise))\n"
        "(define tokenise 0)\n",
    )
    result = resolve_modules(
        "user/compiler/lexer", search_paths=[tmp_path]
    )
    assert len(result) == 1
    assert result[0].name == "user/compiler/lexer"


def test_synthetic_host_module_is_auto_resolved(tmp_path: Path) -> None:
    """Importing ``host`` works without a ``host.tw`` file.

    The resolver synthesises the module on the fly with the v1
    export surface (``write-byte``, ``read-byte``, ``exit``).
    """
    _write(
        tmp_path,
        "user/hello.tw",
        "(module user/hello (import host))\n(host/write-byte 65)\n",
    )
    result = resolve_modules("user/hello", search_paths=[tmp_path])
    names = [r.name for r in result]
    assert names == [HOST_MODULE_NAME, "user/hello"]
    host = next(r for r in result if r.name == HOST_MODULE_NAME)
    assert host.source_path is None, "host module has no on-disk source"
    assert host.program.module is not None
    assert tuple(host.program.module.exports) == HOST_EXPORTS


def test_multiple_search_paths_first_hit_wins(tmp_path: Path) -> None:
    """Search paths are consulted in order; earlier paths shadow later
    ones — same semantics as Python's ``sys.path``.
    """
    a = tmp_path / "a"
    b = tmp_path / "b"
    _write(a, "io.tw", "(module io (export from-a))\n(define from-a 1)\n")
    _write(b, "io.tw", "(module io (export from-b))\n(define from-b 2)\n")
    result = resolve_modules("io", search_paths=[a, b])
    assert len(result) == 1
    assert result[0].source_path is not None
    assert result[0].source_path.is_relative_to(a)


def test_re_imported_module_appears_once(tmp_path: Path) -> None:
    """If a module is imported through multiple paths in the graph,
    the resolver visits it once.  Plain DAG-join behaviour.
    """
    _write(
        tmp_path,
        "a.tw",
        "(module a (import b c))\n(define x 1)\n",
    )
    _write(tmp_path, "b.tw", "(module b (import c) (export f))\n(define f 2)\n")
    _write(tmp_path, "c.tw", "(module c (export g))\n(define g 3)\n")
    result = resolve_modules("a", search_paths=[tmp_path])
    names = [r.name for r in result]
    assert names.count("c") == 1
    assert names == ["c", "b", "a"]


# ── Cycle detection ───────────────────────────────────────────────────────


def test_two_module_cycle_rejected(tmp_path: Path) -> None:
    """``a`` imports ``b`` imports ``a``."""
    _write(tmp_path, "a.tw", "(module a (import b))\n(define x 1)\n")
    _write(tmp_path, "b.tw", "(module b (import a))\n(define y 2)\n")
    with pytest.raises(TwigCompileError, match=r"cycle.*a -> b -> a"):
        resolve_modules("a", search_paths=[tmp_path])


def test_three_module_cycle_rejected(tmp_path: Path) -> None:
    """Longer cycle: ``a → b → c → a``.  The error message names
    the full path so the user can see which edge to break."""
    _write(tmp_path, "a.tw", "(module a (import b))\n(define x 1)\n")
    _write(tmp_path, "b.tw", "(module b (import c))\n(define y 2)\n")
    _write(tmp_path, "c.tw", "(module c (import a))\n(define z 3)\n")
    with pytest.raises(TwigCompileError, match=r"cycle.*a -> b -> c -> a"):
        resolve_modules("a", search_paths=[tmp_path])


def test_self_cycle_rejected(tmp_path: Path) -> None:
    """A module that imports itself."""
    _write(tmp_path, "a.tw", "(module a (import a))\n(define x 1)\n")
    with pytest.raises(TwigCompileError, match=r"cycle.*a -> a"):
        resolve_modules("a", search_paths=[tmp_path])


# ── Missing imports ───────────────────────────────────────────────────────


def test_missing_entry_module(tmp_path: Path) -> None:
    with pytest.raises(TwigCompileError, match=r"not found"):
        resolve_modules("nope", search_paths=[tmp_path])


def test_missing_transitive_import(tmp_path: Path) -> None:
    """Entry exists but imports a module that doesn't."""
    _write(tmp_path, "a.tw", "(module a (import nope))\n(define x 1)\n")
    with pytest.raises(TwigCompileError, match=r"'nope'.*not found"):
        resolve_modules("a", search_paths=[tmp_path])


def test_no_search_paths_configured() -> None:
    """A clearer error message when the resolver has nowhere to look.

    ``include_stdlib=False`` opts out of the auto-stdlib injection so
    the empty search_paths list triggers the guard.
    """
    with pytest.raises(TwigCompileError, match=r"No module search paths"):
        resolve_modules("anything", search_paths=[], include_stdlib=False)


# ── Path / name mismatches ────────────────────────────────────────────────


def test_file_with_no_module_form_rejected_when_imported(
    tmp_path: Path,
) -> None:
    """A file reached through an import MUST declare a
    ``(module ...)`` form whose name matches.  Implicit-default-
    module is only legal for the direct compile-source API."""
    _write(tmp_path, "a.tw", "(define x 1)\n")  # no (module ...)
    with pytest.raises(TwigCompileError, match=r"no \(module \.\.\.\)"):
        resolve_modules("a", search_paths=[tmp_path])


def test_file_declaring_wrong_module_name_rejected(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "stdlib/io.tw",
        "(module wrong-name (export f))\n(define f 1)\n",
    )
    with pytest.raises(
        TwigCompileError, match=r"declares.*'wrong-name'.*'stdlib/io'"
    ):
        resolve_modules("stdlib/io", search_paths=[tmp_path])


def test_resolved_module_carries_program_and_path(tmp_path: Path) -> None:
    """``ResolvedModule`` exposes both the parsed Program and the
    source path so downstream phases (4c+) can attribute errors
    to specific files."""
    src = "(module x (export f))\n(define f 42)\n"
    written = _write(tmp_path, "x.tw", src)
    result = resolve_modules("x", search_paths=[tmp_path])
    assert isinstance(result[0], ResolvedModule)
    assert result[0].source_path == written
    assert result[0].program.forms[0] is not None  # the (define f 42)
