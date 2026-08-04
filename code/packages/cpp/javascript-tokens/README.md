# javascript-tokens (C++)

The shared JavaScript/TypeScript token vocabulary in pure ISO C++17, header-only,
in namespace `ca::jstokens`. A faithful port of the Rust `javascript-tokens`
crate.

- **`EsVersion`** — the ECMAScript editions with a grammar (`es1` … `es2025`),
  with string round-tripping and a chronological order.
- **`Span`** — a half-open `[start, end)` byte range within one source file.
- **`TokenKind`** — the broad classification of a token, plus an `Other` variant
  carrying a grammar-specific token name.

## API

```cpp
#include "javascript_tokens.hpp"
namespace jst = ca::jstokens;

jst::EsVersion v = jst::es_version_parse("es2020");   // throws UnknownEsVersion
auto maybe = jst::es_version_try_parse("es9");        // std::nullopt
jst::EsVersion::Es2015 < jst::EsVersion::Es2025;      // chronological order

jst::Span s = jst::Span::make(10, 20);                // len() == 10
jst::TokenKind::of(jst::TokenKind::Tag::Comment).is_trivia();  // true
```

`es_version_parse` throws `ca::jstokens::UnknownEsVersion` (whose message names
the input and the valid set); `es_version_try_parse` returns `std::optional`.
`Span` is a value struct with `len` / `is_empty` and the full comparison
operators (lexicographic). `TokenKind` supports `==` and `<`, so it works as an
associative-map key. `Span`'s `make` / `len` / `is_empty` are `constexpr`.

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
