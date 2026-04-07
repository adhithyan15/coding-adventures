# register-vm

A generic register-based virtual machine implementing the V8 Ignition execution
model.  No external dependencies.

## What is V8 Ignition?

V8 is the JavaScript engine powering Chrome and Node.js.  Its bytecode
interpreter, *Ignition*, uses an **accumulator-register** hybrid model:

- Most instructions implicitly read from and write to a single **accumulator**
  register, keeping instruction encoding compact.
- A **register file** holds named general-purpose registers that supply
  secondary operands.
- A **feedback vector** records the runtime types flowing through each
  instrumented operation (arithmetic, property loads, call sites) so that an
  optimising compiler can later generate specialised machine code.

This package simulates that model in safe Rust.

## Architecture

```
register_vm
├── opcodes   — ~70 opcode constants (LDA_CONSTANT, ADD, CALL_ANY_RECEIVER, …)
├── types     — VMValue, CodeObject, RegisterInstruction, CallFrame, VMResult
├── feedback  — FeedbackSlot (Uninitialized → Monomorphic → Polymorphic → Megamorphic)
├── scope     — Context chain for lexical scope / closures
└── vm        — VM struct, execute(), run_frame() dispatch loop
```

## Value types

| VMValue variant | JavaScript equivalent |
|---|---|
| `Integer(i64)` | small integer (SMI) |
| `Float(f64)` | double-precision float |
| `Str(String)` | string |
| `Bool(bool)` | boolean |
| `Null` | null |
| `Undefined` | undefined |
| `Object(Rc<RefCell<VMObject>>)` | `{}` — property bag with hidden-class tracking |
| `Array(Rc<RefCell<Vec<VMValue>>>)` | `[]` — ordered list |
| `Function(Rc<CodeObject>, ctx)` | first-class function / closure |

## Opcode categories

| Range | Category |
|---|---|
| `0x00`–`0x06` | Accumulator loads (`LDA_CONSTANT`, `LDA_SMI`, …) |
| `0x10`–`0x12` | Register moves (`LDAR`, `STAR`, `MOV`) |
| `0x20`–`0x27` | Arithmetic (`ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `NEG`, `INC`, `DEC`) |
| `0x30`–`0x35` | Comparison (`TEST_EQUAL`, `TEST_LESS_THAN`, …) |
| `0x40`–`0x43` | Logical / bitwise (`LOGICAL_NOT`, `BITWISE_AND`, …) |
| `0x50`–`0x56` | Control flow (`JUMP`, `JUMP_IF_FALSE`, `RETURN`) |
| `0x60`–`0x63` | Property access (`LDA_NAMED_PROPERTY`, `STA_GLOBAL`, …) |
| `0x70`–`0x72` | Array/element access (`LDA_KEYED_PROPERTY`, `GET_LENGTH`) |
| `0x80`–`0x84` | Calls (`CALL_ANY_RECEIVER`, `TAIL_CALL`, `INTRINSIC_PRINT`) |
| `0x90`–`0x92` | Construction (`CREATE_OBJECT_LITERAL`, `ARRAY_PUSH`) |
| `0xA0`–`0xA2` | Context / closures (`LDA_CONTEXT_SLOT`, `CREATE_CONTEXT`) |
| `0xB0`–`0xB2` | Type ops (`TYPE_OF`, `TEST_UNDEFINED`, `TEST_NULL`) |
| `0xFE` | Stack-overflow guard (`STACK_CHECK`) |
| `0xFF` | Halt (`HALT`) |

## Quick start

```rust
use register_vm::{VM, CodeObject, RegisterInstruction, VMValue};
use register_vm::opcodes::{LDA_SMI, STAR, ADD, HALT};

// Compute 10 + 32 = 42
let code = CodeObject {
    name: "main".to_string(),
    instructions: vec![
        RegisterInstruction { opcode: LDA_SMI, operands: vec![10], feedback_slot: None },
        RegisterInstruction { opcode: STAR,    operands: vec![0],  feedback_slot: None },
        RegisterInstruction { opcode: LDA_SMI, operands: vec![32], feedback_slot: None },
        RegisterInstruction { opcode: ADD,     operands: vec![0],  feedback_slot: None },
        RegisterInstruction { opcode: HALT,    operands: vec![],   feedback_slot: None },
    ],
    constants: vec![],
    names: vec![],
    register_count: 1,
    feedback_slot_count: 0,
    parameter_count: 0,
};

let mut vm = VM::new();
let result = vm.execute(&code);
assert_eq!(result.return_value, VMValue::Integer(42));
```

## Running tests

```bash
cargo test -p register-vm -- --nocapture
```

## Where this fits in the stack

This package sits above the bytecode-compiler packages (e.g. `starlark-vm`,
`lisp-vm`) and below any language-level compiler that wishes to target a
register-based bytecode.  It is designed as a reusable execution core that
can be driven by any frontend.
