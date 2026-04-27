# coding_adventures_json_lexer

Grammar-driven JSON lexer for Dart.

This package is a thin wrapper around the shared Dart `lexer` and
`grammar-tools` packages. It embeds the `json.tokens` grammar as Dart code and
delegates tokenization to `grammarTokenize`.
