# Build Tool (C#)

An incremental, parallel monorepo build tool implemented in C# on .NET 9.

## What it does

This port follows the same core flow as the other build-tool implementations in
the repo:

1. Discover packages via recursive `BUILD` file walking
2. Resolve inter-package dependencies from language-specific manifest files
3. Use `git diff` for primary change detection
4. Fall back to content hashing plus `.build-cache.json` when git metadata is unavailable
5. Execute independent packages in parallel topological batches
6. Emit build plans and CI toolchain flags when requested
7. Validate caller-supplied orphan-crate and tracked-artifact snapshots without
   consulting Git or the filesystem
8. Parse exact `# needs-toolchain: NAME` metadata from the selected platform
   BUILD front and schedule canonical extra CI toolchains for affected packages

## Usage

```bash
dotnet run -- --help
dotnet run -- --dry-run
dotnet run -- --language dotnet --force
dotnet run -- --emit-plan --plan-file build-plan.json
```

## Design notes

- Uses only the .NET base class library: `System.Text.Json`, `System.Xml`,
  `System.Security.Cryptography`, and `System.Diagnostics`.
- Keeps the handwritten engine in one literate source file and the
  hash-verified generated Unicode data in one clearly marked generated source.
- Mirrors the current practical feature set of the TypeScript and Rust ports:
  shell `BUILD` files, manifest-based dependency resolution, git diff, cache,
  reporting, and plan emission.
- `Validator.ValidateTrackedArtifactSnapshot` is a pure security boundary for
  the shared build-tool conformance corpus. It rejects unsafe portable paths,
  redacts hostile path text, and detects Unicode compatibility aliases of an
  exact `node_modules` component for regular files, symlinks, and reparse
  entries alike. Length limits and path ordering use Unicode scalar values,
  while Windows reserved basenames use full Unicode uppercase mapping before
  comparison. Trailing slash and backslash separators are rejected as empty
  path components. The snapshot pins Unicode 17.0.0, and all four Unicode
  operations use source-embedded tables rather than host or operating-system
  globalization data.
- `Validator.ValidateOrphanCrateSnapshot` consumes the closed normalized
  directory, Cargo-manifest, BUILD-state, and exemption-ledger records from the
  shared corpus. It applies exact artifact-component exclusions, ancestor BUILD
  coverage, redacted invalid-exemption diagnostics, stale-ledger detection,
  and deterministic pending-debt accounting without walking a checkout or
  invoking Git, a process, the environment, or the network.
- `ToolchainDetection.EvaluateSnapshot` consumes all successful neutral
  toolchain-detection fixtures entirely in memory. It applies the canonical
  16-key registry, `c`/`cpp`, .NET, and WASM language normalization, exact
  declaration grammar, stable deduplication, platform-front precedence,
  affected-only and forced-full scheduling, deterministic unsupported-language
  diagnostics, CRLF-only carriage-return stripping, and the
  64-KiB/4,096-line/1-MiB input ceilings. A lone carriage return remains inert.
  Production
  discovery records declarations from the same BUILD front whose shell commands
  it selects; unselected platform declarations remain inert.
- Declares the mixed MIT and Unicode-3.0 licensing of the engine and derived
  tables and copies the full `UNICODE-LICENSE.txt` notice beside build,
  publish, and package outputs.
