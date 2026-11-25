/// CSS lexer backed by the compiled token grammar. The Dart lane of
/// `css-lexer`: one language definition (`code/grammars/css/css.tokens`)
/// powers every implementation. No callback — CSS needs no group stack.
library;

export 'src/tokenizer.dart';
