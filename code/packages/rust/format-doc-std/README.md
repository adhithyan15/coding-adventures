# format-doc-std

**Reusable templates over [`format-doc`](../format-doc/).**  The
"80% layer" in the formatter stack.  Rust port of
[P2D04](../../specs/P2D04-format-doc-std.md).

`format-doc` owns the primitive document algebra; this crate owns
the common syntax shapes most languages reuse.  Language-specific
formatters compose these templates and override the remaining
unusual constructs.

---

## Architecture

```text
language-specific AST printer
  → format-doc-std templates       ← this crate
  → format_doc::Doc
  → format_doc::DocLayoutTree
  → PaintScene                     (format-doc-to-paint, future)
  → paint-vm-ascii                 (downstream)
```

---

## What's in v1

Four templates that cover the bulk of real formatter output:

| Template          | Function          | Covers                                                                 |
|-------------------|-------------------|------------------------------------------------------------------------|
| Delimited list    | `delimited_list`  | Arrays, tuples, parameter lists, argument lists, object fields         |
| Call-like         | `call_like`       | Function and constructor calls (callee + delimited args)               |
| Block-like        | `block_like`      | Braces / `begin … end` / indented block bodies                         |
| Infix chain       | `infix_chain`     | Arithmetic, boolean, pipeline, type-operator chains                    |

Each has a `*_with` variant accepting a config struct for the
optional knobs.

## Public API at a glance

```rust
pub fn delimited_list(open: Doc, items: Vec<Doc>, close: Doc) -> Doc;
pub fn delimited_list_with(open: Doc, items: Vec<Doc>, close: Doc, &DelimitedListConfig) -> Doc;

pub fn call_like(callee: Doc, args: Vec<Doc>, &CallLikeConfig) -> Doc;

pub fn block_like(open: Doc, body: Doc, close: Doc) -> Doc;
pub fn block_like_with(open: Doc, body: Doc, close: Doc, &BlockLikeConfig) -> Doc;

pub fn infix_chain(operands: Vec<Doc>, operators: Vec<Doc>, &InfixChainConfig) -> Doc;

pub enum TrailingSeparator { Never (default), Always, IfBreak }
```

### `DelimitedListConfig`
- `separator: Doc` — default `text(",")`.
- `trailing_separator: TrailingSeparator` — default `Never`.
- `empty_spacing: bool` — `false` by default; `[]` vs `[ ]`.

### `CallLikeConfig`
- `open: Doc` / `close: Doc` — default `(` / `)`.
- `separator: Doc` — default `text(",")`.
- `trailing_separator: TrailingSeparator` — default `Never`.

### `BlockLikeConfig`
- `empty_spacing: bool` — `true` by default; `{ }` vs `{}`.

### `InfixChainConfig`
- `break_before_operators: bool` — `false` by default (operators trail
  previous line, C / Java / JavaScript convention).  `true` for
  break-before-operator (Haskell / Elixir / SQL convention).

---

## Design principles (mirrors P2D04)

- **Build Docs, not strings.**  Every template returns a `Doc`.
- **Flat by default, broken when needed.**  Templates rely on
  `group()` / `line()` / `softline()` / `indent()` so the
  width-fitting algorithm picks the layout.
- **Small policy surface.**  Configs cover the choices that
  usually vary by language.  Edge cases stay in language packages.
- **Escape hatches.**  When a language has unusual rules, build
  the `Doc` directly from format-doc primitives or wrap these
  templates with a thin local helper.

---

## Example

```rust
use format_doc::{layout_doc, render_text, text, LayoutOptions};
use format_doc_std::{call_like, infix_chain, CallLikeConfig, InfixChainConfig};

// print(x + y, z)
let sum = infix_chain(
    vec![text("x"), text("y")],
    vec![text("+")],
    &InfixChainConfig::default(),
);
let call = call_like(
    text("print"),
    vec![sum, text("z")],
    &CallLikeConfig::default(),
);

let layout = layout_doc(call, &LayoutOptions::default());
assert_eq!(render_text(&layout), "print(x + y, z)");
```

---

## Hardening

Single dep on `format-doc`.  Every Doc is internally `Arc`-shared,
so the `config.separator.clone()` / `config.open.clone()` calls in
template bodies are O(1) refcount bumps, not deep copies.  No
recursion in this crate (templates compose; format-doc handles
realisation).  Only panic surface is `infix_chain`'s arity-mismatch
`assert_eq!` (programmer error, not attacker input) and the
guarded `unwrap()` after the `is_empty()` check.

Security review: clean, no findings.

---

## Dependencies

- [`format-doc`](../format-doc/) — Doc algebra + width-aware realisation.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

---

## Tests

25 unit tests + 1 doctest covering:
- Each template's empty / flat / broken cases
- All `TrailingSeparator` variants in flat and broken layouts
- Custom separators / brackets / open-close
- Empty-spacing toggle for delimited and block templates
- `infix_chain` break-after vs break-before-operator
- `infix_chain` arity-mismatch panic (documented contract)
- Composability: nested templates, realistic expression
  (`print(x + y, z)`)

```sh
cargo test -p format-doc-std
```

---

## Roadmap

- More templates as patterns recur (e.g. `assignment`, `if_then_else`,
  `triadic_op` for languages with `cond ? then : else`).
- Theme-aware variants once `format-doc-to-paint` ships and
  language formatters want syntax-coloured spans.
- Annotation pass-through helpers — currently `text("foo")` becomes
  one span; helpers could attach token-class annotations
  automatically per template (e.g. `delimited_list` open/close
  annotated as `punctuation.bracket`).
