# OCAML06 — Capability analyzer

Status: in progress

## Purpose

This contract adds the OCaml implementation of Spec 13 Layers 3 and 4. The
analyzer parses every `.ml` and `.mli` file in one package with OCaml
`compiler-libs`, detects direct operating-system access and constructs that can
evade static analysis, and compares that evidence with the package's
`required_capabilities.json` profile.

The analyzer is a publication gate, not a sandbox. It reports source evidence
and fails closed when source, manifest, or name-resolution input cannot be
understood. It does not grant access, rewrite source, generate wrappers, or
claim that a declaration alone confines a process.

## Package and toolchain

The package root is `code/packages/ocaml/ca-capability-analyzer`. Its public
library is `Coding_adventures_capability_analyzer`; its executable and opam
package are `coding-adventures-capability-analyzer`.

It uses the locked OCAML03 toolchain: OCaml 5.2.1, opam 2.5.2, Dune 3.17.2
with Dune language 3.16, Alcotest 1.9.0, bisect_ppx 2.8.3, ocamlformat 0.27.0,
and Yojson 3.0.0. `compiler-libs.common` supplies the parser and AST. The
analyzer does not invoke the compiler, shell, package manager, or another
language analyzer.

Its own manifest declares the filesystem reads and directory listing needed to
load source and the manifest, standard output for CLI reports, and the
create/write/delete actions used by isolated temporary-fixture tests. Parsing
via `compiler-libs` does not require a banned-construct exception.

## Inputs and exit behavior

The CLI accepts `--dir PACKAGE_DIR` and optional `--verbose`. The directory
defaults to `.`. It recursively analyzes regular `.ml` and `.mli` files in
deterministic relative-path order, while rejecting symlinked source inputs and
skipping `_build`, `.git`, `_opam`, and `node_modules` directories. `.mll` and
`.mly` inputs, Dune preprocessing/generator/include/copy/compiler-flag stanzas,
Dune workspace files, preprocessor flags, foreign-code stanzas, AST extensions,
and source attributes are rejected, except for the single reviewed native-clock
profile below. The sole instrumentation exception remains the exact pinned
`bisect_ppx` coverage stanza in a package's reviewed `src`, `bin`, and `test`
Dune files; a root-level stanza, every other instrumentation stanza, and every
other backend declaration fails closed until the analyzer can inspect its
generated output. The independent CI gate also rejects any repository
`dune-workspace`, which could otherwise alter package builds from outside an
analyzed package root.

The scanner applies fixed resource ceilings before parsing: at most 10,000
source files, at most 4 MiB per source file, at most 64 MiB across all source
files, and at most 1 MiB for `required_capabilities.json`. Crossing any ceiling
is an exit-2 input error. Reads are bounded to the size checked on the opened
channel so a growing file cannot force an unbounded allocation.

`required_capabilities.json` is loaded only from the package root. An absent
manifest is the Spec 13 zero-capability profile. A present manifest must be a
schema-v1 object with the closed 25 category/action pairs. Unknown fields,
missing fields, duplicate capabilities, invalid pairs, empty targets, short
justifications, non-OCaml banned exceptions, and malformed JSON are errors.

Exit codes are stable:

- `0`: every detection is declared and no non-exempt banned construct remains;
- `1`: one or more `CAP001` or `CAP002` violations; and
- `2`: an input, parse, manifest, traversal, or internal error.

Unlike the older Go analyzer, an unparseable source file is never a warning.
Skipping it could produce a false-clean verdict, so OCaml parse failure is a
tool error.

## Capability matching

Detections use `category:action:*`. A declaration with the same category and
action covers that detection regardless of its narrower runtime target. The
static analyzer proves declaration presence; a runtime cage remains
responsible for target enforcement. No declaration with a different action or
category covers the detection.

The analyzer emits at most one `CAP001` violation per capability while keeping
every source detection for verbose reporting. Results are sorted by relative
path, one-based line, zero-based column, capability, and evidence.

## Detection table

The embedded rule table covers these direct calls, including module aliases and
supported opens:

