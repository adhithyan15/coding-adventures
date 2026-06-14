# PR54: Prolog Text Inspection Predicates

## Goal

Close another source-level usability gap in the Prolog-on-Logic-VM path by
adding finite atom and string inspection predicates. These predicates let
programs measure text values and enumerate or validate substring positions
without leaving the Logic VM runtime.

## Scope

The library layer exposes:

```text
atom_lengtho(atom, length)
string_lengtho(string, length)
sub_atomo(atom, before, length, after, sub_atom)
sub_stringo(string, before, length, after, sub_string)
```

The Prolog adapter exposes the conventional source-level names:

```text
atom_length/2
string_length/2
sub_atom/5
sub_string/5
```

## Semantics

These relations intentionally support finite, VM-friendly modes:

- `atom_length/2` and `string_length/2` measure a bound atom or string and
  unify the result with a non-negative integer length.
- `sub_atom/5` and `sub_string/5` enumerate finite substring slices when the
  full text value is bound.
- Bound position, length, and substring arguments filter the finite slice
  enumeration.
- If the full text value is open, the predicates only construct the exact
  finite case where `Before = 0`, `After = 0`, and the sub-text value is bound.
- Fully open calls fail rather than enumerating infinite atom or string spaces.
- Empty atom slices are not produced yet because the current symbol primitive
  rejects empty symbol names.

## Verification

- `logic-builtins` tests cover atom/string length, finite slice enumeration,
  filtering, and exact text construction.
- `prolog-loader` tests prove the source-level predicate names adapt into
  executable builtins.
- `prolog-vm-compiler` stress coverage runs text inspection predicates through
  parser, loader, compiler, VM, and named-answer extraction.
