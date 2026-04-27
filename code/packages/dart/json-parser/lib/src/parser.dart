import 'package:coding_adventures_json_lexer/json_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

GrammarParser createJsonParser(String source, {GrammarParserOptions? options}) {
  final tokens = tokenizeJson(source);
  return GrammarParser(tokens, parserGrammar, options: options);
}

ASTNode parseJson(String source, {GrammarParserOptions? options}) {
  final parser = createJsonParser(source, options: options);
  return parser.parse();
}
