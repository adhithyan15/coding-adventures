# VER07aa — Reusable WASM microbenchmark library

## Goal and relationship to the guest runners

The repository has two distinct WASM benchmark capabilities. The native Rust
`wasm-guest-microbenchmark` package measures functions run by the repository's
WASM interpreter. This specification adds a reusable WebAssembly library that
an independent host can load and call to collect, analyze, and replay paired
microbenchmark reports. The interpreter runner remains useful, but its Rust
host wrapper and its `sum_recursive`, `sum_loop`, and `empty` guest workload
exports do not by themselves satisfy this library contract.

The new package lives at `code/packages/wasm/microbenchmark`. It is a plain
Rust `cdylib` targeting `wasm32-unknown-unknown`, with no JavaScript binding
framework. It depends on the repository's `rust/microbenchmark` core for
collection, strict report parsing, and analysis, which in turn uses the shared
`rust/rank-statistics` implementation. It must not copy or fork those
algorithms. Its release artifact is
`target/wasm32-unknown-unknown/release/microbenchmark_wasm.wasm`.

The Rust host runner and the WASM library serve different users. Keep the
host runner's prepared interpreter, no-op probe, empty-entry probe, CLI,
sidecar format, and replay behavior. The WASM package offers a bounded,
portable ABI so consumers can benchmark callbacks implemented by their own
host runtime. Mosaic, Starlark, and Twig guest workloads are not prerequisites
for this package or its acceptance checks. Shared collection and build
infrastructure remains maintained independently of those workloads.

## Guest module boundary

The module receives no ambient clock, random source, process state, filesystem,
or network access. It imports exactly two synchronous functions from the host
module named `microbenchmark_host`:

| Import | WASM signature | Contract |
|---|---|---|
| `now_ns` | `() -> i64` | Return a monotonic, nonnegative nanosecond tick. The collector checks that ticks do not regress and that measured durations are positive. |
| `run_batch` | `(variant: i32, entries: i32, checksum_out: i32) -> i32` | Run the selected caller-owned workload `entries` times and write its signed 64-bit checksum in little-endian form to `checksum_out` in the module's exported linear memory. Return zero on success and a nonzero host-defined failure code otherwise. |

`variant` is `0` for baseline and `1` for candidate. The module passes the
caller-configured positive `entries` count. The host owns workload selection,
implementation, and all host-side effects; the module owns the comparison
protocol, timer placement, raw observations, and statistical conclusion. A
host callback must not call back into any module export. One synchronous ABI
operation may be in progress per instance. Separate instances have independent
linear memory and may be used independently. Concurrent calls against the same
instance are unsupported; the guard rejects nested calls caused by host
callbacks even if the host violates its no-reentrancy rule.

The module must export its linear memory and the following functions. Every
mutating or operation export (`alloc`, `dealloc`, and the three JSON operations)
uses a per-instance busy guard. If an import calls back into one while an
operation is active, the nested call is rejected without changing allocations
or operation state (`alloc` returns zero, `dealloc` returns 2, and a JSON
operation returns zero). A failed guard acquisition never clears another
operation's guard. Read-only `abi_version` remains callable.

| Export | WASM signature | Contract |
|---|---|---|
| `abi_version` | `() -> i32` | Return ABI version `1`. |
| `alloc` | `(byte_len: i32) -> i32` | Allocate and zero-initialize `byte_len` bytes in module memory, or return zero on invalid size, busy instance, allocation-table exhaustion, or allocation/budget failure. Lengths are interpreted as unsigned 32-bit values and checked against the documented input limit before use. Zero-byte allocation returns zero. |
| `dealloc` | `(ptr: i32, byte_len: i32) -> i32` | Release a nonempty live allocation previously returned by `alloc` or by a result export, using the exact pointer and length. Return `0` on release, `1` for no matching live allocation, and `2` when the instance is busy. Invalid, mismatched, duplicate, or foreign frees do not release memory and never trap. |
| `collect_json` | `(request_ptr: i32, request_len: i32) -> i64` | Validate a JSON collection request, call the host clock and workload imports through the shared Rust collector, and return a packed pointer/length for a JSON core report or structured error response. |
| `analyze_json` | `(request_ptr: i32, request_len: i32) -> i64` | Validate a JSON object containing confidence, minimum factor, and paired observations, analyze it through the shared core, and return a packed pointer/length for the analysis or structured error response. |
| `replay_json` | `(request_ptr: i32, request_len: i32) -> i64` | Strictly parse and reanalyze a portable core report through `Report::from_json`, rejecting malformed, duplicated, unknown, or forged fields, and return a packed pointer/length for the validated report or structured error response. |

