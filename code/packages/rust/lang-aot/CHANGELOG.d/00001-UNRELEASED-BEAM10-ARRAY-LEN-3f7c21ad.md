## Unreleased — real-`erl` coverage for `array_len` on BEAM (BEAM10)

New `tests/beam_array_len.rs`, following the `beam_cons.rs` convention: every
case compiles ALGOL 60 to `.beam` and **runs it on a real `erl`**, skipping
when `erl` is absent.

Running rather than merely lowering is the point. `iir-to-beam`'s `array_len`
has to choose between two array substrates that report length differently —
`:atomics`, which knows its declared extent, and `:ets`, which does not and
whose `ets:info(Tab, size)` counts *inserted entries*. The wrong choice emits
perfectly valid code that returns a wrong number, so a test asserting only
"compiles without error" would pass against an implementation reporting `3`
for a ten-element array with three cells written.

Five cases: an integer array (atomics), a sparsely-written real array (ets,
where the highest index is only reachable if the declared length was recorded
at allocation), both substrates in one module, a loop that exercises the
bounds-check path repeatedly, and an array passed **by value**.

That last one is the sensitivity test. Call-by-value is the only construct in
which `array_len`'s value is *observed* rather than compared — it becomes the
copy count in `emit_array_value_copy`, so a too-small length drops elements and
a too-large one reads past the end. Everywhere else the length only feeds a
bounds check, which is one-sided: a length that is too LARGE sails through it
silently. Without a by-value case the suite would be insensitive in exactly
that direction.

`beam_array_len` is added to the `BUILD` test invocation in the same change. A
file under `tests/` that no CI filter names is a file CI never runs — the gap
VM-062 was opened for.

Measured effect of the backend change this covers: across all 292 ALGOL matrix
rows compiled to `.beam` and executed on OTP 27, programs running correctly
went from 254 to 277, with the 31 previous `array_len` compile refusals falling
to zero and no previously-passing program changing behaviour.
