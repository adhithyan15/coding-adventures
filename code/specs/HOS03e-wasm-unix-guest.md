# HOS-003e — Bounded WebAssembly Unix teaching guest

## Purpose and fidelity

This lane makes the existing HOS00 file-read teaching image available as a
small WebAssembly guest component. It is an in-memory semantic model of the
same nine `historical-unix-v1` file-read cases. It is not a historical Unix
binary, syscall ABI, CPU, board, or cycle-accurate machine. The guest is
WebAssembly code run by a host WebAssembly engine; the Node adapter used by
the BUILD is test infrastructure and is not part of the simulated operating
system.

The canonical oracle remains
`code/specs/fixtures/historical-unix-v1/file-read.json`. The WAT source and
the host-side adapter together must produce the same output and ordered event
records as those cases. Component assertions and shared-fixture replay are
separate evidence: component tests check guest state transitions, while replay
checks the complete guest-plus-adapter boundary.

## State owned by the guest

The guest owns a fixed, immutable teaching filesystem image and all mutable
Unix-like state. Its state includes path resolution results, inode identity,
four descriptor slots, descriptor-to-inode associations, per-descriptor read
offsets, directory-list iteration, and the result code for the last operation.
The expected image has five inodes, including `/hello`, `/empty`, and `/etc`.
Opening, reading, closing, listing, and resolving paths operate only on this
guest state. Guest memory is private WebAssembly linear memory; no host file
or process state is consulted.

The component surface provides `reset`, `resolve`, `open`, `read`, `close`,
`kind`, descriptor inspection, and directory-entry access through
`entry_count`, `entry_name_ptr`, and `entry_name_len`. The guest's directory
entry functions are the list operation; the host adapter does not define
directory contents. Numeric results and error codes are guest API values.
They do not depend on JavaScript exceptions or host errno values. The guest
imports no host functions. Its supported image and operation set are
deliberately limited to the teaching fixture.

The guest accepts a one-page memory. Path input is in `[4096, 8192)`, read
output is restricted to `[8192, 12288)`, path length is at most 4096 bytes,
read capacity at most 4096 bytes, and path traversal at most 512 components.
Both guest and host adapter validate arithmetic before slicing or copying. A
rejected read does not advance the descriptor offset. The exported memory is
visible to the trusted embedding host for marshalling; direct host writes are
outside the guest API's immutability guarantee.

## Host and browser boundary

The reusable guest artifact contains no command-line parser, Node imports,
filesystem reads, environment access, standard input/output, or JSON event
formatting. It accepts bounded bytes and operation arguments, then returns
guest results and state. In a browser, a host can instantiate the same module
with the WebAssembly API, provide bytes through linear memory, and inspect the
same result values without a Node-specific dependency.

The external fixture adapter owns host responsibilities: loading the WAT or
compiled module asset, translating fixture command strings into guest calls,
marshalling bounded buffers, mapping the guest's numeric operation results to
the fixture's event vocabulary, and emitting one JSON result for replay. This
translation is part of the tested adapter contract, not guest behavior. It
must not read any host file beyond the declared fixture and package asset.

Keep host-only code and fixture-driven tests beside the canonical fixture,
outside the package root, for example under
`code/specs/fixtures/historical-unix-v1/test-adapters/wasm/`. The package BUILD
may compile the WAT asset and run those external checks. The package runtime
itself should have an explicit empty `required_capabilities.json`; a host test
adapter's file access does not grant file access to the reusable guest.

## Fixture and routing contract

The nine shared cases cover `/hello`, `/empty`, `/etc`, missing paths,
normalization, directory-as-file behavior, and empty or whitespace commands.
Every case must retain the fixture's exact output and ordered event list.
External component assertions should also cover descriptor allocation and
reuse, independent offsets, end-of-file, empty-file reads, close behavior,
directory entries, and bounds rejection. Tests must be capable of detecting a
guest-state defect independently of event formatting.

The fixture and its external adapter are repository inputs to the package's
affected-only route and fallback hash. Register only the exact files actually
read or compiled by BUILD: the canonical `file-read.json`, shared
`replay_cli.py` if invoked, the WAT component adapter and external component
test, plus any package-local build manifest or WAT asset that is not already
covered by the package source hash. Do not register a directory prefix or
reuse the OS-architecture route. Preserve the existing TypeScript and Go
architecture fixture routes.

The route implementation depends on the ordered HOS-003d route state. If this
lane's implementation begins before HOS-003d is merged, rebase it after that
merge and add WAT as the next exact historical-Unix consumer while retaining
all existing rows and tests. Do not use a stale whole-file copy of the shared
selector or hash implementation.

## Build and portable evidence

The package uses the repository-pinned Node `22.23.5`, npm `11.13.0`, and
WABT `1.0.39` toolchain. Keep the WABT dependency and integrity in the package
lockfile. Declare both `typescript` and `python` toolchain requirements in
each BUILD front: TypeScript provisioning covers Node/WABT, and Python runs
the shared replay helper. These declarations combine with the existing
Rust-based WebAssembly runner classification; they do not justify changing the
global WASM/Rust defaults.

Each POSIX BUILD line must run independently under `/bin/sh -c`; avoid Bash-only
options such as `set -o pipefail`. The Windows front must use CMD-compatible
commands and paths, not PowerShell syntax. Both fronts must run guest component
checks and all nine shared replay cases. Report those as combined guest plus
host-adapter evidence, not as nine WAT-only observations.

## Reconstruction status and toolchain check

This is a reconstruction, not a byte-for-byte recovery of an unpublished
implementation. No prior HOS-003e implementation commit or WAT guest was
available when work began. The new authored module lives at
`code/packages/wasm/unix-history-simulator/src/unix_history.wat`; its public
ABI is documented in that package README.

The package's locked toolchain is Node `22.23.5`, npm `11.13.0`, and WABT
`1.0.39`. The Linux guest compilation, component checks, and all nine shared
fixture replays passed locally with these versions. Windows source protocol is
prepared but has not been executed on Windows. Fixture replay validates the
guest and Node host adapter together, not WAT event formatting in isolation.
