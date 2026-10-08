# MacroNib lexer

This crate tokenizes Nib source plus `.include`, `.set`, `.ifdef`, `.else`,
and `.endif` directives for PREP01 slice 3. `STR_LIT` exists only for quoted
include names. The preprocessor consumes those additions before passing tokens
to Nib's unchanged parser.

`code/grammars/macronib/macronib.tokens` copies Nib's lexical rules. A test
compares every ordinary token definition, keyword, and skip rule against
`nib.tokens` in order, and another checks the compiled `_grammar.rs` artifact.
Malformed input should use `try_tokenize_macronib` to receive an error.