Each `*_json` result is a nonzero `i64` whose low unsigned 32 bits are the
result pointer and whose high unsigned 32 bits are the UTF-8 byte length. The
result is not NUL-terminated. The host must read/copy those bytes before
calling `dealloc(result_ptr, result_len)`. A JSON operation returns zero when
the busy guard rejects reentry or when no output allocation can be registered
within the table/budget; otherwise application and input failures are JSON
error results, not traps.
Successful operations return `{"ok":true,"value":...}`; `value` is the
portable core `Report` for collection and replay, and the core `Analysis` for
analysis. Failures return
`{"ok":false,"error":{"code":"...","message":"..."}}`. Error codes
are stable machine-readable names for invalid input, bounds exceeded, host
callback failure, clock failure, checksum failure, invalid report, and
internal serialization failure. Messages are descriptive and are not a
compatibility key.
Inputs and results are caller-owned allocations in the module's linear memory;
the host must free each with the exact byte length. The module retains no
pointer into an input after a call returns and holds no global result buffer.

All pointer-plus-length pairs are interpreted unsigned and must fall entirely
inside the exported memory without integer overflow. Requests are UTF-8 JSON.
Unknown fields are rejected. The maximum input document is 1 MiB; result
documents are limited to 1 MiB. `alloc` accepts only lengths from 1 through
1 MiB, so it can only prepare a valid input document. Collection requests have
exactly `{"config": <core Config>, "expected_checksum": <signed i64>}`.
Analysis requests have exactly `{"confidence_percent": <u8>,
"minimum_factor": <finite number>, "pairs": <Pair[]>}`. Replay input is one
portable core report. Collection `pairs` is limited to 1 through 10,000,
`max_batch` to 1 through 65,536, and the total number of entries requested
across warmup, calibration, and measured baseline/candidate callbacks in one
collection to 1,000,000. An operation that would exceed any bound returns a
structured error before invoking the callback that crosses it. These limits
bound untrusted JSON, allocation, and callback work at this ABI boundary; the
generic Rust collector and rank-statistics library retain their existing
contracts and are not given an arbitrary global cap.

Analysis accepts at most 10,000 pairs, matching the core collector's supported
report size. Replay inherits the 1 MiB document limit and the core's strict
report validation. Before building a generic JSON value, a bounded preflight
must reject documents deeper than 16 levels or containing more than 100,000
JSON values/members. Those bounds admit the supported 10,000-pair report while
preventing a short but structurally explosive document from consuming the
module's memory budget. Each JSON operation first copies its request into
module-owned temporary storage and completes UTF-8, size, structural, and
request-schema validation before invoking either host import. No host callback
may invalidate a borrowed request slice because none is retained. The package
must check sizes before allocating, parsing, walking arrays, or calling the
host. It must use checked integer conversions for addresses, lengths, batch
counts, and packed results. A host callback error returns a structured failure
and yields no partial report. Invalid module inputs never invoke a callback.
Clock regression, negative ticks, invalid durations, checksum mismatch,
malformed JSON, invalid workload status, and allocation failure are distinct
documented error cases.

The module tracks every live exported input and returned result in a fixed
table of at most 32 records. Exact pointer-and-length matches are required
before any input slice is formed and before any allocation is freed. At most
8 MiB of exported allocations may remain live per instance. Calls to `alloc`
are limited to 7 MiB of live input buffers, reserving up to 1 MiB for one
result; returned results use the remaining budget and should be freed before
the next operation. If retained results consume that reserve, the next
operation safely returns zero until the caller frees one. An invalid
allocation or free, a failed busy-guard acquisition, or an allocation rejected
for table/budget limits does not change any existing table entry. A JSON
operation with invalid input returns an owned error result when one can be
allocated and therefore adds that result to the table; it never changes or
consumes the input allocation. Temporary JSON copies and result serialization
are also checked against the document/output limits. The 64 MiB linear-memory
ceiling leaves at least 56 MiB above the maximum retained exported buffers for
Rust stack, parsed JSON values, core report construction, and serialization.
The input byte, nesting, and value-count limits bound those transient core
allocations; the implementation must exercise the worst accepted report and
malformed structural cases under the pinned WASM runtime to verify they return
normally within the memory ceiling. If memory pressure prevents creation of a
JSON error result, an operation returns zero. It must not rely on allocator
abort or `unwrap` for any allocation or input failure. Corruption of the
module's linear memory by a hostile host is outside this contract.

