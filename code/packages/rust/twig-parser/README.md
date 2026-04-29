# twig-parser

Rust binding to [`code/grammars/twig.grammar`](../../grammars/twig.grammar) — wraps the generic [`GrammarParser`](../parser) and lifts the resulting `GrammarASTNode` tree into a typed Twig AST.

```
Twig source --> [twig-lexer] --> tokens --> [twig-parser] --> AST --> [twig-ir-compiler] --> IIRModule
```

Same pattern as every other Rust language frontend (`brainfuck`, `dartmouth-basic`, …) and as the Python [`twig` package](../../python/twig)'s `parser.py` + `ast_extract.py` pair.

## Grammar (one screen, lives in `code/grammars/twig.grammar`)

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

The crate's `ast_extract` module walks the generic `GrammarASTNode` tree and lifts each meaningful subtree into one of these typed variants:

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

The function-sugar form `(define (f x) body+)` lowers to `Define { name: "f", expr: Lambda { params: ["x"], body } }` during extraction — downstream code only ever sees the lambda shape.  Both quote forms (`'foo` and `(quote foo)`) collapse to a single `SymLit`.

Every node carries 1-indexed `line` / `column` of its source location, propagated from the underlying tokens.

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

For lower-level access:
- `parse_to_ast(source)` returns the generic `GrammarASTNode`.
- `create_twig_parser(source)` returns the underlying `GrammarParser`.

## Stack-overflow defence

The `GrammarParser` is recursive (one stack frame per matched rule).  Pathological untrusted input like `(((...))))` with deep nesting would exhaust the OS thread stack and abort the process — Rust does not catch stack overflow.

`parse()` pre-scans the token stream for LPAREN depth and rejects sources whose nesting exceeds [`MAX_PAREN_DEPTH`](src/lib.rs) (64) before invoking the parser.  The AST extractor adds its own depth bound via [`MAX_AST_DEPTH`](src/ast_extract.rs) for callers that bypass `parse()` and feed in a hand-built `GrammarASTNode`.

## Tests

```bash
cargo test -p twig-parser
```

31 unit tests covering every form, the function-sugar lowering, multi-expression bodies, position tracking, and error/depth-cap paths.
