# format-doc

**Wadler-style document algebra for pretty-printers.**  Rust port of
[P2D03](../../specs/P2D03-format-doc.md) (the TypeScript
`@coding-adventures/format-doc`).

The semantic IR every formatter builds: language-specific formatters
compile AST → `Doc`, this crate realises `Doc` → `DocLayoutTree`.

---

## Why a doc algebra

A formatter shouldn't emit strings directly.  Every "should I keep this
on one line or break across multiple?" decision is a layout problem
that's easier to express declaratively as a `Doc` tree of combinators
than as imperative recursion through your AST.

The Wadler/Hughes algorithm (also used by Prettier) walks the tree
with a width budget and decides, group by group, whether each group
fits on the current line — automatically choosing flat vs broken
mode without the formatter author having to predict line widths.

---

## Architecture

```text
AST + trivia + formatter rules
  → Doc                         ← stable semantic IR
  → DocLayoutTree               ← first realised layout form  (this crate)
  → PaintScene                  ← first concrete rendering scene  (format-doc-to-paint, future)
  → paint-vm-ascii              ← terminal string  (downstream)
```

The same `Doc` builds drive ASCII output today, canvas / SVG /
editor-native paint pipelines tomorrow, all without the formatter
author re-emitting strings.

---

## Public API

| Builder                              | Meaning                                                       |
|--------------------------------------|---------------------------------------------------------------|
| `nil()`                              | Empty doc; neutral element of `concat`                        |
| `text(s)`                            | Literal text (auto-splits on `\n` / `\r`)                     |
| `concat(parts)`                      | Emit children in sequence; flattens nested concats            |
| `join(sep, parts)`                   | Convenience: join with separator                              |
| `group(d)`                           | Try `d` flat; if it doesn't fit, print broken                 |
| `indent(d, levels)`                  | Indent broken lines inside `d` by `levels` × `indent_width`   |
| `line()` / `softline()` / `hardline()`| Mode-dependent line breaks (see table below)                  |
| `if_break(broken, flat)`             | Pick between two docs based on enclosing group's mode         |
| `annotate(annotation, d)`            | Attach metadata to spans without changing layout              |

| Realisation                           | Returns                                                        |
|---------------------------------------|----------------------------------------------------------------|
| `layout_doc(doc, options)`            | `DocLayoutTree` — line/span structure with monospace coordinates |
| `render_text(layout)`                 | `String` — flatten layout to plain text with newlines + indent |

| Types                                 |
|---------------------------------------|
| `Doc` enum (8 variants, `#[non_exhaustive]`) — the doc tree |
| `LineMode` — `Soft` / `Normal` / `Hard` |
| `DocAnnotation` — `Str` / `Int` / `Bool` / `Null` (`#[non_exhaustive]`) |
| `DocLayoutSpan { column, text, annotations }` |
| `DocLayoutLine { row, indent_columns, width, spans }` |
| `DocLayoutTree { print_width, indent_width, line_height, width, height, lines }` |
| `LayoutOptions { print_width, indent_width, line_height }` (Default = 80 / 2 / 1) |

### Line modes

| Mode                       | Flat behaviour | Broken behaviour       |
|----------------------------|----------------|------------------------|
| `line()` / `LineMode::Normal` | space (1 col) | newline + indent       |
| `softline()` / `LineMode::Soft` | empty       | newline + indent       |
| `hardline()` / `LineMode::Hard` | newline (forces break) | newline + indent |

---

## Hardening (security review)

The reader-side surface is the `Doc` tree built by an upstream
formatter — could be adversarial.  Two HIGH issues caught and
fixed before this PR:

- **`fits()` stack-clone DoS.**  Original implementation cloned
  the entire pending stack at every group via `to_vec()`,
  giving O(N²) memory for N nested groups.  Fix: borrow the
  parent stack and only clone the few descended children.
  1000-nested-group regression test added.
