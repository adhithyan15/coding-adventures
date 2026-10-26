# TE01 — Portable CommonMark 0.31.2 conformance

## Purpose

The repository currently carries sixteen byte-identical copies of the official
CommonMark 0.31.2 example corpus. Sixteen native package test readers across
seven established language lanes use those copies: the CommonMark families and
their GFM extensions both exercise the official base corpus. Duplication makes
provenance, drift, bounds, and fixture-triggered CI selection difficult to review.

This profile establishes one language-neutral foundation in
`code/specs/fixtures/commonmark-0.31.2/`. It pins the official 652 parser examples
without rewriting their expectations and adds a separate, independently authored
set of TE00 Document-AST-to-HTML vectors. It does not migrate a native reader or
qualify any language implementation by itself.

## Normative artifacts

The foundation contains these normative files:

- `spec.json` — the exact official CommonMark 0.31.2 `spec.json` bytes;
- `provenance.json` — the immutable source URL, version, publication date,
  author, license, byte length, digest, and example count;
- `renderer-cases.json` — independently authored TE00 AST/options/HTML triples;
- `corpus-schema.json`, `provenance-schema.json`, and
  `renderer-schema.json` — closed Draft 2020-12 transport shapes; and
- `code/scripts/commonmark_conformance.py` — the dependency-free strict
  validator used by repository checks and native adoption tests.

`spec.json` stays byte-for-byte compatible with the upstream artifact. The
foundation must not wrap, reformat, normalize, reorder, or repair it. A native
migration can therefore change only its fixture location and reader boundary,
not its expected CommonMark outcomes.

## Provenance

The canonical source is
`https://spec.commonmark.org/0.31.2/spec.json`, published with CommonMark
version 0.31.2 on 2024-01-28. The specification identifies John MacFarlane as
author and publishes the work under CC BY-SA 4.0.

The admitted artifact is exactly 140,487 bytes with SHA-256:

```text
d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20
```

The same bytes already exist in all sixteen repository copies at selection
time. Provenance validation is local and deterministic: no validator or native
test fetches the network, follows redirects, reads a clock, or trusts ambient
metadata.

## Official parser corpus contract

The corpus root is an array of exactly 652 objects. Every object has exactly:

| Field | Contract |
|---|---|
| `markdown` | Unicode-scalar string, at most 8,192 UTF-8 bytes |
| `html` | Unicode-scalar string, at most 32,768 UTF-8 bytes |
| `example` | integer equal to its one-based array position |
| `start_line` | positive integer no larger than 100,000 |
| `end_line` | integer greater than `start_line`, no larger than 100,000 |
| `section` | nonempty Unicode-scalar string, at most 128 UTF-8 bytes |

Examples must remain in official order. Source line ranges must increase and
must not overlap the preceding example. The exact raw-byte digest and a second
canonical semantic digest make coordinated field changes fail even if an edit
still satisfies the structural schema.

## Independent renderer corpus

`renderer-cases.json` is not derived from `spec.json` and is not generated from
any current renderer. It contains exactly sixteen ordered cases:

1. `empty-document`
2. `headings-all-levels`
3. `paragraph-escaping-and-breaks`
4. `inline-emphasis-strong-code`
5. `code-blocks`
6. `nested-blockquote`
7. `tight-unordered-list`
8. `loose-ordered-list-start`
9. `thematic-break`
10. `link-image-attributes`
11. `dangerous-destinations`
12. `url-and-email-autolinks`
13. `raw-html-enabled`
14. `raw-html-suppressed`
15. `non-html-raw-skipped`
16. `mixed-block-order`

Together they exercise all nineteen TE00 node types, nesting, source order,
tight and loose list paragraph handling, non-unit ordered starts, HTML text and
attribute escaping, code/language escaping, URL rejection, URL and email
autolinks, raw HTML passthrough, explicit raw suppression, and non-HTML raw-node
elision.

