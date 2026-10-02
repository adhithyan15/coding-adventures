# Changelog — @coding-adventures/forme-cli

## Unreleased

- Compose installed plugin `StageRef`s with manifest-bound persistent grants,
  the current platform's native sandbox factory, and lazy orchestrator-owned
  plugin sessions. Direct-only pipelines retain their process-free path, and
  a mixed direct/plugin product DAG runs in every supported platform gate.
  The product host supplies bounded storage plus grant-mediated network,
  environment, and filesystem backends while hiding host-owned plugin and
  cache state from every plugin grant. Project config selects trusted
  non-default runtime distributions; product tests exercise both a granted
  storage call and a real Python plugin.
- Enable Windows installation and discovery only after the native verifier
  proves the complete named tree excludes reparse points and untrusted write
  authority while retaining stable file identities. Product discovery binds
  the verified root identity across the host snapshot pass and propagates CLI
  cancellation into plugin discovery and authority loading.
- Added `forme install <PACKAGE>` for bounded local plugin directories with
  preflighted identity/trust classification, control-safe cancellable
  per-capability review against the configured runtime roots, user trust-store
  loading, manifest-bound grants, and atomic immutable discovery-root
  publication. Linked, special, oversized, changing, or package-authority
  inputs fail closed.
- Made the install capability-review assertion use the manifest contract's
  colon-free path representation on Windows as well as native paths on Unix.
- Let the GET-only GitHub Pages dry-run boundary use the fixed `GITHUB_TOKEN`
  when available, avoiding shared anonymous API limits while preserving
  tokenless local inspection.

## 0.5.0 — 2026-09-27

- Added `forme deploy` through CLI Builder with mutually exclusive directory,
  canonical bundle, and explicit-descriptor inline content stores; strict
  filesystem and GitHub Pages target configuration; target-aware dry-run;
  deterministic reporting; and explicit legacy ownership bootstrap.
  Retained bootstrap configuration becomes a no-op after ownership exists, so
  subsequent content changes do not require deleting migration evidence.
- Added `forme build --deploy-input DIR` to merge named in-memory `dist-tree`
  outputs into one collision-checked manifest-bound content store.
- Restricted v0 hosted credentials to `GITHUB_TOKEN`, used a GET-only anonymous
  GitHub boundary for dry-run, and rejected duplicate-key or oversized JSON,
  unsafe descriptors, linked content files, bundle metadata ambiguity, and
  manifest/store digest-set drift.
- Made deploy-input replacement use the filesystem adapter's atomic,
  caller-parent-bound publication path; bounded directory-store enumeration in
  one pass; closed bundle handles on every configuration failure; and rejected
  blocking inline descriptor types before reading.

- Canonicalized nested output summaries in `--report` so fresh stage values and
  values restored from canonical checkpoints produce byte-identical output
  sections when their artifacts are equal.
- Made repository bootstrap installs compatible with npm 10 by bypassing its
  crashing peer-dependency resolver for the local development package graph.
- Made the CLI test harness use platform-native fixture paths so the complete
  package suite exercises the same injected services on Unix and Windows.

## 0.4.0 — 2026-09-01

- Added per-instance input/output revision, external source-state revision, and
  prior-run `inputChanged` fields to the deterministic `--report` output.
- The report now exposes the project-persistent revision ledger comparisons
  needed to audit upcoming exact affected-set scheduling.

## 0.3.0 — 2026-09-01

- Wired `settings.cacheDir` to the filesystem cache backend so safe pure-stage
  outputs are reused across separate CLI processes.
- Refused project-root and outside-project cache paths before any cache access,
  matching the containment contract already enforced by `forme clean`.
- Added deterministic per-stage cache statistics to `--report` output for
  product-level incremental-build verification.

## 0.2.0 — 2026-08-31

- Added `forme watch` with declarative `--port` and `--debounce` options.
- Added recursive project watching with generated output, cache, dependency,
  and VCS trees excluded from rebuild notifications.
- Wired the orchestrator watch session to the in-memory Forme dev server so
  successful builds reload browsers and failed builds retain the last good site.
- Added CLI and project-watcher tests plus live landing/blog dogfood coverage.

## 0.1.0 — 2026-08-28

- Added `forme build` (`forme run` alias), `forme check`, and
  containment-checked `forme clean`.
- Added `--config`, automatic config discovery, and `--reproducible`.
- Added `--report` with deterministic output manifests and per-file hashes for
  post-build acceptance without duplicating artifact bytes.
- Added stable diagnostic formatting and documented exit codes.
- Declared the command surface in `forme.cli.json` and delegated routing,
  flag validation, fuzzy flag suggestions, help, and version output to the shared
  TypeScript `@coding-adventures/cli-builder` package.
- Added cooperative SIGINT cancellation with exit code 130.
- Added a portable Node/tsx npm launcher for TypeScript-first Forme packages.
- Added a centralized, deterministic local `file:` dependency bootstrap helper.
- Declared each dogfood site's complete local Forme dependency set to the
  monorepo scheduler so bootstrap installs cannot race dependency test runs.
- Kept recursive bootstrap in the standalone site scripts while making their
  scheduler recipes install only the site itself, so the sites remain safe to
  build in parallel without writing the same local dependency tree.
- Proved config loading and execution from a temporary project outside the
  repository tree.
