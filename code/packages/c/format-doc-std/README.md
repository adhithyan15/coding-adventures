# format-doc-std (C)

Reusable pretty-printing **templates** over [`format-doc`](../format-doc), in
pure ISO C17 — the "80% layer" of the formatter stack. A faithful port of the
Rust [`format-doc-std`](../../rust/format-doc-std) crate.

## What it does

`format-doc` owns the primitive document algebra; this layer owns the common
syntax shapes most languages reuse. Each template builds and returns an
`FdDoc *`, and the width-fitting layout later decides flat-vs-broken.

| Template             | Covers                                                     |
| -------------------- | ---------------------------------------------------------- |
| `fds_delimited_list` | arrays, tuples, parameter/argument lists, object fields    |
| `fds_call_like`      | function & constructor calls (callee + delimited args)     |
| `fds_block_like`     | braces / `begin … end` / indented block bodies             |
| `fds_infix_chain`    | arithmetic / boolean / pipeline / type-operator chains     |

## Ownership

A template **consumes** the content documents handed to it directly (the
`open`/`close` delimiters, the `items`/`args`/`operands`/`operators`, the
`body`, the `callee`) and returns a freshly-owned `FdDoc *` you release with
`fd_free`. A config only **borrows** the delimiter documents it carries — the
template clones what it needs, so a config is reusable and you free its
documents yourself.

## Dependency

Depends on `c/format-doc`; `run.sh` compiles `../format-doc/src/format_doc.c`
alongside and adds `../format-doc/include`. This port also uses the `fd_clone`
and `fd_is_nil` primitives now exposed by `format-doc`.

## Usage

```c
#include "format_doc.h"
#include "format_doc_std.h"

/* print(a, b, c) */
FdDoc *args[3] = {fd_text("a"), fd_text("b"), fd_text("c")};
FdDoc *o = fd_text("("), *c = fd_text(")"), *sep = fd_text(",");
FdsCallLikeConfig cfg = {o, c, sep, FDS_TRAILING_NEVER};
FdDoc *doc = fds_call_like(fd_text("print"), args, 3, &cfg);

FdLayoutOptions opts = fd_layout_options_default();
FdLayoutTree tree = fd_layout_doc(doc, &opts);
char *s = fd_render_text(&tree);   /* "print(a, b, c)" */

free(s); fd_layout_free(&tree); fd_free(doc);
fd_free(o); fd_free(c); fd_free(sep);
```

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
