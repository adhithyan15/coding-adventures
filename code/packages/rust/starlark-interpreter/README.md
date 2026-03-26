# starlark-interpreter

The complete Starlark execution pipeline: source code in, result out.

## What It Does

This crate is the top-level entry point for running Starlark programs. It chains together all the lower-level crates in the computing stack:

```text
Source Code
    | (starlark-lexer)
Token Stream
    | (starlark-parser)
AST
    | (starlark-compiler / stub compiler)
CodeObject (bytecode)
    | (virtual-machine + starlark-vm)
InterpreterResult
```

It also provides the critical `load()` function that makes BUILD files work, with file caching so each loaded file is evaluated at most once.

## Where It Fits

```text
Layer 5: starlark-interpreter  <-- THIS CRATE (orchestrator)
Layer 4: starlark-vm           (VM with Starlark builtins)
Layer 3: starlark-compiler     (opcodes + compilation)
Layer 2: starlark-parser       (tokens -> AST)
Layer 1: starlark-lexer        (source -> tokens)
Layer 0: virtual-machine       (generic execution engine)
```

## Key Types

| Type                   | Purpose                                    |
|------------------------|--------------------------------------------|
| `StarlarkInterpreter`  | Configurable interpreter with caching      |
| `FileResolver` trait   | Pluggable file resolution for `load()`     |
| `DictResolver`         | In-memory resolver (for testing)           |
| `FsResolver`           | Filesystem resolver (for production)       |
| `InterpreterResult`    | Execution result with variables and output |
| `InterpreterError`     | Errors from any pipeline stage             |

## Usage

### Execute bytecode directly

```rust
use starlark_interpreter::{interpret_bytecode, Op, CodeObject, Instruction, Operand, Value};

let code = CodeObject {
    instructions: vec![
        Instruction { opcode: Op::LoadConst as u8, operand: Some(Operand::Index(0)) },
        Instruction { opcode: Op::StoreName as u8, operand: Some(Operand::Index(0)) },
        Instruction { opcode: Op::Halt as u8, operand: None },
    ],
    constants: vec![Value::Int(42)],
    names: vec!["x".to_string()],
};
let result = interpret_bytecode(&code).unwrap();
assert_eq!(result.get_int("x"), Some(42));
```

### Execute source code (via stub compiler)

```rust
use starlark_interpreter::interpret;

let result = interpret("x = 1 + 2\nprint(x)\n", None).unwrap();
assert_eq!(result.get_int("x"), Some(3));
assert_eq!(result.output, vec!["3"]);
```

### With load() support

```rust
use starlark_interpreter::{DictResolver, StarlarkInterpreter};

let resolver = DictResolver::new(vec![
    ("//lib.star".to_string(), "ANSWER = 42\n".to_string()),
]);
let mut interp = StarlarkInterpreter::new(Some(&resolver), 200);
let module = interp.load_module("//lib.star").unwrap();
assert_eq!(module.get("ANSWER").unwrap(), &starlark_vm::StarlarkValue::Int(42));
```

## Stub Compiler

The full AST-to-bytecode compiler (`starlark-ast-to-bytecode-compiler`) is being built separately. In the meantime, this crate includes a stub compiler that handles a subset of Starlark:

- Integer, float, string, boolean, and None assignments
- Arithmetic expressions (`+`, `-`, `*`, `//`, `%`)
- `print()` calls
- Variable references
- Comments and blank lines

When the full compiler is ready, the `compile_source` function will delegate to it.

## Testing

```bash
cargo test -p starlark-interpreter
```

62 tests covering:
- Bytecode execution (all opcode categories)
- Stub compiler (source -> result)
- File resolver behavior
- Module loading and caching
- Error handling
- Value conversion utilities
