# VER07p — Mosaic VM and Starlark VM microbenchmarks

This port adds two Rust host drivers around the VER07b paired collector and
VER07c rank statistics. Both execute the actual guest package APIs. It does not
change Mosaic or Starlark runtime internals, and it makes no claim about
physical CPU cycles or cache behavior.

The portable `Report` stays closed to the VER07b fields. Each driver writes a
versioned guest envelope containing that unchanged report plus the workload
ID, variant names, setup and timing boundaries, diagnostic empty-entry
overhead, host and guest runtime versions, compiler/build profile, revision,
OS, architecture, and workload-source identity. Envelope replay validates its
own schema and delegates the nested report to `Report::from_json`; consumers can
also extract and replay the nested report with other VER07 readers.

## Mosaic tree walk

The workload ID is `mosaic-ir-tree-walk-event-stream-4096`. Generate one
Mosaic component with 4,096 leaf nodes, then parse and analyze it before
measurement. The timed operation invokes `MosaicVM::run` on that same immutable
analyzed file and consumes the event stream through one of two renderers:

- `vector-event-log` stores each callback event in a `Vec<String>` and joins
  the events in `emit`.
- `string-event-log` appends those same event strings directly to a `String`.

Both renderers must produce byte-identical event streams and the same
component name. Their per-operation checksum is computed from that stream and
checked by the collector. Parsing, analysis, source construction, and VM input
construction are outside timing; depth-first guest tree walking, property
resolution, callbacks, event serialization, and checksum work are inside.
This compares output accumulation strategies around Mosaic's actual compiler
tree walker; it is not an indexed-array experiment.

## Starlark bytecode execution

The workload ID is `starlark-bytecode-integer-add-chain-256`. It computes the
integer 256 using Starlark VM bytecode. The baseline is produced by the current
stub `compile_source` API from 256 simple `total = total + 1` assignments. The
candidate is a hand-assembled `CodeObject` that keeps the accumulator on the
operand stack and performs the same 256 additions. Both execute through
`interpret_bytecode`, return `total == 256`, and expose the same per-operation
checksum. Source generation, stub compilation, bytecode assembly, and code
object setup are outside timing. Each fresh interpreter and bytecode execution
are inside timing. The workload contains no Starlark source loop: the current
stub compiler does not implement loops. Its label and envelope must identify
the source-compiled and hand-assembled bytecode variants accurately.

## Collection and validation

Both drivers default to 40 pairs, 95% confidence, factor 1.05, seed 0,
100,000 ns target, and a maximum batch size of 65,536 operations. Both variants
must use the same prepared input/code objects, return identical checksums, and
produce 40 positive measured durations at the default configuration. Raw pair
observations are replayed by the host Rust `Report` implementation. The
diagnostic empty-entry overhead is measured separately, identified as an
estimate, placed only in the envelope, and never subtracted from observations.

Tests reuse the shared host rank/collector implementation and its independent
bounded sign-enumeration oracle, frozen wide-cutoff vectors, and generated
checks. Guest-driver tests cover event/result equality, exact workload outputs,
negative and excessive operation counts, checksum overflow, fake-clock order,
callback errors, strict report replay, malformed envelopes, and setup versus
timed boundaries. Each package has a downstream consumer check. Native timing
is evidence that each guest runtime executes through its documented boundary;
it is not a speed threshold.
