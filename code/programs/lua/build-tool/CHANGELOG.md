# Changelog

## Unreleased - BUILD-001 registry update

- Source-input registry and repository boundary updated for BUILD-001 (generated wasm artifacts are built, not committed): `code/packages/rust/engram-wasm/pkg/engram_engine.wasm` is no longer an exact package input of `rust-engram-wasm-build-inputs`, and `visicalc-html/vendor/spreadsheet-engine-wasm.js` is no longer a cross-package input of the Deno VisiCalc boundary. Registry digest is now `8f3d9ee3...`, boundary digest `fdf54a19...`.

## Unreleased

### Added

- A process-free bounded graph and diff-selection core now dynamically
  consumes all eight graph and twelve diff-selection neutral fixtures. It
  provides canonical levels and closures, stable all-or-error diagnostics,
  Unicode-scalar strict-glob matching with a 50,000,000-unit preflight, exact
  BUILD-front handling, and domain-separated repository-boundary reverse
  selection including shared-input multi-consumer fanout.
- Process-free dependency and combined SHA-256 digests plus inert cache-hit,
  miss, and corrupt-recovery decisions now consume all 11 neutral
  `hashing-cache-*` fixtures, including dependency-only changes and shared
  input consumers; native persistence and CLI activation remain separate.
- A process-free portable source selector and package-digest API consumes the
  complete checked 23-language, seven-role source-input registry from generated
  package data. The native Busted suite exercises all seven package-local
  neutral cases, SHA-256 v1 path/content framing, generated and link pruning,
  invalid glob rejection, path aliases, and raw-byte/rename sensitivity.
- Native discovery tests now project the shared language-registry fixture's
  OCaml package and exact Dune `_build` decoy, with direct case-variant and
  near-name coverage.
- A pure bounded toolchain-detection adapter now independently consumes all 11
  language-neutral snapshots, including exact extra-CI declarations,
  platform BUILD precedence, affected and forced scheduling, complete
  canonical flags, stable unsupported diagnostics, and shared resource
  ceilings without host or execution authority.
- A pure orphan-crate snapshot validator now independently consumes all four
  language-neutral fixtures, with artifact filtering, fixed ancestor BUILD
  precedence, redacted portable-path and ledger failures, Unicode full-fold
  duplicate identities, stale exemptions, active pending counts, and stable
  ASCII-canonical diagnostic ordering.
- A pure tracked-artifact snapshot validator now independently consumes the
  five language-neutral fixtures, including redacted portable-path failures,
  Unicode-scalar boundaries and ordering, inert link metadata, and exact,
  case, nested, and Unicode-compatible `node_modules` aliases.
- Generated source-embedded Unicode 17.0.0 normalization, full default-fold,
  and full-uppercase tables keep validation independent of host Lua tables;
  the generator verifies the emitted Lua module against every official vector.

### Fixed

- CLI parsing now rejects unknown options, unexpected positional arguments,
  and missing `--root` or `--language` values with status 2 before discovery.
  Stable argument diagnostics do not repeat raw, potentially path-bearing
  input.
- `DEPENDENCY_SELF_EDGE` diagnostics cut the manifest path at the **last**
  `code/packages/` or `code/programs/` pair (security review, round 2).
  Cutting at the first `/code/` printed the checkout's own directory for the
  common `~/code/<repo>` layout; the self-edge tests now place the checkout
  under `code/packages/checkout/` to pin it.
- Cargo link-table security fixes. Inline values continuing onto later lines
  are joined in linear time: `toml_bracket_scan` carries the bracket depth,
  the open string's delimiter and a pending escape from line to line (so a
  string left open at a line end still continues after the joining space),
  and the pieces are joined once with `table.concat`, instead of rescanning
  the growing value for every appended line, which was quadratic. Quoted
  field and sub-table keys (`a = { "path" = "../a" }`, `'package' = "..."`)
  are read as the bare key, compared verbatim and case-sensitively, per the
  amended contract. A Cargo path entry that resolves to the crate itself is
  now a `DependencySelfEdgeError` (`DEPENDENCY_SELF_EDGE`, repository-relative
  manifest, `Resolver.is_dependency_self_edge_error`; `build.lua` exits 2)
  instead of a silently drawn self-loop. `trim_spaces_and_tabs` is an index
  scan: its `gsub("[ \t]+$", "")` took 12.6 s on a 40 KB interior run of
  spaces. Tests: the 11 new one-shape cases (57, now with a row-count
  assert), a 20,000-line continuation, a 100 KB run of spaces, 100,000 blank
  lines before the `[package]` name, self-edges by alias, by name and from a
  target table, and a real CLI exit-2 test.
- Rust resolution now reads path dependencies from `[build-dependencies]`,
  `[build_dependencies]` and every `[target.<cfg>.…]` form of the three link
  tables, not just the top-level `[dependencies]`, and reads the
  `[dependencies.<name>]` sub-table form, so a change to a crate used only by
  a build script or on one target now rebuilds its dependents. This implements
  the Rust line procedure of `code/specs/build-tool-conformance.md`, ported
  from the Go reference: headers are split into dotted-key segments outside
  strings (quoted cfgs stay one segment, table names compare case-sensitively,
  `[[...]]` and malformed headers end the table); comments are stripped
  outside strings; inline values whose arrays continue onto later lines are
  joined; `path` and `package` field keys must be exact and hold one non-empty
  string; names are ASCII-lowercased and deduplicated. Dev-dependencies (every
  form), `[patch]`, `[replace]`, `[workspace]` and registry entries add no
  edge.
