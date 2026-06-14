# Prolog dialect grammars

The Rust + Python Prolog stack is **dialect-aware** from day one. Each
dialect owns its own `<name>.tokens` and `<name>.grammar` file in this
directory. Adding a dialect means **adding two files**, not editing
parser source.

## Dialects shipped today

| Dialect | Tokens | Grammar | Status | Notes |
|---|---|---|---|---|
| **ISO/Core Prolog** | `iso.tokens` | `iso.grammar` | ✅ | Reference dialect. The Rust crates `prolog-lexer` / `prolog-parser` embed the ISO grammar today. |
| **SWI-Prolog** | `swi.tokens` | `swi.grammar` | scaffolded | Token + grammar files are written; a dedicated `swi-prolog-lexer` / `swi-prolog-parser` pair will plug them in (mirrors the ISO crates). |

## Why grammar-driven dialects

Prolog has the messiest "every system disagrees about the operator table"
problem in mainstream programming languages. Burying dialect quirks in
parser code (the way SWI / SICStus / GNU each end up doing) makes the
parser a forking battleground. The pipeline here keeps every dialect
quirk in a **grammar file**:

```text
   code/grammars/prolog/<dialect>.tokens    (canonical)
        │  cargo run -p <dialect>-prolog-lexer --example regenerate_grammar
        ▼
   <dialect>-prolog-lexer/src/_grammar.rs   (auto-generated embed)
```

Same for `<dialect>.grammar` and `<dialect>-prolog-parser`.

Downstream consumers (loaders, semantic-source-map pipelines, etc.) all
program against the same `Term` shape from `logic-core` — the dialect
decides syntax, not semantics. A single `prolog_loader::load_source`
serves every dialect by accepting a dialect parameter (future API).

## What every dialect must include

The minimum a Prolog dialect grammar must define:

**Tokens:**

- `WHITESPACE`, `LINE_COMMENT`, `BLOCK_COMMENT` in `skip:`.
- `QUERY = "?-"`, `RULE = ":-"`, `NAF = "\+"` as structural tokens
  ahead of `ATOM_SYMBOLIC`.
- `LPAREN` / `RPAREN` / `LBRACKET` / `RBRACKET` / `LCURLY` / `RCURLY`
  / `BAR` / `COMMA` / `SEMICOLON` / `CUT` / `DOT`.
- Numeric: `FLOAT`, `INTEGER`. Textual: `STRING`, `QUOTED_ATOM`.
- Identifiers: `ANON_VAR`, `VARIABLE`, `ATOM`, `ATOM_SYMBOLIC`.

**Grammar:**

- `program = { statement }`.
- Statements: at least `query_statement | rule_statement | fact_statement`.
- `goal_primary` must include `CUT | grouped_goal | naf_goal |
  equality_goal | callable_goal` (NAF is required for any practical
  Prolog program).
- `term = list_term | compound_term | atom_term | variable_term |
  anonymous_term | number_term | string_term`.

## Differences between today's dialects

`iso.tokens` vs `swi.tokens`:

- SWI adds `RANGE = ".."` (CLP-style operators).
- SWI adds `BACKQUOTED_STRING` (back-tick string syntax).

`iso.grammar` vs `swi.grammar`:

- SWI adds `directive_statement = RULE goal DOT` for top-level
  `:- directive.` lines.
- SWI adds `dcg_statement = callable_term DCG dcg_body DOT` plus the
  `dcg_*` sub-productions.
- SWI adds `clpfd_goal = term clpfd_operator term` for CLP(FD)
  constraints.

## Future dialects

Likely additions, in rough priority order:

1. **GNU Prolog** — closest cousin to SWI; mostly module-system
   differences.
2. **SICStus Prolog** — commercial, similar operator table to SWI.
3. **Edinburgh Prolog** — historical; useful for reading classic
   textbook examples verbatim.
4. **B-Prolog** / **XSB** / **YAP** — research / specialty
   implementations.

Each gets its own crate pair following the
`<dialect>-prolog-lexer` / `<dialect>-prolog-parser` naming.

## ProbLog and other probabilistic extensions

ProbLog's `0.7 :: edge(a, b).` clause syntax is **not** a dialect of
Prolog under this taxonomy — it's an extension on top of any
Prolog dialect. The engine already handles probabilistic clauses
(`Fact::with_probability` / `Rule::with_probability` in
`logic-engine`); the source-level syntax is gated on lexer +
grammar work tracked separately. Today's recommended path: use
`prolog_loader::ProblogProgram` builder API (in
`prolog-loader::problog`) to construct probabilistic programs
without source parsing.
