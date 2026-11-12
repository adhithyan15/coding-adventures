# BUILD-002 — Every language is supported natively on Linux, macOS and Windows

**Status:** roadmap, in progress. **Owner decision (2026-11-11):** "support all
languages natively on all the OSes." A package is supported on an OS when its
real build and tests run there and pass. An `echo ... skipping` recipe is a gap
to close, not a form of support.

## Why a spec

Windows support had drifted into a long list of silent skips. Each skip was
reasonable when written ("not enabled in CI yet") and none had an owner. Fixes
that were made shipped blind, because a Windows recipe could merge without ever
running on Windows. This document fixes the order of work so each step can be
proven before the next relies on it.

## Inventory (origin/main, 2026-11-11)

3,842 `BUILD_windows` files. 336 are echo-only skips; Linux has none.

| Language | Skips | Reason recorded in the recipe |
|---|---|---|
| haskell | 158 | "haskell support not enabled in windows CI yet". The toolchain is already installed on Windows by `ci.yml`. |
| perl | 114 | 100 "Perl testing is not supported on Windows" (no Windows Perl toolchain in `ci.yml`); the rest are native extensions |
| native bridges (lua, dart, elixir, go, ruby, typescript, python, csharp, fsharp, cpp, rust) | about 40 | import library or loader not configured for CI; `ELIXIR_WINDOWS_NIF_*`; P/Invoke needs cross-compile |
| rust | 14 | `tcp-reactor` needs Unix fds; `unix-tools`; Metal |
| swift, starlark, other | 8 | "swift support not enabled in windows CI yet"; prerequisite chains |

macOS has one `BUILD_mac*` file. Its gaps are failures, not skips: shards
4, 9, 12 and 14 of the first cross-OS full build.

## Principles

1. **Prove before you merge.** A Windows recipe change runs on its own pull
   request (the `build-windows-recipes` gate, see `ci-gate-registry.md`).
   Nothing below may be rolled out blind.
2. **Batches a runner can prove.** One Windows runner builds the Windows
   closure of the changed recipes, so a change spanning dozens of packages
   builds all of them and their dependents. Keep batches small enough for the
   step's 300 minute ceiling.
3. **Make the work cheaper rather than skip it.** A slow suite
   (`typescript/human-language-data`: 66 minutes on macOS against 24 on Linux)
   is fixed by shrinking its fixtures, not by opting a platform out.
4. **Measure with the daily cross-OS full build.** Do not dispatch extra full
   builds; runner capacity is shared.
5. **Intrinsic limits are named, not hidden.** Anything bound to a platform API
   (the four Metal packages) is listed with its reason and an owner decision.

## Workstreams

| Id | Work | Depends on |
|---|---|---|
| W0 | Windows recipe changes prove themselves on the PR (`build-windows-recipes`) | none (this change) |
| W1 | Haskell on Windows: real `BUILD_windows` for the 158 packages, proving a leaf, a mid-stack package and `conduit` (network/hsc2hs linking) first | W0 |
| W2 | Perl on Windows: install Strawberry Perl and `Test2::V0` when Perl is needed, then real recipes for 100 pure-Perl packages; finish the native extensions (`perl-bridge` shim compile) | W0 |
| W3 | Native bridges: one language at a time (node import library, Lua `lua54.lib`, Dart FFI, Elixir NIF link, Go cgo, Ruby MinGW vs MSVC ABI, .NET P/Invoke) | W0 |
| W4 | Remaining skips: Swift, Starlark, `tcp-reactor` (IOCP path), `unix-tools`, prerequisite-chain skips; decide Metal | W1-W3 |
| W5 | Fix what is currently red from the first full build (c/cpp runtime, Ruby natives, OCaml `ppxlib`, macOS shards 4/9/12/14) | W0 |
| W6 | Guardrail: an echo-only recipe must appear in an allowlist with a reason and an owner, and the allowlist may only shrink; same for macOS | W1-W4 |

### W2 recipe contract (pure-Perl packages)

