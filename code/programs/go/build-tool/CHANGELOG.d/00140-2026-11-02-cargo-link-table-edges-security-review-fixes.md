## 2026-11-02 — Cargo link-table edges: security-review fixes

- **Workspace inheritance is refused in link tables (review round 2).**
  `a = { workspace = true }` in `[dependencies]`, `[build-dependencies]` or a
  target form, or a `workspace` key in a `[D.a]` sub-table (quoted or not),
  passed the link-table screen yet drew no edge: Cargo takes the path from the
  workspace root's `[workspace.dependencies]`, which the edge reader never
  opens. A later change to that dependency would then re-run none of this
  crate's tests. The strict reader now refuses it, as it already did in
  dev-dependency tables. No repository manifest uses it.
- **Manifest-derived keys are quoted** (`%q`) in the new link-screen
  messages, so a quoted key cannot carry control bytes into a CI log.
- **`readCargoManifestBytes` checks the open handle.** It `Stat`s the opened
  file and requires `os.SameFile` with the `Lstat` it size-checked, so a
  symlink swapped in between is refused instead of followed (portable, unlike
  `O_NOFOLLOW`, which Go does not spell for Windows).
- **Linear continuation join.** An inline Cargo entry continuing onto later
  lines was joined by appending each line to the whole value and rescanning
  the value's brackets per line — quadratic; a crafted 400 KB manifest took
  64 s. `cargoLinkDependencyNames` now carries a `tomlBracketScanner`
  (bracket depth, open-string delimiter, pending escape) from line to line,
  collects the pieces and joins them once. The result is identical to the
  rescan, including a string left open at the end of a line, which continues
  across the joining space.
- **Size guards on the edge reader.** `parseRustDeps` runs before the strict
  test-trigger reader, so it now carries its own limits: it reads the
  manifest through `readCargoManifestBytes` (a regular file of at most
  1 MiB) and refuses one entry continuing past 256 non-blank lines or 64 KiB.
  Both refusals are `*CargoManifestError` and fail the whole resolution;
  `parseRustDeps` and `cargoLinkDependencyNames` now return an error.
- **Quoted field keys.** `a = { "path" = "../a" }`, `'package' = "..."` and
  `"path" = "../a"` in a `[dependencies.<name>]` sub-table are the keys TOML
  and Cargo read, and are now edges (they were silently dropped). Keys stay
  case-sensitive (`"Path"` is not `path`) and escapes are not decoded, per the
  amended conformance contract.
- **Fail closed on unmodelled link-table shapes.** For every Rust and WASM
  package, the strict reader (`readCargoTriggerEntries`, `screenLinks`) now
  also screens `[dependencies]`, `[build-dependencies]`,
  `[build_dependencies]` and their `[target.<cfg>.…]` forms, refusing any
  write the lenient edge reader would drop or misread: dotted keys inside
  them (`a.path = ...`), at the root (`build-dependencies.a.path = ...`) or
  under `[target]`/`[target.<cfg>]`; inline tables spelling one
  (`build-dependencies = { ... }` under `[target.<cfg>]`); deeper headers and
  arrays of tables (`[dependencies.a.features]`, `[[dependencies]]`); entry
  values that are neither a string nor an inline table; a multi-line value in
  a sub-table; dotted or non-`key = value` inline fields; and any backslash.
  Every shape the contract models is still accepted. The forced plan of the
  real repository is unchanged (exit 0, identical edge list).
- **`DEPENDENCY_SELF_EDGE` instead of a panic.** A dependency that resolves
  to the package itself (a Cargo path entry naming its own directory alias or
  `[package]` name, for example) made `directed-graph.AddEdge` panic. It is
  now a `*resolver.DependencySelfEdgeError` with a repository-relative
  manifest, reported by the CLI as a stable diagnostic with exit status 2.
- Tests: 11 new rows in the one-shape Cargo table (57 in all), linear-time
  and entry-cap tests, manifest-guard tests, self-edge tests, and
  `test_triggers_link_screen_test.go` (24 refused shapes, the multi-line
  string backstop, 5 accepted groups, and the Rust/WASM-only scope).
