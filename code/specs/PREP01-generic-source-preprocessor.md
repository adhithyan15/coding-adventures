# PREP01: a generic source preprocessor engine

**Status:** Draft — design spec, committed before implementation (sign-off = merge).

## 1. Why this exists

Several languages in this repo need a text- or token-level rewriting pass that
runs *before* parsing. Today none of them have one, and at least one frontend is
explicitly blocked on it:

- **C** — `code/specs/SIR27-c-to-semantic-ir.md` §"Preprocessor" scopes the v1
  frontend to "only `#include <stdint.h>` / `#include <stdio.h>` (ignored). No
  `#define`, no macros, no other directives," and its non-goals list names "the
  full preprocessor." The Rust `c-lexer` does not merely ignore directives, it
  *discards* them, and not even via a hook: `code/grammars/c/c.tokens` line 123
  carries `PREPROC = /#[^\n]*/` inside the grammar's `skip:` section, so every
  directive line is dropped before the parser or any `post_tokenize` hook could
  see it. There is no real C until this exists.
- **COBOL** — `COPY … REPLACING` is a text-substitution facility in the
  standard. `cobol-lexer` / `cobol-parser` implement none of it: `COPY` is not
  in the COBOL keyword list at all, and the only `REPLACING` present belongs to
  the runtime `INSPECT … REPLACING` statement, which is a different feature.
- Others in the same shape: PL/I's `%INCLUDE`, Fortran's `INCLUDE`, Erlang's
  `-define`/`-include`, and assembler macro facilities.

Writing one preprocessor per language duplicates the genuinely hard parts (the
source map, the non-recursive expansion algorithm, include-cycle and
resource bounds) four or five times, each with its own bugs. This spec defines a
shared **engine** with per-language **dialect** plug-ins.

## 2. What is shared and what is not

This is the central design question. A preprocessor that tries to be universal
at the *semantics* layer serves no language well: C macro expansion and COBOL
`COPY … REPLACING` pseudo-text matching are different algorithms with different
matching rules, and unifying them would produce a facility that is wrong for
both.

**Shared (this engine owns it):**

| Concern | Why it is shared |
|---|---|
| Source map | Keeping a token's true origin across splicing, inclusion and expansion is the same problem everywhere, and is brutal to retrofit. |
| Physical→logical line splicing | Backslash-newline (C), and the general shape of continuation. |
| Include resolution | Search-path policy, cycle detection, depth bounds, virtual filesystem. |
| Conditional-compilation stack | `if`/`elif`/`else`/`endif` nesting, skipped-group tracking, "skip but still track nesting" rules. |
| Macro table + expansion driver | The hide-set ("blue paint") non-recursive expansion algorithm is subtle, identical wherever token-level macros exist, and is the single most-commonly-miscoded part of a preprocessor. |
| Resource bounds | Expansion bombs, include bombs and path escapes are engine-level safety, not per-language policy. |

**Per-language (the dialect supplies it):**

| Concern | Example divergence |
|---|---|
| Directive syntax and names | `#define` vs `-define(…)` vs `COPY … REPLACING`. |
| Tokenization | Each frontend already owns its `.tokens` grammar. |
| `#if` expression semantics | C's integer constant expressions with `defined()` are not PL/I's. |
| Stringize / paste | C has `#` and `##`; COBOL pseudo-text has neither. |
| Replacement matching | C substitutes by parameter name; COBOL matches pseudo-text runs. |

**Explicitly not unified:** COBOL's `COPY … REPLACING` will use the include,
source-map, dispatch and bounds layers and bring *its own* replacement matcher.
That is the intended shape, not a shortfall. Slice 4 exists specifically to
prove the trait boundary is not C-shaped.

## 3. Layer position, and a correction to an existing spec

`code/specs/lexer-parser-hooks.md` already anticipates this work. It defines
`pre_tokenize` (`str → str`) and `post_tokenize` (`[Token] → [Token]`) hook
chains on the shared lexer, and names as use cases "C #include file insertion"
and "line splicing" at `pre_tokenize`, and "C #define macro expansion",
"C #ifdef conditional compilation" and "Token pasting (C ## operator)" at
`post_tokenize`.

