# HOS03d — Go semantic OS simulator slice

## Scope

This Go package implements the HOS03 semantic event-machine boundary as a
standalone, dependency-free library. It provides profile reducers, JSON-only
stimuli and effects, bounded `Step` and `Run`, logical ticks, snapshots,
restore, and history replay. Its two synthetic profiles are the batch monitor
and protected message scheduler described by HOS03. It does not implement a
Unix guest, CPU, bus, device, board, hardware timing, or VOS integration.

The first publishable slice is the pure library and its inline unit tests. The
stdout demonstration command is omitted from this package so it requires no
host filesystem or stdout capability. A demo can be reconsidered later as a
separate program with its own permission manifest.

## Portable fixture conformance

`profiles.json` in `code/specs/fixtures/os-architecture-v1` is the sole
conformance fixture for this slice. The package BUILD runs a fixture-adjacent
Go verifier that imports the public package API and reads this file outside
the package tree. The verifier checks both shared profiles, their budgets,
final ticks, states, and ordered effects, then compares snapshot restore and
history replay continuations. The package's own tests stay inline and do not
read repository files.

The parity claim is limited to these profile transitions, budgets, traces, and
snapshot/replay behavior for fixture schema version 1. It does not claim
complete behavioral, error-message, resource-limit, or serialization API
parity with TypeScript.

The library payload type is native JSON data: `nil`, booleans, strings, finite
numbers, `[]any`, and `map[string]any`. Inputs and profile outputs are copied
into canonical JSON-shaped values before they enter machine state. Unsupported
Go values, non-finite numbers, cycles, values deeper than 64 levels, more than
100,000 nodes, or string/key/aggregate text beyond the HOS03 limits (counted as
UTF-16 code units) are rejected. A run budget is bounded to at most 1,000,000
transitions per call. Go returns Go errors; their text and exact boundary
handling are not cross-language ABI guarantees. In particular,
`encoding/json` fixture loading is not a strict duplicate-key JSON parser.

## API boundary

`Profile` owns initial state and deterministic transition policy.
`Machine` owns queue order, virtual time, event stamping, bounded stepping,
and replay. A failed transition is committed only after returned state and
effects have passed JSON validation. Snapshots record schema version, profile
ID, tick, state, queued stimuli, and processed history; `Restore` validates by
replaying history before restoring pending stimuli. The library has no host
I/O. Arbitrary callbacks, hardware, and guest instruction execution are
outside this package.

## Build and evidence

The Linux and Windows BUILD fronts run package tests, `go vet`, and the
fixture-adjacent conformance verifier. CI pins Go 1.26.1; the package declares
`go 1.26` and depends only on the standard library.
