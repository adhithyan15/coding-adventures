# GBL-050a TypeScript input-frame binding and proof/cache acceptance

This companion narrows the TypeScript boundary and delivery evidence for
[GAME input action frame v1](GAME-input-action-frame.md). It does not replace
that normative reducer contract. The selected implementation must expose the
full reducer and canonical vector surface in the TypeScript package; source
proof and cache integration are development-time facilities and add no runtime
Rust or npm dependency.

## Public TypeScript boundary

Use an exported pure reducer with typed immutable records for prior state,
ordered changes, result, and the five portable frame errors. Preserve every
Rust raw-domain value that fits the wire representation until reducer
validation: button and axis slots are unsigned byte values, and axis values
are signed 16-bit values. Thus a raw slot outside the action profile and
`-32768` are semantic reducer errors, not JavaScript binding-width errors.
Ticks and masks are `bigint` in the complete unsigned 64-bit interval, player
IDs are integer `number`s in the complete unsigned 32-bit interval, and button
levels must be primitive booleans. Do not coerce strings, boxed primitives,
booleans-as-numbers, or objects with conversion hooks.

The JS binding raises `TypeError` for wrong runtime/container/value types and
`RangeError` for non-finite, fractional, or out-of-width numeric values. It
accepts integer `-0` as zero for Number-backed fields and canonicalizes it to
positive zero before output or error construction, matching integer semantics
and Rust's zero value. BigInt fields reject Number values, including `-0`.
Semantic profile violations and ordering/length violations use the portable
`FrameError` result, not host exceptions.

Binding and semantic validation preserve observable order without eagerly
walking input records:

1. Require the changes container to be an array (`TypeError` otherwise), then
   check its length. More than 256 yields `TooManyChanges` before reading any
   previous-state, tick, or element property.
2. Bind the tick as u64 `bigint`; bind the previous-state record, player ID,
   and held mask in that order; then require an axes array of length exactly
   16. Wrong array type is `TypeError`, wrong length is `RangeError`. Read
   axes by own index in ascending order: a hole is `TypeError` even if an
   inherited array value exists, and a present value with the wrong runtime
   type is `TypeError`. A present integer outside i16 width is `RangeError`;
   return the first in-width but semantically invalid prior axis as
   `InvalidPreviousAxisValue`.
3. Visit changes in index order. Bind only the current element. For a button
   or axis change, validate slot width/type and semantic slot range before
   reading or validating `down`/`value`; then return the corresponding first
   portable error. Do not map, spread, destructure, or otherwise pre-read all
   events before reducing them. In particular, a failure at one index must
   not access a later element or field, including a throwing accessor.

The public API has a discriminated success/error result whose portable error
codes and fields are exactly `TooManyChanges`; `InvalidPreviousAxisValue`
(`axis_slot`, `value`); `InvalidButtonActionId` (`change_index`, `action_id`);
`InvalidAxisActionId` (`change_index`, `action_id`); and `InvalidAxisValue`
(`change_index`, `axis_slot`, `value`). The result, error payload, frame, and
axes are fresh frozen values. Inputs are never frozen or modified. Returned
axes and nested records cannot alias caller-owned mutable arrays/objects. Any
late failure is atomic and returns no partially reduced frame. Exact payload
field names and wire shape must match the shared 21-case corpus and Rust public
errors.

## Production-source proof binding

The consumer owns the exact production `src/digital.ts` bytes and digest used
by the reducer. Extend the generic `typescript-warrant-ir` development tooling
with a reusable CLI/example as needed; do not put game-specific
`include_bytes!` tests into the generic bridge and do not substitute a copied
formula or scalar model for source lowering. The tool receives the actual
consumer path/bytes through a declared package input, parses with the real
`ts5.8` grammar, lowers the actual AST, and records source path, byte count,
SHA-256, helper declarations, parser/lowerer version, WIR inputs, solver, and
query result. It must fail on digest/path mismatch, unsupported syntax,
parser/lowering errors, or unexpected SAT verdicts. Production helper proof
claims are bound to these exact bytes; the bridge fixture remains fixture-only.

For each of the three helpers, prove the healthy lowered source against an
independent per-bit/mux reference over all 64-bit masks and slots `0..63`.
The WIR must preserve the actual source operator tree and the agreed unsigned
BigInt restrictions. Keep helper equivalence separate from the bounded
record-based reducer miter; neither implies a whole-reducer source proof.

