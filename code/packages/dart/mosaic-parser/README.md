# coding_adventures_mosaic_parser

Grammar-driven Mosaic parser for Dart.

The package compiles `code/grammars/mosaic/mosaic.grammar` into Dart data and
combines it with `coding_adventures_mosaic_lexer` and the shared Dart parser
runtime. No repository-relative grammar files are read at run time.

```dart
import 'package:coding_adventures_mosaic_parser/mosaic_parser.dart';

final ast = parseMosaic('component Card { Box { } }');
```

`parseMosaic` returns the shared `ASTNode` representation. Call
`createMosaicParser` when parser options or direct parser access are needed.