Each case has exactly `id`, `suppress_raw_html`, `ast`, and `expected_html`.
The AST uses the TE00 field names and containment rules. `suppress_raw_html`
selects only the renderer's raw-node option; fixture data is never interpreted
as executable code, a plugin name, a filesystem path, or a network location.

The renderer fixture is closed to TE00. It deliberately excludes TE04 GFM node
types, sanitization-policy composition, arbitrary Markdown parsing, browser
execution, and host rendering. GFM retains its separate TE04 corpus.

## Bounds and strict decoding

All artifacts are UTF-8 JSON with unique object keys, Unicode scalar strings,
no byte-order mark, and no fractional, exponent, or non-finite numbers. The
validator reads at most one byte beyond each transport ceiling:

| Artifact | Maximum bytes |
|---|---:|
| `spec.json` | 262,144 |
| `renderer-cases.json` | 131,072 |
| `provenance.json` | 8,192 |

Serialized JSON nesting may not exceed 32 containers. Public validation of an
already decoded value applies the same work boundary: the graph must be a true
tree with no cycles or shared container identities, may contain at most 32,768
visited items, and may contain at most 524,288 aggregate UTF-8 string bytes.
Bounds are checked before scheduling child work or copying caller-controlled
strings.

The renderer document adds these per-case bounds:

| Resource | Maximum |
|---|---:|
| Cases | exactly 16 |
| AST nodes per case | 256 |
| AST depth | 32 |
| Children in one container | 64 |
| One AST string, UTF-8 bytes | 8,192 |
| Expected HTML, UTF-8 bytes | 32,768 |

The validator rejects open fields, unknown node kinds, invalid heading levels,
list/start inconsistencies, nested links, malformed raw formats, unsupported
options, changed case order, changed normative literals, and any value whose
host-language type only resembles the required JSON type (for example, a
boolean where an integer is required).

## Validation and adoption

The strict validator must prove:

- the raw upstream byte digest and byte count;
- the exact provenance record;
- all 652 structural, ordinal, line-range, string, and semantic-digest rules;
- all sixteen renderer shapes, TE00 containment rules, limits, and
  semantic-digest rules; and
- equivalent bounded rejection for serialized and decoded-object callers.

Tests must include independent corruptions of official Markdown, official HTML,
official metadata, renderer AST, renderer option, and renderer HTML. They must
also cover duplicate keys, invalid UTF-8, BOMs, ambiguous numbers, excessive
transport/depth/breadth/string data, aliases, cycles, open fields, unknown node
types, invalid containment, and a stale digest.

The always-run repo-wide metadata-contract CI step must execute both the
foundation validator and its dedicated falsification suite. A package-diff
planner reporting no changed native package is not validation evidence for a
specs-and-scripts-only foundation.

Workflow self-validation can independently select the Venture acceptance
matrix even when no TypeScript package is selected. That selection must also
install the workflow's existing Node.js toolchain and pinned npm resolver before
the generated React and Electron install/test/build gates run. The native
package language flag must not be their sole toolchain prerequisite. Preserve
the acceptance gates and ordinary peer dependency resolution.

A native parser earns CommonMark corpus adoption credit only after its public
API dynamically reads the canonical `spec.json`, compares all 652 exact HTML
results, retains its package tests, and is selected by CI when the canonical
fixture changes. A native renderer earns renderer-fixture credit only after its
public API dynamically consumes all sixteen AST/options/HTML cases. The neutral
foundation alone grants neither credit.

## Delivery boundary

This foundation PR may add the normative files, validator, validator tests,
documentation, changelog, and parity backlog receipts. It must not:

- delete or redirect any existing native fixture copy;
- edit a parser, renderer, package build front, dependency, or lockfile;
- claim that an existing lane passes all 652 examples or all renderer vectors;
- add GFM behavior or TE04 nodes; or
- introduce network, process, filesystem-write, browser, plugin, clock, or
  credential authority into fixture validation.

Reader migrations are separately reviewable children. Before any such child is
selected, split the aggregate migration owner by language or another set of
non-overlapping package and CI surfaces.
