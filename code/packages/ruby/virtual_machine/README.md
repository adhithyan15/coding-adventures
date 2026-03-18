# coding_adventures_virtual_machine

A general-purpose stack-based bytecode virtual machine. This is the Ruby port of the Python `virtual-machine` package.

## What It Does

Executes bytecode compiled from any source language (Python, Ruby, or custom). Uses a stack-based architecture like the JVM, .NET CLR, and CPython.

## Instruction Set

- **Stack**: LOAD_CONST, POP, DUP
- **Variables**: STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL
- **Arithmetic**: ADD, SUB, MUL, DIV
- **Comparison**: CMP_EQ, CMP_LT, CMP_GT
- **Control flow**: JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE
- **Functions**: CALL, RETURN
- **I/O**: PRINT
- **VM control**: HALT

## Usage

```ruby
require "coding_adventures_virtual_machine"

VM = CodingAdventures::VirtualMachine
OC = VM::OpCode

code = VM::CodeObject.new(
  instructions: [
    VM::Instruction.new(opcode: OC::LOAD_CONST, operand: 0),
    VM::Instruction.new(opcode: OC::LOAD_CONST, operand: 1),
    VM::Instruction.new(opcode: OC::ADD),
    VM::Instruction.new(opcode: OC::PRINT),
    VM::Instruction.new(opcode: OC::HALT)
  ],
  constants: [3, 4]
)

vm = VM::VM.new
traces = vm.execute(code)
puts vm.output  # => ["7"]
```

## Trace Recording

Every instruction execution produces a VMTrace with stack snapshots (before/after), variable state, and a human-readable description.

## Language-Agnostic

No Python-specific or Ruby-specific instructions. Any language can compile to this bytecode.