**That split is not faithful to C, and this spec corrects it.** Inclusion cannot
be a text-level pass that runs before conditional compilation, because the two
are mutually dependent:

```c
#if USE_FAST_PATH      /* whether the #include happens at all …   */
#  include "fast.h"    /* … and fast.h #defines symbols that …    */
#endif
#if FAST_VERSION > 2   /* … later #if directives test.            */
```

A text-level include pass would pull in `fast.h` unconditionally, and a
token-level conditional pass running afterwards could not have seen the macros
`fast.h` defined in time to affect the inclusion decision. Real C is defined as
a single ordered traversal in which inclusion, conditional selection and macro
definition all interleave.

Therefore:

- **`pre_tokenize` carries line splicing only** (and, for COBOL, the existing
  column strip). These are genuinely context-free text transforms.
- **`post_tokenize` carries the whole preprocessor as one pass.** The engine
  owns include resolution itself and re-lexes included files through the
  dialect's own lexer, rather than pasting their text in beforehand.

The hook signature still fits: the engine is constructed with its configuration
and hands out a closure, so each invocation is the pure `[Token] → [Token]`
function the hook contract requires. `lexer-parser-hooks.md` gets an amendment
noting the corrected ordering; its API is unchanged.

```text
source text
   │
   ├─ pre_tokenize:  line splicing (and COBOL column strip)
   ▼
dialect lexer  (existing per-language .tokens grammar)
   │
   ├─ post_tokenize: PREP01 engine — one pass:
   │                   include ⟷ conditionals ⟷ macro definition/expansion
   ▼
[Token] + SourceMap  →  existing parser (unchanged)
```

## 4. Source locations: a side table, not a wider `Token`

`lexer::token::Token` carries `line` and `column` but **no file identity**. Once
tokens can arrive from an included file or a macro body, `line:column` alone is
ambiguous.

The obvious fix — add a `file` field to `Token` — is rejected: **136 crates in
`code/packages/rust/` depend on the shared `lexer`**, so widening that struct is
a repo-wide blast radius for a feature two frontends need.

Instead the engine keeps a **side table**, the design GCC's line maps and LLVM's
`SourceManager` both use:

- Each emitted token keeps a **presumed** `line`/`column`: the location in the
  file whose text it came from. Consumers that do not care about inclusion see
  plausible, useful numbers, exactly as today.
- The engine additionally returns a `SourceMap` with one `Locus` per emitted
  token, resolving to `(FileId, line, column, expansion_chain)`, where
  `expansion_chain` records the macro invocations a token passed through.
  Diagnostics that need full fidelity ("in expansion of `FOO`, from `bar.h:12`,
  included from `main.c:3`") consult it.

`Token` is not modified. No existing frontend changes.

**Known limitation, stated deliberately:** the per-token `Locus` vector is
positional, so it is valid only for a consumer that does not reorder or
synthesise tokens between the engine and the parser. That holds for the intended
pipeline. If a future consumer needs to reorder, the map must move into `Token`
or into a wrapper type, and that will be a separate, scoped decision — not
something to guess at now.

## 5. Engine interface (Rust, `source-preprocessor` crate)

```rust
/// What a language plugs in. The engine never lexes, and never decides
/// what a directive means.
pub trait Dialect {
    /// Recognise a logical line as a directive. `None` = ordinary source.
    fn directive_of(&self, line: &[Token]) -> Option<Directive>;

    /// Evaluate a conditional's controlling expression.
    fn eval_condition(&self, toks: &[Token], macros: &MacroTable)
        -> Result<bool, PpError>;

    /// Lex an included unit's text. The engine calls this, never a lexer
    /// directly, so each language keeps its own grammar.
    fn lex(&self, text: &str, file: FileId) -> Result<Vec<Token>, PpError>;

    /// Stringize (C `#`). `None` = the dialect has no such operator.
    fn stringize(&self, toks: &[Token]) -> Option<Token> { None }

    /// Token paste (C `##`). `None` = unsupported.
    fn paste(&self, left: &Token, right: &Token) -> Option<Token> { None }
}