The Windows runner image carries two Perls and a bare `perl` resolves to the
wrong one: Git for Windows' MSYS Perl (POSIX `PERL5LIB`, no `cpanm`, no
`Test2::V0`) comes before Strawberry Perl (`C:\Strawberry`). Evidence from the
first full build: the packages whose `BUILD_windows` passed sibling libraries as
`-I..\dep\lib` to the PATH `perl` failed with `Can't locate ...pm in @INC`
(`perl/tree-set`, `perl/bloom-filter`), while `x509-extension`, which selects
`C:\Strawberry\perl\bin\perl.exe` explicitly, passed.

So every pure-Perl package uses one shared helper,
`code/scripts/run-perl-tests-windows.ps1`, invoked as

```
REM prereqs: ../grammar-tools ../algol-lexer
powershell -NoProfile -ExecutionPolicy Bypass -File ..\..\..\scripts\run-perl-tests-windows.ps1 -Deps grammar-tools,algol-lexer
```

The helper picks Strawberry Perl (falling back to the PATH `perl`), puts its bin
directories first on `PATH` so `prove`/`cpanm` follow, installs `Test2::V0` only
if `perl -MTest2::V0` fails, resolves each `-Deps` name to the absolute
`code\packages\perl\<name>\lib` (failing loudly if it is missing, so a renamed
sibling cannot silently drop out of `@INC`), and runs `prove -I... -l -v t`.
The `REM prereqs:` line is load-bearing: the build-tool validator requires
Perl `BUILD*` files to mention every transitive sibling, and the `-Deps` value
alone does not look like a `../name` reference. The `-Deps` list is the same
set the Unix `BUILD` puts on `PERL5LIB`/`-I`.

Proof (PR #19234, Windows leg): all 13 batch-1 packages built with real durations
(1.5-11.9 s; `x25519` 403.5 s because its big-integer tests are slow on
Windows), including the four that failed before (`tree-set`, `bloom-filter`,
`hash-map`, `in-memory-data-store-engine`). The same run showed the MSYS-perl
hazard directly: `in-memory-data-store`'s older recipe ran Strawberry's `prove`
script under MSYS perl and died with `Can't locate TAP/Harness/Env.pm`. Any
dependent of a converted package that still lacks a working recipe fails the
Windows leg (the `barcode-2d` dependents `aztec-code`, `data-matrix`,
`micro-qr`, `pdf417`, `qr-code` had no `BUILD_windows`), so convert a package's
dependents in the same PR.

Batch 2 converts every remaining mechanical package (about 210, including the
older hand-written recipes that ran a bare `perl`/`prove`, so there is one
recipe shape) and checks mechanically that each `-Deps` list and `REM prereqs:`
line equals the set of `../<name>` siblings in the package's Unix `BUILD`.

`barcode-layout-1d` keeps its echo-skip for now: its package tree is pinned by the
lane-evidence contract in `code/specs/fixtures/barcode-layout-1d-v1/targets.json`
(`package_tree`, `verified_revision`, and the owner item's
`implementation_revision` in `.claude/package-parity-loop-state.json`), so any
change to it must re-run the lane's tests and update that evidence in the same
PR. Its Windows run also showed a real portability bug (`File::Temp::tempfile()`
dies with "Permission denied" in the runner's `%TEMP%`; `tempdir(CLEANUP => 1)`
works), so it gets its own PR that fixes the test, converts the recipe and
attests the evidence after the lane's tests pass.

Still hand-written or skipped (tracked in W2/W4): `tcp-client` and
`process-manager` (sockets, fork and signals need a Windows design), `barcode-1d`
(its Unix `BUILD` stages the macOS-only Metal and PNG native extensions),
`aes-modes` (`Makefile.PL`/`make test`), `der-asn1`, `der-tlv` and
`x509-extension` (explicit-Strawberry compile checks that already pass), and the
native extensions `conduit`, `irc-server-native`, `gf256_native`,
`polynomial_native`, `paint_codec_png_native` and `paint_vm_metal_native`
(Perl import library, MSVC/MinGW ABI choice, Metal is macOS-only).

## Done

Every package either passes its real build and tests on all three OSes, or is
on the W6 allowlist with a named platform limit. The daily cross-OS full build
is green, and the allowlist contains only intrinsic limits.
