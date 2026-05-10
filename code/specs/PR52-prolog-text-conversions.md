# PR52: Prolog Text Conversion Predicates

## Goal

Close the next source-level usability gap in the Prolog-on-Logic-VM path by
adding finite atom, string, number, character, and code conversion predicates.
These predicates make parsed Prolog programs able to move between symbolic
terms and list representations without requiring a separate language runtime.

## Scope

The library layer exposes:

```text
atom_charso(atom, chars)
atom_codeso(atom, codes)
number_charso(number, chars)
number_codeso(number, codes)
char_codeo(char, code)
string_charso(string, chars)
string_codeso(string, codes)
```

The Prolog adapter exposes the conventional source-level names:

```text
atom_chars/2
atom_codes/2
number_chars/2
number_codes/2
char_code/2
string_chars/2
string_codes/2
```

## Semantics

These relations intentionally support finite, VM-friendly modes:

- If the scalar term is bound, the list side is unified with the corresponding
  proper list representation.
- If the scalar term is open and the list side is a proper ground char/code
  list, the scalar is constructed and unified.
- Fully open conversion calls fail rather than enumerating an infinite space of
  possible atoms, strings, or numbers.
- Character lists use one-character atoms.
- Code lists use integer Unicode code point numbers.
- Number conversion parses either integer text or floating-point text.

## Verification

- `logic-builtins` tests cover bidirectional atom/string conversions, number
  parsing/rendering, and `char_codeo/2`.
- `prolog-loader` tests prove the source-level predicate names adapt into
  executable builtins.
- `prolog-vm-compiler` stress coverage runs all text conversion predicates
  through parser, loader, compiler, VM, and named-answer extraction.
