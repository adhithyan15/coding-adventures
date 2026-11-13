## 2026-11-01 — Cargo build and target-specific dependencies are graph edges

- `parseRustDeps` (`internal/resolver/resolver.go`) now reads path
  dependencies from `[dependencies]`, `[build-dependencies]`,
  `[build_dependencies]`, and the same three tables under
  `[target.<cfg>.…]` for any cfg (quoted or a bare triple), as a union
  over all targets. Before, only the top-level `[dependencies]` table was
  read, so a change to a build or target-specific dependency rebuilt and
  re-tested none of its dependents. On the real repository the forced plan
  gains 22 Rust edges and loses none, and the graph stays acyclic: among
  them paint-metal and paint-vm-direct2d into barcode-1d/-2d, the sandbox
  crates into the chief-of-staff launcher, host and e2e crates, sha256 into
  chief-of-staff-spawn-isolation, the board-vm UART/serial crates into
  board-vm-uno-r4-firmware, paint-vm-direct2d and paint-vm-gdi into
  paint-vm-direct2d-c, and objc-bridge into markdown-reader and
  venture-browser-macos.
- The `[dependencies.<name>]` / `[target.<cfg>.dependencies.<name>]`
  sub-table form with `path = "..."` (and optional `package = "..."`) is
  read; it was silently dropped before.
- Headers are split into dotted-key segments outside strings and each
  segment is compared by value, so `[target.'cfg(any(target_os = "linux",
  windows))'.dependencies]` and `["dependencies"]` are read. Every header,
  including malformed ones and `[[bin]]`, ends the current table.
- Inline entries: comments are stripped outside strings first (a trailing
  comment no longer hides the `path` field), fields are split at commas
  outside strings and outside nested `[...]`/`{...}`, the `path` and
  `package` keys must be exact and lowercase, and a value whose `features`
  array continues onto later lines is joined until its brackets close
  (TOML 1.0; `vault-pm-cli-drill` uses it).
- `[dev-dependencies]` in every form, `[patch]`, `[replace]`, `[workspace]`,
  registry-only entries, features, and comments still contribute no edge.
  Dev-dependencies stay in the separate test-trigger relation; the
  `test_triggers.go` comments now say the graph records build and target
  edges and explain why the trigger walk still follows package roots
  (nested and orphan crates, native-extension manifests, path vs. name).
- The Go tool now reads the shared `resolution/rust-field-aware` case
  (`TestRustResolutionConformanceFixture`), and the resolution-fixture CI
  routing in `main.go` and its selection test sends that case to Go as well
  as Haskell. New table-driven `TestCargoLinkDependencyNames` pins each line
  rule of the contract.
- The Lua, Perl, Python, Ruby, Rust, Swift and TypeScript fronts now implement
  the same contract and read the same case, so the routing (and
  `ci-gate-registry.md`) sends `rust-field-aware` to G, H, L, P, Y, B, R, S
  and T.
