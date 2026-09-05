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
9. Hash package sources through the complete checked language registry plus
   exact repository-boundary inputs, using one stable tracked-index snapshot
   before and after the package batch

## Usage

```bash
dotnet run -- --help
dotnet run -- --dry-run
dotnet run -- --language dotnet --force
dotnet run -- --emit-plan --plan-file build-plan.json
```

## Design notes

- Uses no external managed dependencies. Registry, hashing, XML, and process
  work use the .NET base class library; secure file traversal calls the native
  kernel32 or libc APIs directly.
- `SourceInputRegistries.Generated.cs` is the immutable production projection
  of both neutral source-input registries. Regenerate it from the repository
  root with
  `powershell -File code/programs/dotnet/build-tool-csharp/tools/generate-source-input-registries.ps1`;
  neither executable locates or decodes fixture JSON at runtime.
- Source selection applies exact generated-component pruning, all seven
  package-local selector roles, direct Starlark `srcs` globs, and the reviewed
  repository-boundary registry. The same boundary projection reverse-indexes
  a changed shared input to every exact consumer.
- Package hashes sort canonical repository-relative UTF-8 paths and frame each
  path and exact raw file body with unsigned 64-bit big-endian lengths before
  SHA-256. Dependency and combined-digest framing remain a separate contract.
- Live hashing incrementally bounds 100,000 candidates, 50,000 selected files,
  50,000,000 declared-glob match-work units, 64 MiB per file, and 1 GiB per
  package. Native no-follow opens reject linked,
  reparse, non-regular, or multiply linked inputs. Each package retains the
  repository root while reopening and revalidating directory and file identity
  with constant descriptor use; complete Git
  mode/OID/stage/path evidence is checked before and after batch hashing.
  Package-hash failures expose only a stable quoted package identity; tracked
  snapshot failures expose only a stable `SOURCE_HASH_*` code.
- Keeps the handwritten engine and secure source reader in focused literate
  source files, with hash-verified generated registries and Unicode data in
  clearly marked generated sources.
- Mirrors the current practical feature set of the TypeScript and Rust ports:
  shell `BUILD` files, manifest-based dependency resolution, git diff, cache,
  reporting, and plan emission.
- Discovery consumes the complete language-neutral registry. The exact bucket
  immediately below `packages` or `programs` is the sole language
  discriminator, BUILD roots outside those containers do not become packages,
  and exact case-sensitive generated components including `_build` and
  `dist-newstyle` are pruned while near names and case variants remain source.
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
- Neutral snapshots validate every selected language even in forced-full mode.
  Production forced-full orchestration already provisions all 16 canonical
  toolchains, so it does not attempt to classify repository-only fixture and
  special buckets; affected production packages remain strict, with the
  repository's Starlark build-language bucket normalized to its Go bootstrap.
- Declares the mixed MIT and Unicode-3.0 licensing of the engine and derived
  tables and copies the full `UNICODE-LICENSE.txt` notice beside build,
  publish, and package outputs.