| OCaml surface | Capability |
|---|---|---|
| `open_in`, `open_in_bin`, `In_channel.open_text`, `In_channel.open_bin`, `In_channel.with_open_text`, `In_channel.with_open_bin` | `fs:read:*` |
| `open_out`, `open_out_bin`, `open_out_gen`, `Out_channel.open_text`, `Out_channel.open_bin`, `Out_channel.with_open_text`, `Out_channel.with_open_bin` | `fs:write:*` |
| `Sys.file_exists`, `Sys.is_directory`, `Sys.read_directory`, `Unix.stat`, `Unix.lstat`, `Unix.realpath`, `Unix.opendir`, `Unix.readdir` | `fs:list:*` |
| `Sys.remove`, `Unix.unlink`, `Unix.rmdir` | `fs:delete:*` |
| `Unix.mkdir`, `Unix.symlink` | `fs:create:*` |
| `Sys.rename`, `Unix.rename`, `Unix.chmod`, `Unix.chown` | `fs:write:*` |
| `Unix.openfile` | both `fs:read:*` and `fs:write:*` conservatively |
| `Unix.socket`, `Unix.socketpair`, `Unix.connect`, `Unix.send*`, `Unix.recv*`, `Unix.shutdown` | `net:connect:*` |
| `Unix.bind`, `Unix.listen`, `Unix.accept`, `Unix.accept_non_intr` | `net:listen:*` |
| `Unix.getaddrinfo`, `Unix.getnameinfo`, `Unix.gethostbyname`, `Unix.gethostbyaddr` | `net:dns:*` |
| `Sys.command`, `Unix.system`, `Unix.exec*`, `Unix.create_process*`, `Unix.open_process*` | `proc:exec:*` |
| `Unix.fork` | `proc:fork:*` |
| `Unix.kill` | `proc:signal:*` |
| `Sys.getenv`, `Sys.getenv_opt`, `Unix.getenv`, `Unix.getenv_opt` | `env:read:*` |
| `Unix.putenv` | `env:write:*` |
| `Sys.time`, `Unix.time`, `Unix.gettimeofday`, `Unix.times`, `Unix.clock_gettime` | `time:read:*` |
| `Unix.sleep`, `Unix.sleepf` | `time:sleep:*` |
| `read_line`, `read_int`, `read_float`, or `input*` whose channel is `stdin`/`In_channel.stdin` | `stdin:read:*` |
| `print_*`, `prerr_*`, `Printf.printf`, `Printf.eprintf`, `Format.printf`, `Format.eprintf`, or `output*` to `stdout`/`stderr` | `stdout:write:*` |
| `Random.self_init`, `Random.State.make_self_init` | `rand:read:*` |
| `Thread.create`, `Domain.spawn` | `thread:spawn:*` |
| any OCaml `external` declaration | `ffi:call:*` |
| `Dynlink.loadfile`, `Dynlink.loadfile_private` | `ffi:load:*` |

OCaml 5's default `Random` state starts from a fixed seed, so `Random.int` and
the other default-state draws are deterministic unless `Random.self_init` ran.
They are therefore not capability uses on their own; `self_init` is.
`Random.full_init` and `Random.State.make` take their seed from the caller and
are pure computation.

Matching is syntactic and deliberately conservative. When flags or targets
cannot be resolved statically, the broad action-level capability is emitted.

## Names, aliases, opens, and shadowing

The analyzer resolves direct and constrained module aliases such as
`module U = Unix`, local aliases introduced by `let module`, includes, and
`Stdlib.Unix`-style qualified paths. It resolves `open Unix`,
`let open Unix in ...`, and `Unix.(...)` for the rule table above. A sensitive
function used as a first-class value is detected at the reference, so value
aliases and higher-order arguments cannot bypass a capability or hard ban.

A locally bound value shadows an unqualified standard-library name in its
lexical scope. Function parameters, `let`, `let rec`, match cases, loops, and
exception handlers contribute bindings. A locally defined module shadows a
previous alias. Qualified calls through an unknown or shadowing module are not
attributed to the standard library.

Dynamic module expressions that contain a capability-bearing module but cannot
be resolved, first-class-module dispatch used as a call receiver, generated
source inputs, attributes, and extension nodes are unsupported and fail with an
explicit analysis error instead of being silently ignored.

## Banned constructs

The analyzer emits `CAP002` for constructs that can bypass ordinary static
reasoning:

- any call through `Obj`, including `Obj.magic`;
- `Marshal.from_channel`, `Marshal.from_string`, and `Marshal.from_bytes`;
- `Marshal.Closures` used while marshaling; and
- native loading through `Dynlink.loadfile` or `Dynlink.loadfile_private`;
- any `external` declaration.

`Obj` and unsafe `Marshal` findings are hard failures. They cannot be cleared by
a manifest exception. Native loading may pass only with both an exact OCaml
`banned_construct_exceptions` entry for the detected construct and an
`ffi:load` declaration. An `external` declaration normally requires the exact
`external` exception and an `ffi:call` declaration; only the profile below can
narrow this package-wide dual opt-in to three digest-pinned bindings. Missing
either half remains a `CAP002` violation with an actionable hint.

The manifest schema therefore recognizes `ocaml` as an exception language.
Adding a nonempty exception or capability remains subject to the separate
Spec 13 signed-approval policy.

### Single reviewed monotonic-clock shim

The only exception to the foreign-code-stanza rejection is
`ocaml/microbenchmark`'s native clock. Its only allowed native and Dune inputs are these exact files:

