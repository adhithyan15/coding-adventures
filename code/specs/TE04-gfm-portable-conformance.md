# TE04 — Portable GFM Extension Conformance

## Purpose

The repository has eleven structurally present `gfm-parser` lanes, but package
presence does not prove that they agree on GitHub Flavored Markdown.  The base
CommonMark examples also cannot prove the four TE04 additions: strikethrough,
task items, pipe tables, and source-preserving fenced blocks.

This profile freezes a small language-neutral oracle for those additions before
the missing Swift, Java, Kotlin, and Dart packages are implemented.  It is a
portable, deterministic parsing and rendering contract.  It does not execute a
fence payload, load a diagram engine, access the network or filesystem, or
claim support for arbitrary future GFM syntax.

The normative files live in `code/specs/fixtures/gfm-extension-v1/`:

- `cases.json` contains independently authored source, Document AST, and HTML
  triples;
- `schema.json` closes their transport shape; and
- `code/scripts/gfm_extension_conformance.py` performs stricter validation that
  JSON Schema alone cannot express.

## Admitted document profile

Every case starts with a supplied Unicode string and produces a fresh TE00/TE04
`document` node plus exact UTF-8 HTML.  Implementations must not mutate the
source or return aliases to fixture-owned arrays or objects.

The v1 corpus covers these observable rules:

1. A paired `~~` delimiter produces a `strikethrough` inline node.  A lone
   tilde stays text.  Inline children inside a strike retain their own nodes.
2. A list item whose first paragraph begins with `[x]`, `[X]`, or `[ ]` plus
   GFM whitespace becomes a `task_item`.  The marker is removed from the first
   paragraph.  HTML uses an inert disabled checkbox; checked items additionally
   carry the checked state.  Similar text outside that position stays text.
3. A header row followed immediately by a valid delimiter row produces a
   `table`.  `:---`, `---:`, `:---:`, and `---` map to left, right, center, and
   null alignment.  Header/body identity, cell order, inline content, missing
   trailing cells, and excess trailing cells are explicit fixture data.
4. Fences preserve `name`, full normalized `info`, and literal `value` as the
   TE04 `fenced_block` node.  Anonymous, empty, tilde, code-like, and inert
   diagram-like fences remain distinguishable.  HTML uses the ordinary escaped
   code fallback; metadata never becomes executable markup.
5. One mixed document proves that all four extensions preserve source order and
   compose through a single document rather than four unrelated helper APIs.

The existing TE04 fenced-block contract remains authoritative for info-string
normalization and payload newline behavior.  The new corpus repeats only the
fence cases needed to make the complete extension profile self-contained.

## Bounds and closed transport

The fixture is UTF-8 JSON with unique object keys, Unicode scalar strings, no
byte-order mark, and no non-finite or fractional numbers.  The validator reads
at most 131,073 bytes to enforce a 131,072-byte transport ceiling and rejects a
JSON nesting depth above sixteen.

The callable validator applies equivalent limits even when given a decoded
object directly: the value must be a true tree (no cycles or shared container
aliases), may contain at most 8,192 visited items, and may contain at most
131,072 UTF-8 bytes of string data.  These bounds prevent a small aliased
object graph from expanding into exponential validation work.

The closed corpus contains exactly sixteen ordered cases.  Each case obeys:

| Resource | Maximum |
|---|---:|
| Source UTF-8 bytes | 8,192 |
| Source lines | 128 |
| One source line UTF-8 bytes | 1,024 |
| Expected AST nodes | 256 |
| Expected AST depth | 16 |
| Text in one AST field, UTF-8 bytes | 8,192 |
| Expected HTML UTF-8 bytes | 32,768 |

These are conformance-corpus admission bounds, not a claim that every native
parser already exposes a hostile-input resource-limit API.  A lane adoption
must process the admitted cases within its normal package test budget.  A later
strict parsing profile may define portable refusal errors for larger arbitrary
documents; it must not silently widen this v1 contract.

## Validation and adoption

The independent validator checks the exact profile/version, case roster and
order, closed node field sets, node-specific invariants, scalar/size/depth
bounds, unique case IDs, table shape, task placement, inert fence metadata, and
exact published literals.  Fence-name validation uses the TE04 ECMAScript
whitespace set rather than the validator host's native whitespace rules. Tests
must falsify coordinated AST/HTML changes, not only malformed JSON.

A language earns adoption credit only when its real public parser reads the
committed corpus, compares both AST and HTML through native APIs, retains its
ordinary package tests, and is selected by CI when `cases.json` changes.  The
neutral fixture and validator alone grant no language credit.  Missing-lane
ports remain separate PRs so their package metadata, tests, coverage, and build
closure can be reviewed independently.

## Delivery order

1. Land this neutral fixture and validator.
2. Adopt it in the repaired TypeScript reference lane.
3. Reconcile the structurally present C#, F#, and Haskell packages against it.
4. Add the missing Swift parser/facade after its CommonMark prerequisites.
5. Add Java, Kotlin, and Dart after their CommonMark parser and renderer
   prerequisites.

This ordering restarts package parity from a shared oracle instead of copying
the current Rust or TypeScript directory tree blindly.
