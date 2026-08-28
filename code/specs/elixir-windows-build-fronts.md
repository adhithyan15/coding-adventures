# Elixir Windows BUILD-front contract

## Status

This document defines how Elixir package and program BUILD roots participate in
the repository's pull-request Windows build. The authoritative platform-file
precedence remains [12 — Build System](12-build-system.md), and result semantics
remain [Build Tool Conformance](build-tool-conformance.md).

The machine-readable contract is
[`fixtures/elixir-windows-build-front-v1/contract.json`](fixtures/elixir-windows-build-front-v1/contract.json).
The repository audit is `code/scripts/validate_elixir_windows_build_fronts.py`.

## Supported toolchain

Pull-request CI MUST include a pinned `windows-2025` build leg. When an affected
package needs the Elixir lane, that leg MUST install and verify the exact BEAM
toolchain recorded in the fixture and MUST run the ordinary affected-package
build step. Installing a toolchain without running the package BUILD front is
not execution evidence.

The setup action is part of the supply-chain boundary and therefore uses an
exact reviewed commit, not a moving tag. Elixir, OTP, and the action's strict
version mode are likewise fixture-owned. All workflow occurrences of the setup
action use the same reviewed commit so a helper job cannot silently drift to a
different installer implementation.

## Root discovery and front selection

The audit MUST inspect Git-visible direct children beneath both
`code/packages/elixir` and `code/programs/elixir`. A root is in scope when it
has a canonical `BUILD` file. A `BUILD_windows` without that companion is an
error.

On Windows, the selected front is:

1. `BUILD_windows`, when present;
2. otherwise the canonical `BUILD` file.

An absent `BUILD_windows` is therefore a native fallback, never an implicit
skip. Declarative Starlark BUILD files remain platform-neutral and are
evaluated by the build tool rather than passed to `cmd /C` as shell text.

Shell fronts selected on Windows MUST use CMD-compatible syntax. In
particular, POSIX environment prefixes such as `MIX_ENV=test mix compile`,
`/dev/null`, command substitution, shell tests, `cd -`, `export`, `source`,
`mkdir -p`, and brace groups are forbidden in a selected shell front. A
Windows environment assignment uses `set NAME=value&& command`; there is no
space after the value because CMD retains it.

## Reviewed unsupported fronts

A package may be unsupported on Windows only when the fixture lists its exact
root and stable diagnostic code. Its selected `BUILD_windows` MUST contain
exactly these two logical records:

```text
# build-tool: unsupported=STABLE_DIAGNOSTIC_CODE
echo BUILD_TOOL_UNSUPPORTED:STABLE_DIAGNOSTIC_CODE -- skipped
```

The comment is declarative and is removed by BUILD discovery before command
execution. The echo record is a closed build-tool protocol, not a human-message
heuristic: the executor recognizes the whole command, executes no shell, and
returns `unsupported` with the exact code. It MUST NOT return `built`, enter the
success cache, or run a second command. A dependent without its own reviewed
unsupported front returns `dep-unsupported` with
`DEPENDENCY_UNSUPPORTED`; it does not execute after an unavailable
prerequisite.

The v1 exception set contains only:

- NIF packages whose native Rust library cannot link to the Windows ERTS DLL
  without an import-library integration; and
- the Metal-native package, whose backend exists only on macOS.

Pure-BEAM ciphers are not exceptions. Once CI provisions BEAM on Windows they
must use their canonical Mix fronts and provide native Windows evidence.

## Fail-closed audit

The audit treats BUILD files and the fixture as bounded UTF-8 data. It invokes
Git without a shell, rejects symlink or path escape companions, rejects JSON
duplicate keys, and never executes package commands. It fails when:

- the pinned runner, setup action, versions, setup/verification guards, or
  affected-build condition drifts;
- a selected shell front contains POSIX-only syntax;
- an unsupported command is unregistered, malformed, or paired with a
  different code;
- a registered exception disappears or stops selecting its exact
  `BUILD_windows`; or
- a Windows override exists without a canonical root.

The JSON report is deterministic and records every root's selected front,
classification, and stable unsupported code. `unsupported` and
`dep-unsupported` are machine statuses and never contribute to the built
count.

## Validation

Repository validation runs the audit's unit suite and the live audit in the
parity-metadata CI gate. A delivery also runs:

1. the Go build-tool executor and reporter suites;
2. the exact pure-BEAM Windows fronts, including a dependency-bearing package;
3. an exact unsupported front through the real Windows build tool, proving the
   report says `UNSUPPORTED` rather than `BUILT`;
4. the Windows platform plan and affected closure; and
5. diff, formatting, lint, coverage, dependency, and security checks required
   by the repository workflow.
