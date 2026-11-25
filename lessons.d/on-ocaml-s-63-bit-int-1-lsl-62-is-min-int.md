---
category: Testing & coverage
---

# On OCaml's 63-bit int, 1 lsl 62 is min_int, so an upper-bound guard written with it accepts everything

A native OCaml `int` has 63 bits, so `max_int` is `2^62 - 1` and `1 lsl 62` wraps to
`min_int`. The OCaml PBT engine's `Prng.bounded_int` guarded `n >= 1 lsl 62`, which is
`n >= min_int` and therefore rejected every call; the first test of the pinned rejection draw
failed with `Argument_error`. The fix was to drop the meaningless upper bound (every native
`int` already fits a 64-bit word).

Do differently: never spell a 63-bit limit as a shift; use `max_int`, or compare in `Int64`.
When a bound only exists to protect a conversion, check whether the type already guarantees it.
