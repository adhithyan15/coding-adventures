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
| W4 | Remaining skips: Swift, Starlark, `tcp-reactor` (IOCP path: needs a decision, see "the Rust remainder"), the 44 Ruby `unix-tools` test files held back on Windows, prerequisite-chain skips; decide Metal | W1-W3 |
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
works). The Perl lane's tests can be run locally (`t/00-load.t`, `01-basic.t`,
`02-conformance.t` pass), so the blocker is the registry, not the tests: the Perl
entry shares its owner item (`barcode-layout-existing-lanes-v1-conformance`,
merged as #16343) with the C#, F#, Go, Python, Rust and TypeScript entries, and
`test_target_registry_lifecycle_and_repository_evidence` requires
`entry.verified_revision == item.implementation_revision` and
`entry.adoption_pr == item.pr_number`. Moving Perl's pin alone breaks that
equality, and moving the shared item re-attests six lanes nobody re-ran, so it
needs its own PR that registers a dedicated owner item (with the real PR number)
for the Perl lane, fixes the test with `tempdir`, converts the recipe and pins the
new tree. `barcode-1d` does not depend on that recipe (only on the sibling's
`lib\`), so it is converted separately.

### W2 progress: the remaining Perl recipes

- **Programs** (`programs/perl/cowsay`, `grammar-tools`, `parrot`) use the same
  helper. `-Deps` is always resolved against `code\packages\perl` (the helper
  finds it relative to its own location), so a program passes the package names
  the Unix `BUILD` reaches as `../../../packages/perl/<name>/lib`, and its
  `REM prereqs:` line uses those relative paths. `grammar-tools` and `parrot` put
  their sibling library on `@INC` with `use lib` inside the script instead of
  declaring it in `cpanfile`, so their `BUILD_windows` adds
  `# build-tool: deps=perl/grammar-tools` / `perl/repl`; that comment is both the
  validator's allow-list entry and a real graph edge for the Windows plan.
- **`process-manager`** is an in-memory process-table simulation (no OS `fork`,
  `kill` or signals), so it needed no porting. **`tcp-client`** talks to loopback
  sockets and forks a server child. Loopback TCP and `IO::Select` work on
  Strawberry Perl, and `fork()` is thread-emulated there, so the suite stays and
  its tests were made portable instead of skipped: a server child now ends when
  the client hangs up (no `sleep(10)` + `kill('TERM')`, which cannot interrupt a
  pseudo-process in `accept`), pauses use `Time::HiRes::sleep` (a 4-argument
  `select` with three `undef`s is not a Windows sleep), and the connect-timeout
  test accepts `connection_refused` as well as `timeout` and `io_error`.
- **`barcode-1d`**: Metal and the PNG codec stay native-extension work, but
  `render_png` used to `require` the codec before checking for a backend, so a
  host without the extension died in DynaLoader instead of reporting the
  documented "no native Paint VM" error. It now renders first. On Windows the
  recipe lists `paint_codec_png_native` and `paint_vm_metal_native` as
  prerequisites and puts their `lib\` on `@INC`; the Metal one is still an
  intentional macOS-only skip.
- **Rust XS packages** (`gf256_native`, `polynomial_native`,
  `paint_codec_png_native`) reuse the conduit design rather than inventing one:
  `code/scripts/check-perl-extension-windows.ps1` (its `-ExtensionName` set now
  names them) selects the native Strawberry Perl, builds the cdylib with the
  default MSVC host target (`perl-bridge` compiles its C shim with that Perl's
  compiler and links the CORE import library `libperl*.a`, which `rust-lld`
  accepts), stages `<Name>.dll` as `lib\auto\CodingAdventures\<Name>\<Name>.xs.dll`
  and runs `prove`. A GNU-target build (as `go/conduit` needs for cgo) is not
  used here because nothing links through gcc: Perl loads the DLL itself and the
  only C is the shim, built by Strawberry's gcc. `gf256_native` and
  `polynomial_native` previously only loaded the module in their Unix `BUILD`;
  they now have real `t/01-basic.t` suites that both recipes run. The three are
  repository-input consumers of the helper and `perl-bridge` (and members of the
  `build-windows-os-suites` gate), like `conduit` and `irc-server-native`.

Still hand-written or skipped (tracked in W2/W4): `barcode-layout-1d` (above),
`aes-modes` (`Makefile.PL`/`make test`), `der-asn1`, `der-tlv` and
`x509-extension` (explicit-Strawberry compile checks that already pass), and
`paint_vm_metal_native` (Metal is macOS-only).

### W3 progress: Go cgo (`go/conduit`, `go/conduit-hello`)

Done. Both recipes were echo-skips and now run `go test` for real on Windows.
Go's cgo drives MinGW-w64 gcc (never `cl.exe`), and a gcc/ld link needs a
GNU-ABI library, so the recipe does not use the MSVC `conduit_capi.lib` or the
DLL that the Haskell recipe stages:

- `code/scripts/build-conduit-capi-windows-gnu.ps1` (shared by both recipes)
  runs `rustup target add x86_64-pc-windows-gnu`, then
  `cargo rustc --release --target x86_64-pc-windows-gnu --lib --crate-type staticlib`.
  Asking for the staticlib alone avoids linking the crate's `cdylib`, which
  would need a MinGW linker for rustc; the result is
  `target\x86_64-pc-windows-gnu\release\libconduit_capi.a`.
- The same helper then picks a MinGW-w64 gcc (`C:\mingw64\bin` first, other
  known MinGW locations next, then any `gcc` on PATH; Ruby's UCRT gcc is not a
  candidate because Rust's GNU std links the msvcrt runtime), prepends it to
  PATH only after cargo has run, sets `CC=gcc`, and asserts gcc is an x86_64
  MinGW and Go is `windows/amd64`.
- `conduit.go` gains `#cgo windows LDFLAGS:` naming that archive by full path
  plus `-lws2_32 -luserenv -lbcrypt -lntdll -ladvapi32 -ldbghelp -lkernel32
  -lgcc_eh -static` (the libraries `rustc --print native-static-libs` reports
  for the GNU target, plus `libgcc_eh` for the `_Unwind_*` symbols; `-static`
  keeps `libwinpthread` from becoming a runtime DLL dependency).
- `tools\run-tests.ps1` in each package mirrors `tools/run-tests.sh`, and the
  recipe is `powershell -NoProfile -ExecutionPolicy Bypass -File tools\run-tests.ps1`.

### W3 progress: C++ (`cpp/conduit`, `cpp/conduit-hello`)

Done. Both recipes were echo-skips ("uses POSIX sockets") and now build and run
the same tests as their Unix `BUILD` on Windows. Unlike Go's cgo, `cl.exe` links
the default MSVC Rust output, so no GNU target is involved:

- `code/scripts/build-conduit-capi-windows-msvc.ps1` (shared by both recipes)
  drops Git for Windows' `usr\bin` from `PATH` (a bare `link` would be GNU
  coreutils', see `lessons.d/on-the-windows-runner-a-bare-link-is-git-s-gnu-coreutils.md`),
  asserts `cl`/`link`/`cargo` and an `x86_64-pc-windows-msvc` rustc host, runs
  `cargo rustc --release --crate-type staticlib -- --print native-static-libs`
  (the staticlib alone, as on Unix) and returns `conduit_capi.lib`, its header
  directory and the system libraries to link: `kernel32 advapi32 ntdll userenv
  bcrypt ws2_32 synchronization`, merged with whatever rustc reports except the
  cargo-private `windows.0.52.0.lib`. `synchronization.lib` holds
  `WaitOnAddress`/`WakeByAddress*`; without it the link ends in LNK2019/LNK1120.
- `tools\run-tests.ps1` (`cpp/conduit`) and `tools\run.ps1` (`cpp/conduit-hello`)
  mirror the Unix scripts: `cl.exe /std:c++17 /EHsc /W4 /MD` (dynamic CRT, which
  is what Rust's MSVC std uses), each suite run in turn and any failure turned
  into a non-zero exit. The recipes are
  `powershell -NoProfile -ExecutionPolicy Bypass -File tools\run-tests.ps1` /
  `...tools\run.ps1` and keep the Unix `# build-tool: deps=` directive.
- The E2E clients (`tests/test_server.cpp`, `tests/smoke.cpp`) go through a small
  `tests/net_compat.h` (BSD sockets on Unix, Winsock on Windows: `WSAStartup`
  once, `SOCKET`/`closesocket`, int-sized `send`/`recv`, and `SO_RCVTIMEO` as a
  DWORD of milliseconds instead of a `timeval`). Unix behaviour is unchanged.
- `CMakeLists.txt` selects `conduit_capi.lib` + the system libraries under MSVC
  and the `x86_64-pc-windows-gnu` archive under MinGW; the Unix result is the
  same `libconduit_capi.a` as before.

Proof so far: Unix scripts pass; the shimmed tests compile with
`-Wall -Wextra -Wpedantic -Werror` under mingw-w64, link against the GNU-target
staticlib and pass under Wine (loopback E2E included); the build tool's
`-validate-build-files` accepts both `BUILD_windows` files. The MSVC/PowerShell
path itself is first exercised by the Windows CI leg.

### W3 progress: Go cgo (`go/paint-codec-png-native`)

Done; this was the last Go cgo consumer, so Go cgo in W3 is complete. The package
used to link and test only on macOS arm64 (`darwin_arm64.go` plus a `uname`-gated
`BUILD` and a Windows echo-skip). It now has one cgo file under
`cgo && (linux || darwin || (windows && amd64))` with per-OS `#cgo LDFLAGS`, and a
stub for everything else (and for `CGO_ENABLED=0`); `SupportedRuntime()` and
`Available()` report a single `nativeLinked` constant defined beside the tags.

- Linux links `target/release/libpaint_codec_png_c.a` by full path (so the linker
  cannot pick the cdylib `.so` that `cargo build` also writes) plus
  `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`, which is what
  `cargo rustc ... -- --print native-static-libs` reports for this crate on
  Rust 1.100.0. `BUILD` is now `deps=rust/paint-codec-png-c`, `cargo build
  --release`, `CGO_ENABLED=1 go test ./... -v -cover` with no `uname` gating.
- macOS keeps the flags that already worked (`-L... -lpaint_codec_png_c`); the
  tag now covers arm64 and amd64 with the same flags. amd64 is unverified
  locally and is first exercised by the macOS CI leg.
- Windows reuses the go/conduit recipe: `tools\run-tests.ps1` calls
  `code\scripts\build-conduit-capi-windows-gnu.ps1`, which gained `-CrateDir`
  and `-ArchiveName` parameters (defaults unchanged, so go/conduit is
  byte-for-byte the same), to build the `x86_64-pc-windows-gnu` staticlib and put
  MinGW gcc on `PATH`. For this crate rustc reports only
  `-lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -lsynchronization`; the link
  line adds `-lgcc_eh -static`, as for conduit.
- Tests assert the PNG signature, IHDR dimensions and a pixel-exact round trip
  through `image/png` wherever the codec is linked, and cover the nil, inconsistent
  and empty error paths everywhere.

Proof so far: `go test` passes natively on Linux; the Windows test binary
cross-compiles with mingw-w64 against the Rust 1.100.0 GNU staticlib and passes
under Wine; `go/conduit` still cross-compiles; the build tool's
`-validate-build-files` accepts both recipes. The PowerShell wrapper itself and
macOS amd64 are first exercised by CI.

Remaining in W3: Lua `lua54.lib`, Dart FFI, Elixir NIF link and .NET P/Invoke.
The node import library is done (raw-dylib, see `typescript/conduit`), and the
Rust bridge crates are covered in the next section.

### W3/W4 progress: the Rust remainder

The echo-skips left in `code/packages/rust` and `code/programs/rust` were
`node-bridge`, `font-parser-node`, `python-bridge`, `font-parser-python`,
`ruby-bridge`, `font-parser-ruby`, `tcp-reactor`, `twig-module-driver`,
`os-kernel`, `unix-tools` (Rust and Ruby) and the Metal packages. Status:

- **Bridges (converted).** `font-parser-{node,python,ruby}` each build their
  cdylib into the package's own `target/` (all three emit `font_parser_native`,
  and the build tool runs independent packages in parallel), run
  `cargo clippy -D warnings`, and run a smoke test that loads the library into a
  real Node.js, CPython or Ruby and calls every export, error paths included
  (`tests/smoke.test.js`, `tests/smoke_test.py`, `tests/smoke_test.rb`).
  `node-bridge`, `python-bridge` and `ruby-bridge` have no tests of their own;
  they build and run clippy on every OS and are exercised through the above.
  `# build-tool: deps=` on the pure-language `font-parser` sibling makes CI set
  up the matching toolchain. The skips were hiding real defects: `font-parser-node`
  declared five `napi_*` imports without the `node.exe` raw-dylib marker (Windows
  link failure, reproduced by a cross-link); `font-parser-python` had no
  `build.rs` for `pythonXY.lib`; `font-parser-ruby` had never been loaded and
  failed three ways (`rb_data_object_get` is `static inline`, receiver and
  arguments swapped, `Qnil` hard-coded). On Windows its `build.rs` scans the
  `rb_*` declarations of the crate and ruby-bridge into a `.def` file and has MSVC
  `lib.exe` (PATH, else vswhere) build an import library for Ruby's DLL; the GNU
  target links RubyInstaller's `.dll.a`. **Only Windows CI can prove:** that the
  MSVC-linked cdylib loads into RubyInstaller's MinGW Ruby 3.4, that
  `rb_raise`'s longjmp across the Rust frames is clean there, and that
  `lib.exe` is found.
- **`os-kernel` (converted).** The skip said it cannot be built on a std host;
  the binary cannot, the package can. The recipe adds the
  `x86_64-unknown-uefi` target, runs the `Kernel` unit tests on the host,
  cross-builds `os-kernel.efi` and runs clippy for both targets, identically on
  every OS. The QEMU boot test stays manual.
- **`unix-tools`, Rust (converted).** The crate already compiled for Windows and
  gated its Unix-only tests behind `#[cfg(unix)]`. The whole suite built for
  `x86_64-pc-windows-gnu` and run under Wine found ten tests failing, all spawning
  a real `echo` or `false`; they now fall back to the test executable itself, and
  Wine reports 1278 passed, 0 failed.
- **`unix-tools`, Ruby (partly converted).** The Rakefile runs every test file on
  Unix and, on Windows, the twelve whose tools are pure stream transformers, and
  prints which 44 it held back. The rest need a Windows run to classify.
- **`tcp-reactor` (needs a decision, not converted).** It is a readiness
  reactor: nonblocking sockets, `epoll`/`kqueue` readiness, read-until-`WouldBlock`.
  Windows IOCP is completion-based, and `event-loop-native-backends.md`
  ("Windows should not be forced into a fake readiness abstraction"; `WSAPoll`:
  "avoid as primary design") and the `iocp` crate both say not to fake readiness
  on Windows. An IOCP `tcp-reactor` is a redesign (`AcceptEx`/`WSARecv`/`WSASend`
  with owned buffers, per-connection outstanding-operation counts, completion
  based backpressure): that is what `transport-platform`'s Windows provider and
  `tcp-runtime` already are, with real Windows tests. Nothing depends on
  `tcp-reactor`. Options: list it on the W6 allowlist as intrinsic with
  `tcp-runtime` named as the Windows equivalent, or retire it. Note also that its
  only tests are `kqueue` ones, so `cargo test -p tcp-reactor` runs zero tests on
  Linux.
- **`twig-module-driver` (converted).** The recorded reason (768 MiB and 3 GiB
  thread stacks fail to spawn on Windows) had no observation behind it and does
  not hold: Windows reserves thread stacks as address space and commits on use,
  and the suite peaks at about 1.3 GiB resident. Nothing was shrunk. Running the
  suite for `x86_64-pc-windows-gnu` under Wine found the real Windows failure: the
  tests canonicalised the Twig compiler directory, which on Windows is a verbatim
  `\\?\C:\...` path, and the Twig compiler appends `"/" + name`, which Win32
  rejects inside a verbatim path (`os error 123`). The test helpers now strip the
  prefix. After that, 111 of 111 Windows-applicable tests pass under Wine,
  including the 768 MiB and 3 GiB threads. **Only Windows CI can prove:** real
  Windows commit behaviour with four parallel test threads (each up to about
  1.75 GiB of stack in debug builds), and wall time (Wine took 25 minutes with two
  threads; Linux 12.5 minutes single-threaded).

## Done

Every package either passes its real build and tests on all three OSes, or is
on the W6 allowlist with a named platform limit. The daily cross-OS full build
is green, and the allowlist contains only intrinsic limits.
