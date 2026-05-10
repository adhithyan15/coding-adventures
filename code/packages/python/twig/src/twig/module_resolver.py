"""TW04 Phase 4c — Twig module resolver (refactored).

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
Two-pass resolution using the ``directed-graph`` package:

**Pass 1 — Discovery.**  A simple recursive scan that reads and
parses every reachable ``.tw`` file (or synthesises the ``host``
stub), collecting results in a ``dict[str, ResolvedModule]``.
No ordering happens here; the goal is purely to gather every
module and surface file-not-found / name-mismatch errors early.

**Pass 2 — Topological ordering.**  Build a
``DirectedGraph[str]`` where each import declaration
``(import B)`` in module A adds the directed edge ``A → B``
(reads: "A depends on B").  Then call ``topological_sort`` from
the ``directed-graph`` package, which uses Kahn's BFS-based
algorithm and raises ``ValueError`` if the graph contains a
cycle.  ``topological_sort`` returns nodes in *u-before-v* order
for each edge ``u → v`` — i.e. importers before dependencies —
so we **reverse** the result to get the deps-first order
backends need.

Cycle detection
---------------
When ``topological_sort`` raises ``ValueError`` (cycle present),
we call ``strongly_connected_components`` to identify which
nodes are involved, then trace a concrete path through the
smallest SCC for the error message::

    cycle: a → b → c → a

This preserves the same precise error format that Phase 4b
users have come to expect, without reimplementing the cycle
detection algorithm itself.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from directed_graph import (
    DirectedGraph,
    strongly_connected_components,
    topological_sort,
)

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
    include_stdlib: bool = True,
) -> list[ResolvedModule]:
    """Resolve ``entry_module`` and its transitive imports.

    Returns the resolved modules in topological order — every
    module appears AFTER all of its (transitive) imports, with
    the entry module last.  This is exactly the order each
    backend wants to lower modules in: a downstream consumer
    can iterate the list once and emit code without forward
    references.

    Parameters
    ----------
    entry_module:
        The Twig module name to resolve (e.g. ``"user/hello"``).
    search_paths:
        Ordered list of directories to search for ``.tw`` files.
        Earlier entries shadow later ones, matching Python's
        ``sys.path`` semantics.
    include_stdlib:
        When ``True`` (default), the bundled Twig standard library
        is appended to ``search_paths`` automatically.  This allows
        Twig programs to ``(import stdlib/io)`` without the caller
        needing to know where the stdlib lives on disk.  Pass
        ``False`` if you are supplying the stdlib path manually
        (e.g. during stdlib development / testing) or if you
        intentionally want to exclude it.

    Raises :class:`TwigCompileError` on:

    * **No search paths** — ``search_paths`` is empty and
      ``include_stdlib=False``.
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
    # Append the bundled stdlib search root when requested.
    # Import here to avoid a circular dependency at module load time
    # (twig/__init__.py imports from this module, and we import back).
    effective_paths: Sequence[Path] = search_paths
    if include_stdlib:
        from pathlib import Path as _Path

        _stdlib = _Path(__file__).parent / "stdlib_twig"
        if _stdlib.exists():
            effective_paths = [*search_paths, _stdlib]

    if not effective_paths:
        raise TwigCompileError(
            "No module search paths configured.  "
            "Pass at least one directory via search_paths."
        )

    # Pass 1: recursively discover every reachable module.
    discovered: dict[str, ResolvedModule] = {}
    _discover(entry_module, effective_paths, discovered)

    # Pass 2: build the import graph and topo-sort it.
    order = _topo_order(discovered)

    return [discovered[name] for name in order]


# ── Pass 1: discovery ─────────────────────────────────────────────────────


# Maximum allowed depth of the import chain during discovery.  This
# guards against a non-cyclic but extremely deep import chain that
# would exhaust Python's default recursion limit (~1000) before
# Pass 2 cycle detection can fire.  200 levels is more than any
# real program needs while still fitting within the default limit
# even if the call stack already has some depth from the caller.
_MAX_IMPORT_DEPTH: int = 200


def _discover(
    name: str,
    search_paths: Sequence[Path],
    discovered: dict[str, ResolvedModule],
    *,
    _depth: int = 0,
) -> None:
    """Recursively load and validate ``name`` and all its imports.

    Results accumulate in ``discovered``.  Already-seen names are
    skipped (DAG-join semantics — a module imported along multiple
    paths is only parsed once).

    The synthetic ``host`` module is materialised without touching
    the file system.

    ``_depth`` tracks the current recursion depth so we can raise a
    clean :class:`TwigCompileError` before hitting Python's recursion
    limit on pathologically deep (but non-cyclic) import chains.
    """
    if _depth > _MAX_IMPORT_DEPTH:
        raise TwigCompileError(
            f"import chain exceeds maximum depth ({_MAX_IMPORT_DEPTH}) "
            f"while resolving module {name!r}"
        )

    if name in discovered:
        return  # already resolved on a previous branch

    if name == HOST_MODULE_NAME:
        discovered[name] = _synthetic_host_module()
        return

    path = _find_module_file(name, search_paths)
    program = _load_and_validate(name, path)
    assert program.module is not None  # _load_and_validate enforces

    discovered[name] = ResolvedModule(
        name=name, program=program, source_path=path
    )

    # Recurse into imports AFTER inserting ourselves so that any
    # future visits to ``name`` (via a diamond import) short-circuit
    # above without re-parsing the file.
    for imp in program.module.imports:
        _discover(imp, search_paths, discovered, _depth=_depth + 1)


