# logic-core (C++)

The semantic core of a **logic programming engine** (à la Prolog),
**header-only** in pure ISO C++17 (namespace `ca::logic_core`): terms,
substitutions, and first-order **unification**. A faithful port of the Rust
[`logic-core`](../../rust/logic-core) crate.

## What it does

Logic programming reasons over *terms* — atoms, numbers, strings, variables, and
compound `functor(args…)` structures. A **substitution** binds variables to
terms. **Unification** asks whether some substitution makes two terms
syntactically equal; `unify` answers, with the occurs-check on so `X = f(X)`
fails rather than looping.

## API

- Constructors: `atom`, `integer`, `real`, `string`, `var` + `var_term`,
  `compound`, `logic_list`.
- `to_string(Term)`, `Term::operator==`.
- `Substitution`: `empty`, `extend` (returns a NEW substitution), `walk` /
  `walk_var`, `size`, `occurs`, `operator==`.
- `unify(a, b, s)` → `std::optional<Substitution>` (`nullopt` if not unifiable).

## Design notes

- **Value semantics.** `Term` is a `std::variant` tree; `Number` is
  `std::variant<std::int64_t, double>` so `1` and `1.0` are distinct. `unify`
  returns `std::optional<Substitution>` (Rust's `Option`).
- **Faithful divergences.** Rust's `int` / `float` builders are `integer` /
  `real` here (`int`/`float` are keywords); Rust's `AtomicU64` id counter is a
  `static` counter; float display uses `%g` (matches Rust's `1.0` → `"1"`).
- **Header-only.** `#include "logic_core.hpp"` and go.

## Usage

```cpp
#include "logic_core.hpp"
using namespace ca::logic_core;

// father(homer, X) ?= father(homer, bart)  ⇒  X = bart
LogicVar x = var("X");
Term query = compound("father", {atom("homer"), var_term(x)});
Term fact  = compound("father", {atom("homer"), atom("bart")});

auto s = unify(query, fact, Substitution::empty());   // has_value()
Term bound = s->walk_var(x);                           // atom("bart")
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
