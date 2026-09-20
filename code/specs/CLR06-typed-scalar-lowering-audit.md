# CLR06 typed scalar lowering audit

Current main after CLR05 still returns Int(-2147483648) for explicitly i64
2147483647+1; all three locals are int32. Reproduced via lang-aot integration
probe; evidence outside checkout in lang-vm-clr06-baseline.log. No lowering
changes yet. Temporary probe removed.

IIR functions expose parameter type hints and a return type; instructions expose
type hints. lower.rs discards parameter hints in slot allocation and creates
all-int32 local/signature metadata. Constants use ldc.i4, arithmetic loads raw
slots and only applies narrow unsigned masks. Comparisons produce i32; direct
calls need argument/return width agreement. The arithmetic helper in simulator
rejects mixed integer widths, so widening literals alone is insufficient.

Configuration currently has only assembly_name; a new field would affect
external struct literals. Decide whether a separate strict typed entrypoint or
validated selection within existing lowering best preserves legacy untyped and
closure consumers. Do not silently select a wider ABI from one instruction hint.
Map every accepted opcode and consumer before committing the implementation
contract. Any strict scalar slice must define consistent variable assignments,
parameter/return/call signatures, comparison booleans, moves and branch truth.
Unsupported closure/boxing/array paths must not enter that slice accidentally.

Remaining emitted-opcode gap found: lower.rs mod emits raw 0x5d rem while the
simulator has no corresponding handler. This may be refused by a bounded scalar
contract rather than treated as executable. Preserve the wide-immediate/input
gates and structural i32 values until a complete chosen contract is tested.