- **`text("a\nb")` not validated.**  The doc claims "no newlines
  inside text" but the implementation didn't enforce it; literal
  `\n` flowed into spans, breaking downstream backends (paint-vm,
  canvas, SVG) that assume monospace single-line cells.  Fix:
  `text()` auto-splits on `\n`, normalises `\r` and `\r\n`.
  Three regression tests added (LF auto-split, CRLF/CR
  normalisation, blank-line preservation).

Other safeguards:

- `assert!(print_width > 0)` is the only panic surface.
- `Doc` is internally `Arc`-shared so cloning a built tree is
  cheap (O(1) for refcount bumps; no deep copy).
- Annotation lists aren't deduplicated; a future optimisation
  could replace `Vec<DocAnnotation>` with `Arc<[DocAnnotation]>`
  if profiling shows it matters.

---

## Caller responsibilities

- `text(s)` rejects no UTF-8 — pass valid UTF-8 strings.
- `LayoutOptions::print_width` must be `> 0`.
- The width metric is char-count (`chars().count()`) — fine for
  ASCII source code, wrong for full-width CJK or zero-width
  joiners.  A future version can plug in `unicode-width` without
  breaking the API.
- Indent / column arithmetic uses `usize`; adversarial inputs
  with `Indent { levels: usize::MAX }` could overflow.  Don't
  feed untrusted indents above thousands.

---

## Example

```rust
use format_doc::{
    concat, group, indent, layout_doc, line, render_text, softline,
    text, LayoutOptions,
};

// (foo, bar, baz) — flat if it fits, broken otherwise.
let doc = group(concat([
    text("foo("),
    indent(concat([
        softline(),
        text("bar,"),
        line(),
        text("baz"),
    ]), 1),
    softline(),
    text(")"),
]));

let narrow = layout_doc(doc.clone(), &LayoutOptions { print_width: 8, ..Default::default() });
assert_eq!(render_text(&narrow), "foo(\n  bar,\n  baz\n)");

let wide = layout_doc(doc, &LayoutOptions { print_width: 80, ..Default::default() });
assert_eq!(render_text(&wide), "foo(bar, baz)");
```

---

## Dependencies

**None.**  Pure data + algorithms.  See `required_capabilities.json`.

---

## Tests

40 unit tests + 1 doctest covering:
- Builders (`nil`, `text`, `concat` flattening / nil-dropping /
  singleton-unwrap, `join`, `indent` zero-noop)
- All three line modes in flat and broken modes
- `group` flat vs broken decisions
- `if_break` picking the right branch
- Annotations (single, nested, layout-neutral, span coalescing)
- Layout tree shape (dimensions, line widths, line height)
- `render_text` indent handling and blank lines
- The spec's worked example
- Look-ahead `fits()` corner cases (hardline forces fail,
  outer-broken provides escape)
- Idempotency-of-layout
- **Hardening:** newline auto-split, CRLF/CR normalisation,
  blank-line text, 1000-deep nested groups (formerly O(N²)),
  500-deep nested-with-siblings stress test

```sh
cargo test -p format-doc
```

---

## Roadmap

- **`format-doc-to-paint`** — bridge `DocLayoutTree` → `PaintScene`
  for the paint-vm pipeline (mirrors the existing `layout-to-paint`
  shape).  The first downstream consumer.
- **`format-doc-std`** — reusable templates (`delimited_list`,
  `call_like`, `block_like`, `infix_chain`) for the common syntax
  shapes formatters reach for repeatedly.  Mirrors the TS
  `@coding-adventures/format-doc-std`.
- **`twig-formatter`** — Twig prettifier built on top.  The
  authoring-experience deliverable that originally motivated this
  PR.
- **Richer combinators** — `fill`, `align`, `line_suffix`,
  `break_parent` — additive, won't break the algebra.
- **`unicode-width` integration** — replace char-count with
  proper monospace cell widths for CJK / emoji.
