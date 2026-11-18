# Portable input action frame v1 (GBL-050)

## Purpose and boundary

An input action frame is the deterministic, caller-ticked reduction of one
player's prior logical levels and an ordered batch of normalized action
changes. It gives a simulation a stable button/axis snapshot for one tick.

The v1 reducer is pure. It does not poll an operating system, hold device or
window handles, consult a host clock, assign or increment ticks, or bind
physical keys and controls to actions. It does not perform multi-device
aggregation, focus-loss or disconnect releases, text input, relative pointer
motion, device quantization, or replay/version management. Those are distinct
contracts and must not be inferred from this frame primitive.

The primitive is the Rust package `game-input-frame`. It is a dependency of
simulation code, not a replacement for `window-core`, `native-event-core`, or
`event-loop`: those packages describe window events, native readiness, and
event dispatch. The historical UI11b roadmap covers host polling/cadence and
gamepad integration; this primitive starts after host input has been normalized.

## Stable v1 identifiers and values

- `PlayerId` is an opaque unsigned 32-bit value. Every value from zero through
  `u32::MAX` is valid; the reducer does not allocate players.
- Button action IDs are the stable slots `0..=63`. The slot is the bit index in
  the button masks. Bit 63 is part of the public contract.
- Axis action IDs are the stable slots `0..=15`.
- Axis levels are signed integers in the symmetric range `-32767..=32767`.
  `-32768` is invalid. Axis values are absolute levels, not deltas.
- A frame carries the caller-assigned `u64` simulation tick unchanged. Every
  value, including `u64::MAX`, is accepted. This API does not require adjacent
  ticks and never increments one.
- A frame accepts at most 256 ordered changes. The reducer performs bounded
  work over that sequence and fixed-size state; it does not allocate while
  reducing a frame.

The fixed action slots are a v1 profile, not a claim that games need only 64
buttons or 16 axes. Larger layouts, banks, and layout identity/versioning need
a separate explicit contract.

## Inputs, result, and reduction

The previous state contains a player ID, a 64-bit held-button mask, and exactly
16 axis levels. A change is one of:

- set button slot `b` to down or up;
- set axis slot `a` to an absolute value.

Changes are processed in slice order. The resulting frame contains the same
player ID, supplied tick, final held-button mask, pressed and released edge
masks, and all 16 final axis levels.

For each button transition, a false-to-true change sets that slot in
`pressed_buttons`, and a true-to-false change sets it in `released_buttons`.
The masks summarize whether at least one such transition occurred anywhere in
the frame. They do not preserve intra-tick order or repeated-tap counts; callers
that need those facts must retain the ordered changes or use a future
ordered/versioned replay contract. A down/up tap from an initially-up level
sets both edge bits even though the final held bit is clear. Repeating the
current level produces no edge. Axis values persist until another change sets
that axis. An empty change list preserves held buttons and axes while both edge
masks are zero.

All outputs are computed in private local state. The public function borrows
the prior state and changes immutably. On any error it returns no partial frame
and leaves both inputs unchanged.

## Validation and first-error precedence

Validation order is observable and fixed:

1. If the change count exceeds 256, return `TooManyChanges`. This check happens
   before reading or validating state fields and before inspecting any change.
   The error carries no host-sized count; the v1 limit is fixed.
2. Validate prior axis values in ascending axis-slot order. The first value
   outside `-32767..=32767` returns `InvalidPreviousAxisValue` with the slot and
   signed value.
3. Validate and reduce changes in sequence order. For each change, validate
   its action slot before validating its value. Invalid button slots return
   `InvalidButtonActionId`; invalid axis slots return `InvalidAxisActionId`;
   invalid axis values return `InvalidAxisValue`. Event indices are zero-based
   and at most 255, so they fit `u8` in every supported implementation.

All error payloads have fixed-width portable fields: action/axis slot and event
index are `u8`; axis values are signed `i16`. No `usize`, pointer, native enum
layout, or host-width value is part of the portable result. Player IDs, button
mask bits, and ticks have no invalid values.

## Independent vectors

`fixtures/game-input-frame-v1/cases.json` is the canonical, language-neutral
case set. It uses decimal strings for all `u64` ticks and button masks, JSON
integers for `u32` player IDs, slots and signed axis values, and exact error
codes and payloads. `schema.json` fixes the wire shape. `validate.py` reads the
original fixture bytes before parsing, rejects unknown/missing fields and
duplicate IDs, checks the full order-independent case-ID set, and recomputes
expected results with an independent Python reference model.

Successful and rejected vectors are both required. The corpus pins empty
frames, held-level persistence, duplicate same-level events, press/release,
same-tick taps and repeated cycles, axis persistence and replacement, slot 63,
`u32::MAX`, `u64::MAX`, the exact 256/257 event boundary, invalid state and
action values, validation precedence, and refusal without a partial result.
Consumers must compare complete outputs and exact error payloads; merely
parsing the fixture or observing a nonempty test count is not conformance.

## Evidence and claim limits

The Rust package must include focused unit and boundary tests, exact public-API
fixture consumption, and library line coverage above 95%. Seeded property tests
use an independently coded array/count reference surface and record seed,
strata, and count. Recorded bounded fuzz campaigns exercise both the typed
reducer boundary and the fixture decoding boundary. Negative controls run
through the same comparisons and must detect always-refuse, endpoint-only
edges, and dropped high bits.

A bounded SAT miter encodes actual 64-button masks and ordered transitions,
proves mismatch UNSAT for the specified edge reducer, and requires SAT mutants
for endpoint-only edge detection and high-bit loss. Each SAT witness is decoded
and replayed against the real Rust public implementation. The encoded bound is
reported; no unbounded reducer claim follows from it.

Where Warrant's Rust frontend supports it, parse and lower the exact small
production scalar helper used by the reducer, prove that helper against an
independent bit-level predicate, and falsify a mutant with native witness
replay. The evidence must bind the exact source path/hash and function. A
handwritten predicate or model run is not a production-source binding. This
helper proof does not prove the record-based reducer, fixture decoder, or whole
crate. Whole-reducer universal proof, source extraction beyond the helper,
zero-allocation/constant-time claims, device aggregation, host behavior, and
cross-language parity remain open unless separately established.

## Follow-up contracts

Keep separate backlog items for:

- TypeScript parity after the Rust v1 contract stabilizes;
- Go SAT-solver port first when the next package-parity lane is selected;
- every missing established implementation lane and every missing emerging
  implementation lane, with their inventories reported separately;
- device binding and deterministic multi-device aggregation;
- caller-owned tick assignment, event ordering, and ordered/versioned replay;
- focus-loss, disconnect, and device-removal release semantics;
- text and relative-pointer input;
- quantization and richer action layouts/banks.

Package presence in the parity inventory is not behavioral parity.