| Path | Plain SHA-256 | Coverage SHA-256 |
|---|---|
| `dune-project` | `1c370421547869ef0737e151d707375828230bdd1b01e35a8f28e0fe8896ad25` | same |
| `bin/dune` | `b18f0c6eb1d5e3fbf9e3db649baacbf4507ff66944cc25f4ba7a4c2f0c0a5277` | `d7d97a75f9e25c23c32d1a69e3e87a2b83c20b74742f224282c98522b313ce3e` |
| `src/dune` | `0ec6c89861cdf189084f0c56f487c5b4db035c7f090a89ae96dca39546215d90` | `cf007d48b1d097fd2678810bb875557508724f269c92f515935df3cac75386b3` |
| `test/dune` | `44fab3370f8023882b7c9b0e1dac5d477f8e8e4e5e777e8f0559e8ff3fcdfccd` | same |
| `src/native_clock.ml` | `fafe791c3237c183c09425a0e8926374719cfd789e22c8c2daa9d227a1fa6e6d` | same |
| `src/monotonic_clock.c` | `8bdbb842a125ff4199a819ffa60eb13e88e149a4757ba8d4eb09310ddbe2d86d` | same |

The analyzer may pass only one of two complete pinned Dune-context bundles
(plain build or exact repository coverage build), each with the `src/dune` C
stub stanza naming `monotonic_clock`, the pinned OCaml wrapper,
and the three corresponding bindings `now_ns`, `os_name`, and `architecture`.
No local C/header/assembly source or object/archive is permitted except the
pinned `src/monotonic_clock.c`; reject symlinks and extra source inputs. Dune
configuration in this package must match every file in one pinned bundle. The
coverage bundle differs only by the exact `(instrumentation (backend
bisect_ppx))` stanza in `bin/dune` and `src/dune`. Mixed bundles, any other
instrumentation backend, and all other Dune changes fail closed. The existing
repository gate must continue rejecting any `dune-workspace`. Continue
rejecting every other `foreign_stubs`,
`foreign_archives`, `foreign_library`, `ctypes`, `extra_objects`, compiler/link
flags, generated input, include/copy rule, and additional external declaration,
including elsewhere in the same package. A source, binding, Dune-context, or
native-input edit therefore fails closed until a reviewer updates the
analyzer-owned pins and exact profile. Hash raw file bytes; the repository LF
checkout rule makes these pins identical on Windows and POSIX. SHA-256 uses the
explicitly pinned pure OCaml `digestif` implementation; this does not add C
parsing or claim that the analyzer proves the shim's semantics.

The manifest's FFI and time entries are exactly `ffi:call:monotonic_clock` and
`time:read:*`; its sole exception is the OCaml `external` entry. The remaining
declared capabilities are the analyzer-observed CLI effects: `stdout:write:*`,
`env:read:*`, `fs:read:*`, and `fs:list:*`. The validator requires this exact
six-entry capability set, so a source edit needs a source-pin and profile
review before its privileges can change. These permissions are not inferred
from the hash. The shim itself remains reviewed
source: Windows calls QueryPerformanceCounter, POSIX calls
`clock_gettime(CLOCK_MONOTONIC)`, and checked conversion or native-call errors
raise into OCaml. Do not replace it with wall time, process CPU time, or a
clamped/fabricated clock.

This profile requires a Spec 13 capability-escalation approval bound to the
reviewed source commit. The repository currently has no tracked capability
signer list or escalation-verification workflow; a proposed unsigned payload
is not approval. Do not treat release signers as capability signers, claim
static C analysis, or merge the nonempty manifest/exception without the
required owner hardware-key authorization. Tests must accept the exact profile
and reject C/wrapper/Dune drift, added native inputs, extra declarations,
other package paths, and missing or mistargeted permissions.

## Language-neutral executable fixture

`code/specs/fixtures/ocaml-capability-analyzer-v1/cases.json`, validated by
`cases.schema.json`, is the repository-neutral behavior oracle. Each case
contains source text, source kind, and sorted expected capability and banned
construct identifiers. It covers pure code, qualified calls, aliases, opens,
local shadowing, filesystem, network, process, environment, time, standard
streams, interfaces, FFI, native loading, unsafe marshaling, and `Obj`.

The published package embeds its rules and does not read the fixture at
runtime. Package tests load the fixture from the checkout and also verify every
manifest and CLI branch, deterministic recursion, symlink rejection, malformed
source failure, and exception/capability dual opt-in.

## Build and CI integration

`BUILD` and `BUILD_windows` install locked dependencies, run `dune build @fmt`,
run the test suite with bisect_ppx, and require at least 95% production-line
coverage while naming every production `.ml` file.

The generic build continues to execute this package like any other OCaml
package. In addition, the protected capability gate independently installs the
locked analyzer and scans every discovered OCaml package and program. That
independent invocation cannot be removed by editing an analyzed package's
BUILD file. Repository tests pin the workflow invocation, analyzer package,
schema language, and fixture paths.

This tranche does not add secure wrappers, a native sandbox, signed approvals,
the OCaml build-tool implementation, representative three-platform package
execution, or OCaml denominator promotion. Those remain separately owned.
