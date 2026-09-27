# BEAM11 — default reads for ets-backed arrays

**Status:** implementation contract for VM-071
**Package:** `code/packages/rust/iir-to-beam`
**Depends on:** BEAM10's `[Table | Length]` array handle

## Observed defect

`array<f64>` and `array<str>` use an Erlang `ets` table. `ets:new/2` does not
populate its keys, so `ets:lookup_element/3` raises `badarg` for an in-range
element that has never been written. The corresponding IIR array starts with
zero values. Six sparse ALGOL array-by-value programs reach this mismatch on
real Erlang (VM-071, issue #15880).

## Contract

For an `array<f64>` of declared length `N`, a read at an integer index in
`0..N` returns the stored value, or `0.0` if that index has never been written.
For `array<str>`, the unwritten value is the empty Erlang character list `[]`.
An index below zero or at least `N` still raises `badarg`. `array_len` remains
the declared length, independent of the number of written elements. Repeated
writes retain last-write-wins behavior. The integer-array `:atomics` path is
unchanged.

## Lowering

BEAM10's handle is `[Table | Length]`. `array_get` already extracts `Table`;
retain `Length` in a scratch register. Before looking up the key, emit guards
for `Index >= 0` and `Index < Length`, branching to the existing
`erlang:error(badarg)` trap on failure. On success, call
`ets:lookup_element(Table, Index, 2, Default)` with arity 4, where `Default`
is `0.0` for `f64` or `[]` for `str`. OTP 27, the CI runtime, supports this
function. It returns the stored value when the key exists and the default
only when the key is absent.

This keeps allocation O(1) and adds no imported calls to `alloc_array`.
Bulk pre-population would make every float/string array allocation O(N) and
would have to preserve the newly created table through several GC-capable
calls; BEAM10's earlier table-handle failure demonstrates that risk.

## Acceptance

1. Execute direct `array<f64>` and `array<str>` IIR modules on real `erl`:
   in-range unwritten reads return the typed default, and written values
   still round-trip.
2. Execute negative and upper-bound reads and assert a `badarg` trap.
3. Retain BEAM10's declared-length and sparse-array tests, then execute the
   six affected ALGOL programs on real `erl` and report the exact before/after
   count. A separate ALGOL owner decides when to declare BEAM matrix cells.
4. Run `iir-to-beam` tests and Clippy; retain the non-ALGOL matrix's existing
   BEAM cells.

`array_set` range enforcement is outside this change; frontends currently
emit their own bounds checks. Audit direct out-of-range stores separately if
the new read tests expose a reachable language-level discrepancy.
