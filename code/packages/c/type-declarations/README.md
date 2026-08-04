# type-declarations (C)

A language-agnostic type-declaration format, in pure ISO C17 — a faithful port
of the Rust `type-declarations` crate. It's the analogue of TypeScript `.d.ts`
files: any parser can emit these declarations, and a generic checker consumes
them to infer a kind for every expression.

## The model

The base value is a **`TdKind`** — the kind of an expression:

| Kind | IIR `type_hint` |
|------|-----------------|
| `Int` | `"i64"` |
| `Bool` | `"bool"` |
| `Str` | `"str"` |
| `Function(arity)` | `"closure"` |
| `Nil` / `Symbol` / `List` / `Named(n)` / `Any` | `"any"` |

A **`TypeDeclarations`** holds named types (record / union / alias), global
binding kinds, and a typed-mode setting. It can:

- **`td_resolve`** a kind through alias chains — depth-limited to 32, returning
  `Any` on a cycle (`Nat → Int` resolves; `A → A` yields `Any`);
- **`td_union_variants`** — the variant names of a named union.

A **`TdAnnotatedNode`** is the checker's output tree: a rule-named node carrying
its inferred kind plus annotated children (nested nodes or token leaves).

## API sketch

```c
#include "type_declarations.h"

TypeDeclarations *d;
td_new(&d, "twig");

TdKind int_kind = td_kind_int();
TdNamedType alias;
td_named_alias(&int_kind, &alias);
td_insert_named_type(d, "Nat", alias);          /* takes ownership of `alias` */

TdKind nat, resolved;
td_kind_named("Nat", &nat);
td_resolve(d, &nat, &resolved);                 /* resolved.tag == TD_INT */
td_kind_free(&resolved); td_kind_free(&nat);
td_free(d);
```

## Divergence from the Rust crate

Rust returns owned values / `Option`; this port writes through out-parameters
and signals allocation failure with `-1`. `td_union_variants` returns `1`
(present), `0` (not a union / absent), or `-1` (OOM). Every value that owns
strings / arrays / sub-trees pairs a constructor with a matching `*_free`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors` /
`/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness); the test suite also runs clean under
AddressSanitizer + UndefinedBehaviorSanitizer.
