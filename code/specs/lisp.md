# McCarthy's 1960 Lisp

## Overview

This is a complete implementation of John McCarthy's original Lisp from his 1960 paper "Recursive Functions of Symbolic Expressions and Their Computation by Machine." It includes a lexer, parser, bytecode compiler, and VM plugin — the full pipeline from source code to execution.

McCarthy's Lisp is historically significant: it introduced garbage collection, the if/else conditional, recursion as the primary control structure, dynamic typing, and the idea that a language can be defined by its own interpreter (the meta-circular evaluator). It was the first functional programming language.

Our implementation compiles Lisp to bytecode and runs it on the GenericVM, following the same pluggable architecture used by Starlark and Brainfuck. This means Lisp shares infrastructure (stack, call frames, instruction dispatch) with other languages.

## Layer Position

```
Logic Gates → Arithmetic → CPU → Assembler → Lexer → Parser → Compiler → GC → VM
                                                ↑       ↑         ↑        ↑    ↑
                                            lisp-lexer  |   lisp-compiler  |  lisp-vm
                                                    lisp-parser        garbage-collector
```

## McCarthy's Seven Primitives

McCarthy showed that all of computation can be built from seven primitives:

| Primitive | Purpose | Example |
|-----------|---------|---------|
| `quote` | Return expression unevaluated | `(quote foo)` → `foo` |
| `atom` | Is this an atom (not a list)? | `(atom 42)` → `t` |
| `eq` | Are two atoms equal? | `(eq 1 1)` → `t` |
| `car` | First element of a cons cell | `(car (cons 1 2))` → `1` |
| `cdr` | Second element of a cons cell | `(cdr (cons 1 2))` → `2` |
| `cons` | Create a pair | `(cons 1 2)` → `(1 . 2)` |
| `cond` | Conditional branching | `(cond ((eq x 0) 1) (t x))` |

Plus `lambda` for functions, `define` for bindings, and arithmetic operators.

## Grammar

### Tokens (`lisp.tokens`)

```
skip:
  WHITESPACE = /[ \t\r\n]+/
  COMMENT    = /;[^\n]*/

NUMBER = /-?[0-9]+/
SYMBOL = /[a-zA-Z_+\-*\/=<>!?&][a-zA-Z0-9_+\-*\/=<>!?&]*/
STRING = /"([^"\\]|\\.)*"/
LPAREN = "("
RPAREN = ")"
QUOTE  = "'"
DOT    = "."
```

Key design: NUMBER comes before SYMBOL so `-42` tokenizes as NUMBER, not SYMBOL `-` followed by NUMBER `42`.

### Grammar (`lisp.grammar`)

```
program   = { sexpr } ;
sexpr     = atom | list | quoted ;
atom      = NUMBER | SYMBOL | STRING ;
list      = LPAREN list_body RPAREN ;
list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;
quoted    = QUOTE sexpr ;
```

The `list_body` rule handles dotted pairs: `(a . b)` is a cons cell where car=a, cdr=b. Regular lists `(a b c)` are syntactic sugar for `(a . (b . (c . nil)))`.

## Bytecode Design

### Opcodes (`LispOp`)

```
0x0_ Stack:       LOAD_CONST, POP, LOAD_NIL, LOAD_TRUE
0x1_ Variables:   STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL
0x2_ Arithmetic:  ADD, SUB, MUL, DIV
0x3_ Comparison:  CMP_EQ, CMP_LT, CMP_GT
0x4_ Control:     JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE
0x5_ Functions:   MAKE_CLOSURE, CALL_FUNCTION, TAIL_CALL, RETURN
0x7_ Lisp:        CONS, CAR, CDR, MAKE_SYMBOL, IS_ATOM, IS_NIL
0xA_ I/O:         PRINT
0xF_ VM:          HALT
```

### Tail Call Optimization

The `TAIL_CALL` opcode is a GenericVM-level feature, not Lisp-specific. When a function call is the last thing a function does before returning (tail position), the compiler emits `TAIL_CALL` instead of `CALL_FUNCTION`.

The VM handles `TAIL_CALL` by reusing the current call frame — rebinding arguments in the existing local variable slots and resetting the program counter to 0 — instead of pushing a new frame. This means tail-recursive functions use O(1) stack space regardless of recursion depth.

Tail positions:
- The body of a `lambda` (last expression)
- Each consequent expression in `cond` branches
- NOT: arguments to function calls
- NOT: predicate expressions in `cond`

Any future functional language (Scheme, ML, Haskell) benefits from the same opcode.

### Compilation examples

**Arithmetic**: `(+ 1 2)` →
```
LOAD_CONST 1
LOAD_CONST 2
ADD
```

**Conditional**: `(cond ((eq x 0) 1) (t x))` →
```
LOAD_NAME x
LOAD_CONST 0
CMP_EQ
JUMP_IF_FALSE L1
LOAD_CONST 1
JUMP END
L1: LOAD_TRUE
JUMP_IF_FALSE L2
LOAD_NAME x
JUMP END
L2: LOAD_NIL
END:
```

**Lambda + define**: `(define square (lambda (x) (* x x)))` →
```
; Compile the lambda body as a sub-CodeObject:
;   LOAD_LOCAL 0    (x)
;   LOAD_LOCAL 0    (x)
;   MUL
;   RETURN
LOAD_CONST <CodeObject for lambda>
MAKE_CLOSURE
STORE_NAME "square"
```

**Tail-recursive factorial**:
```lisp
(define factorial-iter
  (lambda (n acc)
    (cond ((eq n 0) acc)
          (t (factorial-iter (- n 1) (* n acc))))))
```
The `(factorial-iter ...)` call is in tail position (last expression in a `cond` branch inside a `lambda` body), so it compiles to `TAIL_CALL 2` instead of `CALL_FUNCTION 2`.

## NIL Sentinel

NIL is a distinct Python object — not `None`, not `0`, not `False`:

```python
NIL = object()
```

- `LOAD_NIL` pushes `NIL` onto the stack
- `IS_NIL` checks `value is NIL` (identity, not equality)
- NIL is falsy for `JUMP_IF_FALSE`
- `(eq x nil)` works via CMP_EQ treating NIL as equal to itself

## Package Structure

Five packages, following existing patterns:

```
code/packages/python/garbage-collector/    # GC framework (separate spec)
code/packages/python/lisp-lexer/           # Thin wrapper → lisp.tokens
code/packages/python/lisp-parser/          # Thin wrapper → lisp.grammar
code/packages/python/lisp-vm/              # Opcodes + handlers + factory
code/packages/python/lisp-compiler/        # AST → bytecode + run_lisp()
```

## Test Strategy

### Unit tests
- Lexer: tokenize atoms, lists, nested lists, dotted pairs, quotes, comments
- Parser: parse all forms, verify AST structure
- VM handlers: each opcode tested in isolation (CONS/CAR/CDR, MAKE_CLOSURE, TAIL_CALL)
- Compiler: each special form produces correct bytecode

### End-to-end tests
- `(factorial 5)` → 120
- `(car (cons 1 2))` → 1, `(cdr (cons 1 2))` → 2
- `(atom (quote foo))` → 1, `(atom (cons 1 2))` → 0
- `(eq (quote foo) (quote foo))` → 1
- `(append (quote (1 2)) (quote (3 4)))` → (1 2 3 4) as cons chain
- `(factorial-iter 100 1)` → 100! (verifies TCO — no stack overflow)
- GC reclaims unreachable cons cells
