# format-doc (C)

A **Wadler-style document algebra** for pretty-printers, in pure ISO C17. A
faithful port of the Rust [`format-doc`](../../rust/format-doc) crate.

## What it does

You build a backend-neutral *document* (`FdDoc`) out of primitives, then
`fd_layout_doc` decides — for each `group` — whether it fits on the current line
(printed *flat*) or must *break* across lines, producing a `FdLayoutTree` of
positioned text spans. `fd_render_text` flattens that tree to a string. This is
the layout engine a code formatter sits on top of.

## Primitives

| Builder              | Meaning                                              |
| -------------------- | ---------------------------------------------------- |
| `fd_text`            | literal text (embedded `\n` auto-split to hardlines) |
| `fd_concat`          | sequence child docs (flattens, drops nil)            |
| `fd_group`           | print flat if it fits, else broken                   |
| `fd_indent`          | add indentation for broken lines inside              |
| `fd_line`            | space when flat, newline when broken                 |
| `fd_softline`        | empty when flat, newline when broken                 |
| `fd_hardline`        | always newline (forces the enclosing group to break) |
| `fd_if_break`        | emit `broken` in broken mode, else `flat`            |
| `fd_annotate`        | attach metadata to spans without changing layout     |
| `fd_nil` / `fd_join` | the empty doc / join docs by a separator             |

## Ownership

An `FdDoc *` is an immutable owned tree: each builder **takes ownership** of the
docs handed to it, so you compose bottom-up and release the whole tree with a
single `fd_free`. `fd_layout_doc` only borrows the doc; the returned
`FdLayoutTree` and the `char *` from `fd_render_text` are owned separately
(`fd_layout_free` / `free`). A builder returns `NULL` on allocation failure,
having freed the docs you passed in.

## Design notes

- **Status/ownership, not Rust `Result`/`Arc`.** Rust's ref-counted shared
  subtrees become a plain owned tree with consuming builders; a stack-machine
  interpreter walks it. Active annotations are threaded as an immutable cons-list
  in a per-run arena so commands stay trivially copyable — only emitted spans pay
  for a materialised copy.
- **Linear `fits` look-ahead.** The break/flat decision borrows the parent
  command stack instead of cloning it, so deeply nested groups stay O(work)
  rather than O(depth²). All growable buffers guard against `size_t` overflow.
- **`visible_width` counts UTF-8 code points**, matching Rust's `chars().count()`.

## Usage

```c
#include "format_doc.h"

FdDoc *inner[4] = {fd_softline(), fd_text("bar,"), fd_line(), fd_text("baz")};
FdDoc *parts[4] = {
    fd_text("foo("),
    fd_indent(fd_concat(inner, 4), 1),
    fd_softline(),
    fd_text(")"),
};
FdDoc *doc = fd_group(fd_concat(parts, 4));

FdLayoutOptions opts = fd_layout_options_default();  /* width 80 */
opts.print_width = 8;
FdLayoutTree tree = fd_layout_doc(doc, &opts);
char *s = fd_render_text(&tree);   /* "foo(\n  bar,\n  baz\n)" */

free(s);
fd_layout_free(&tree);
fd_free(doc);
```

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
