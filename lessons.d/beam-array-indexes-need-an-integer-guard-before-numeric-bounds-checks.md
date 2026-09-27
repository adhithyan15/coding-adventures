---
category: Rust
---

# BEAM array indexes need an integer guard before numeric bounds checks

BEAM11 initially checked `Index >= 0` and `Index < Length` before an
`ets:lookup_element/4` read. Erlang orders numeric values across integer and
float types, so a fractional key such as `1.5` passed both comparisons and
received the unwritten-cell default. The security review caught this before
push. Emit `is_integer` before the range guards, and execute a malformed
direct-IIR fractional-index case on real Erlang to prove it raises `badarg`.
When a backend assumes an IIR operand type, validate the assumption at the
machine boundary before a permissive runtime operation can mask it.
