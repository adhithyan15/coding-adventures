import 'package:coding_adventures_algol_lexer/algol_lexer.dart';
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Returns the embedded parser grammar for a supported ALGOL version.
ParserGrammar loadAlgolParserGrammar({String version = defaultAlgolVersion}) {
  resolveAlgolVersion(version);
  return parserGrammar;
}

/// Creates a parser configured with the embedded canonical ALGOL 60 grammar.
GrammarParser createAlgolParser(
  String source, {
  String version = defaultAlgolVersion,
  GrammarParserOptions? options,
}) {
  final grammar = loadAlgolParserGrammar(version: version);
  final tokens = tokenizeAlgol(source, version: version);
  return GrammarParser(tokens, grammar, options: options);
}

/// Parses a complete ALGOL 60 program into the shared grammar AST.
ASTNode parseAlgol(
  String source, {
  String version = defaultAlgolVersion,
  GrammarParserOptions? options,
}) =>
    createAlgolParser(source, version: version, options: options).parse();
