# lisp-compiler (C)

A **Lisp bytecode compiler** — S-expression AST → bytecode — in pure ISO C17. A
faithful port of the Rust [`lisp-compiler`](../../rust/lisp-compiler) crate. It
completes the Lisp toolchain on top of [`lisp-parser`](../lisp-parser) and
[`lisp-lexer`](../lisp-lexer): source in, an `LcCodeObject` of bytecode out.

## What it does

Lisp's grammar has only 6 rules and doesn't distinguish `(define x 1)`,
`(+ 1 2)`, and `(lambda (n) n)` — they're all lists. The **compiler** inspects
the first element of each list to assign meaning: special forms (`define`,
`lambda`, `cond`, `quote`), arithmetic / comparison operators, cons cells,
predicates (`atom`, `is-nil`, `car`, `cdr`, `print`), or an ordinary function
call. It tracks *tail position* so calls in tail position emit `LC_TAIL_CALL`
instead of `LC_CALL_FUNCTION` (tail-call optimisation).

The output `LcCodeObject` (instructions + a constant pool + a name pool) is a
plain struct a VM — or a test — can walk directly. Lambda bodies are themselves
`LcCodeObject`s stored as `LC_VAL_CODE` constants.

## API

- `int lc_compile(const char *source, LcCodeObject *out, LcCompileError *err)` —
  parse + compile; 1 on success (fills `*out`), 0 on error.
- `int lc_compile_ast(const LpProgram *program, LcCodeObject *out, LcCompileError *err)`.
- `void lc_code_object_free(LcCodeObject *code)` — frees the *contents* (the
  struct itself is caller-owned, e.g. a stack variable).
- `int lc_value_is_falsy(const LcValue *v)`.

## Dependency

Depends on `c/lisp-parser` and `c/lisp-lexer`; `run.sh` compiles both siblings'
sources alongside and adds their include paths. (This port adds a few AST child
accessors to `lisp-parser` so a tree walker can traverse an opaque `LpSExpr`.)

## Design notes

- **Recursive-descent, buffer-based.** One function per special form. Errors —
  bad syntax or OOM — set a `failed` flag; the emit/add helpers become guarded
  no-ops, so the walk unwinds and the partial output is discarded.
- **Recursive owned `LcValue`.** A `LC_VAL_CODE` constant owns a heap
  `LcCodeObject`; constant-pool deduplication compares values structurally
  (including deep code equality). The lambda state save/restore mirrors the
  Rust `mem::take` dance.

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
