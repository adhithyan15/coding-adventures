# coding_adventures_toml_lexer

Grammar-driven TOML lexer for Dart.

This package is a thin wrapper around the shared Dart `lexer` and
`grammar-tools` packages. It embeds the canonical `toml.tokens` grammar as
Dart data and delegates tokenization to `grammarTokenize`, so tokenization does
not depend on repository-relative files at run time.

```dart
import 'package:coding_adventures_toml_lexer/toml_lexer.dart';

final tokens = tokenizeToml('title = "TOML Example"');
```

The lexer recognizes TOML strings, integers, floats, booleans, date/time
literals, bare keys, structural delimiters, and significant newlines. Comments
and horizontal whitespace are skipped.
