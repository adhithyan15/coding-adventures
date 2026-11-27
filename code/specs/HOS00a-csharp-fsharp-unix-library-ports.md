# HOS00a — Reusable C# and F# First Edition Unix libraries

## Purpose

Implement the HOS-003a C# and F# lanes as independently reusable libraries for
the existing First Edition Unix teaching model. The C# and F# packages expose
in-memory file-system, descriptor-table, kernel, and shell operations. Their
public libraries do not read files, write output, start processes, or accept a
fixture path.

This is a deterministic teaching model over an in-memory ASCII demo image. It
does not execute guest instructions or claim historical binary, shell, or
syscall compatibility.

## Shared behavior

Both libraries implement the existing HOS00 component contract: resolve paths
and return the specified errno-style errors; keep descriptor offsets isolated
per open; update offsets by bytes returned; free only the closed descriptor;
reject negative descriptor-table capacity; and translate the supported
teaching commands through the in-memory kernel.
The same nine cases in
`code/specs/fixtures/historical-unix-v1/file-read.json` are the parity contract.

The reusable package trees contain library code only. Required-capability
profiles declare an empty capability list because the runtime components use
only in-memory values. Component checks and fixture replay belong to the
BUILD-only external adapters under
`code/specs/fixtures/historical-unix-v1/test-adapters/{csharp,fsharp}`. The
external runner reads the canonical fixture, invokes the adapter for each
command, and compares output and ordered events. It also runs the component
regression checks. The adapter is build tooling, not a package executable or
public command-line interface.

## Build and fixture boundary

CI uses .NET SDK 9.0.319 and these projects target `net9.0`. Each package has a
Unix `BUILD` and a Windows `BUILD_windows` front. Each front calls one external
Python driver command so the temporary adapter and its build artifacts live
for the complete replay. The driver sends all .NET artifacts, SDK user state,
NuGet package state, and temporary data to a temporary directory; it does not
leave `bin`, `obj`, adapter output, or fixture files in a package tree.

The Go build-tool external-input registry selects each language package only
for the canonical fixture, shared replay helper, common .NET runner, and that
language's adapter source and project. Its fallback package hash contains the
same exact file list. Similar names, fixture-directory prefixes, READMEs, and
other consumers do not broaden selection. A deletion or rename of any exact
input still selects the package by its old path.

## Evidence

Run the C# and F# BUILD commands through the repository executor. Each must
compile the external adapter against its corresponding library, execute the
component checks, and replay all nine shared cases. The build-tool tests verify
exact C# and F# fixture selection, language filters, rename/deletion behavior,
and equality between selector inputs and package hash inputs. Windows CI must
select `BUILD_windows` and run the same contract with the pinned SDK.
