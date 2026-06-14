# PR51 - Prolog Term Generality Predicates

## Goal

Add non-binding term generality checks to the Python logic builtin layer and
carry them through parsed Prolog source into the Logic VM path.

## Motivation

Full Prolog systems expose more than strict identity (`==/2`) and unification
(`=/2`). Metaprogramming code often needs to ask whether two terms are variants
of each other, or whether one term is more general than another, without
binding either term. These checks are useful for clause indexing, memoization,
deduplication, and source-transformation code.

## API

Library callers use:

```python
variant_termo(left, right)
not_variant_termo(left, right)
subsumes_termo(general, specific)
```

Parsed Prolog source uses:

```prolog
Left =@= Right
Left \=@= Right
subsumes_term(General, Specific)
```

## Semantics

- `variant_termo/2` succeeds when two reified terms differ only by a bijective
  variable renaming.
- `not_variant_termo/2` succeeds when that variant relationship does not hold.
- `subsumes_termo/2` succeeds when the second reified term is an instance of
  the first.
- None of these predicates bind caller variables.
- Existing bindings are respected because inputs are reified before checking.

## Parser Support

`prolog-core` adds ISO/Core operator defaults for `=@=/2` and `\=@=/2` at the
same precedence as the other term comparison predicates.

## Acceptance Tests

- Library tests cover positive and negative variant checks, non-variant checks,
  and subsumption with repeated variables.
- Loader tests prove parsed `=@=/2`, `\=@=/2`, and `subsumes_term/2` are
  adapted into builtin goals.
- VM stress tests prove the predicates run end to end from source through the
  parser, loader, compiler, and Logic VM.
