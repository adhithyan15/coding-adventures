# Lisp VM

McCarthy's 1960 Lisp running on the pluggable GenericVM framework.

## What is this?

A VM plugin that registers Lisp-specific opcodes (CONS, CAR, CDR, MAKE_SYMBOL, etc.) with the GenericVM. Includes garbage collection integration, symbol interning, closures, and tail call optimization.

## How it fits in the stack

```
Logic Gates → Arithmetic → CPU → Assembler → Lexer → Parser → Compiler → GC → [VM]
                                                                           ↑      ↑
                                                                   garbage-collector
                                                                        lisp-vm plugin
```

## Usage

```python
from lisp_vm import create_lisp_vm, LispOp, NIL
from virtual_machine import CodeObject, Instruction

# Create a Lisp-configured VM
vm = create_lisp_vm()

# Execute bytecode: (cons 1 2)
code = CodeObject(
    instructions=[
        Instruction(opcode=LispOp.LOAD_CONST, operand=0),  # push 2
        Instruction(opcode=LispOp.LOAD_CONST, operand=1),  # push 1
        Instruction(opcode=LispOp.CONS),                     # cons(1, 2)
        Instruction(opcode=LispOp.HALT),
    ],
    constants=[2, 1],
    names=[],
)
output = vm.execute(code)
```

## Opcodes

| Range | Category | Opcodes |
|-------|----------|---------|
| 0x0_ | Stack | LOAD_CONST, POP, LOAD_NIL, LOAD_TRUE |
| 0x1_ | Variables | STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL |
| 0x2_ | Arithmetic | ADD, SUB, MUL, DIV |
| 0x3_ | Comparison | CMP_EQ, CMP_LT, CMP_GT |
| 0x4_ | Control flow | JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE |
| 0x5_ | Functions | MAKE_CLOSURE, CALL_FUNCTION, TAIL_CALL, RETURN |
| 0x7_ | Lisp-specific | CONS, CAR, CDR, MAKE_SYMBOL, IS_ATOM, IS_NIL |
| 0xA_ | I/O | PRINT |
| 0xF_ | VM control | HALT |

## Tail Call Optimization

The `TAIL_CALL` opcode is a GenericVM-level feature. When a function call is in tail position, the compiler emits `TAIL_CALL` instead of `CALL_FUNCTION`. The VM reuses the current call frame, enabling unbounded recursion for tail-recursive functions.
