/// XML lexer backed by the compiled token grammar and
/// callback-driven pattern groups. The Dart lane of `xml-lexer`:
/// one language definition (`code/grammars/xml/xml.tokens`) powers
/// every implementation; this lane embeds the compiled grammar and
/// ports the group-stack callback.
library;

export 'src/tokenizer.dart';
