# source-preprocessor

The shared preprocessor engine for this repo's language frontends (PREP01).

## What it is for

A preprocessor rewrites source *before* the parser sees it — pulling in other
files, selecting between alternatives, and expanding macros. Several languages
here need one and none have one, and at least one is hard-blocked without it:

> `SIR27` scopes the C frontend to *"only `#include <stdint.h>` / `#include
> <stdio.h>` (ignored). No `#define`, no macros, no other directives."*
> `c-lexer` does not merely ignore directives, it **destroys** them —
> `c.tokens` carries `PREPROC = /#[^\n]*/` in its `skip:` section, so a
> directive line never reaches the parser or any hook.

Writing one preprocessor per language would duplicate the genuinely hard parts
several times over. The hard parts are *not* the directive syntax, which is
easy and different everywhere. They are the source map, the non-recursive
expansion algorithm, and being safe to point at hostile input.

So this crate is an **engine**, and each language supplies a **dialect**.

## How it fits in the stack

```
source text
   │
   ├─ pre_tokenize:  line splicing only
   ▼
the language's own lexer   (its existing .tokens grammar, unchanged)
   │
   ├─ post_tokenize:  THIS ENGINE, in one pass —
   │                  include ⟷ conditionals ⟷ macros
   ▼
[Token] + SourceMap  →  the language's existing parser, unchanged
```

The engine conforms to the `post_tokenize` hook contract in
`code/specs/lexer-parser-hooks.md`. Note that PREP01 **amends** that spec:
it had placed `#include` at `pre_tokenize`, which cannot work for C, because an
`#if` decides whether an `#include` happens while the included file defines
names that later `#if`s test. They interleave, so they are one pass.

## Division of labour

| The engine owns | A `Dialect` owns |
|---|---|
| source map | directive syntax and names |
| include resolution, cycles, bounds | tokenization |
| the conditional stack | what a condition means |
| expansion algorithm (slice 2) | stringize / paste, replacement matching |

Deliberately **not** unified: COBOL's `COPY … REPLACING` is pseudo-text
matching, a different algorithm from macro expansion. It will reuse the
include, source-map, dispatch and bounds layers and bring its own matcher.

## Usage

```rust
use coding_adventures_source_preprocessor::{Bounds, MemoryFs, preprocess};

let out = preprocess(tokens, file, &MyDialect, &mut fs, Bounds::default())?;
// out.tokens  — feed straight to your existing parser
// out.map     — per-token (file, line, column, expansion chain)
```

## Three decisions worth knowing

**One pass, not include-then-conditionals.** See above; also `engine.rs`.

**A side table, not a wider `Token`.** `lexer::token::Token` has `line` and
`column` but no file identity, and **136 crates** depend on the shared `lexer`.
Widening that struct would be a repo-wide change to serve two frontends. So
tokens keep a *presumed* line/column — every existing parser sees sensible
numbers and needs no changes — and provenance lives beside the stream, the way
GCC's line maps and LLVM's `SourceManager` do it.

**Bound work and bytes, not only shape.** Counting nesting depth and emitted
tokens feels thorough and leaves three doors open:

- tokens expanded to evaluate a condition are *consumed*, never emitted, so a
  40-deep doubling chain inside `@if` makes 2⁴⁰ tokens a token cap never sees;
- stringize and paste grow *bytes* while holding the token count flat;
- a fan-out include graph is neither deep nor cyclic — ten includes eight
  levels down is 10⁸ file reads that both the depth bound and cycle detection
  wave through.

So the counters are cumulative totals, and there is a `fuel` budget as a
catch-all for the dimension nobody enumerated. Every bound is finite by default
and the configuration surface is **tighten-only** — for the embedding host as
much as for a dialect. A budget that *can* be set to infinity eventually is.

## Status

**Slice 1**: includes, conditionals, source mapping, bounds, `MemoryFs` and
`RootedFs`.

**Slice 2** (this release): macro expansion. `MacroTable`, object-like and
function-like macros, argument pre-expansion, and Prosser's per-token hide sets
— which is what makes expansion terminate on the self-referential and mutually
recursive cases rather than looping. Stringize and paste stay routed through the
`Dialect` hooks and are **not** built in; MacroOct declines both while
nevertheless having a full macro facility, which is a stronger genericity result
than a dialect that quietly needed them.

`Directive::Undef(name)` removes the current definition of a raw name, whether
object-like or function-like. Removing an absent name has no effect; a directive
inside a skipped conditional group leaves the table unchanged. The C dialect
supplies the `#undef` syntax while this engine owns the table mutation.

Controlling expressions are macro-expanded before `Dialect::eval_condition`
sees them, so `@if LED_PORT == 1` takes the true branch after
`@define LED_PORT 1`. A name that *survives* expansion is genuinely undefined
and still reads as 0, which is C's rule.

Operators whose operands must not expand have a deliberately narrower seam.
`Dialect::prepare_condition` runs before ordinary condition expansion and can
query `MacroTable::is_defined`; a C dialect can therefore reduce
`defined(NAME)` to a truth-value token before `NAME` becomes an expansion
candidate. The engine then expands the rest of the expression itself. The
default hook changes nothing, and the engine rejects a result with more tokens
than the source expression so preparation cannot bypass the production budget.

That was broken when `@define` first landed and is worth recording for where it
had to be fixed: `eval_condition` receives a bare `&[Token]` with no table and
no expansion applied, so *no dialect could have fixed it* — it was an engine
defect surfacing in a dialect (VM-068).

VM-069 wires the expansion arena through the engine. An expanded token keeps
the macro body's physical file/line/column, while `Locus::expansion` names an
interned chain whose nodes record the macro name, definition site, invocation
site, and parent expansion. Nested object-like expansion and function-argument
pre-expansion both preserve that chain, while the side table remains
`O(tokens + expansions)`.
Forwarding an already expanded argument through another function macro keeps
the inner macro in the chain. Shared expansion nodes stay immutable; copied
prefixes are memoized and capped by the expansion-round limit.

One implementation limitation remains recorded rather than papered over:
argument pre-expansion is the one place the expander recurses natively; it is
bounded by `macro_depth`, which assumes roughly 1 MiB of stack. The generic
`defined()` operand contract is now ready; recognizing C's exact tokens belongs
to the later C dialect slice.

**Slice 3**: MacroNib, a second dialect in a third syntax. **Slice 4**: C.
**Slice 5**: COBOL `COPY`.

See `code/specs/PREP01-generic-source-preprocessor.md`.
