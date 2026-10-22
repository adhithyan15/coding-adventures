---
category: Security boundaries
---

# A permissive IR parser can construct COMMENT opcodes without comment text

The Go strict BF03 facade initially assumed every legacy `OpComment` came from
a semicolon line with one `IrLabel` payload. Independent review showed ordinary
`COMMENT` opcode lines also reach that enum: zero operands caused an index
panic, and register/immediate operands caused a type-assertion panic.

Check arity and dynamic type before projecting native comments to strict text,
then require exact canonical reprinting to reject opcode-form aliases. Keep
regressions for missing, numeric, register, multiple and label operands. The
actual failing test reproduced the panic before the guarded fix.

The same review caught an unbounded `os.ReadFile` before a bounded JSON reader.
Bound the actual file read before hashing or decoding; a LimitReader applied
only after bytes are already materialized does not enforce the entry budget.
