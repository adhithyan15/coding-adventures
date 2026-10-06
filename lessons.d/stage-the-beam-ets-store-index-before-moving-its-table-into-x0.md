---
category: Rust
---

# Stage the BEAM ETS store index before moving its table into x0

VM-073's real-Erlang x0-index regression exposed a store that returned an
unwritten `0.0` after `array_set(p, i, 42.0)` even though `i = 1` was valid.
The ETS tuple path staged `i` in a scratch register, but before `test_heap` it
moved the table to x0 and then copied the **original** index register to x1.
When the index was a function parameter in x0, that second move copied the
table instead. The valid store silently wrote the wrong key. Move the staged
index into x1; use original operand registers only before any destination
move can overlap them. Exercise a real-Erlang parameter-index round-trip,
because literal-index tests usually allocate the source outside x0.
