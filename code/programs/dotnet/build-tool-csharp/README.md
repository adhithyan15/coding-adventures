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
7. Validate caller-supplied tracked-artifact snapshots without consulting Git
   or the filesystem

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
- Keeps the implementation in a single literate source file so the control
  flow is easy to trace while reading.
- Mirrors the current practical feature set of the TypeScript and Rust ports:
  shell `BUILD` files, manifest-based dependency resolution, git diff, cache,
  reporting, and plan emission.
- `Validator.ValidateTrackedArtifactSnapshot` is a pure security boundary for
  the shared build-tool conformance corpus. It rejects unsafe portable paths,
  redacts hostile path text, and detects Unicode compatibility aliases of an
  exact `node_modules` component for regular files, symlinks, and reparse
  entries alike. Length limits and path ordering use Unicode scalar values,
  while Windows reserved basenames use full Unicode uppercase mapping before
  comparison.
