# VER07q — Twig and WASM guest microbenchmark entry points

This slice adds two Rust library/CLI packages that time work executed by the
actual Twig VM and WASM interpreter. Both reuse the Rust VER07 collector and
rank-statistics packages. Compilation, parsing, validation, and guest-instance
construction happen before collection; each timed batch invokes the prepared
guest program repeatedly and verifies its returned checksum after the timer.
Both packages use `guest-microbenchmark-support` for command parsing, strict
core/envelope replay, metadata, and raw diagnostics. The support and guest
packages are members of the canonical Rust workspace so one Cargo invocation
can resolve and test them without nested workspace roots.

## Guest workload boundary

The Twig guest is compiled once with `TwigVM::compile`; timed entries call the
compiled module through `twig_vm::dispatch::run`. The WASM guest is parsed,
validated, and instantiated once; timed entries call its exported function
through `WasmRuntime::call`, which includes per-call engine setup and instance
state transfer in this runtime. The selected guest workloads must be bounded and
deterministic. Their workload IDs describe the operations actually executed;
neither CLI may label host-side loops as guest work or imply that interpreter
time represents simulated processor cycles.

Host monotonic time encloses the call boundary and guest execution. Each
package must also retain raw repeated empty guest-entry timings through the
same wrapper, plus a separate host-clock/no-op probe. The envelope records
calibration settings, sample counts, entries per sample, and every elapsed
duration. These observations describe boundary and timer costs; they are not
subtracted from raw pairs. The core `Report` JSON shape remains unchanged. The
strict guest envelope records runtime details and a replayable copy of the
exact core report without decorating the portable JSON.

The CLI default is 40 paired observations, and it emits standard replayable
core report JSON to stdout. An explicit `--manifest FILE` writes the strict
guest envelope separately; `--replay FILE` reads a core report, while
`--replay-envelope FILE` validates the sidecar envelope.
The public library APIs expose prepared guest runners and a callback collector
for caller-prepared guest entry functions. Package tests cover fixture
provenance, deterministic guest results,
setup-before-timing, callback failures, bounds, sidecar semantics, malformed
and mismatched core/envelope replay input, and CLI report replay. Native timing has no speed
threshold. The reported result describes this interpreter, guest program,
host call boundary, and host machine only.
