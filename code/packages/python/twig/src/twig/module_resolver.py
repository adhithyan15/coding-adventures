"""TW04 Phase 4b — Twig module resolver.

What this does
==============
Given an *entry module name* (e.g. ``user/hello``) and a list of
*module search paths* (directories to look in), the resolver
walks the import graph and returns every reachable module's
parsed AST in topological order — deepest dependencies first,
entry module last.

Why it's its own module
-----------------------
Module resolution is a discrete compilation phase that sits
between parsing (TW04 Phase 4a) and IR emission (Phase 4c+).
Keeping it isolated lets the per-backend compilers stay
single-module — they just ask the resolver "give me everything
this entry needs, in the right order" and lower each module
in turn.  The resolver knows nothing about IR or backends; the
backends know nothing about file paths or the ``host``
synthetic module.

What it does NOT do
-------------------
- **No code generation.**  Returns ASTs only.  Phase 4c adds
  cross-module IR ops; 4d–4f wire per-backend lowering.
- **No name resolution within a module.**  The resolver checks
  that imports exist (their files are findable, their declared
  names match their paths, no cycles).  Whether
  ``(host/write-byte 65)`` actually refers to a real export is
  a Phase 4c concern.
- **No transitive re-exports.**  ``(import a)`` brings in ``a``'s
  exports, not ``a``'s imports' exports.  Re-exports are a v2
  feature per the TW04 non-goals.
- **No package-manager behaviour.**  Search paths are just a
  list of directories; resolution is purely positional.

The synthetic ``host`` module
-----------------------------
The compiler's ``host`` module isn't on disk — it's the
cross-backend host-call surface (``write-byte`` / ``read-byte`` /
``exit``).  The resolver synthesises it on demand when something
imports ``host``.  Each backend's Phase 4d/e/f lowering will
intercept the host-named exports and emit the per-runtime
implementations instead of looking for a ``host.tw`` file.

Algorithmic shape
-----------------
Standard DFS post-order topological sort with three-colour
cycle detection:

* **WHITE** — name not yet visited (absent from both sets).
* **GRAY**  — name in the current DFS stack (in ``visiting``).
              Encountering a GRAY name means a cycle.
* **BLACK** — name fully resolved (in ``cache``).

Because we record GRAY entries on a stack rather than just a
set, we can produce a precise cycle path for the error message
("a → b → c → a") rather than just naming the offending node.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

from twig.ast_extract import extract_program
from twig.ast_nodes import Module, Program
from twig.errors import TwigCompileError
from twig.parser import parse_twig

# ── The synthetic ``host`` module ─────────────────────────────────────────

HOST_MODULE_NAME: str = "host"

# v1 host surface — see TW04 spec §"The host package".  Three primitives,
# fixed signatures, every backend lowers them to the corresponding
# runtime facility.
HOST_EXPORTS: tuple[str, ...] = ("write-byte", "read-byte", "exit")


# ── Public API surface ────────────────────────────────────────────────────


@dataclass(frozen=True)
class ResolvedModule:
    """One module discovered by the resolver.

    * ``name`` — the module's declared / synthesised name
      (``stdlib/io``, ``user/compiler/lexer``, ``host``).
    * ``program`` — the parsed :class:`Program` AST.  Always
      has a non-``None`` ``module`` field after resolution
      (the resolver rejects files that lack a ``(module ...)``
      declaration — that's an unambiguous error when the file
      was reached via an import).
    * ``source_path`` — the file the module was loaded from,
      or ``None`` for the synthetic ``host`` module which has
      no on-disk source.
    """

    name: str
    program: Program
    source_path: Path | None


def resolve_modules(
    entry_module: str,
    *,
    search_paths: Sequence[Path],
) -> list[ResolvedModule]:
    """Resolve ``entry_module`` and its transitive imports.

    Returns the resolved modules in topological order — every
    module appears AFTER all of its (transitive) imports, with
    the entry module last.  This is exactly the order each
    backend wants to lower modules in: a downstream consumer
    can iterate the list once and emit code without forward
    references.

    Raises :class:`TwigCompileError` on:

    * **Missing module file** — ``entry_module`` or any
      imported module isn't found in any search path.
    * **Path / name mismatch** — a file at ``stdlib/io.tw``
      declares ``(module foo)`` (or omits the form entirely
      while being imported by name).
    * **Import cycle** — module ``A`` imports ``B`` which
      imports ``A`` (transitively).  The error message names
      the full cycle path.

    The synthetic ``host`` module is auto-resolved without
    consulting the search path.
    """
    # Topo state — three-colour DFS.
    cache: dict[str, ResolvedModule] = {}      # BLACK: fully resolved
    visiting: set[str] = set()                 # GRAY: on current DFS stack
    out: list[ResolvedModule] = []
    stack: list[str] = []                      # for cycle-path messages

    _resolve_dfs(
        entry_module,
        search_paths=search_paths,
        cache=cache,
        visiting=visiting,
        stack=stack,
        out=out,
    )
    return out


# ── Implementation ────────────────────────────────────────────────────────


def _synthetic_host_module() -> ResolvedModule:
    """Build the ``host`` module on the fly.

    The Module declares the v1 host surface; the body is empty
    because every export is implemented by per-backend lowering,
    not by Twig source.  The resolver hands the same Program
    shape back as for on-disk modules so downstream code never
    needs a "host?" branch — it iterates ``ResolvedModule``s
    uniformly.
    """
    program = Program(
        forms=[],
        module=Module(
            name=HOST_MODULE_NAME,
            exports=list(HOST_EXPORTS),
            imports=[],
        ),
    )
    return ResolvedModule(
        name=HOST_MODULE_NAME,
        program=program,
        source_path=None,
    )


def _module_name_to_relpath(name: str) -> Path:
    """Map a module name to its relative file path on disk.

    ``stdlib/io`` → ``stdlib/io.tw`` (path-shaped).  The
    forward slashes in the module name map directly to path
    separators so module hierarchy mirrors directory hierarchy
    on every OS — ``pathlib.Path`` normalises the result for
    the host platform.
    """
    parts = name.split("/")
    return Path(*parts).with_suffix(".tw")


def _find_module_file(name: str, search_paths: Sequence[Path]) -> Path:
    """Locate the file backing ``name`` in the search paths.

    Search paths are tried in order; the first hit wins.  This
    matches Python's ``sys.path`` and Rust's ``--extern``
    semantics — earlier paths shadow later ones.
    """
    rel = _module_name_to_relpath(name)
    tried: list[str] = []
    for sp in search_paths:
        candidate = sp / rel
        tried.append(str(candidate))
        if candidate.is_file():
            return candidate
    if not tried:
        raise TwigCompileError(
            f"module {name!r} not found.  No module search "
            f"paths configured."
        )
    raise TwigCompileError(
        f"module {name!r} not found.  Looked in:\n  "
        + "\n  ".join(tried)
    )


def _load_and_validate(name: str, path: Path) -> Program:
    """Parse a file and check its (module ...) declaration.

    A file reached via the resolver MUST declare a
    ``(module ...)`` form whose name matches the import path.
    Implicit-default-module (no ``(module ...)`` form) is only
    legal when compiling a single-file program through the
    direct compile-source API — never when the module is
    reached by name through the import graph.
    """
    source = path.read_text()
    program = extract_program(parse_twig(source))

    if program.module is None:
        raise TwigCompileError(
            f"file {path} has no (module ...) declaration but was "
            f"reached as module {name!r} through an import"
        )
    if program.module.name != name:
        raise TwigCompileError(
            f"file {path} declares (module {program.module.name!r}) "
            f"but was reached as module {name!r} — module names "
            f"must match their file paths"
        )
    return program


def _resolve_dfs(
    name: str,
    *,
    search_paths: Sequence[Path],
    cache: dict[str, ResolvedModule],
    visiting: set[str],
    stack: list[str],
    out: list[ResolvedModule],
) -> None:
    """Visit ``name``, recurse into its imports, then append it.

    Post-order DFS over the import graph yields topo-sorted
    modules: every node appears AFTER all of its transitive
    dependencies in ``out``.  See module docstring for the
    three-colour invariant.
    """
    # BLACK: already resolved.  Nothing to do — re-imports are fine
    # (they're DAG joins, not cycles).
    if name in cache:
        return

    # GRAY: revisiting a node still on the DFS stack — cycle.
    # The cycle path is ``stack[idx..] + [name]`` where ``idx``
    # is where the offending node first appeared.
    if name in visiting:
        idx = stack.index(name)
        cycle = stack[idx:] + [name]
        raise TwigCompileError(
            f"module import cycle detected: {' -> '.join(cycle)}"
        )

    # The synthetic host module short-circuits.  Visit-order
    # matters: it goes into ``out`` BEFORE any module that
    # imports it, so backends emit the host stubs first.
    if name == HOST_MODULE_NAME:
        host = _synthetic_host_module()
        cache[name] = host
        out.append(host)
        return

    visiting.add(name)
    stack.append(name)

    path = _find_module_file(name, search_paths)
    program = _load_and_validate(name, path)
    assert program.module is not None  # _load_and_validate enforces

    # Recurse into imports first (post-order = topo sort).
    for imp in program.module.imports:
        _resolve_dfs(
            imp,
            search_paths=search_paths,
            cache=cache,
            visiting=visiting,
            stack=stack,
            out=out,
        )

    # Done with this node — flip GRAY → BLACK and emit.
    resolved = ResolvedModule(name=name, program=program, source_path=path)
    cache[name] = resolved
    out.append(resolved)

    visiting.remove(name)
    stack.pop()
