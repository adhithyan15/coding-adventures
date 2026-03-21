# lisp-vm

Executes compiled Lisp bytecode. This is the final stage of the Lisp pipeline: bytecode goes in, results come out.

## Features

- **Stack-based execution**: push/pop values, arithmetic, comparison
- **Cons cells**: heap-allocated pairs with car/cdr operations
- **Symbol interning**: identical symbols share the same heap address
- **Closures**: lambda functions that capture their environment
- **Tail call optimization**: tail-recursive functions run in O(1) stack space
- **Garbage collection**: simple heap with cons cells, symbols, and closures

## Usage

```rust
use lisp_vm::run;

let result = run("(+ 1 2)").unwrap();
assert_eq!(result, lisp_compiler::Value::Integer(3));

let result = run("(car (cons 1 2))").unwrap();
assert_eq!(result, lisp_compiler::Value::Integer(1));
```

## How It Fits in the Stack

```
Source --> [lisp-lexer] --> tokens --> [lisp-parser] --> AST
  --> [lisp-compiler] --> bytecode --> [lisp-vm] --> result
```

## Architecture

The VM maintains:
- A **value stack** for computation
- A **variable table** for global bindings
- A **local variable array** for function parameters
- A **heap** for cons cells, symbols, and closures
- A **symbol table** for interning
- A **program counter** (PC) pointing to the current instruction
