---
category: Rust
---

# Inspect backend register and opcode helpers before adding a lowering arm

A BEAM getchar implementation initially used a guessed `FnMeta::reg` method and
an undefined `OP_GET_HD` constant. The build rejected both. Existing arms use
`var_reg!`; a registered `erlang:hd/1` BIF supplies checked list extraction without
adding a new opcode. Inspect adjacent lowering arms and opcode definitions first,
then compile before expanding runtime validation.

The first all-byte probe unrolled 256 reads and hit the existing 255-variable
backend limit. Use a bounded tape-counter loop to test stream values and liveness
without accidentally testing register-capacity expansion.
