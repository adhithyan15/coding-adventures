# PR55: Prolog Term Text I/O

## Goal

Add the next parser-backed text bridge for the Prolog-on-Logic-VM path:
converting runtime terms to atom text and parsing atom text back into runtime
terms. This closes a practical metaprogramming gap without moving parser
dependencies into the pure `logic-builtins` package.

## Scope

The parser layer exposes:

```text
parse_operator_named_term_tokens(tokens, operator_table)
parse_swi_term(source)
```

The loader adapter exposes the source-level predicates:

```text
term_to_atom/2
atom_to_term/3
```

## Semantics

These predicates intentionally support finite, VM-friendly modes:

- `term_to_atom/2` renders a bound runtime term into canonical Prolog atom text.
- If the atom side of `term_to_atom/2` is already bound and the term side is
  open, the atom text is parsed and unified with the term.
- `atom_to_term/3` parses a bound atom as one SWI-Prolog term, unifies that
  parsed term with the second argument, and unifies the third argument with a
  `Name = Var` list for named variables found in the parsed text.
- Invalid term text fails rather than raising out of the logic query.
- Fully open calls fail rather than enumerating infinite term text.
- Rendering is canonical and deliberately conservative; it does not attempt to
  preserve source operator spelling or comments.

## Verification

- `prolog-operator-parser` tests cover single-term parsing with named variable
  retention.
- `swi-prolog-parser` tests expose the dialect-specific single-term parser.
- `prolog-loader` tests prove `term_to_atom/2` and `atom_to_term/3` adapt into
  executable goals and preserve parsed variable bindings.
- `prolog-vm-compiler` stress coverage runs both predicates through parser,
  loader, compiler, VM, and named-answer extraction.
