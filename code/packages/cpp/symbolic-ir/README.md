# symbolic-ir (C++)

The universal symbolic-expression IR, header-only in pure ISO C++17 (namespace
`ca::symbolic_ir`) — a faithful port of the Rust `symbolic-ir` crate. One shared
tree every computer-algebra frontend compiles to and every backend consumes.

## The six node variants

| Variant | Notes |
|---------|-------|
| `Symbol(name)` | variable / constant / operation head |
| `Integer(i64)` | |
| `Rational(n, d)` | **always** reduced, `d > 0` |
| `Float(f64)` | |
| `Str(text)` | string literal |
| `Apply(head, args)` | `head(arg0, arg1, …)` |

`Node` has value semantics; the recursive `Apply` payload is shared via a
`std::shared_ptr` (immutable, so copies are cheap).

## Usage

```cpp
#include "symbolic_ir.hpp"
namespace s = ca::symbolic_ir;

// Build  Add(Pow(x, 2), 1)
s::Node expr = s::apply(s::sym(s::ADD),
    { s::apply(s::sym(s::POW), { s::sym("x"), s::integer(2) }),
      s::integer(1) });

expr.to_string();          // "Add(Pow(x, 2), 1)"
expr == expr;              // structural equality
expr.hash();               // consistent with ==

s::Node half = s::rat(2, 4);   // Rational(1, 2), reduced
s::Node two  = s::rat(6, 3);   // collapses to Integer(2)
```

## Divergence from the Rust crate

Rust's `rational` panics on a zero denominator; this port throws
`std::invalid_argument`. Equality compares floats by bit pattern (identical-bit
NaNs are equal). Float `to_string` uses the shortest `%g`-style round-tripping
decimal (always with a decimal point), matching Rust's `{:?}` for common values.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
