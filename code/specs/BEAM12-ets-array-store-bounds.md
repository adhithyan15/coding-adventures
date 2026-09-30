# BEAM12 — bounds checks for ets-backed array stores

**Status:** implementation contract for VM-073
**Package:** `code/packages/rust/iir-to-beam`
**Depends on:** BEAM10's `[Table | Length]` handle and BEAM11's read guards

## Observed defect

The `array_set` lowering for `array<f64>` and `array<str>` extracts the table
from `[Table | Length]` and calls `ets:insert/2` with the supplied index.
Unlike `:atomics:put/3`, an ETS table has no declared extent and accepts
negative, upper-bound, and noninteger keys. Direct IIR can therefore store
outside the array without raising an error. Dartmouth BASIC's `LET A(i)` and
`READ A(i)` paths emit `array_set` after flattening the subscript, without an
independent range guard, so the discrepancy is reachable from a frontend.

## Contract

For an ets-backed array of declared length `N`, `array_set` accepts only an
integer `Index` with `0 <= Index < N`. It stores the supplied value at that
key, including overwrites. A negative, upper-bound, or noninteger index
raises `badarg` before any ETS insertion. `array_len` stays `N`, and the
integer-array `:atomics` path is unchanged. This matches BEAM11 `array_get`
and the reference VM's bounds-checked array store.

## Lowering

Retain both fields of `[Table | Length]` until the store's index is checked.
Use `is_integer`, `is_ge Index 0`, and `is_lt Index Length`, each failing to
one `erlang:error(badarg)` block. Emit these guards before the heap reservation,
tuple construction, or imported ETS call. The length scratch register can
then be reused for the temporary `[Index, Value]` list and tuple. No value
crosses an imported call solely to perform the guard.

The store already stages its sources above the live SSA register set. Before
`test_heap`, move the staged index into x1, since moving the table into x0
first would overwrite an index whose original source was x0. Restore the
staged values after the heap test for tuple construction.

## Acceptance

Execute direct IIR modules on real Erlang for both `f64` and `str` stores.
Assert that an in-range write and overwrite round-trip, and that negative,
upper-bound, and fractional indexes raise `badarg`. Retain existing BEAM10
and BEAM11 array regressions. Execute an in-range store whose index arrives
in x0 to prove heap preparation preserves it. Run the `iir-to-beam` package
tests and Clippy.
