# TE06 — Dotnet Document Pipeline

## Overview

This note scopes the first pure .NET document packages for the monorepo.
The goal is not to bolt F# on top of C#, or to wrap an external Markdown
engine. The goal is to grow a native document stack in **both** languages.

For this work:

- C# packages must be implemented in C#
- F# packages must be implemented in F#
- Neither language may delegate its core parsing work to the other
- No third-party Markdown or HTML rendering libraries are used

The only shared foundation is the .NET runtime and base class library.

## Package Order

The dependency chain is:

1. `document-ast`
2. `document-ast-to-html`
3. `commonmark-parser`
4. `commonmark`
5. `gfm-parser`
6. `gfm`

That order matters because the higher-level pipeline packages are intentionally
thin composition layers over lower-level pure implementations.

## First Tranche

The first implementation tranche focuses on:

- `document-ast`
- `commonmark-parser`
- `gfm-parser`

This gets the core typed IR in place first, then establishes a parser
architecture in both languages before the HTML rendering wrappers are added.

## Parser Architecture

The parser work follows the same two-phase shape used elsewhere in the repo:

1. **Block parsing**
   Markdown is scanned line by line into structural blocks such as headings,
   lists, code fences, blockquotes, tables, and paragraphs.
2. **Inline parsing**
   Paragraph and heading text is scanned for emphasis, strong, code spans,
   links, images, autolinks, breaks, raw HTML, and GFM extensions.

The initial implementation is intentionally modest in surface area but is laid
out so the remaining CommonMark and GFM edge cases can be added incrementally
without changing the public package boundaries.

## Non-Goals For This Tranche

- Full CommonMark 0.31.2 example-suite parity in one pass
- `commonmark` and `gfm` convenience wrappers before `document-ast-to-html`
- Bridging to Rust, C#, or any external Markdown runtime

## Success Criteria

The first tranche is successful when:

- both languages expose the same core document AST concepts
- both languages have pure parser entry points
- GFM-only constructs live behind `gfm-parser`, not inside the AST consumer API
- tests demonstrate the same baseline document behaviour in C# and F#
