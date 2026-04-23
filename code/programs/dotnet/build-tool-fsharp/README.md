# Build Tool (F#)

An F# entry point for the monorepo build tool on .NET 9.

## What it does

This program exposes the same incremental build engine as the new C# build
tool, but with an F# executable and test surface so the repo now has both .NET
language front doors represented.

## Why share the engine?

The build tool touches almost every language in the monorepo. Keeping the core
dependency parsing, hashing, planning, and execution logic in one .NET engine
avoids immediate drift between the C# and F# variants while still giving the
repo an idiomatic F# program entry point.

## Usage

```bash
dotnet run -- --help
dotnet run -- --force --language dotnet
dotnet run -- --emit-plan --plan-file build-plan.json
```
