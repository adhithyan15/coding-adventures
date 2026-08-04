import 'package:coding_adventures_mosaic_lexer/mosaic_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Creates a parser configured with the embedded canonical Mosaic grammar.
GrammarParser createMosaicParser(
  String source, {
  GrammarParserOptions? options,
}) {
  final tokens = tokenizeMosaic(source);
  return GrammarParser(tokens, parserGrammar, options: options);
}

/// Parses a complete Mosaic source file into the shared grammar AST.
ASTNode parseMosaic(String source, {GrammarParserOptions? options}) =>
    createMosaicParser(source, options: options).parse();