# ── Pass 2: topological ordering ─────────────────────────────────────────


def _build_graph(
    discovered: dict[str, ResolvedModule],
) -> DirectedGraph[str]:
    """Build a directed import graph from the discovered modules.

    Edge direction: ``A → B`` for each ``(import B)`` in module A,
    meaning "A depends on B".  With this convention,
    ``topological_sort`` returns A before B (u-before-v for edge
    u → v).  We **reverse** the sort result so that B (the
    dependency) appears before A (the importer) in the final list —
    the deps-first order backends need.

    Self-loops (``(import a)`` in module ``a``) are permitted by
    the graph so that ``topological_sort`` catches them as cycles
    rather than being rejected at graph-construction time.
    """
    g: DirectedGraph[str] = DirectedGraph(allow_self_loops=True)
    for name, rm in discovered.items():
        g.add_node(name)
        if rm.program.module is not None:
            for imp in rm.program.module.imports:
                if imp in discovered:   # host is always present; others too
                    g.add_edge(name, imp)
    return g


def _topo_order(
    discovered: dict[str, ResolvedModule],
) -> list[str]:
    """Return module names in deps-first topological order.

    Raises :class:`TwigCompileError` with a concrete cycle path
    if the import graph contains a cycle.
    """
    graph = _build_graph(discovered)
    try:
        # topological_sort (Kahn's) returns importers before
        # dependencies (u before v for edge u → v).  Reverse to
        # put dependencies first.
        return list(reversed(topological_sort(graph)))
    except ValueError:
        # A cycle exists.  Find the smallest SCC with > 1 member
        # (or any SCC containing a self-loop) and reconstruct the
        # concrete path for the error message.
        sccs = strongly_connected_components(graph)
        cyclic = next(
            s for s in sccs
            if len(s) > 1 or any(graph.has_edge(n, n) for n in s)
        )
        path = _cycle_path(graph, cyclic)
        raise TwigCompileError(
            f"module import cycle detected: {' -> '.join(path)}"
        ) from None


def _cycle_path(
    graph: DirectedGraph[str],
    scc_nodes: frozenset[str],
) -> list[str]:
    """Reconstruct one concrete cycle path within a strongly
    connected component.

    Starts at the lexicographically smallest node in ``scc_nodes``
    (for deterministic output) and follows outgoing edges that stay
    within the SCC until the starting node is revisited.  Returns
    the path as ``[start, ..., start]`` — the same format the
    Phase 4b tests assert on.

    Example: SCC = {a, b, c} with edges a→b, b→c, c→a:
      start = "a"; path = ["a", "b", "c", "a"]
    """
    start = min(scc_nodes)
    path = [start]
    current = start
    visited_in_trace: set[str] = {start}

    while True:
        # Follow the first successor that stays in the SCC.
        nexts = graph.successors(current) & scc_nodes
        # For a self-loop the only successor is the node itself.
        if current in nexts:
            path.append(current)
            break
        # Pick deterministically; sorted() keeps output stable
        # across runs regardless of set ordering.
        nxt = next(iter(sorted(nexts - visited_in_trace)), None)
        if nxt is None:
            # Fallback: allow revisiting if we've exhausted fresh
            # nodes (shouldn't happen in a valid SCC, but be safe).
            nxt = next(iter(sorted(nexts)))
            path.append(nxt)
            break
        path.append(nxt)
        if nxt == start:
            break
        visited_in_trace.add(nxt)
        current = nxt

    return path


# ── Shared helpers (unchanged from Phase 4b) ─────────────────────────────


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

    Security note
    ~~~~~~~~~~~~~
    Each path component is validated before constructing the
    ``Path`` object so that crafted module names like
    ``foo/../../../etc/passwd`` cannot escape the search root.
    Empty components, ``.``, and ``..`` are all rejected here
    rather than relying on the caller to check containment.
    """
    parts = name.split("/")
    for part in parts:
        if not part or part in (".", ".."):
            raise TwigCompileError(
                f"invalid module name {name!r}: path components must not be "
                "empty, '.', or '..'"
            )
    return Path(*parts).with_suffix(".tw")


def _find_module_file(name: str, search_paths: Sequence[Path]) -> Path:
    """Locate the file backing ``name`` in the search paths.

    Search paths are tried in order; the first hit wins.  This
    matches Python's ``sys.path`` and Rust's ``--extern``
    semantics — earlier paths shadow later ones.

    Security note
    ~~~~~~~~~~~~~
    After constructing the candidate path we resolve it and
    confirm it is contained within the search-path directory.
    This is a second-line defence against path-traversal attacks
    (the first line is the component validation in
    ``_module_name_to_relpath``).  An attacker who somehow
    slipped past the component check (e.g. via a symlink in the
    search tree pointing outside) would still be caught here.
    """
    rel = _module_name_to_relpath(name)
    tried: list[str] = []
    for sp in search_paths:
        candidate = (sp / rel).resolve()
        sp_resolved = sp.resolve()
        tried.append(str(candidate))
        # Reject any path that escapes the search root.
        try:
            candidate.relative_to(sp_resolved)
        except ValueError as exc:
            raise TwigCompileError(
                f"module name {name!r} resolves to a path outside "
                f"the search root {sp_resolved}"
            ) from exc
        if candidate.is_file():
            return candidate
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
