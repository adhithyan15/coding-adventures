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

## F# HTML Renderer Tranche

The next F# tranche adds only `document-ast-to-html` as a native, pure renderer
over the existing `CodingAdventures.DocumentAst.FSharp` discriminated unions.
It does not edit or qualify `commonmark-parser`, add the `commonmark` facade,
adopt the GFM corpus, or bridge to the C# renderer.  Rendering the table,
task-item, and strikethrough cases already present in the AST is consumer
support, not a GFM parser-conformance claim.

### Public API

The package namespace is `CodingAdventures.DocumentAstToHtml.FSharp`.  Its
qualified `DocumentAstToHtml` module exposes:

```fsharp
val defaultOptions: RenderOptions
val escapeHtml: string -> string
val render: DocumentNode -> string
val renderWithOptions: RenderOptions -> DocumentNode -> string
```

`RenderOptions` is an immutable record containing `SuppressRawHtml: bool`.
`render` is equivalent to `renderWithOptions defaultOptions`.  Both borrow the
input tree without changing any record, union, or child list and return a newly
owned HTML string.

### Deterministic rendering contract

- documents concatenate block output in source order;
- headings, paragraphs, code blocks, blockquotes, thematic breaks, ordered and
  unordered lists follow the established TE00/CommonMark HTML spellings;
- tight list paragraphs omit `<p>` while loose list paragraphs retain it;
- task items prepend inert disabled checkboxes, with `checked=""` only for a
  checked item;
- tables group header and body rows into `thead` and `tbody`, preserve column
  order, and emit invariant `left`, `right`, or `center` alignment values;
- text, code, language hints, titles, alternative text, and attributes escape
  `&`, `<`, `>`, and `"`;
- HTML raw nodes pass through only when their format is exactly `html` and raw
  suppression is false; other formats are omitted; and
- all numeric rendering uses invariant culture and output uses `\n`, so locale
  and host line-ending settings cannot affect the fragment.

Link and image destinations preserve safe relative and arbitrary-scheme URLs
after removing C0/C1 controls and U+200B/U+200C/U+200D/U+2060/U+FEFF.  Scheme
inspection also ignores leading ASCII spaces and blocks `javascript:`,
`vbscript:`, `data:`, and `blob:` case-insensitively.  Non-email autolinks
percent-encode non-URL Unicode scalars as uppercase UTF-8 bytes.  This targeted
URL boundary and raw-node suppression do not make the renderer a complete HTML
sanitizer; callers needing policy-level sanitization must sanitize the AST.

### Resource and error contract

The document root is depth zero.  Each rendered child node increases depth by
one.  Rendering fails with `InvalidOperationException` before returning any
partial result when depth exceeds 256, rendered nodes exceed 100,000, or HTML
output exceeds 1,000,000 UTF-16 characters.  URL stripping, autolink expansion,
and the public escaping helper obey the same character ceiling.  In addition,
one render charges every link, image, and autolink destination against a shared
1,000,000 UTF-16 input-unit normalization budget before allocation or scanning.
The charge includes repeated immutable aliases and values that are later
stripped or rejected, accepts the exact ceiling, and resets for each public
render call. One render also charges at most 100,000 table-alignment entries
before converting them for indexed row rendering, including every visit to a
repeated immutable list alias. Heading levels outside 1 through 6 are rejected.
Rendering performs no filesystem, network, process, environment, clock,
randomness, or third-party renderer access.

### Package evidence

The new root contains a publishable NuGet project, README and changelog, empty
capability manifest, Unix `BUILD`, and Windows `BUILD_windows`.  Native xUnit
tests enter only through public APIs and exercise every existing F# AST case,
exact layout, raw routing, URL attacks, ownership, limit boundaries, and repeat
execution.  Coverlet is an explicit test dependency, filtered to the F#
renderer assembly, with line coverage enforced above the repository's 80%
floor and targeted at 95% or better.
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
- one render admits at most 1,000,000 aggregate UTF-16 code units of raw URL
  destinations across links, images, and autolinks. Every visit, including an
  alias of an earlier node or string, charges the shared budget before URL
  normalization allocates or scans. Values stripped empty or rejected as unsafe
  still count. The inclusive budget resets for every `ToHtml` call and bounds
  normalization work independently of the resulting HTML size;
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
The Windows front must still execute native tests and coverage when its setup
directories already exist; a successful directory check is not build evidence.

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