/// All file access goes through this. There is no ambient filesystem
/// access in the engine, so tests run fully in memory.
pub trait SourceFs {
    fn resolve(&self, request: &IncludeRequest) -> Result<FileId, PpError>;
    fn read(&self, file: FileId) -> Result<&str, PpError>;
}
```

## 6. Resource and safety bounds

These are engine-level and on by default; a dialect may tighten but not remove
them. Each has a test in the slice that introduces it.

| Bound | Default | Failure mode it prevents |
|---|---|---|
| Include nesting depth | 200 | Include bomb via deep chains. (C requires ≥15 to work.) |
| Include cycle detection | always on | `a.h` includes `b.h` includes `a.h`. |
| Path containment | always on | `#include "../../../../etc/passwd"`. Resolved paths are canonicalised and must remain under a declared search root. |
| Macro expansion depth | 200 | Mutually recursive function-like macros. |
| Total emitted tokens | configurable cap | Expansion bombs: chained `#define A B B` doubles per level, so ~40 levels exhausts memory from a few lines of input. |
| Conditional nesting depth | 200 | Pathological nesting. |

Every bound produces a diagnostic naming the construct and its source location,
never a panic, an abort, or a silent truncation.

## 7. Implementation sequencing

One slice per PR, following the discipline `AOT00-T2-exceptions.md` §12 uses.
Each slice states an executable acceptance criterion.

**Slice 1 — engine core, no macros.**
`SourceFs`, `FileId`, `SourceMap`/`Locus`, line splicing, include resolution,
the conditional stack, directive dispatch, and every §6 bound except macro
depth. Proven against a deliberately non-C **synthetic dialect** defined in the
crate's own tests, so nothing about the boundary can be quietly shaped by C.
*Acceptance:* a synthetic-dialect program exercising nested includes,
conditionals and every §6 bound preprocesses to an exact expected token stream,
and each bound has a test proving it yields a located diagnostic.

**Slice 2 — macro expansion.**
`MacroTable`, object-like then function-like macros, argument pre-expansion, and
the hide-set algorithm that makes expansion non-recursive. `#`/`##` are routed
through the dialect hooks, not built in.
*Acceptance:* the classic self-referential and mutually-recursive cases
terminate with the standard-mandated output rather than looping; argument
pre-expansion ordering matches the C rules on a table of cases drawn from the
standard's own examples.

**Slice 3 — the C dialect, and real C.**
`c.tokens` stops discarding `#…` lines; `c-lexer` surfaces directive tokens; a
`CDialect` implements §5; `c-to-semantic-ir` runs the engine as its
`post_tokenize` hook. `SIR27`'s preprocessor scope statement is updated.
*Acceptance:* a C program using `#define` (object- and function-like), `#if`/
`#ifdef`/`#else`/`#endif` and a real project-local `#include` compiles through
`c-to-semantic-ir` and executes with the expected result — the first C program
in this repo to depend on preprocessing.

**Slice 4 — COBOL `COPY … REPLACING`, the boundary proof.**
A `CobolDialect` that reuses the include, source-map, dispatch and bounds layers
while supplying its own pseudo-text replacement matcher.
*Acceptance:* a COBOL program using `COPY … REPLACING` compiles and runs; and
the engine core is unchanged by this slice, or any change it did require is
called out explicitly in the PR as a genuine genericity defect found by the
second customer.

**Later slices (not scoped here):** `__LINE__`/`__FILE__`, `#pragma once`,
variadic macros, `__has_include`, and a minimal in-VFS `stdint.h`/`stdio.h`.

## 8. Non-goals

- C++ templates or two-phase name lookup.
- Lisp reader macros and Rust-style procedural macros: these operate on an AST
  *after* parsing, and belong to `lexer-parser-hooks.md`'s `post_parse` hook.
- Replacing any existing `.tokens` grammar or parser.
- Preprocessing as a user-facing CLI. This is a library consumed by frontends.

## 9. Open questions for sign-off

1. **C conformance target.** This spec assumes C89/C99 macro-expansion
   semantics for the core algorithm, with variadic macros and `__has_include`
   deferred to later slices. Confirm that is the right line.
2. **Header provenance.** Slice 3 proves project-local `#include "…"`. Whether
   system `#include <…>` resolves against a small repo-vendored header set or
   against the host toolchain's real headers is a policy decision with
   reproducibility consequences, and is deliberately left open here.
