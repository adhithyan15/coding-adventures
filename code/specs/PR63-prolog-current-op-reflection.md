# PR63: Prolog Operator Reflection

## Status

Implemented for the Python Prolog loader and Logic VM compiler path.

## Goal

Expose loaded Prolog operator declarations through `current_op/3` so source
programs and ad-hoc VM queries can inspect the same operator table that the
parser used.

## Behavior

- `current_op(Precedence, Type, Name)` enumerates every operator declaration in
  the active source operator table.
- `Precedence` is unified with a numeric term.
- `Type` is unified with the associativity atom, such as `xfx`, `xfy`, `yfx`,
  `fx`, `fy`, `xf`, or `yf`.
- `Name` is unified with the operator atom.
- Custom `op/3` directives are visible after loading, compiling, and running
  through the Logic VM.
- Ad-hoc VM runtime queries use the loaded source operator table, so later
  queries can reflect custom operators declared by the source.

## Layering

The operator table remains parser and loader metadata. The Prolog goal adapter
receives that table explicitly when source is lowered into executable VM goals,
which keeps the Logic VM instruction format independent of Prolog-specific
parser state while still making operator reflection executable.
