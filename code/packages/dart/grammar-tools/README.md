# coding_adventures_grammar_tools

Parsers, validators, and source compilers for the shared `.tokens` and
`.grammar` formats used across the repository.

This is the first Dart bring-up of the grammar-driven toolchain. It is meant
to unblock the Dart lexer/parser ports by handling:

- `.tokens` parsing into `TokenGrammar`
- `.grammar` parsing into `ParserGrammar`
- token/parser cross-validation
- source-code compilation into embedded Dart objects