Guarded scalar-source mutant queries must model the reducer call-site
conditions: for set, include a press only when the selected prior bit is zero;
for clear, include a release only when it is one. Include admitted mutants
for `!==`→`===` (down while clear), set `|`→`&`, and clear-mask `~` removal.
The solver's symbolic inputs are the raw mask and slot; label any fixed tick,
player, button-level, or axes chosen by the witness replay as reachability
harness constants, not solver-proved variables. Require SAT for each expected
mutant, decode the full-width witness, then replay the unchanged prior mask,
slot, button level, tick, and exactly 16 zero axes through the production
public reducer. Compare the entire healthy and mutant frames against an
independent oracle. Execute the exact healthy and mutated helper source bytes
through a public-reducer harness that binds each helper variant, and compare
both full results; the mutated witness must reach the guarded call site. A
validation refusal, rejected mutant syntax, UNKNOWN, or unexpected UNSAT is a
failed control, never a mutation kill.

Separately retain the bounded reducer miter with its actual encoded state and
transition bound, independent Boolean oracle, endpoint-only edge and
high-bit-loss mutants, and public witness replay. Report helper equivalence,
guarded scalar-source mutation queries, bounded reducer miter, and public
replay as distinct evidence layers. None is a whole-reducer source proof.

## Build selection and effective cache invalidation

The proof tool is a development-time input to this consumer. Use the existing
repository build-tool graph and cache conventions; a metadata annotation or a
toolchain comment alone is not an edge. The actual diff-selection plan must
select the TypeScript consumer when the proof CLI, its invocation, or any
transitive parser/WIR/solver source changes. The persisted cache must also
include those same transitive inputs when deciding whether the consumer is a
cache hit.

The Go executor's persisted-cache path hashes package inputs and dependency
hashes in the package loop; `HashDeps` can observe an empty hash for a
dependency discovered later. The proof closure therefore must be hashed from
the complete unfiltered package discovery in two passes, or by an equivalent
bounded mechanism that demonstrably gives the consumer the closure hash.
Language filtering (including `--language typescript`) must retain a stable
hash for the proof closure even when proof packages are filtered out of the
execution list. Do not mistake diff mode for cache evidence: when an affected
set is supplied the executor bypasses persisted caching.

Acceptance uses the real build-tool resolver, selector, executor, and hasher
paths, with a temporary or isolated consumer package and full restoration:

- prove changed proof-tool inputs select the consumer in the diff plan;
- with a warm persisted cache, verify unchanged inputs hit and a same-path,
  same-length direct tool change misses. Because the cache records the last
  successful digest, restoration must recover the original hash and then
  produce a hit on an unchanged follow-up run (or use an isolated copy of the
  original warm cache for the restore probe);
- change a transitive solver/parser/WIR input that sorts after its consumer
  and prove a cache miss, then restore it and prove the original hit;
- repeat the persisted-cache assertion with TypeScript-only language filtering
  so filtered proof packages cannot erase the consumer's closure hash;
- preserve an existing package-local `.ts` or declared BUILD input control to
  show unrelated source selection and hashing remain intact.

Record exact selected package rows, cache-key/hash transitions, command,
restore checks, platform, and source revision. Do not claim effective cache
invalidation from registry synchronization, graph annotations, or a unit test
of a selector alone.

## Consumer assurance and delivery

The consumer uses all 21 canonical vector IDs and the fixture SHA-256
`8db2c0fdc9bb6ab93785f3bd6d5dc52183b67682ca0e8d693fcdbfaa30aff785`, compares
complete outputs and exact portable error payloads, and
keeps JSON decoding errors separate from reducer errors. Retain the Rust
contract's exhaustive 256 raw slot IDs and all 65,536 raw i16 values at each
of 16 axis positions through prior-state and update paths (2,097,152 core
calls, plus explicit count/index/precedence controls). Use an independent
Boolean `[64]` and 16-axis oracle; do not reuse production bit helpers.
Property tests must generate bounded variation, retain seeds and actual call
counts, and shrink failures. Run the existing TypeScript fuzz engine with
valid-biased typed traces plus a separate malformed JSON-boundary campaign.
SAT controls must be non-vacuous and replay through the public API.

The library line coverage claim is strictly above 95%, with instrumentation scope and
excluded tooling clearly stated. Provide README, changelog, package metadata,
capability classification, BUILD and BUILD_windows, and canonical package
inventory updates. Run the repository's effective strict preflights, not only
declared BUILD commands. No browser, OS polling, key binding, device merging,
focus handling, text input, relative pointer, quantization, or tick assignment
is added here.
