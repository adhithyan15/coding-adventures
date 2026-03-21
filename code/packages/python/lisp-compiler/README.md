# Lisp Compiler

Compiles McCarthy's 1960 Lisp into bytecode for the GenericVM, using the pluggable GenericCompiler framework.

## What It Does

This compiler transforms Lisp source code into `CodeObject` bytecode that the Lisp VM can execute. It handles all of McCarthy's original special forms plus modern conveniences like tail call optimization.

## How It Fits in the Stack

```
Source Code → [Lisp Lexer] → Tokens → [Lisp Parser] → AST → [Lisp Compiler] → CodeObject → [Lisp VM] → Result
```

The compiler is the bridge between the parser's AST and the VM's bytecode. It transforms nested s-expressions into flat sequences of instructions.

## Special Forms

| Form | Example | Description |
|------|---------|-------------|
| `quote` | `(quote foo)` or `'foo` | Return data as-is, don't evaluate |
| `atom` | `(atom x)` | Test if value is an atom |
| `eq` | `(eq x y)` | Test equality |
| `car` | `(car x)` | First element of a cons cell |
| `cdr` | `(cdr x)` | Second element of a cons cell |
| `cons` | `(cons x y)` | Create a cons cell |
| `cond` | `(cond (p1 e1) (p2 e2))` | Conditional branching |
| `lambda` | `(lambda (x) (* x x))` | Create a function |
| `define` | `(define name value)` | Bind a name to a value |

## Tail Call Optimization

The compiler detects when a function call is in **tail position** (the last expression evaluated before returning) and emits `TAIL_CALL` instead of `CALL_FUNCTION`. This enables unbounded recursion for tail-recursive functions.

## Usage

```python
from lisp_compiler import compile_lisp, run_lisp

# Compile to bytecode
code = compile_lisp("(+ 1 2)")

# Compile and run
result = run_lisp("(+ 1 2)")  # => 3

# Factorial
result = run_lisp("""
    (define factorial
      (lambda (n)
        (cond ((eq n 0) 1)
              (t (* n (factorial (- n 1)))))))
    (factorial 5)
""")  # => 120
```
