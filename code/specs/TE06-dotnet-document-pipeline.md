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

## C# HTML Renderer Tranche

The next C# tranche adds only `document-ast-to-html` as a pure renderer over
the public `document-ast` types. It does not duplicate or silently promote the
already-substantive `commonmark-parser` package, and it does not add the thin
`commonmark` facade before its parser prerequisite is qualified.

Renderer tests are independently authored from the TE00 public contract. They
exercise every C# `document-ast` node through `DocumentAstToHtml.ToHtml`,
including tight and loose lists, ordered starts, escaping, raw-format routing,
tables, task items, and dangerous URL schemes. These vectors are deliberately
separate from GFM parser adoption: rendering extension node types already in
the AST does not claim that C# consumes the GFM conformance corpus.

### Public API

```csharp
using CodingAdventures.DocumentAst;
using CodingAdventures.DocumentAstToHtml;

var document = new DocumentNode([
    new HeadingNode(1, [new TextNode("Hello")]),
]);
var html = DocumentAstToHtml.ToHtml(document);
```

`CodingAdventures.DocumentAstToHtml.DocumentAstToHtml.ToHtml` accepts a
`DocumentNode` plus immutable render options. Raw HTML passes through by
default for CommonMark compatibility; callers rendering untrusted input must
request raw-node suppression or sanitize the AST before rendering. Text,
attribute values, code, and link/image destinations are escaped. The renderer
removes browser-ignored control and zero-width characters before rejecting
`javascript:`, `vbscript:`, `data:`, and `blob:` destinations.

### Resource And Ownership Contract

- parsing retains the existing bounded public C# parser entry point;
- rendering is an in-memory tree walk with explicit node-depth, node-count,
  and output-character ceilings;
- input AST records and child collections are borrowed and never mutated;
- rendering returns a newly owned string and performs no filesystem, network,
  process, environment, clock, or random access; and
- a limit failure throws `InvalidOperationException` before returning partial
  output.

### Package And Build Evidence

The new renderer root requires normal NuGet metadata, `README.md`,
`CHANGELOG.md`, `required_capabilities.json`, Unix `BUILD`, and Windows
`BUILD_windows` fronts. The build fronts run native .NET tests with production
line coverage above the repository's 80% floor. The renderer declares only
`document-ast`; test-only dependencies remain in the test project.

### Discovered Parser And Facade Sequence

Executing the existing C# parser and new renderer against every record in the
official CommonMark 0.31.2 corpus produced 233 exact matches out of 652. That
is useful qualification evidence, but it is not CommonMark conformance and
must not be hidden behind a facade that implies parity.

The remaining C# pipeline therefore has two separately owned steps:

1. `commonmark-parser-csharp-652-reconciliation` reconciles the existing parser
   against all 652 official cases without using a fixture lookup, external
   process, cross-language bridge, or third-party Markdown engine.
2. `commonmark-csharp-facade-lane-parity` adds the thin facade only after both
   the parser reconciliation and this renderer package have merged.

This renderer tranche does not implement or qualify `commonmark-parser`, add
the `commonmark` facade, implement or qualify `gfm`, consume the GFM extension
fixture, centralize the repository's duplicated CommonMark corpus, or claim
sanitization policy beyond the renderer's documented raw-node and URL rules.
