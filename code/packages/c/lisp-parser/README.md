# lisp-parser (C)

A **Lisp parser** — token stream → S-expression AST — in pure ISO C17. A
faithful port of the Rust [`lisp-parser`](../../rust/lisp-parser) crate. Builds
on the sibling [`lisp-lexer`](../lisp-lexer).

## What it does

Lisp's grammar is famously tiny (6 rules), so this is a small recursive-descent
parser: it turns the flat token stream from the lexer into a tree of `LpSExpr`
nodes.

```
Input:  (+ (* 2 3) 4)
AST:    List[ Atom(Symbol,"+"), List[ Atom(Symbol,"*"), Atom(Number,"2"),
              Atom(Number,"3") ], Atom(Number,"4") ]
```

Node kinds: **Atom** (number / symbol / string), **List**, **DottedPair**
(`(a . b)`), and **Quoted** (`'x` = sugar for `(quote x)`).

## API

- `int lp_parse(const char *source, LpProgram *out, LpError *err)` — tokenize +
  parse; 1 on success (fills `*out`, release with `lp_program_free`), 0 on error.
- `int lp_parse_tokens(const LlToken *tokens, size_t n, LpProgram *out, LpError *err)`
  — parse a pre-tokenized stream.
- Node inspection: `lp_sexpr_kind`, `lp_sexpr_atom_kind` / `lp_sexpr_atom_value`,
  `lp_sexpr_find_atoms` / `lp_sexpr_count_lists` / `lp_sexpr_count_quoted`
  (and `lp_program_*` variants across all top-level forms).

## Dependency

This package depends on `c/lisp-lexer` for tokenization — its `run.sh` compiles
`../lisp-lexer/src/lisp_lexer.c` and adds `../lisp-lexer/include` to the include
path. Both packages ship together.

## Design notes

- **Recursive-descent, owned AST.** One function per grammar rule over a token
  cursor. The AST is an owned tagged-union tree; a failed parse frees whatever
  it built and reports a message. Peeking past the end reads as EOF, so a
  truncated stream never reads out of bounds.

## Usage

```c
#include "lisp_parser.h"

LpProgram prog;
LpError err;
if (lp_parse("(+ 1 2)", &prog, &err)) {
    /* prog.n == 1; prog.exprs[0] is a List of three atoms */
    LpStrList atoms = lp_program_find_atoms(&prog);  /* "+", "1", "2" */
    lp_strlist_free(&atoms);
    lp_program_free(&prog);
}
```

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
