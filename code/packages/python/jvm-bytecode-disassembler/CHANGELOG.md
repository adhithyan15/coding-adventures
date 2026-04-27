# Changelog

## [0.2.0] — 2026-04-19

Added twenty new opcodes to `JVMOpcode`, `disassemble_method_body`, and
`assemble_jvm` to cover the full instruction set emitted by the
`ir-to-jvm-class-file` backend.

### New opcodes

| Opcode | Value | Description |
|--------|-------|-------------|
| `NOP` | 0x00 | No-operation placeholder |
| `ICONST_M1` | 0x02 | Push integer constant -1 |
| `LDC_W` | 0x13 | Load constant from pool (wide 2-byte index) |
| `IALOAD` | 0x2E | Load int from int array |
| `BALOAD` | 0x33 | Load byte from byte array (sign-extended to int) |
| `IASTORE` | 0x4F | Store int to int array |
| `BASTORE` | 0x54 | Store byte to byte array |
| `POP` | 0x57 | Discard top operand-stack value |
| `ISHL` | 0x78 | Integer left shift |
| `ISHR` | 0x7A | Integer arithmetic right shift |
| `IAND` | 0x7E | Integer bitwise AND |
| `IOR` | 0x80 | Integer bitwise OR |
| `I2B` | 0x91 | Truncate int to signed byte, sign-extend back |
| `IFEQ` | 0x99 | Branch if top of stack == 0 |
| `IFNE` | 0x9A | Branch if top of stack != 0 |
| `IF_ICMPNE` | 0xA0 | Branch if two ints are not equal |
| `IF_ICMPLT` | 0xA1 | Branch if first int < second int |
| `PUTSTATIC` | 0xB3 | Store to static field (2-byte cp index) |
| `INVOKESTATIC` | 0xB8 | Invoke static method (2-byte cp index) |
| `NEWARRAY` | 0xBC | Allocate new primitive array (1-byte type tag) |

### Decode changes (`disassemble_method_body`)

- `ICONST_M1`: handled as a 1-byte instruction with `literal=-1`, inserted
  before the `ICONST_0..5` range check.
- `LDC_W`: 3-byte instruction; reads a 2-byte unsigned constant-pool index.
- `NOP`, `IALOAD`, `BALOAD`, `IASTORE`, `BASTORE`, `POP`, `ISHL`, `ISHR`,
  `IAND`, `IOR`, `I2B`: added to the existing 1-byte no-operand set.
- `IFEQ`, `IFNE`, `IF_ICMPNE`, `IF_ICMPLT`: added to the 3-byte signed-offset
  branch set alongside `IF_ICMPEQ`, `IF_ICMPGT`, `GOTO`.
- `PUTSTATIC`, `INVOKESTATIC`: added to the 3-byte unsigned-cp-index set
  alongside `GETSTATIC`, `INVOKEVIRTUAL`.
- `NEWARRAY`: 2-byte instruction; reads a 1-byte array-type tag stored as
  both `operands[0]` and `literal`.

### Assemble changes (`assemble_jvm`)

- `NOP`, `ICONST_M1`, `IALOAD`, `BALOAD`, `IASTORE`, `BASTORE`, `POP`,
  `ISHL`, `ISHR`, `IAND`, `IOR`, `I2B` added to `one_byte_opcodes`.
- `NEWARRAY` added to `one_byte_operand_ops`.
- `IFEQ`, `IFNE`, `IF_ICMPNE`, `IF_ICMPLT` added to `signed_short_ops`.
- `PUTSTATIC`, `INVOKESTATIC`, `LDC_W` added to `unsigned_short_ops`.

## 0.1.0

- add a standalone version-aware JVM bytecode disassembler package
