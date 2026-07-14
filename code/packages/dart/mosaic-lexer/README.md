# coding_adventures_mosaic_lexer

Grammar-driven Mosaic lexer for Dart.

This package is a thin wrapper around the shared Dart `lexer` and
`grammar-tools` packages. It compiles the canonical
`code/grammars/mosaic/mosaic.tokens` grammar into Dart data and delegates
tokenization to `grammarTokenize`, so published packages do not depend on the
repository layout at run time.

```dart
import 'package:coding_adventures_mosaic_lexer/mosaic_lexer.dart';

final tokens = tokenizeMosaic('component Card { Box { } }');
```

The returned token list always ends with `EOF`. Whitespace, line comments,
and block comments are omitted, while every emitted token carries a 1-based
line and column.
