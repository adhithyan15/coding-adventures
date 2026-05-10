# PR69 - Prolog Atom Number Conversions

## Goal

Close another everyday Prolog compatibility gap by adding `atom_number/2` to
the library-first Logic VM path.

## Scope

- `logic-builtins` exposes `atom_numbero/2`.
- `prolog-loader` rewrites source `atom_number/2` to the shared builtin.
- `prolog-vm-compiler` stress coverage proves the predicate works through
  parsing, loading, VM compilation, and execution.

## Semantics

`atom_numbero(Atom, Number)` supports finite, non-enumerating modes:

- If `Number` is bound to a number, `Atom` is unified with that number's text
  representation.
- If `Atom` is bound to a plain atom and `Number` is a variable, the atom text
  is parsed as an integer or float and unified with `Number`.
- Invalid text, non-atom text inputs, and non-number `Number` inputs fail
  deterministically.

This mirrors the existing `number_chars/2`, `number_codes/2`, and
`number_string/2` conversion style without introducing unbounded enumeration.
