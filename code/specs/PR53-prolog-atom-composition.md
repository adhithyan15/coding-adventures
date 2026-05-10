# PR53: Prolog Atom Composition Predicates

## Goal

Close the next source-level usability gap in the Prolog-on-Logic-VM path by
adding finite atom composition predicates. These predicates let Prolog programs
build and decompose symbolic names, delimited atom lists, and numeric string
representations without leaving the Logic VM runtime.

## Scope

The library layer exposes:

```text
atom_concato(left, right, combined)
atomic_list_concato(items, combined)
atomic_list_concato_with_separator(items, separator, combined)
number_stringo(number, string)
```

The Prolog adapter exposes the conventional source-level names:

```text
atom_concat/3
atomic_list_concat/2
atomic_list_concat/3
number_string/2
```

## Semantics

These relations intentionally support finite, VM-friendly modes:

- `atom_concat/3` joins two bound atoms, extracts a missing side from a bound
  combined atom, and enumerates non-empty finite splits when only the combined
  atom is bound.
- `atomic_list_concat/2` joins a proper ground list of atomic terms into an
  atom.
- `atomic_list_concat/3` joins a proper ground list with a bound separator and
  can split a bound atom into atom parts when the separator is non-empty.
- `number_string/2` renders a bound number as a string term and parses a bound
  string term into an integer or float.
- Fully open calls fail rather than enumerating infinite atom or string spaces.
- Empty atom results are not produced yet because the current symbol primitive
  rejects empty symbol names.

## Verification

- `logic-builtins` tests cover atom concatenation modes, delimited atomic-list
  joins and splits, and `number_stringo/2`.
- `prolog-loader` tests prove the source-level predicate names adapt into
  executable builtins.
- `prolog-vm-compiler` stress coverage runs all atom composition predicates
  through parser, loader, compiler, VM, and named-answer extraction.
