# Build Tool (F#)

An F# entry point for the monorepo build tool on .NET 9.

## What it does

This program exposes the same incremental build engine as the new C# build
tool, but with an F# executable and test surface so the repo now has both .NET
language front doors represented. It also exposes
`validateTrackedArtifactSnapshot`, an F# facade for validating an inert
tracked-artifact snapshot supplied by a caller, and
`validateOrphanCrateSnapshot`, an F# facade for validating an inert
orphan-crate and exemption-ledger snapshot. The
`evaluateToolchainSnapshot` facade evaluates bounded, caller-supplied package
and BUILD-front records through a native F# symbol.

## Why share the engine?

The build tool touches almost every language in the monorepo. Keeping the core
dependency parsing, hashing, planning, and execution logic in one .NET engine
avoids immediate drift between the C# and F# variants while still giving the
repo an idiomatic F# program entry point.

The tracked-artifact facade deliberately reuses the reviewed C# data and result
types. Its F# test surface independently consumes all five language-neutral
conformance fixtures, including hostile-path redaction, Unicode-scalar length
and ordering boundaries, Unicode compatibility aliases, and Windows reserved
basenames. The shared cases also require trailing slash and backslash paths to
report redacted `EMPTY_SEGMENT` diagnostics and carry the exact Unicode 17.0.0
table version consumed by the generated shared .NET engine. The facade does
not enumerate Git, inspect the filesystem, launch a process, read the
environment, or access the network.

The orphan-crate facade likewise consumes all four shared validation fixtures
through an F# symbol. Those cases cover direct and ancestor BUILD ownership,
empty BUILD diagnostics, exact generated-directory exclusions, valid pending
debt, invalid exemption redaction, and stale exemption cleanup. The F# boundary
accepts only caller-supplied records; it does not discover directories, open a
manifest or BUILD file, consult Git, launch a process, read the environment, or
access the network.

The toolchain facade independently consumes all 11 language-neutral
toolchain-detection cases through the F# boundary. Those cases cover the full
16-key registry; C, C++, .NET, F#, and WASM language normalization; selected
Windows, Darwin, Linux, shared-Unix, and generic BUILD precedence; exact and
stably deduplicated declarations; affected-only, null-all, empty, forced, and
full scheduling; unsupported diagnostics; CRLF-only carriage-return stripping;
and per-file, per-line, and aggregate bounds. The adapter accepts only records
already supplied by its caller and returns the shared deterministic result. It
does not inspect a checkout, consult Git, open BUILD files, launch a process,
read the environment, access the network, or execute a declaration.

Build, publish, and package outputs declare the shared engine's mixed MIT and
Unicode-3.0 licensing and include the full `UNICODE-LICENSE.txt` notice.

## Usage

```bash
dotnet run -- --help
dotnet run -- --force --language dotnet
dotnet run -- --emit-plan --plan-file build-plan.json
```
