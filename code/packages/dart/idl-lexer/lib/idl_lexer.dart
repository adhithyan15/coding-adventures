/// IDL lexer backed by the compiled token grammar. The Dart lane of
/// `idl-lexer`: one language definition (`code/grammars/idl/idl.tokens`)
/// powers every implementation. No callback, no hooks — the grammar is
/// entirely declarative (notably, `/` is always plain division; the
/// `/KEYWORD` shorthand is the parser's problem).
library;

export 'src/tokenizer.dart';
