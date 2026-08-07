# type-declarations (C++)

A language-agnostic type-declaration format, header-only in pure ISO C++17
(namespace `ca::type_declarations`) — a faithful port of the Rust
`type-declarations` crate. The analogue of TypeScript `.d.ts` files.

## The model

`KindDecl` is the kind of an expression, each mapping to an IIR `type_hint`:

| Kind | `to_iir_hint()` |
|------|-----------------|
| `Int` | `"i64"` |
| `Bool` | `"bool"` |
| `Str` | `"str"` |
| `Function(arity)` | `"closure"` |
| `Nil` / `Symbol` / `List` / `Named(n)` / `Any` | `"any"` |

`TypeDeclarations` holds `named_types` (a `std::variant` of record / union /
alias), `globals`, and an optional typed mode. `resolve` follows alias chains
(depth-limited to 32, `Any` on a cycle); `union_variants` returns a union's
variant names. `AnnotatedNode` is the checker's output tree.

## Usage

```cpp
#include "type_declarations.hpp"
namespace td = ca::type_declarations;
using td::KindDecl;

td::TypeDeclarations d("twig");
d.named_types["Nat"] = td::AliasType{KindDecl::Int()};
d.resolve(KindDecl::Named("Nat"));   // == KindDecl::Int()

d.named_types["Shape"] =
    td::UnionType{{{"Circle", {}}, {"Rect", {}}}};
auto vs = d.union_variants("Shape"); // {"Circle", "Rect"}
```

## Divergence from the Rust crate

`resolve` / `union_variants` return owned values / `std::optional` as in Rust.
`AnnotatedNode` child sub-trees are held by `std::shared_ptr`, so copying a node
shares its (build-once, then read) children rather than deep-cloning — a
documented simplification.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness); the test suite also runs clean under
AddressSanitizer + UndefinedBehaviorSanitizer.
