# lisp-compiler

Compiles Lisp S-expression ASTs into bytecode for the Lisp VM. This is the third stage of the pipeline: AST goes in, executable bytecode comes out.

## How It Works

The compiler walks the S-expression tree and emits bytecode instructions. Unlike languages with many grammar rules, Lisp's compiler inspects the first element of each list to decide what to do:

- `(define x 42)` -- compile as variable definition
- `(lambda (n) n)` -- compile as closure creation
- `(cond (...) ...)` -- compile as conditional branching
- `(+ 1 2)` -- compile as arithmetic
- `(f x y)` -- compile as function call

## Special Forms

| Form     | Description                                |
|----------|--------------------------------------------|
| `define` | Bind a name to a value                     |
| `lambda` | Create a closure                           |
| `cond`   | Conditional branching                      |
| `quote`  | Return data without evaluating             |
| `cons`   | Create a cons cell                         |
| `car`    | First element of a cons cell               |
| `cdr`    | Second element of a cons cell              |
| `atom`   | Test if value is an atom                   |
| `eq`     | Test equality                              |
| `print`  | Output a value                             |

## Tail Call Optimization

The compiler tracks whether a function call is in "tail position" (the last thing a function does before returning) and emits `TAIL_CALL` instead of `CALL_FUNCTION` in that case. This enables unbounded recursion for tail-recursive functions.

## Usage

```rust
use lisp_compiler::compile;

let code = compile("(+ 1 2)").unwrap();
// code contains instructions, constants, and names
```

## How It Fits in the Stack

```
Source --> [lisp-lexer] --> tokens --> [lisp-parser] --> AST --> [lisp-compiler] --> bytecode --> [lisp-vm]
```
