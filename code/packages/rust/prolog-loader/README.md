# prolog-loader (Rust)

Load Prolog source text into a `logic_engine::KnowledgeBase`. Closes
the text → engine pipeline.

## Where It Fits

```text
   Prolog source text
        │
        ▼
   prolog-lexer                                (grammar-driven tokens)
        │
        ▼
   prolog-parser                               (GrammarASTNode + ProgramItem)
        │
        ▼
   prolog-loader                               ← this crate
        │
        ▼
   logic_engine::KnowledgeBase + Vec<query>
        │
        ▼
   logic_engine::search(...)
```

After this crate, a Prolog program written in plain text runs
end-to-end on the Rust engine.

## API

```rust
use prolog_loader::{load_source, LoadedProgram};
use logic_engine::{search, SearchMode, SearchResult};

let src = "\
    father(homer, bart).\n\
    father(homer, lisa).\n\
    parent(X, Y) :- father(X, Y).\n\
    ?- parent(homer, Who).\n\
";

let LoadedProgram { kb, queries } = load_source(src)?;

for query in &queries {
    // A query body is a conjunction of goals; we run them as one
    // synthetic conjunction by calling search on each goal in turn
    // (or by building a `,/2` term, depending on caller preference).
    for goal in query {
        let r = search(goal, &kb, SearchMode::AutoDetect);
        // ...
    }
}
```

## Negation-as-Failure

The parser produces a compound term `'\+'(G)` for `\+ G`. The loader
recognizes this pattern and emits `BodyLiteral::Neg(G)` in the
rule's body. Every other body goal becomes `BodyLiteral::Pos(_)`.

## What's Supported (this slice)

- Facts (`p.`, `p(a).`, `p(X, Y).`)
- Rules (`p :- q.`, `p :- q, r.`, with NAF in body literals)
- Queries (`?- ...`)
- Lists (via the parser's canonical cons-cell encoding)
- Variable identity shared within a clause; fresh anonymous variables

## What's Not in This Slice

- Module declarations
- Operator directives
- DCG expansion (`-->`)
- Probabilistic facts / rules (the LP19 probabilistic surface is
  available via `logic-engine` directly; a Prolog-textual syntax
  for probability annotations would be a follow-up sub-spec)

## Status

Experimental. Sufficient to load and execute the example programs
used in `LP19`'s and `ADJ`'s worked examples.
