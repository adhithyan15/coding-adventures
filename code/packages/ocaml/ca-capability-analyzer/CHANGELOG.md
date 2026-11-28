# Changelog

All notable changes to this package are recorded here.

## Unreleased

### Added

- Formatter-clean analyzer sources and package-only tests for recursive
  bindings and excluded build directories. The isolated package now meets its
  95% Bisect-PPX source-coverage gate without relying on a sibling package.
- A digest-pinned exception for the single reviewed microbenchmark monotonic
  clock shim, accepting only the plain or exact Bisect Dune context and three
  pinned OCaml bindings. Digestif SHA-256 pins reject source and Dune drift;
  the exact manifest permissions and native-input allowlist fail closed.
  Generic external dual opt-in remains unchanged.
- A source snapshot in the analyzer test fixtures lets standalone package tests
  exercise the same profile even when the sibling microbenchmark package is
  not present in the consumer checkout.
- Regression cases cover Dune scanner token boundaries, the reviewed native
  profile's directory-depth ceiling, and acceptance of exactly 10,000 source
  files followed by rejection of the next file. Coverage results are reported
  as Bisect-PPX source points, not line or branch coverage.

- D21's nondeterminism categories (VER00 §6.3). Manifests accept the 25-pair
  taxonomy, including `rand:read`, `thread:spawn`, `thread:message`,
  `ui:read`, `ui:write` and `ui:event`, and still reject every cross-pair.
- `Random.self_init` and `Random.State.make_self_init` are detected as
  `rand:read:*`, and `Thread.create` and `Domain.spawn` as `thread:spawn:*`,
  through aliases, opens and first-class references like every other rule.
  Default-state draws such as `Random.int`, and generators seeded by the
  caller (`Random.full_init`, `Random.State.make`), are deterministic in
  OCaml 5 and are not reported.
- Four shared behaviour fixture cases cover the new rules and the seeded,
  pure case.

## 0.1.0 — 2026-09-07

### Added

- Compiler-libs parsing for implementation and interface files.
- Deterministic recursive package discovery with symlink rejection.
- Capability rules for filesystem, network, process, environment, FFI, time,
  standard input, and standard output access.
- Module-alias, supported-open, and lexical-shadowing resolution.
- Hard bans for `Obj`, unsafe Marshal operations, and closure serialization.
- Dual exception-and-capability opt-in for `external` and Dynlink boundaries.
- Strict schema-v1 manifest parsing with the closed Spec 13 taxonomy.
- Stable `CAP001` and `CAP002` findings, CLI exit codes, library API, shared
  behavior fixtures, package integration tests, and measured coverage gates.
- Fail-closed sensitive first-class references and constrained includes,
  package-bound regular manifests, unsupported generated-source rejection, and
  fixed file-count, depth, per-file, manifest, and aggregate byte ceilings.
- A self-contained package fixture mirror, numeric 95% source-coverage gate,
  and executable Windows BUILD syntax.
- Coverage and subprocess tests for the public CLI entrypoint, strict
  `$schema` typing, wrapped-sensitive-module rejection, and fail-closed Dune
  include, workspace, copy, generator, and preprocessor detection.
- Exact package-local `bisect_ppx` coverage support across reviewed OCaml
  packages while root-level and alternate instrumentation remain fail-closed.
