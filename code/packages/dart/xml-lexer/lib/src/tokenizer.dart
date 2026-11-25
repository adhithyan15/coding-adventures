/// XML tokenization over the compiled grammar: the group-stack
/// callback plus the factory and one-shot entry points.
/// Token types keep their grammar names verbatim; the final token is
/// always `EOF`. Lex failures throw the engine's [LexerError] with
/// line, column, and a stable error id (the reference panics; Dart
/// lanes propagate the structured error instead of wrapping it).
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Drives the group stack and skip toggling per token.
/// OPEN/CLOSE_TAG_START pushes `tag`; TAG_CLOSE/SELF_CLOSE pops;
/// COMMENT/CDATA/PI starts push their group and freeze skips (their
/// whitespace is content); ends pop and re-enable. PI_TARGET swaps
/// `pi` for `pi_body` so the target pattern is never re-offered.
void xmlOnToken(Token token, LexerContext ctx) {
  switch (token.type) {
    case 'OPEN_TAG_START':
    case 'CLOSE_TAG_START':
      ctx.pushGroup('tag');
    case 'TAG_CLOSE':
    case 'SELF_CLOSE':
      ctx.popGroup();
    case 'COMMENT_START':
      ctx.pushGroup('comment');
      ctx.setSkipEnabled(false);
    case 'COMMENT_END':
      ctx.popGroup();
      ctx.setSkipEnabled(true);
    case 'CDATA_START':
      ctx.pushGroup('cdata');
      ctx.setSkipEnabled(false);
    case 'CDATA_END':
      ctx.popGroup();
      ctx.setSkipEnabled(true);
    case 'PI_START':
      ctx.pushGroup('pi');
      ctx.setSkipEnabled(false);
    case 'PI_TARGET':
      // Swap (not push) from `pi` to `pi_body`: PI_TARGET is only
      // ever the first token in a PI, so once matched, the rest of
      // the body must never be re-offered PI_TARGET's pattern (see
      // xml.tokens' pi/pi_body groups). PI_END's single popGroup
      // below still returns straight past this swap.
      ctx.popGroup();
      ctx.pushGroup('pi_body');
    case 'PI_END':
      ctx.popGroup();
      ctx.setSkipEnabled(true);
  }
}

/// Creates a [GrammarLexer] for [source] with the XML grammar and
/// callback wired up.
GrammarLexer createXmlLexer(String source) {
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.setOnToken(xmlOnToken);
  return lexer;
}

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unexpected sequences.
List<Token> tokenizeXml(String source) => createXmlLexer(source).tokenize();
