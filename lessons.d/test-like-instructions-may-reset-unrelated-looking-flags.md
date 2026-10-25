---
category: Rust
---

# Test-like instructions may reset unrelated-looking flags

The first MC6800 `TST` implementation updated N and Z and cleared V, but
incorrectly preserved C. Motorola's programming reference explicitly defines
both V and C as cleared, even though `TST` does not store an arithmetic result.

For historical instruction sets, do not infer flag behavior from an
instruction's name or from a later related architecture. Translate every flag
column from the primary instruction definition, and include a witness that
preloads each always-cleared or always-set flag to the opposite value.