The module must not use a system clock or WASI imports. Its explicit host
imports are the complete runtime authority boundary; the WASM library's OS
capability profile is empty. The exported `memory` has an initial limit of 32
pages and a maximum of 1,024 pages (64 MiB); the chosen minimum must accommodate
the pinned Rust WASM stack and static data, and the emitted artifact's limits
must be inspected. Hosts must not grow memory from inside a callback. An
independent host consumer may itself require
clock or workload capabilities, but those belong to that host package and are
not inherited by the guest library. Build-time compiler identity is not a
runtime capability: shared guest support reports a bounded explicit compiler
label only if supplied by the caller/build, otherwise the truthful value
`unknown`; it does not execute `rustc` from `build.rs` to discover a version.

## Host package integration

The existing native `wasm-guest-microbenchmark` runner keeps its actual host
permissions. Its capability manifest must include `time:read:*` because its
CLI collector calls the host monotonic clock, `fs:create:*` because the
manifest sidecar uses create-new semantics, and only the filesystem read/write
permissions justified by replay and sidecar output. Do not assign these host
permissions to the pure WASM library.

The existing `rust-guest-microbenchmark-consumer` remains a shared consumer
owned by its package BUILD integration. Its ownership documentation and
`code/BUILD-EXEMPTIONS` entry must describe each actual owning BUILD and
assertion; adding a standalone WASM consumer must not silently transfer or
duplicate ownership. The new installed/external WASM consumer is a separate
`code/tests/wasm-microbenchmark-consumer` root with its own BUILD. It consumes
the emitted artifact by path after the WASM package BUILD constructs it; it
must not link to the Rust crate as a substitute for loading the `.wasm` file.
That consumer uses the pinned Node runtime's built-in WebAssembly APIs and no
third-party test framework. It supplies `now_ns` and `run_batch`, loads the
artifact in a fresh host process, and exercises the public ABI end to end.

The consumer acceptance checks cover ABI version and import/export names,
collection with deterministic clock/workload callbacks, expected paired
report values, independent analysis, valid replay, duplicate/unknown/oversized
input rejection, host callback failure, checksum mismatch, and returned
allocation ownership. It reads and frees every result, frees every input, and
verifies all report values rather than only checking that the module loads.
Build acceptance must construct the pinned `wasm32-unknown-unknown` artifact,
inspect its imports/exports for the declared boundary, and then run this
external consumer. The consumer may not use an unpinned compiler target or a
host Rust call as evidence for an emitted artifact.

## The illustrative guest WAT

The WAT under the native interpreter runner describes fixed sum workloads and
is illustrative documentation for that runner, not the implementation source
of the new reusable library. The library ABI is defined by this specification
and its Rust exports/import declarations. Keep WAT descriptions explicitly
labelled illustrative; if a future change claims executable parity, add a
parser/round-trip check at that time rather than introducing a parser
dependency solely to validate explanatory WAT.

## Acceptance and non-goals

The new library is accepted only when a consumer independent of the Rust crate
loads the built `.wasm`, calls collection and analysis/replay exports, and
observes the specified bounds and ownership behavior. Unit tests also cover
boundary pointer arithmetic, JSON edge cases, callback ordering, cumulative
entry budgets, clock behavior, and every structured error. The crate must
include its own package README, BUILD, changelog, and capability manifest.
The Rust workspace, package inventory, build ownership, and consumer/exemption
records must identify their real roots and integrations.

This specification does not claim publication to a registry, guest workloads
for Mosaic/Starlark/Twig, a particular runtime performance, OS sandboxing of a
host, or simulated CPU cycles. The collected values are host-supplied
monotonic nanoseconds around host-supplied workload callbacks. No baseline or
candidate timing is subtracted from another.
