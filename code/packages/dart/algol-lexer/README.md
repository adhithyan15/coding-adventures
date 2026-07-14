# coding_adventures_algol_lexer

Grammar-driven ALGOL 60 lexer for Dart.

This package embeds the canonical
`code/grammars/algol/algol60.tokens` grammar as Dart data and delegates token
matching to the shared Dart lexer. The wrapper supplies ALGOL-specific
normalization for case-insensitive keywords, `comment ...;` comments, and the
publication symbols used by the Revised Report.

```dart
import 'package:coding_adventures_algol_lexer/algol_lexer.dart';

final tokens = tokenizeAlgol('begin integer x; x := 42 end');
```

The result always ends with `EOF`. `algol60` is currently the only supported
grammar version, and installed packages perform no repository-relative file
I/O at run time.
