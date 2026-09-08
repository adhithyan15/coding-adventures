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

Its own manifest declares only the filesystem reads and directory listing
needed to load source and the manifest, plus standard output for CLI reports.
Parsing via `compiler-libs` does not require a banned-construct exception.

## Inputs and exit behavior

The CLI accepts `--dir PACKAGE_DIR` and optional `--verbose`. The directory
defaults to `.`. It recursively analyzes regular `.ml` and `.mli` files in
deterministic relative-path order, while rejecting symlinked source inputs and
skipping `_build`, `.git`, `_opam`, and `node_modules` directories.

`required_capabilities.json` is loaded only from the package root. An absent
manifest is the Spec 13 zero-capability profile. A present manifest must be a
schema-v1 object with the closed 19 category/action pairs. Unknown fields,
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
|---|---|
| `open_in`, `open_in_bin`, `In_channel.open_text`, `In_channel.open_bin` | `fs:read:*` |
| `open_out`, `open_out_bin`, `open_out_gen`, `Out_channel.open_text`, `Out_channel.open_bin` | `fs:write:*` |
| `Sys.file_exists`, `Sys.is_directory`, `Sys.read_directory`, `Unix.stat`, `Unix.lstat`, `Unix.opendir`, `Unix.readdir` | `fs:list:*` |
| `Sys.remove`, `Unix.unlink`, `Unix.rmdir` | `fs:delete:*` |
| `Sys.rename`, `Unix.rename`, `Unix.mkdir`, `Unix.chmod`, `Unix.chown` | `fs:write:*` |
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
| any OCaml `external` declaration | `ffi:call:*` |
| `Dynlink.loadfile`, `Dynlink.loadfile_private` | `ffi:load:*` |

Matching is syntactic and deliberately conservative. When flags or targets
cannot be resolved statically, the broad action-level capability is emitted.

## Names, aliases, opens, and shadowing

The analyzer resolves direct module aliases such as `module U = Unix`, local
aliases introduced by `let module`, and `Stdlib.Unix`-style qualified paths.
It resolves `open Unix`, `let open Unix in ...`, and `Unix.(...)` for the rule
table above.

A locally bound value shadows an unqualified standard-library name in its
lexical scope. Function parameters, `let`, `let rec`, match cases, loops, and
exception handlers contribute bindings. A locally defined module shadows a
previous alias. Qualified calls through an unknown or shadowing module are not
attributed to the standard library.

Dynamic module expressions that affect a capability-bearing open or alias,
first-class-module dispatch used as a call receiver, and extension nodes in
call position are unsupported and fail with an explicit analysis error instead
of being silently ignored.

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
`ffi:load` declaration. An `external` declaration may pass only with both an
exact `external` exception and an `ffi:call` declaration. Missing either half
remains a `CAP002` violation with an actionable hint.

The manifest schema therefore recognizes `ocaml` as an exception language.
Adding a nonempty exception or capability remains subject to the separate
Spec 13 signed-approval policy.

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
