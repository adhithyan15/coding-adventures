---
category: Compiler / VM / language pipeline
---

# A BEAM gc_bif1 length Live operand is a clobber line: x[Live] to x[Live+2] are overwritten

LANG83 PR5a added a string-ceiling check in front of iir-to-beam's
`str_concat`: `length(left) -> x[live]`, `length(right) -> x[live+1]`, then
`'+'`. Every real-`erl` test that concatenated died with `badarith
erlang:'+'("K", 1)` -- the right-hand string plus its own length. The unit
tests, which only pinned the instruction stream, all passed.

`length/1` can yield on a long list, so since OTP 22 the loader rewrites
`gc_bif1 Fail Live erlang:length/1 Src Dst` into

    i_length_setup Live Src   ; x[Live] = Src, x[Live+1] = 0, x[Live+2] = Src
    i_length Fail Live Dst    ; walk, maybe trap and resume, then write Dst

Those three registers are its save area and are written before `Dst`. Both
calls said `Live = live`, so the second setup stored `right` over the first
result. The fix was to give the second call `Live = s_la + 1`: the first
result is live, so say so, and the save area moves above it.

What to do instead:
- Treat `Live` on a `gc_bif1 length` as "everything from x[Live] to x[Live+2]
  is destroyed", not as a GC hint. Any value needed afterwards must sit below
  `Live` or be written after the call.
- Reserve `Live + 3` registers in the scratch-register guard.
- Pin it without `erl`: `iir-to-beam/tests/lang83_pr5a.rs`'s
  `length_save_area_never_holds_a_value_still_needed` scans the straight-line
  code after every `length` for a read of a clobbered register.
- Verify against the CI's OTP (27.x). Ubuntu's apt `erlang-base` is OTP 25,
  which refuses iir-to-beam's modules at load time (`code_server` DOWN), so
  every real-`erl` test fails there for an unrelated reason.
