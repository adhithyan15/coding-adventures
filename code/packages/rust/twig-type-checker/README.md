# twig-type-checker

Static type checker for the Twig language (TW05-B + TW05-C).

## What it does

Walks a parsed Twig `Program`, builds a type environment from all top-level
declarations, infers a base [`TwigKind`] for every expression, and reports
violations as `TypeErrorDiagnostic` spans.

**TW05-C (LANG53)** extends TW05-B with refinement-type checking:
call-site proof obligations for annotated parameters (`(Int 0 128)`,
`(Member int …)`) and flow-sensitive narrowing from `if`-guards.

## Checks performed

| Check | Example error |
|-------|--------------|
| Unresolved variable | `unresolved variable 'foo'` |
| Call arity | `arity error: 'f' expects 1 argument, got 2` |
| Non-exhaustive match | `non-exhaustive match on union 'Expr': unmatched variants: 'NameRef'` |
| Refinement violation (TW05-C) | `refinement error: argument 0 to 'ascii-info' violates annotation: value 200 violates (Int 0 128)` |
| Refinement unknown in strict mode | `refinement error: argument 0 to 'ascii-info' cannot be proven to satisfy annotation (strict mode)` |

## Typed modes

The `(typed …)` clause in a `(module …)` declaration controls enforcement:

| Clause | Behaviour |
|--------|-----------|
| `(typed off)` or absent | Skip type checking — zero overhead |
| `(typed lenient)` | Check; errors are warnings; `ok: true` always |
| `(typed strict)` | Check; any error → `ok: false` |

## Quick start

```rust
use twig_type_checker::type_check;

// Parse + check in one call.
let result = type_check("(define (f x) x) (f 1)").unwrap();
assert!(result.ok);
assert!(result.errors.is_empty());

// Wrong arity (in a (typed strict) module).
let bad = type_check(
    "(module m (typed strict)) (define (f x) x) (f 1 2)"
).unwrap();
assert!(!bad.ok);
assert!(bad.errors[0].message.contains("arity error"));
```

## Pipeline position

```
twig-parser → twig-type-checker → twig-ir-compiler
```

`twig-ir-compiler` calls this crate's `check_program` before emitting IIR
when the module declares `(typed lenient|strict)`.

## TwigKind

| Kind | Maps from |
|------|-----------|
| `Int` | integer literals, `int`, unrefined int annotations |
| `RefinedInt(Predicate)` | `(Int lo hi)`, `(Member int v…)` annotations; narrowed by `if`-guards |
| `Bool` | `#t` / `#f`, `bool` |
| `Nil` | `nil` |
| `Symbol` | quoted symbols `'foo` |
| `Str` | `String` type annotation |
| `List` | `List` annotation |
| `Record(name)` | declared `(record Name …)` |
| `Union(name)` | declared `(union Name …)` |
| `Function { arity }` | `(lambda …)`, `(define (f …) …)` |
| `Any` | unannotated / widened |

`RefinedInt` is a subtype of `Int`: `RefinedInt(p) ⊆ Int ⊆ Any`.

## Flow-sensitive narrowing (TW05-C)

Inside `if`-branches, variables are narrowed by the guard:

```scheme
(define (ascii-info (x : (Int 0 128))) x)

(define (process (n : int))
  (if (< n 128)
    ;; true branch: n narrowed to RefinedInt(n < 128) → ascii-info call proven safe
    (ascii-info n)
    ;; false branch: n narrowed to RefinedInt(NOT(n < 128)) → ascii-info would error
    0))
```

Combining guards with `and`:

```scheme
(if (and (>= n 0) (< n 128))
  (ascii-info n)   ; n ∈ [0, 128) proven safe
  0)
```

## Notes

- Pure: no I/O, no FFI, no unsafe.
- Deps (TW05-B): `twig-parser`, `type-checker-protocol`.
- Deps (TW05-C, added by LANG53): `lang-refined-types`, `lang-refinement-checker`.
