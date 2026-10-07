# MacroNib IIR compiler

MacroNib adds `.include`, `.set`, `.ifdef`, `.else`, and `.endif` to Nib.
This crate supplies a small dialect adapter to the generic
`source-preprocessor` engine. The engine consumes directives, expands object
macros, resolves includes, tracks provenance, and enforces resource bounds.
Nib's guarded parser, type checker, and IIR compiler then process the remaining
tokens. No backend changes are needed.

`compile_source` accepts a source string. `compile_source_with_includes` accepts
an in-memory include set. `preprocess_source` accepts a caller-supplied
`SourceFs`; disk-backed callers should use the engine's `RootedFs`.

The tests compare MacroNib output to independently hand-expanded Nib, including
a line-aligned full-IIR comparison. They also cover malformed directives,
missing and cyclic includes, and the generic token budget.
