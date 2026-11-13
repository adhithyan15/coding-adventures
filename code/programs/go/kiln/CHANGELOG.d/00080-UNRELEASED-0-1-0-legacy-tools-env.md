## [0.1.0] - UNRELEASED

### Added

- Tool and environment identity for `kiln legacy-plan` (KILN03 sections 4 and 5,
  loop item 7c-3), in `internal/legacy/toolchains.go`, `env.go`, `hostroots.go`
  and `probe.go`:
  - `code/kiln/legacy/toolchains.json`: per language and extra toolchain, exact
    `pins` (from `mise.toml` and `code/specs/data/toolchain-pins.json`, else
    `image`), `probes` and PATH `roots`; strict loader (sorted keys, unknown
    fields, bad names and floating pins refused); `ToolsFor` builds the key's
    `tools` section (KILN02 kind 2) and `ConstructPATH` the KILN04 PATH.
  - `code/kiln/legacy/env.json`: strict allowlist with trailing-`*` prefixes;
    `EnvFor` fills the key's `env` section; secret-like names after expansion
    and names containing `:` are errors.
  - `code/kiln/legacy/host-roots.json`: strict schema and loader for KILN04
    section 5.3, seeded with the ephemeral list and image and tool roots.
  - `kiln legacy-plan --probe`: runs the toolchain probes (no shell, scrubbed
    environment, 10 s, 64 KiB, binaries inside the checkout refused) and gives
    the soft reason `probe-mismatch:<tool>`.
  - the hard reason `unknown-toolchain:<name>`; consumed entries
    `policy:toolchains.json` and `policy:env.json`.
  - plan fields `env` (names), `probes`, per-record `tools` and the `tools` and
    `env` component digests; `kiln explain` prints them. Env values are never
    printed.
  - `key.ToolsDigest` and `key.EnvDigest`.
- A test that checks every exact pin against `mise.toml` and
  `toolchain-pins.json`, so a toolchain bump that forgets Kiln fails.

### Changed

- The standing reason `m1-incomplete:tools-env` is retired. `m1-incomplete:trace`
  stays, so nothing is `cacheable`. Every static legacy key changes.
- KILN03 gains section 13, implementation decisions for 7c-3.
