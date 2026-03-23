# virtual-machine

Stack-based bytecode interpreter with eval loop, value stack, and variable environment.

## Layer 5

This package is part of Layer 5 of the coding-adventures computing stack. It provides two virtual machines that execute bytecode programs:

- **VirtualMachine** -- Hard-coded opcode dispatch. All 21 opcodes are implemented in a Lua dispatch table. Simple, fast, and easy to follow.
- **GenericVM** -- Handler-based pluggable interpreter. Register callback functions for each opcode, swap instruction sets at will, freeze the VM for sandboxing.

Both VMs support coroutine-based step-through debugging via `create_stepper()`.

## How It Fits in the Stack

The virtual machine sits between the bytecode compiler (Layer 5) and the parser/lexer (Layer 4). Source code is parsed into an AST, compiled into bytecode (a sequence of `Instruction` tables), and then executed by the VM.

## Usage

```lua
local vm_mod = require("coding_adventures.virtual_machine")

-- Create a simple program: push 3, push 4, add, print, halt
local code = vm_mod.assemble_code(
    {
        vm_mod.instruction(vm_mod.OP_LOAD_CONST, 1),
        vm_mod.instruction(vm_mod.OP_LOAD_CONST, 2),
        vm_mod.instruction(vm_mod.OP_ADD),
        vm_mod.instruction(vm_mod.OP_PRINT),
        vm_mod.instruction(vm_mod.OP_HALT),
    },
    { 3, 4 }  -- constants pool
)

-- Execute with VirtualMachine
local vm = vm_mod.VirtualMachine.new()
local traces = vm:execute(code)
print(vm.output[1])  --> "7"

-- Step through with coroutines
local vm2 = vm_mod.VirtualMachine.new()
local stepper = vm2:create_stepper(code)
while true do
    local ok, trace = coroutine.resume(stepper)
    if not ok or trace == nil then break end
    print(string.format("PC=%d  %s", trace.pc, trace.description))
end
```

### GenericVM (Pluggable Handlers)

```lua
local vm_mod = require("coding_adventures.virtual_machine")
local gvm = vm_mod.GenericVM.new()

-- Register custom handlers
gvm:register_opcode(vm_mod.OP_LOAD_CONST, function(vm, instr, code)
    vm:push(code.constants[instr.operand])
    vm:advance_pc()
    return nil
end)

gvm:register_opcode(vm_mod.OP_HALT, function(vm, instr, code)
    vm.halted = true
    return nil
end)

-- Register builtins
gvm:register_builtin("len", function(...)
    local args = {...}
    return #args[1]
end)

-- Freeze to prevent further registration (sandboxing)
gvm:set_frozen(true)

-- Execute
local code = vm_mod.assemble_code(
    { vm_mod.instruction(vm_mod.OP_LOAD_CONST, 1), vm_mod.instruction(vm_mod.OP_HALT) },
    { 42 }
)
gvm:execute(code)
```

## Opcodes

| Opcode | Hex  | Description |
|--------|------|-------------|
| LOAD_CONST | 0x01 | Push constant onto stack |
| POP | 0x02 | Discard top of stack |
| DUP | 0x03 | Duplicate top of stack |
| STORE_NAME | 0x10 | Pop and store in named variable |
| LOAD_NAME | 0x11 | Push named variable |
| STORE_LOCAL | 0x12 | Pop and store in local slot |
| LOAD_LOCAL | 0x13 | Push local slot |
| ADD | 0x20 | Pop two, push sum (numbers or string concat) |
| SUB | 0x21 | Pop two, push difference |
| MUL | 0x22 | Pop two, push product |
| DIV | 0x23 | Pop two, push integer quotient |
| CMP_EQ | 0x30 | Pop two, push 1 if equal, 0 otherwise |
| CMP_LT | 0x31 | Pop two, push 1 if a < b |
| CMP_GT | 0x32 | Pop two, push 1 if a > b |
| JUMP | 0x40 | Unconditional jump |
| JUMP_IF_FALSE | 0x41 | Pop; jump if falsy |
| JUMP_IF_TRUE | 0x42 | Pop; jump if truthy |
| CALL | 0x50 | Call named function (CodeObject) |
| RETURN | 0x51 | Return from function |
| PRINT | 0x60 | Pop and append to output |
| HALT | 0xFF | Stop execution |

## Development

```bash
# Run tests
bash BUILD
```
