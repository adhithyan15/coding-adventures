# z80-backend

Zilog Z80 backend for `jit-core` / `aot-core`. Seventh lane of the
9-architecture expansion. Minimal-viable port — covers `const_*` +
`ret_*` only, mirroring `intel8080-backend`'s shape (the Z80 is a
source/binary-compatible superset of the 8080, sharing the same
`LD A, n` / `HALT` (`MVI A, n` / `HLT`) return convention).

## Scope (WORD01)

| CIR family | Status |
|------------|--------|
| `const_u8`, `const_bool` | `LD A, n` |
| `const_u16` | `LD HL, nn` |
| matching `ret_u8`, `ret_bool`, `ret_u16`; `ret_void` | `HALT` (entry-function exit) |
| Anything else | `None` / `BackendError::UnsupportedOp` |

The historical `const_i64`/`ret_i64` byte-sized smoke path remains for
compatibility. Exactly one value may be live: typed returns must match the
current value's width. The observable result ABI is `A` for `u8`/`bool` and
`HL` for `u16`.

`Backend::run` panics — this backend is emit-only. Load the emitted
bytes into `z80-simulator` to execute them.

## Termination-check convention

Whether a real `HALT` has been emitted is tracked via the *shape* of the
CIR walk (does the function end with a `ret_*`/`ret_void` matching the
last `const_*`'s destination var?), never via comparing trailing byte
*values* against the `HALT` opcode (`0x76`). A `const_*` immediate whose
value happens to equal `0x76` (118) is encoded and executed correctly —
see `tests/test_backend.rs::const_value_equal_to_halt_opcode_byte_is_not_misread`.
This sidesteps the bug class a prior lane (Intel 8051) shipped: a
defensive "already terminated?" check that compared trailing byte values
against the halt sentinel instead of tracking whether a real halt
instruction was actually emitted.
