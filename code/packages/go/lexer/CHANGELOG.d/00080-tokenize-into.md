# `Compiled.TokenizeInto` reuses caller-owned token storage

Added an opt-in standard-tokenizer entry point that writes to caller scratch
while preserving the existing tokenizer and matcher. Indentation and layout
grammars continue through the owned path. `Tokenize` remains allocation-owning;
the new API documents its borrowed-slice lifetime and clears partial output on
lexical errors, including when append grows the destination.
