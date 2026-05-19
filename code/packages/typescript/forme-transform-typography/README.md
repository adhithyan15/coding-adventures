# @coding-adventures/forme-transform-typography

Apply smart-quote / em-dash / en-dash / ellipsis (and optional
ligature) substitution to every `TextNode` in a `DocumentNode`.
FM00 v0 §5.3 transform.

Pure transform: walks the input document and returns a fresh
`DocumentNode` with typography corrections applied to prose
text.  Code blocks, code spans, raw HTML, URLs, and image
alt-text pass through unchanged (smart-quoting code samples
would break syntax).

Seventh FM00 v0 stage package — joins
[`forme-feeds`](../forme-feeds),
[`forme-opengraph`](../forme-opengraph),
[`forme-index-renderer`](../forme-index-renderer),
[`forme-transforms`](../forme-transforms),
[`forme-transform-autolink-headings`](../forme-transform-autolink-headings),
[`forme-transform-toc`](../forme-transform-toc).

## Quick start

```ts
import { typography } from "@coding-adventures/forme-transform-typography";

const prettified = typography(doc);
// "He said \"hello\" -- don't worry, it's fine..."
// →
// "He said “hello” – don’t worry, it’s fine…"

// Opt into copyright / registered / trademark ligatures.
const corp = typography(doc, { ligatures: true });
// "Forme(tm) is free (c) 2026"
// →
// "Forme™ is free © 2026"
```

## Substitutions

| Source                       | Output    | Codepoint |
|------------------------------|-----------|-----------|
| `---`                        | em dash   | U+2014    |
| `--`                         | en dash   | U+2013    |
| `...`                        | ellipsis  | U+2026    |
| `"` (after WS / start)       | left-DQ   | U+201C    |
| `"` (otherwise)              | right-DQ  | U+201D    |
| `'` (after alphanumeric)     | right-SQ  | U+2019    |
|                                (apostrophe — `don't`, `it's`)    |
| `'` (after WS / start)       | left-SQ   | U+2018    |
| `'` (otherwise)              | right-SQ  | U+2019    |
| `(c)` / `(C)` (ligatures)    | copyright | U+00A9    |
| `(r)` / `(R)` (ligatures)    | registered| U+00AE    |
| `(tm)` / `(TM)` (ligatures)  | trademark | U+2122    |

## API

### `typography(doc, options?): DocumentNode`

Walks the document depth-first, returns a fresh copy.  Every
`TextNode.value` is run through `typeset` with the supplied
options.

### `typeset(text, options?): string`

String-level entry — useful for callers that want to typeset a
string outside the AST context (e.g. plain-text metadata).

### Types

```ts
interface TypographyOptions {
  readonly smartQuotes?: boolean;  // default true
  readonly dashes?: boolean;       // default true
  readonly ellipsis?: boolean;     // default true
  readonly ligatures?: boolean;    // default false
}
```

## Why a character loop, not regex?

The naive approach to typography is chained `String.prototype.replace`
with patterns like `/--/g` / `/\.\.\./g` / `/(\w)'(\w)/g`.  Three
problems:

1. **Order matters.**  `---` must be replaced before `--`, or
   the em-dash pattern never matches.  Multi-rule precedence is
   fragile across edits.
2. **Open/close context.**  Whether `"` becomes `"` or `"`
   depends on the previous character's class.  Regex
   lookbehinds are awkward and not universally supported.
3. **ReDoS warnings.**  CodeQL flags polynomial regex on
   uncontrolled data even when the actual runtime is linear.

This package uses a single forward `for` loop over
`charCodeAt`-based lookahead — unambiguously O(n), trivially
passes any ReDoS analysis, and explicit lookahead is more
obvious than a stack of regex alternatives.

## Pass-through nodes (NOT typeset)

| Node                  | Reason                                        |
|-----------------------|-----------------------------------------------|
| `CodeBlockNode.value` | Would break source-code syntax                |
| `CodeSpanNode.value`  | Same — inline code is verbatim                |
| `RawBlockNode.value`  | By definition the renderer wants verbatim     |
| `RawInlineNode.value` | Same                                          |
| `LinkNode.destination`| URLs must not get smart-quoted                |
| `ImageNode.destination` | Same                                        |
| `ImageNode.alt`       | Passthrough by default (v0 chooses safety)    |
| `AutolinkNode.destination` | URL passthrough                           |

Recursed-into nodes: paragraph, heading, blockquote, list,
list_item, task_item, table / table_row / table_cell, emphasis,
strong, strikethrough, link (label only, not URL).

## Reproducibility (FM03)

Same input `DocumentNode` → byte-identical output.  The
substitution algorithm is deterministic; the AST walker is a
pure depth-first descent.  Safe to feed into cache key
derivation.

## Security posture

Four concerns explicitly addressed (pre-push review):

- **No AST mutation.**  Input `DocumentNode` is never mutated;
  every returned node is freshly constructed.  Tests pin
  fresh-tree guarantee (output `!== input` at every level)
  and JSON-snapshot tests confirm no input changes.
- **Deterministic substitution.**  Single forward pass over the
  string; no global state, no Map/Set iteration, no randomness.
  Same input → byte-identical output.
- **No ReDoS.**  The substitution engine uses a `for` loop with
  `charCodeAt`-based lookahead — zero regex, zero backtracking.
- **Transformed text is data, not markup.**  The output of
  `typeset` contains only the source characters plus typographic
  replacements (`U+201C`, `U+2013`, `U+00A9`, etc.) — no HTML
  metacharacters introduced.  Renderers still own the
  HTML-escape boundary as they would for raw text.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

76 tests across 2 files:

- `typeset.test.ts` (44) — every substitution rule (smart
  quotes double + single + apostrophes, dashes precedence,
  ellipsis vs 1/2/4 dots, ligatures all cases plus opt-in
  gating), option toggles, identity fast-path when all
  disabled, combinations, purity / determinism / non-string
  coercion, Unicode passthrough (CJK + emoji).
- `walk.test.ts` (32) — basic prose in paragraph / heading /
  emphasis / strong / strikethrough / link label, pass-through
  nodes (code_block, code_span, raw_block, raw_inline, image
  destination + alt, autolink, hard/soft break, thematic
  break), block containers (blockquote, list, list_item,
  task_item, table cells header + body), nested DocumentNode,
  options propagation to deep text, purity (no input mutation,
  fresh tree even for passthrough nodes, byte-identical
  output, defaults match no-options call), defensive cases for
  non-tree BlockNode variants as direct siblings.

Coverage: **97.32% line / 98.61% branch** across all source
files with logic.  Uncovered lines are TypeScript `never`
exhaustiveness guards that cannot fire at runtime.

## Spec adherence

Implements FM00 v0 §5.3 `transform-typography`.  No spec
divergences.

## v0 simplifications

- **No locale-aware quote pairs.**  Always uses English curly
  quotes (`"" ''`).  German `„""`, French `«»`, etc. need a
  separate option deferred to v1.
- **Image alt-text passes through unchanged.**  v1 might add an
  `imageAlt: true` option for callers that want it typeset.
- **Heuristic limits.**  The apostrophe rule (between
  alphanumerics → right-SQ) doesn't catch every edge case
  ('twas, `rock 'n' roll`, etc.).  Renders correctly for the
  common cases; trying to be exhaustive without context
  would over-fit.
- **No abbreviation protection.**  `Mr...` becomes `Mr…`.
  Common abbreviations should use a hard space or trailing
  zero-width joiner if needed.
