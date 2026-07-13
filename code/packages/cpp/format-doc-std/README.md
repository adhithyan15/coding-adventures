# format-doc-std (C++)

Reusable pretty-printing **templates** over [`format-doc`](../format-doc),
**header-only** in pure ISO C++17 (namespace `ca::format_doc_std`) — the "80%
layer" of the formatter stack. A faithful port of the Rust
[`format-doc-std`](../../rust/format-doc-std) crate.

## What it does

`format-doc` owns the primitive document algebra; this layer owns the common
syntax shapes most languages reuse. Each template returns a `format_doc::Doc`;
the layout later decides flat-vs-broken.

- `delimited_list` — arrays / tuples / parameter & argument lists / object fields
- `call_like` — function & constructor calls (callee + delimited args)
- `block_like` — braces / `begin … end` / indented block bodies
- `infix_chain` — arithmetic / boolean / pipeline / type-operator chains

## Design notes

- **`Doc` is cheaply copyable** (structural sharing), so configs simply hold
  `Doc` values — no cloning ceremony.
- **Exceptions.** Rust's `assert!`/panic on an infix arity mismatch becomes a
  thrown `std::invalid_argument`.
- **Header-only.** `#include "format_doc_std.hpp"` and go. Depends on the
  header-only `cpp/format-doc` (`run.sh` adds `../format-doc/include`).

## Usage

```cpp
#include "format_doc.hpp"
#include "format_doc_std.hpp"
using namespace ca::format_doc;
namespace fds = ca::format_doc_std;

// print(a, b, c)
Doc doc = fds::call_like(text("print"), {text("a"), text("b"), text("c")},
                         fds::CallLikeConfig{});
std::string s = render_text(layout_doc(doc, LayoutOptions{}));  // "print(a, b, c)"
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
