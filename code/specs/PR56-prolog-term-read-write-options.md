# PR56: Prolog Term Read/Write Options

## Goal

Build on PR55's parser-backed term text bridge by adding source-level
read/write helpers that accept option lists. This moves another practical
metaprogramming pattern into the Logic VM path while keeping the supported
modes finite and explicit.

## Scope

The loader adapter exposes:

```text
read_term_from_atom/3
write_term_to_atom/3
```

The VM path supports both predicates through the existing parser, loader,
compiler, and runtime pipeline.

## Semantics

These predicates intentionally support a conservative option subset:

- `read_term_from_atom(Atom, Term, Options)` parses a bound atom as one
  SWI-Prolog term and unifies it with `Term`.
- `read_term_from_atom/3` supports `variable_names(Names)` and
  `variables(Vars)` options, unifying them with the named-variable binding list
  and variable value list from the parsed term.
- `write_term_to_atom(Term, Atom, Options)` renders a bound runtime term into
  canonical Prolog atom text.
- `write_term_to_atom/3` accepts finite `quoted/1`, `ignore_ops/1`, and
  `numbervars/1` boolean options, but rendering remains canonical for now.
- Unsupported options fail cleanly rather than approximating behavior we do
  not yet implement.
- Fully open calls fail rather than enumerating infinite term text or option
  spaces.

## Verification

- `prolog-loader` tests cover successful read/write option handling and
  unsupported option failure.
- `prolog-vm-compiler` stress coverage runs both predicates through parser,
  loader, compiler, VM, and named-answer extraction.
