# Haskell Layout Mode And Versioned Grammar Roadmap

## Overview

This spec adds the first generic lexer feature we need before full Haskell
support is realistic in the repo: a **layout-aware lexer mode** for
Haskell-style offside rules.

The existing `mode: indentation` support is deliberately Python-shaped. It emits
`INDENT` and `DEDENT` tokens at physical line starts and works well for Python
and Starlark. Haskell is different:

- layout is introduced only after specific keywords (`let`, `where`, `do`, `of`)
- the virtual structure is `{`, `;`, and `}`, not `INDENT`/`DEDENT`
- layout can begin after a keyword even when that keyword is not at line start
- an explicit `{ ... }` suppresses implicit layout for that construct

This spec is intentionally **lexer-focused**. Full Haskell parsing also needs a
later fixity-resolution phase, but that is a parser or semantic pass and not
part of this document.

## Why This Exists

We want to support versioned Haskell grammars across the language line, starting
from the first standardized report:

- Haskell 1.0
- Haskell 1.1
- Haskell 1.2
- Haskell 1.3
- Haskell 1.4
- Haskell 98
- Haskell 2010

The exact grammar files for those versions will live under a future
`code/grammars/haskell/` directory. All of them need the same core lexical
capability: convert offside layout into an explicit token stream the parser can
consume consistently.

## Relationship To Existing Specs

- `tokens-format.md`: adds one new lexer mode and one new section
- `lexer-parser-hooks.md`: remains valid; hooks still compose around layout mode
- `lexer-parser-extensions.md`: this spec complements those generic extensions
- `14-starlark.md`: similar motivation, but Haskell layout is not Python-style

## Design Principles

1. Generic capability first, language policy second.
2. Keep the physical token pass and the layout transform separate.
3. Preserve compatibility with the existing grammar-driven lexer API.
4. Make versioned Haskell grammars data-driven, not hardcoded in the engine.

## `.tokens` Format Changes

### New Mode: `mode: layout`

```text
mode: layout
```

This activates a second lexer pass after physical tokenization. The first pass
still produces normal tokens and `NEWLINE`s. The second pass injects virtual
layout tokens according to the rules below.

### New Section: `layout_keywords:`

```text
layout_keywords:
  let
  where
  do
  of
```

These are the token values that introduce a layout context when `mode: layout`
is active. The list is intentionally declarative so other layout-sensitive
languages can reuse the same engine shape without hardcoding Haskell words into
the generic lexer.

## Generic Layout Algorithm

The lexer runs in two stages:

1. **Physical tokenization**
   - tokenize with the normal grammar-driven machinery
   - emit regular tokens plus physical `NEWLINE`
   - preserve source positions

2. **Layout transform**
   - scan the physical token stream
   - after a layout introducer:
     - if the next significant token is an explicit `{`, do nothing
     - otherwise inject a virtual `{` before the next significant token and
       record that token's column as the layout column
   - after each physical newline:
     - if the next significant token is at the same column as the active layout
       context, inject a virtual `;`
     - if it is to the left, inject one or more virtual `}` until the stack
       matches
   - at EOF, close any remaining implicit layout contexts with virtual `}`

### Synthetic Tokens

The layout transform emits:

- `VIRTUAL_LBRACE` with value `{`
- `VIRTUAL_SEMICOLON` with value `;`
- `VIRTUAL_RBRACE` with value `}`

The parser can match these by literal value, which keeps grammars compatible
with both explicit and implicit block structure.

## Prototype Scope

The first prototype does **not** attempt the entire Haskell report. It focuses
on the smallest generic engine slice that proves the model:

- parse `mode: layout`
- parse `layout_keywords:`
- inject virtual `{ ; }` for simple `let`/`where`/`do`/`of` blocks
- cancel implicit layout when an explicit `{` follows the introducer
- close outstanding layout contexts at dedent and EOF

Known prototype limitations:

- nested layout inside parentheses or brackets is not complete yet
- the full Haskell lexical grammar is not implemented yet
- pragmas, nested comments, string gaps, and fixity are out of scope here

That is acceptable for phase 1. The goal is to make the engine shape real, not
to finish Haskell in one change.

## Ruby Prototype

The first implementation language is Ruby because the shared generic lexer and
parser infrastructure already lives there:

- `code/packages/ruby/grammar_tools/`
- `code/packages/ruby/lexer/`
- `code/packages/ruby/parser/`

Phase 1 Ruby changes:

1. Accept `mode: layout` in `TokenGrammar`.
2. Accept `layout_keywords:` in `TokenGrammar`.
3. Extend the Ruby `GrammarLexer` with a `tokenize_layout` path.
4. Add focused tests for virtual token insertion.

## Future Haskell Grammar Layout

Once the lexer mode is stable, versioned Haskell grammars should follow the
same pattern as the Python, Java, C#, and TypeScript grammar families:

```text
code/grammars/haskell/
  haskell1.0.tokens
  haskell1.0.grammar
  haskell1.1.tokens
  haskell1.1.grammar
  ...
  haskell2010.tokens
  haskell2010.grammar
```

Expected wrapper packages later:

- `code/packages/ruby/haskell_lexer/`
- `code/packages/ruby/haskell_parser/`

Other languages can follow once the Ruby prototype is stable.

## Non-Goals

This spec does not define:

- full Haskell parser grammar
- user-declared fixity resolution
- GHC extension support
- semantic analysis or desugaring

Those are separate phases. This document is only about making the lexer capable
of expressing Haskell's layout-sensitive surface syntax.

## Success Criteria

Phase 1 is successful when:

1. the `.tokens` format accepts `mode: layout`
2. the Ruby lexer can inject virtual layout tokens for simple blocks
3. explicit `{ ... }` suppresses implicit layout in the prototype
4. the design is documented well enough to build versioned Haskell grammars on
   top of it next
