# wasm-opcodes (Python)

Complete WASM 1.0 opcode lookup table — 172 instructions with name, byte value,
category, immediates, and stack effect metadata.

## What it does

This package provides a single source of truth for every WebAssembly 1.0
instruction. You can look up an instruction by its byte value or by its
mnemonic name and get back a structured record with:

- **name** — the text-format mnemonic (e.g., `"i32.add"`)
- **opcode** — the byte value in the binary encoding (e.g., `0x6A`)
- **category** — the instruction group (`"numeric_i32"`, `"control"`, etc.)
- **immediates** — tuple of immediate operand type names encoded after the byte
- **stack_pop** — how many values this instruction consumes from the stack
- **stack_push** — how many values it produces onto the stack

## Where it fits in the stack

```
wasm-types      — type system (ValueType, FuncType, etc.)
wasm-leb128     — variable-length integer encoding
wasm-opcodes    ← you are here (instruction metadata table)
wasm-parser     — binary decoder that uses all three layers above
```

## Installation

```bash
pip install coding-adventures-wasm-opcodes
```

Or from source (within the monorepo):

```bash
cd code/packages/python/wasm-opcodes
uv pip install -e .
```

## Usage

```python
from wasm_opcodes import OPCODES, OPCODES_BY_NAME, get_opcode, get_opcode_by_name

# Look up by byte value
info = get_opcode(0x6A)
print(info.name)        # "i32.add"
print(info.category)    # "numeric_i32"
print(info.stack_pop)   # 2
print(info.stack_push)  # 1

# Look up by mnemonic
info = get_opcode_by_name("memory.grow")
print(info.opcode)       # 64  (= 0x40)
print(info.immediates)   # ('memidx',)

# Unknown byte/name returns None
assert get_opcode(0xFF) is None
assert get_opcode_by_name("not_real") is None

# Direct dict access (raises KeyError for unknown entries)
info = OPCODES[0x6A]
info = OPCODES_BY_NAME["i32.add"]
```

## Opcode categories

| Category | Description | Example |
|----------|-------------|---------|
| `control` | Structured control flow | `block`, `br`, `call` |
| `parametric` | Type-agnostic stack ops | `drop`, `select` |
| `variable` | Local/global access | `local.get`, `global.set` |
| `memory` | Loads, stores, size/grow | `i32.load`, `memory.grow` |
| `numeric_i32` | 32-bit integer arithmetic | `i32.add`, `i32.lt_s` |
| `numeric_i64` | 64-bit integer arithmetic | `i64.mul`, `i64.eqz` |
| `numeric_f32` | 32-bit float arithmetic | `f32.sqrt`, `f32.copysign` |
| `numeric_f64` | 64-bit float arithmetic | `f64.div`, `f64.nearest` |
| `conversion` | Type conversions | `i32.wrap_i64`, `f64.promote_f32` |

## Stack effects explained

WASM is a *stack machine*. Instructions communicate via an implicit operand
stack rather than named registers. The `stack_pop` and `stack_push` fields tell
you how many values each instruction takes from and gives back to that stack.

For example, `i32.add` pops two i32 values and pushes their sum (one i32):

```
Before: [ a (i32) | b (i32) ]   ← b is on top
After:  [ a+b (i32) ]
```

## Immediates explained

Immediates are values encoded directly in the binary instruction stream, after
the opcode byte. They are *not* the operand stack values — those are the pop/push.

| Immediate token | What it encodes |
|-----------------|-----------------|
| `"i32"` | Signed LEB128 32-bit constant |
| `"i64"` | Signed LEB128 64-bit constant |
| `"f32"` | 4-byte IEEE 754 float |
| `"f64"` | 8-byte IEEE 754 double |
| `"memarg"` | align (LEB128) + offset (LEB128) |
| `"labelidx"` | Branch target depth (LEB128) |
| `"vec_labelidx"` | Vector of label indices (br_table) |
| `"blocktype"` | Block result type byte |
| `"funcidx"` | Function index (LEB128) |
| `"localidx"` | Local variable index (LEB128) |
| `"globalidx"` | Global variable index (LEB128) |
| `"memidx"` | Memory index, always 0 in WASM 1.0 |
| `"typeidx"` + `"tableidx"` | Two indices for `call_indirect` |

## Development

```bash
cd code/packages/python/wasm-opcodes
uv venv
uv pip install -e ".[dev]"
uv run python -m pytest tests/ -v
uv run ruff check src/ tests/
```

## Dependencies

- Python 3.12+
- No runtime dependencies
