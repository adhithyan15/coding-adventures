# LANG52 — Standard Library Completeness for Self-Hosting

## Motivation

LANG51 unblocked string literals.  The remaining self-hosting blockers are:

1. **`let*`** — sequential bindings where later RHSs can reference earlier names.
   The compiler's self-contained passes all use this pattern.

2. **Boolean logic** — `not`, `and`, `or`.  Every conditional chain in the
   compiler needs these.

3. **Missing comparisons** — `<=`, `>=` for range checks; `equal?` for
   structural equality (comparing tokens, strings, symbols).

4. **Arithmetic** — `modulo`, `remainder`, `quotient` for column arithmetic and
   alignment math.

5. **List stdlib** — `list`, `length`, `append`, `reverse`, `list-ref`, `assoc`,
   `list?`, `symbol-append`.  The compiler builds and traverses lists for the
   parameter tables, env frames, and output IIR.

6. **Host I/O** — `host/write_string`, `host/read_line`, `host/read_file` to read
   Twig source files and emit output.

---

## A. `let*` — Sequential bindings

### Syntax

```scheme
(let* ((a 1)
       (b (+ a 1))
       (c (+ b 1)))
  c)   ; ⇒ 3
```

### Grammar

`twig.tokens` — add `let*` to the keywords list.

`twig.grammar` — add `let_star_form` beside `let_form`:

```
form = define | type_alias | record_def | union_def | expr ;
compound = if_form | let_form | let_star_form | begin_form | lambda_form
         | quote_form | match_form | apply ;
let_star_form = LPAREN "let*" LPAREN { binding } RPAREN expr { expr } RPAREN ;
```

### AST

`LetStar` is identical in shape to `Let` — same fields, different struct name.
`Expr::LetStar(LetStar)`.

### IIR lowering

`compile_let_star`: unlike `compile_let`, each binding's RHS is compiled AFTER
the previous binding's name is added to locals.  This gives each binding access
to all earlier bindings.

```
; (let* ((a 1) (b (+ a 1)))  …)
const a = 1                ; compile a=1 in outer scope
_move a ← a               ; bind 'a' into locals
add b, a, 1               ; compile b=(+ a 1) WITH 'a' in scope
_move b ← b               ; bind 'b' into locals
```

---

## B. Boolean logic

### `not` — runtime builtin

```scheme
(not #f) → #t
(not #t) → #f
(not nil) → #t     ; nil is falsy
(not 42) → #f     ; everything else is truthy
```

Implementation: `lispy-runtime/src/builtins.rs`.  Single arg; returns
`LispyValue::TRUE` iff arg is falsy (`NIL` or `FALSE`).

### `and` / `or` — compiler special forms

Handled in `compile_apply` before the builtin lookup, because short-circuit
evaluation is required — the second operand must not be evaluated if the first
is sufficient.

| Call | Expansion |
|------|-----------|
| `(and)` | `#t` |
| `(and e)` | `e` |
| `(and e1 e2 …)` | `(if e1 (and e2 …) #f)` |
| `(or)` | `#f` |
| `(or e)` | `e` |
| `(or e1 e2 …)` | `(let* ((t e1)) (if t t (or e2 …)))` — standard `or` macro expansion |

`and` and `or` are NOT in the BUILTINS list — they are intercepted at the
apply site in `compile_apply` and lowered inline.  This keeps them off the
runtime dispatch path entirely.

---

## C. Comparison and arithmetic builtins

All added to `lispy-runtime/src/builtins.rs`, registered in `binding.rs`, and
listed in the BUILTINS const in `twig-ir-compiler/src/compiler.rs`.

| Builtin | Args | Returns |
|---------|------|---------|
| `<=` | `(a b)` | `#t` if a ≤ b |
| `>=` | `(a b)` | `#t` if a ≥ b |
| `modulo` | `(a b)` | a mod b, result sign matches divisor (Scheme) |
| `remainder` | `(a b)` | a rem b, result sign matches dividend (C `%`) |
| `quotient` | `(a b)` | truncating integer division |
| `not` | `(a)` | logical negation |
| `boolean?` | `(a)` | `#t` if arg is `#t` or `#f` |
| `equal?` | `(a b)` | structural equality: int by value, bool by value, string by content, symbol by id, cons cells recursively |

---

## D. List builtins

| Builtin | Description |
|---------|-------------|
| `list` | `(list a b c)` → `(cons a (cons b (cons c nil)))` |
| `length` | Length of proper list |
| `append` | Concatenate two proper lists |
| `reverse` | Reverse a proper list |
| `list-ref` | `(list-ref lst i)` — 0-indexed element access |
| `assoc` | `(assoc key alist)` — uses `equal?` for comparison |
| `list?` | `#t` if proper list (nil-terminated) |
| `symbol-append` | `(symbol-append sym1 sym2)` — concatenate symbol names |

All added to `lispy-runtime/src/builtins.rs`, `binding.rs`, and BUILTINS.

---

## E. Host I/O

All added to `twig-vm/src/dispatch.rs` in `exec_host_call`.

| Builtin | Args | Returns | Description |
|---------|------|---------|-------------|
| `host/write_string` | `(str)` | void | Write UTF-8 bytes to stdout |
| `host/read_line` | none | heap string | Read one line from stdin, strip `\n` |
| `host/read_file` | `(path)` | heap string | Read entire file at path; `RunError::HostIo` on failure |

A new helper `host_arg_string(host_fn, instr, frame, pos)` extracts a
`LangString` heap object from an argument, mirroring `host_arg_int`.

---

## Files changed

| File | What changes |
|------|--------------|
| `code/grammars/twig.tokens` | Add `let*` to keywords |
| `code/grammars/twig.grammar` | Add `let_star_form`, update `compound` rule |
| `twig-parser/src/ast_nodes.rs` | `LetStar` struct + `Expr::LetStar` |
| `twig-parser/src/ast_extract.rs` | `extract_let_star`, dispatch in `extract_compound` |
| `twig-parser/src/lib.rs` | Re-export `LetStar` |
| `twig-parser/src/type_decls.rs` | `expr_to_kind` for `LetStar` → `Any` |
| `twig-type-checker/src/check.rs` | `infer_expr` arm for `LetStar` |
| `twig-ir-compiler/src/compiler.rs` | `compile_let_star`, `and`/`or` special cases, update BUILTINS |
| `twig-ir-compiler/src/free_vars.rs` | `free_vars` for `LetStar` |
| `lispy-runtime/src/builtins.rs` | `<=`, `>=`, `modulo`, `remainder`, `quotient`, `not`, `boolean?`, `equal?`, list ops, `symbol-append` |
| `lispy-runtime/src/binding.rs` | Register all new builtins |
| `twig-vm/src/dispatch.rs` | `host/write_string`, `host/read_line`, `host/read_file`, `host_arg_string` |

---

## Version bumps

| Crate | From | To |
|-------|------|----|
| `twig-parser` | 0.4.0 | 0.5.0 |
| `twig-ir-compiler` | 0.6.0 | 0.7.0 |
| `twig-type-checker` | 0.2.0 | 0.3.0 |
| `lispy-runtime` | current | +minor |
| `twig-vm` | current | +minor |

(On this branch — main has not merged LANG51 yet, so starting from pre-LANG51 versions.)

---

## After LANG52

Remaining blockers for self-hosting (reduced set):
- **LANG53**: Multi-file module driver — Twig-callable `(import "path/to/file.tw")` resolver
- **LANG54**: Flow-sensitive type narrowing (`<`, `<=`, `=` guards)
- **Higher-order list ops** — `map`/`filter`/`fold-*` require VM callback — stdlib.tw
