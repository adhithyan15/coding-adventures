# twig-type-checker

Base static type checker for the Twig language (TW05-B).

## What it does

Walks a parsed Twig `Program`, builds a type environment from all top-level
declarations, infers a base [`TwigKind`] for every expression, and reports
violations as `TypeErrorDiagnostic` spans.

## Checks performed

| Check | Example error |
|-------|--------------|
| Unresolved variable | `unresolved variable 'foo'` |
| Call arity | `arity error: 'f' expects 1 argument, got 2` |
| Non-exhaustive match | `non-exhaustive match on union 'Expr': unmatched variants: 'NameRef'` |

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
| `Int` | integer literals, `int`, `(Int lo hi)` annotations |
| `Bool` | `#t` / `#f`, `bool` |
| `Nil` | `nil` |
| `Symbol` | quoted symbols `'foo` |
| `Str` | `String` type annotation |
| `List` | `List` annotation |
| `Record(name)` | declared `(record Name …)` |
| `Union(name)` | declared `(union Name …)` |
| `Function { arity }` | `(lambda …)`, `(define (f …) …)` |
| `Any` | unannotated / widened |

## Notes

- Pure: no I/O, no FFI, no unsafe.
- Deps: `twig-parser`, `type-checker-protocol`.
- TW05-C (refinement solver) will extend this with `lang-refinement-checker`
  to check range/membership predicates.
