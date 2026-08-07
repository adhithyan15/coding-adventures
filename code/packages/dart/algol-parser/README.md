# coding_adventures_algol_parser

Grammar-driven ALGOL 60 parser for Dart.

The package embeds `code/grammars/algol/algol60.grammar` as Dart data and
combines it with `coding_adventures_algol_lexer` and the shared parser runtime.
No repository-relative grammar files are read at run time.

```dart
import 'package:coding_adventures_algol_parser/algol_parser.dart';

final ast = parseAlgol('begin integer x; x := 42 end');
```

`parseAlgol` returns the shared `ASTNode` representation. Use
`createAlgolParser` for parser options or direct parser access. `algol60` is
currently the only supported grammar version.
