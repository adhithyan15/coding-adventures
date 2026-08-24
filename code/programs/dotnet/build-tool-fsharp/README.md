# Build Tool (F#)

An F# entry point for the monorepo build tool on .NET 9.

## What it does

This program exposes the same incremental build engine as the new C# build
tool, but with an F# executable and test surface so the repo now has both .NET
language front doors represented. It also exposes
`validateTrackedArtifactSnapshot`, an F# facade for validating an inert
tracked-artifact snapshot supplied by a caller.

## Why share the engine?

The build tool touches almost every language in the monorepo. Keeping the core
dependency parsing, hashing, planning, and execution logic in one .NET engine
avoids immediate drift between the C# and F# variants while still giving the
repo an idiomatic F# program entry point.

The tracked-artifact facade deliberately reuses the reviewed C# data and result
types. Its F# test surface independently consumes all five language-neutral
conformance fixtures, including hostile-path redaction, Unicode-scalar length
and ordering boundaries, Unicode compatibility aliases, and Windows reserved
basenames. The facade does not enumerate Git, inspect the filesystem, launch a
process, read the environment, or access the network.

## Usage

```bash
dotnet run -- --help
dotnet run -- --force --language dotnet
dotnet run -- --emit-plan --plan-file build-plan.json
```
