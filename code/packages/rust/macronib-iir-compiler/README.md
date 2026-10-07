# MacroNib IIR compiler

MacroNib adds `.include`, `.set`, `.ifdef`, `.else`, and `.endif` to Nib.
This crate supplies a small dialect adapter to the generic
`source-preprocessor` engine. The engine consumes directives, expands object
macros, resolves includes, tracks provenance, and enforces resource bounds.
Nib's guarded parser, type checker, and IIR compiler then process the remaining
tokens. No backend changes are needed.
Every primary or included file is capped before tokenization at the tighter
of the file-byte budget and the token budget minus one EOF token. This
conservative source-byte limit prevents a dense input from allocating more
tokens than the preprocessor allows. The in-memory include API also checks
unused files, names, file count, and aggregate bytes before copying them into
`MemoryFs`; the engine retains its include-read and expansion bounds.

`compile_source` accepts a source string. `compile_source_with_includes` accepts
an in-memory include set. `preprocess_source` accepts a caller-supplied
`SourceFs`; disk-backed callers should use the engine's `RootedFs`.

The tests compare MacroNib output to independently hand-expanded Nib, including
a line-aligned full-IIR comparison against a Nib program executed in the
eight-backend matrix. They also cover malformed directives, missing and cyclic
includes, expanded-token bounds, dense-token lexing, and oversized unused
includes.
