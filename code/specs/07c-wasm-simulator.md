# 04c — WASM Simulator

## Overview

The WASM (WebAssembly) simulator implements a minimal subset of the WebAssembly instruction set. WASM is a modern stack-based virtual machine designed to run in web browsers — it is the newest standard in this repo, contrasting with the Intel 4004 (1971) and RISC-V.

WASM is interesting because it is a **stack-based ISA** (like our bytecode VM) rather than a register-based ISA (like RISC-V and ARM). This means the same `x = 1 + 2` program compiles to very different instruction sequences depending on the target.

This is an alternative Layer 4 alongside ARM and RISC-V.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

## Why WASM?

- **Stack-based** — contrasts with register-based RISC-V/ARM, shows how the same program maps differently
- **Real standard** — runs in every browser, used by Rust/C++/Go for web targets
- **Simple encoding** — instructions are single bytes (opcodes), very easy to decode
- **Modern** — designed 2015-2017, no historical cruft

## MVP Instruction Set (for `x = 1 + 2`)

| Opcode | Hex | Instruction | Stack effect | Description |
|--------|-----|-------------|-------------|-------------|
| 0x41 | i32.const n | → n | Push 32-bit integer |
| 0x6A | i32.add | a, b → result | Pop two, push sum |
| 0x21 | local.set idx | value → | Pop value, store in local |
| 0x20 | local.get idx | → value | Push local's value |
| 0x0B | end | — | End of block/function |

The program `x = 1 + 2`:
```wasm
i32.const 1    ;; push 1
i32.const 2    ;; push 2
i32.add        ;; pop both, push 3
local.set 0    ;; store in local 0 (x)
end
```

Note: this looks almost identical to our bytecode VM! That's because both are stack machines.

## Public API

```python
class WasmSimulator:
    def __init__(self, num_locals: int = 16) -> None: ...

    @property
    def stack(self) -> list[int]: ...

    @property
    def locals(self) -> list[int]: ...

    def load_program(self, bytecode: bytes) -> None: ...
    def step(self) -> WasmInstruction: ...
    def run(self, max_steps: int = 10000) -> list[WasmInstruction]: ...

@dataclass
class WasmInstruction:
    address: int
    opcode: int
    mnemonic: str         # "i32.const", "i32.add", etc.
    arg: int | None
    stack_before: list[int]
    stack_after: list[int]
```

## Test Strategy

- Execute `i32.const 1`: verify stack = [1]
- Execute `i32.add` with [1, 2] on stack: verify stack = [3]
- Execute `local.set 0` / `local.get 0`: verify local storage
- End-to-end: run the `x = 1 + 2` program, verify local 0 = 3
- Compare execution trace with bytecode VM trace — they should be structurally similar

## Future Extensions

- `i32.sub`, `i32.mul`, `i32.div_s` — more arithmetic
- `if`/`else`/`end` — control flow blocks
- `br`, `br_if` — branch instructions
- `call`, `return` — function calls
- Memory instructions (`i32.load`, `i32.store`)
- Full WASM module format with type section, function section, etc.
