# VER07q — Twig and WASM guest microbenchmark entry points

This slice adds two Rust library/CLI packages that time work executed by the
actual Twig VM and WASM interpreter. Both reuse the Rust VER07 collector and
rank-statistics packages. Compilation, parsing, validation, and guest-instance
construction happen before collection; each timed batch invokes the prepared
guest program repeatedly and verifies its returned checksum after the timer.

## Guest workload boundary

The Twig guest is compiled once with `TwigVM::compile`; timed entries call the
compiled module through `twig_vm::dispatch::run`. The WASM guest is parsed,
validated, and instantiated once; timed entries call its exported function
through `WasmRuntime::call`. The selected guest workloads must be bounded and
deterministic. Their workload IDs describe the operations actually executed;
neither CLI may label host-side loops as guest work or imply that interpreter
time represents simulated processor cycles.

Host monotonic time encloses the call boundary and guest execution. Each
package must also measure repeated empty guest entries with the same host
wrapper and expose the observed total and per-entry time in a sidecar record.
This observation helps describe boundary cost; it is not subtracted from raw
pairs. The core `Report` JSON shape remains unchanged. Sidecar data records the
guest boundary, workload ID, empty-entry sample count/duration, runtime, and
the same report identity, and must not rewrite or decorate the core report.

The CLI default is 40 paired observations, and it emits standard replayable
core report JSON. A second command or option writes the sidecar separately.
The public library API supports caller-supplied guest entry closures, so users
can benchmark additional bounded programs without forking collection or
statistics. Package tests cover fixture analysis, deterministic guest results,
setup-before-timing, callback failures, bounds, sidecar semantics, malformed
and mismatched replay input, and CLI report replay. Native timing has no speed
threshold. The reported result describes this interpreter, guest program,
host call boundary, and host machine only.
