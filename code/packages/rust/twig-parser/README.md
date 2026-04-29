# twig-parser

Parses a [Twig](../../specs/TW00-twig-language.md) token stream (from [`twig-lexer`](../twig-lexer)) into a typed AST. This is the second stage of the Rust Twig pipeline:

```
Twig source --> [twig-lexer] --> tokens --> [twig-parser] --> AST --> [twig-ir-compiler] --> IIRModule
```

## Grammar (one screen)

```text
program     = { form } ;
form        = define | expr ;
define      = LPAREN "define" name_or_signature expr { expr } RPAREN ;
name_or_signature = NAME | LPAREN NAME { NAME } RPAREN ;
expr        = atom | quoted | compound ;
atom        = INTEGER | BOOL_TRUE | BOOL_FALSE | "nil" | NAME ;
quoted      = QUOTE NAME ;
compound    = if_form | let_form | begin_form | lambda_form | quote_form | apply ;
if_form     = LPAREN "if"     expr expr expr RPAREN ;
let_form    = LPAREN "let" LPAREN { binding } RPAREN expr { expr } RPAREN ;
binding     = LPAREN NAME expr RPAREN ;
begin_form  = LPAREN "begin"  expr { expr } RPAREN ;
lambda_form = LPAREN "lambda" LPAREN { NAME } RPAREN expr { expr } RPAREN ;
quote_form  = LPAREN "quote"  NAME RPAREN ;
apply       = LPAREN expr { expr } RPAREN ;
```

## Typed AST

The parser emits a small, exhaustive set of variants — one per semantic shape:

| Type      | Used for                         |
|-----------|----------------------------------|
| `IntLit`  | `42`, `-7`                       |
| `BoolLit` | `#t`, `#f`                       |
| `NilLit`  | `nil`                            |
| `SymLit`  | `'foo`, `(quote foo)`            |
| `VarRef`  | `x`, `+`, `null?`                |
| `If`      | `(if c t e)`                     |
| `Let`     | `(let ((x 1)) body+)`            |
| `Begin`   | `(begin e1 e2 ...)`              |
| `Lambda`  | `(lambda (params) body+)`        |
| `Apply`   | `(fn arg0 arg1 ...)`             |
| `Define`  | `(define name expr)` (top-level) |

The function-sugar form `(define (f x) body+)` parses to `Define { name: "f", expr: Lambda { params: ["x"], body } }` — sugar is desugared at parse time so downstream code only ever sees the lambda shape.

Every node carries 1-indexed `line` / `column` of its source location. The IR compiler propagates these into error messages.

## Usage

```rust
use twig_parser::{parse, Form, Expr};

let p = parse("(define (square x) (* x x))").unwrap();
match &p.forms[0] {
    Form::Define(d) => {
        assert_eq!(d.name, "square");
        assert!(matches!(d.expr, Expr::Lambda(_)));
    }
    _ => unreachable!(),
}
```

## Why typed AST instead of generic `ASTNode`?

The Python parser builds a generic `ASTNode` tree and uses a separate `ast_extract.py` pass to lift it into typed dataclasses. The Rust parser folds those passes together — recursive descent into the typed shape directly. Two reasons:

1. **Exhaustive `match` everywhere downstream.** Adding a new compound form means adding a variant to `Expr`, which the type system flags in every match arm.
2. **No `isinstance` ladders.** The IR compiler dispatches on enum discriminants (a single jump table), not chained type checks.

## Tests

```bash
cargo test -p twig-parser
```

Coverage: every form, the function-sugar lowering, multi-expression bodies, position tracking, and error paths (unmatched parens, integer overflow, nested defines, empty applications).
