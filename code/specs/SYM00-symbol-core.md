# SYM00 — Symbol Core: Interned Symbols as a Shared Primitive

## Overview

This package introduces the smallest useful symbolic building block in the
repo: the **symbol**.

A symbol is an immutable, interned, identity-bearing name such as:

- `homer`
- `parent`
- `x`
- `+`
- `ancestor`

This package exists because multiple future tracks need the same primitive:

1. **Logic programming**: Prolog atoms and functor names are symbol-backed.
2. **Symbolic mathematics**: a future SymPy-like system needs stable symbolic
   names for indeterminates like `x`, `y`, and `sin`.
3. **Language tooling**: parsers, ASTs, and interpreters often need canonical
   identifiers that preserve spelling but compare cheaply.

The central design decision is:

> **`Symbol` is the primitive. `Atom` is a logic-layer term built from a
> symbol.**

That distinction matters. In Prolog, `homer` is an atom. In a symbolic algebra
system, `x` may be an indeterminate. In both cases, the underlying stable name
can be the same `Symbol("x")`.

## Why A Separate Package?

It is tempting to let every future package use raw strings for names. That
works for toy systems, but it becomes a tax very quickly:

- repeated string allocations
- repeated normalization logic
- accidental inconsistency between `"X"`, `"x"`, `" x "`, and namespaced forms
- no clear distinction between a symbolic name and an arbitrary user string

By giving symbols a dedicated package, the rest of the stack can say exactly
what it means:

- a **string** is text
- a **symbol** is a canonical name in a symbolic system

## Relationship To Atoms, Variables, And Future Symbolic Math

The user-facing mental model is:

```
Symbol Core
    ↓
Logic Core
    - Atom         = zero-arity symbolic constant term
    - Compound     = functor symbol + argument terms
    - LogicVar     = bindable variable with its own identity and a display symbol
    ↓
Prolog Frontend
    - source text "homer" → Atom(Symbol("homer"))
    - source text "parent(homer, bart)" →
      Compound(Symbol("parent"), [Atom(Symbol("homer")), Atom(Symbol("bart"))])
    ↓
Future Symbolic Math
    - source text "x + y" →
      algebraic expression nodes using Symbol("x"), Symbol("y"), Symbol("+")
```

Two clarifications are important:

1. **An atom is not "just a string".**
   It is a logic term whose value is a symbol and whose arity is zero.

2. **A logic variable is not the same thing as a symbol.**
   A logic variable can be bound during search. It may carry a display name
   symbol like `X`, but its true identity is allocation-based, not name-based.

This separation will let us branch later into a symbolic mathematics track
without having to undo Prolog-specific assumptions.

## Scope

This package defines:

- `Symbol`
- symbol interning
- canonical string normalization rules
- optional qualified names / namespaces
- stable equality and hashing semantics
- conversion helpers from strings and identifiers

This package does **not** define:

- logic variables
- atoms or compound terms
- unification
- algebraic simplification
- parser tokenization rules

Those belong in higher layers.

## Core Concept: Interning

If two symbols have the same canonical name, they should compare equal and
share storage.

```
sym("parent") == sym("parent")   → true
sym("parent") == sym("child")    → false
```

Interning means:

1. normalize the name
2. check a symbol table
3. reuse an existing symbol if present
4. otherwise create one canonical symbol object

This is useful because symbolic systems do many repeated equality checks.
Comparing interned symbol identities is cheaper and semantically cleaner than
repeatedly comparing raw strings.

## Canonical Naming Rules

The first version should be intentionally conservative.

Rules:

- Symbols preserve case by default.
- Leading and trailing whitespace are rejected, not silently trimmed.
- Empty names are invalid.
- The name is stored exactly as written after validation.
- Namespace qualification is structural, not stringly-concatenated.

Examples:

```text
"x"          → valid symbol
"X"          → valid symbol, distinct from "x"
"parent"     → valid symbol
" logic "    → invalid (surrounding whitespace)
""           → invalid
```

Why preserve case? Because the future stack has multiple consumers:

- Prolog distinguishes variables from atoms partly by case in source syntax,
  but after parsing that is no longer the symbol layer's job.
- Symbolic math often wants exact user spelling.
- General language tooling should not impose case-folding policy globally.

## Qualified Symbols

Many symbolic systems eventually need namespacing:

- `math:add`
- `prolog:parent`
- `user:x`

Rather than encoding this as a flat string, the package should support an
optional namespace field:

```text
Symbol {
    namespace: "math" | null,
    name: "sin",
}
```

The canonical key is therefore:

- unqualified: `("sin")`
- qualified: `("math", "sin")`

This avoids later parsing bugs around embedded separators like `:` or `.`.

The Python prototype may keep this simple by storing:

```python
Symbol(namespace: str | None, name: str)
```

## Public API

The package should expose a tiny, stable API:

```python
from symbol_core import Symbol, SymbolTable, sym

s1 = sym("parent")
s2 = sym("parent")
s3 = sym("math", namespace="core")

assert s1 == s2
assert s1 is s2            # interned identity in the Python prototype
assert s1.name == "parent"
assert s1.namespace is None
assert str(s3) == "core:math"
```

Recommended API surface:

```python
@dataclass(frozen=True, slots=True)
class Symbol:
    namespace: str | None
    name: str

class SymbolTable:
    def intern(self, name: str, namespace: str | None = None) -> Symbol: ...
    def contains(self, name: str, namespace: str | None = None) -> bool: ...
    def size(self) -> int: ...

def sym(name: str, namespace: str | None = None) -> Symbol: ...
def is_symbol(value: object) -> bool: ...
```

The module-level `sym()` helper should use a default global table for ergonomic
use in notebooks, tests, and REPL-style examples. The explicit `SymbolTable`
exists for embedders that want isolation.

## Error Model

Invalid symbol creation should raise a dedicated error:

```python
class SymbolError(ValueError): ...
```

Failure cases:

- empty name
- leading/trailing whitespace
- empty namespace string
- non-string inputs

These are programmer errors, not search/runtime errors.

## Data Flow

```
Input text / host-language string
        ↓
validate name + namespace
        ↓
intern in symbol table
        ↓
return canonical Symbol object
```

Downstream packages then consume symbols rather than repeating their own naming
logic.

## Python Prototype Notes

The first implementation should be in:

- `code/packages/python/symbol-core`

Why Python first?

- easiest place to iterate on ergonomics
- good REPL/notebook experience for symbolic experimentation
- natural home for future SymPy-adjacent exploration

Implementation guidance:

- use `@dataclass(frozen=True, slots=True)`
- keep the interning table in ordinary Python dicts
- do not over-engineer weak references or eviction yet
- make string representations friendly for teaching and debugging

## Test Strategy

Required tests:

- interning returns the same object for the same `(namespace, name)`
- different names produce different symbols
- qualified and unqualified symbols do not compare equal
- case is preserved and significant
- empty / invalid names raise `SymbolError`
- hashing works correctly in sets and dict keys
- `repr()` and `str()` are stable and readable

## Future Extensions

- symbol metadata (source span, documentation, tags)
- gensym / hygienic generated names
- non-string symbol payloads for advanced compiler work
- serialization format for persistent symbolic objects
- integration with a future expression tree package

## How Logic Core Will Use This

The next package up the stack will define:

- `Atom(symbol: Symbol)`
- `Compound(functor: Symbol, args: tuple[Term, ...])`
- `LogicVar(name: Symbol | None, id: int)`

So if the user asks, "Is an atom just a symbol?", the precise answer is:

> An atom is a **logic term whose value is a symbol**. All atoms are
> symbol-backed, but not every use of a symbol is an atom.
