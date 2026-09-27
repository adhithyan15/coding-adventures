# z80-backend

Zilog Z80 backend for `jit-core` / `aot-core`. Seventh lane of the
9-architecture expansion. The initial `const_*` + `ret_*` port mirrored
`intel8080-backend`'s shape (the Z80 is a
source/binary-compatible superset of the 8080, sharing the same
`LD A, n` / `HALT` (`MVI A, n` / `HLT`) return convention).

## Scope (WORD02)

| CIR family | Status |
|------------|--------|
| `const_u8`, `const_bool` | `LD A, n` |
| `const_u16` | `LD HL, nn` |
| matching `ret_u8`, `ret_bool`, `ret_u16`; `ret_void` | `HALT` (entry-function exit) |
| `add`, `sub`, `and`, `or`, `xor` on `u8` and `u16` | two live values, wrapping result |
| Other operations | `None` / `BackendError::UnsupportedOp` |

The historical `const_i64`/`ret_i64` byte-sized smoke path remains for
compatibility. Up to two values of the same width may be live. Byte values
occupy `A`/`D`; word values occupy `HL`/`DE`. The liveness pass reuses a dead
slot for each result and explicitly rejects a third live value or simultaneous
byte/word values. Word addition uses `ADD HL,DE` when that pair layout applies;
the other word operations propagate carry/borrow across low and high bytes.
Typed returns must match the value's width. The observable result ABI remains
`A` for `u8`/`bool` and `HL` for `u16`.

`Backend::run` panics — this backend is emit-only. Load the emitted
bytes into `z80-simulator` to execute them.

## Termination-check convention

Whether a real `HALT` has been emitted is tracked by an explicit flag
set by `ret_*`/`ret_void` and reset by later value-producing instructions,
never by comparing trailing byte
*values* against the `HALT` opcode (`0x76`). A `const_*` immediate whose
value happens to equal `0x76` (118) is encoded and executed correctly —
see `tests/test_backend.rs::const_value_equal_to_halt_opcode_byte_is_not_misread`.
This sidesteps the bug class a prior lane (Intel 8051) shipped: a
defensive "already terminated?" check that compared trailing byte values
against the halt sentinel instead of tracking whether a real halt
instruction was actually emitted.
