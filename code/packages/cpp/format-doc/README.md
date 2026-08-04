# format-doc (C++)

A **Wadler-style document algebra** for pretty-printers, **header-only** in pure
ISO C++17 (namespace `ca::format_doc`). A faithful port of the Rust
[`format-doc`](../../rust/format-doc) crate.

## What it does

You build a backend-neutral *document* (`Doc`) out of primitives, then
`layout_doc` decides — for each `group` — whether it fits on the current line
(printed *flat*) or must *break* across lines, producing a `LayoutTree` of
positioned text spans. `render_text` flattens that tree to a `std::string`. This
is the layout engine a code formatter sits on top of.

## Primitives

`nil`, `text` (embedded `\n` auto-split to hardlines), `concat` (flattens, drops
nil), `join`, `group`, `indent`, `line`, `softline`, `hardline`, `if_break`,
`annotate`.

## API

- `layout_doc(doc, options)` → `LayoutTree` (throws `std::invalid_argument` if
  `print_width == 0`).
- `render_text(tree)` → `std::string` (indent + spans, `\n`-joined, no trailing
  newline).

## Design notes

- **Value semantics + structural sharing.** `Doc` is immutable and cheaply
  copyable — subtrees are shared via `std::shared_ptr`, mirroring the Rust `Arc`.
- **`std::variant` for the sum types.** The Rust `Doc` and `DocAnnotation` enums
  become `std::variant`s; Rust's `assert!(print_width > 0)` panic becomes a
  thrown `std::invalid_argument`.
- **Linear `fits` look-ahead.** The break/flat decision borrows the parent
  command stack instead of cloning it, so deeply nested groups stay O(work)
  rather than O(depth²). `visible_width` counts UTF-8 code points.
- **Header-only.** `#include "format_doc.hpp"` and go.

## Usage

```cpp
#include "format_doc.hpp"
using namespace ca::format_doc;

Doc doc = group(concat({
    text("foo("),
    indent(concat({softline(), text("bar,"), line(), text("baz")}), 1),
    softline(),
    text(")"),
}));

LayoutOptions opts;
opts.print_width = 8;
std::string s = render_text(layout_doc(doc, opts));  // "foo(\n  bar,\n  baz\n)"
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
