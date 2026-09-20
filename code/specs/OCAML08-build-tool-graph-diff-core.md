# OCAML08: Process-Free Build-Tool Graph and Diff-Selection Core

## Status and scope

This specification introduces the first native OCaml build-tool domain at
`code/programs/ocaml/build-tool`. It is a bounded library tranche, not a build
tool front door. The implementation manifest remains `missing` for OCaml until
the later native CLI, repository adapters, planner, cache, reporter, and
executor owners are complete.

The production library implements only the language-neutral `graph` and
`diff_selection` domains from `build-tool-conformance.md`. It accepts already
materialized immutable values and has no filesystem, Git, process,
environment, network, clock, randomness, credential, stdin, stdout, native
loading, or execution authority. JSON decoding and fixture discovery belong to
the package-local test adapter and are never linked into the production
library.

## Native surface

The public module exposes closed records and variants for graph edges, package
specifications, repository-boundary rules, graph and diff inputs, successful
results, and stable errors. The two operations are:

```ocaml
val evaluate_graph : graph_input -> (graph_result, error) result
val evaluate_diff_selection :
  diff_selection_input -> (diff_selection_result, error) result
```

Invalid structure returns a stable typed error rather than raising across the
public boundary. Graph cycles return `GRAPH_CYCLE` with no partial edges or
levels. Diff failures return `DIFF_UNKNOWN_PATH`,
`DIFF_MATCH_LIMIT_EXCEEDED`, or the applicable structural code with no partial
changed, affected, or prerequisite sets.

Every edge is `[prerequisite, dependent]`. Successful graph output sorts edges
by numeric Unicode scalar sequence and emits deterministic prerequisite-first
levels. A diff starts from forced, package-local, and exact repository-boundary
seeds; affected closure walks prerequisite-to-dependent edges, while
prerequisite closure walks the reverse graph and excludes the affected set.

## Portable text, path, glob, and digest behavior

Inputs reject malformed UTF-8 before Unicode processing. Path identity uses
Unicode 17 NFC followed by full default case folding. Windows reserved-name
checks use locale-independent full uppercase. Canonical output ordering compares
decoded Unicode scalar values rather than host bytes, UTF-16 code units, or
locale collation.

Strict globs operate on `/`-separated scalar sequences with memoized `*`, `**`,
and portable character classes. Classes support `[!a]`, `[a-c]`, leading
`]`, and leading or trailing `-`; an unclosed `[` is literal and `^` has no
negation meaning. Ambiguous set operators and descending ranges are invalid.
Exact BUILD-front basenames select at zero match-work cost.

After structural, boundary, package, path, edge, and glob validation, the diff
operation performs the complete Cartesian Unicode-scalar match-work preflight.
Every strict-glob/path pair costs `(pattern scalars + 1) * (relative-path
scalars + 1)`. Exactly 50,000,000 units proceeds; overflow or a larger value
returns `DIFF_MATCH_LIMIT_EXCEEDED` before any matcher call or selection.

When a boundary digest is supplied, the core reconstructs the contract's
canonical JSON with the domain separator
`coding-adventures/build-tool-repository-source-input-boundary/v1\0`, prefixes
the JSON with its unsigned 64-bit big-endian byte length, computes SHA-256, and
requires an exact lowercase digest match before selection. Boundary matching is
exact and case-sensitive; it never widens to prefixes, basenames, ancestors, or
globs. The caller supplies the already validated inert boundary required by the
language-neutral contract; before hashing or projecting it, the OCaml core also
enforces independent schema-shape, string-size, 8,192-scope, and
32,768-authorization guards so a misbehaving caller cannot create unbounded
work.

## Dependencies and capabilities

Production dependencies are pinned to pure portable implementations:

- local `coding-adventures-graph` 0.1.0;
- local `coding-adventures-directed-graph` 0.1.0;
- `uunf` 17.0.0 and `uucp` 17.0.0 for pinned Unicode behavior; and
- `digestif` 1.3.1 through the pure `digestif.ocaml` library for SHA-256.

The test adapter alone uses `yojson` 3.0.0 to decode the checked shared
fixtures. Its declared filesystem read/list capabilities are limited to test
fixture discovery; production remains empty-capability. No dependency may add
network, process, environment, Git, credential, or native foreign-function
authority.

## Conformance and release gates

The native Alcotest suite dynamically discovers every shared `graph` and
`diff_selection` fixture and asserts the exact roster of eight graph plus
eleven diff cases. It evaluates each case through the production module and
checks exact results, error codes, and empty failure output. Focused tests also
cover malformed UTF-8, Unicode identity collisions, portable path and glob
rejection, canonical boundary escaping/digests, work-ceiling precedence, and
the no-partial-result rule.

The package provides literal `BUILD` and `BUILD_windows` fronts, warning-clean
format and install builds, and at least 95 percent measured production-file
coverage. Both fronts install local packages in leaf-to-root order before the
program and pin every external tool version. The OCaml capability analyzer
must independently accept the completed root. Shared conformance schema and
runner suites, the Go dry-run build-file oracle, OCaml toolchain locks, package
parity, lessons, diff hygiene, and dependency/license review are publication
gates.

This tranche does not modify the already-governed OCaml `graph` or
`directed-graph` package trees and does not alter the OCaml denominator.