- The old reader counted any `[dependencies]` line mentioning `path`, so a
  registry entry with `features = ["path"]` drew an edge, and it looked up
  the entry key instead of a `package` rename. Both are fixed.
  `Resolver.cargo_link_dependency_names` is exported for the tests.
- Tests: the 46 one-shape cases of the Go reference
  `TestCargoLinkDependencyNames`, raw-byte BOM/CRLF, duplicate collapse, and
  the shared `resolution-rust-field-aware.json` fixture. The reader returns
  the same dependency names as the Go reader for all 1,650 tracked
  `Cargo.toml` files.

- Graph and diff selection runs about 7x faster on large path sets, with the
  same answers. `test_graph_diff.lua`'s full-scale case (4,096 roots × 500-byte
  paths) took 5.5 s CPU locally and 15.6–18.5 s on `windows-2025`, failing
  its 12 s bound on every PR that selects this package; it now takes 0.76 s.
  Measured linear before and after, so this is a constant-factor fix:
  - `root_less` (the root-identity sort comparator, ~40% of samples) finds
    the first differing byte by binary search over prefix equality instead
    of reading every shared byte in Lua. It stays byte-wise; Lua's string
    `<` would go through `strcoll` and follow the host locale.
  - ASCII fast paths for NFC (identity), case folding (`A`..`Z` → `a`..`z`)
    and full uppercase (`a`..`z` → `A`..`Z`), via explicit byte tables, not
    the locale-dependent `string.lower`/`upper`. Non-ASCII strings still go
    through the generated Unicode 17 module.
  - The per-scalar control/forbidden-character loop is one byte-class
    `find` (`[\0-\31` + the %-escaped forbidden set): in valid UTF-8 a
    control scalar is one byte below 32 and every forbidden character is
    ASCII. DEL (127) stays accepted.
  - The separator scan in `prefix_materialization_work` uses `find`.
  New tests pin each fast path's boundaries through the public API, and pass
  against the old implementation too.
- Package discovery without LuaFileSystem now uses `nul` for Windows directory
  probes as well as listings, so `dir /ad` results are not silently discarded
  by a Unix-only `/dev/null` redirection. Deterministic fallback tests cover
  Windows commands, sorting, missing entries, and fail-closed path quoting.
- Diff selection now charges the exact UTF-8 bytes of first-seen unique glob
  declarations against a 4 MiB ceiling before normalization, even when no
  paths changed. Package-root classification uses one reusable changed-path
  index and preflights all slash-prefix materialization against an explicit
  50,000,000-unit ceiling. Boundary digest diagnostics retain precedence over
  that request-dependent preflight, while relevant-input, exact-root,
  descendant-root, and exclusion indexes avoid package-by-rule scans and
  eager input-consumer expansion.
- Strict diff globs now treat the package root as a zero-component relative
  path, so `*` and `?` cannot select it while a whole `**` segment can.
- Glob character-class compilation and folded package-root validation are
  linear in their bounded inputs, and inert snapshots no longer turn package
  limits or malformed edges into generic input errors or default a missing
  `source_globs` field.
- Glob validation no longer retains compiled token graphs for declarations
  that never reach matching; compilation is lazy, one-pattern-at-a-time, and
  reused across a package's candidate paths. Folded-root validation now uses a
  compact separator-aware sorted list instead of per-segment trie tables, and
  ignored record fields no longer trigger unbounded pre-snapshot enumeration.
- Complete glob validation and scalar match-work factors are memoized by
  immutable pattern string across packages, while duplicate detection remains
  package-local and an empty changed-path set skips match preflight entirely.
- Package discovery now classifies the emerging `ocaml` bucket while continuing
  to prune only exact case-sensitive `_build` output; `_Build` and
  `_build-example` remain discoverable source paths.
- The emitted-Lua Unicode verifier now requires an explicit pinned Lua 5.4.7
  executable, ignores Lua initialization and module-path environment state,
  bounds retained child output, and terminates the isolated process tree after
  every exit. Windows starts the verifier suspended inside a kill-on-close Job
  Object; POSIX uses an isolated process group. Timeout, early-root-exit, and
  cleanup-error regressions cover descendant containment, and a Job Object
  setup failure now reaps the still-suspended verifier before propagating.
- Lua `.rockspec` dependency metadata now follows the shared strict UTF-8
  contract. Invalid bytes fail closed with the stable
  `METADATA_INVALID_UTF8` diagnostic and CLI exit code 2 without leaking host
  checkout paths.

## 0.1.0 — 2026-03-23

### Added
- Initial implementation of the Lua build tool.
- Package discovery via recursive BUILD file walk.
- Dependency resolution for all 7 supported languages:
  Python (pyproject.toml), Ruby (.gemspec), Go (go.mod),
  TypeScript (package.json), Rust (Cargo.toml), Elixir (mix.exs),
  Lua (.rockspec).
- Directed graph with Kahn's algorithm for topological sort.
- Sequential build execution with pass/fail tracking.
- Build report with summary statistics.
- Platform-specific BUILD file support (BUILD_mac, BUILD_linux, BUILD_windows).
- CLI with --root, --dry-run, --language, --force options.
