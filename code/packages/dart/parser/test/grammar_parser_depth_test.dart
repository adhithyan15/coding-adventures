/// Tests for the `maxDepth` recursion guard: opt-in uncapped-by-default
/// semantics, exact counting (entry runs at depth 0, the first nested
/// rule call trips a cap of 1), the priority depth message, and clean
/// deep-input refusal.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

void main() {
  Token tok(String type, String value, [int line = 1, int column = 1]) {
    return Token(type: type, value: value, line: line, column: column);
  }

  Token eof([int line = 1, int column = 1]) {
    return Token(type: 'EOF', value: '', line: line, column: column);
  }

  ParserGrammar grammar(String source) => parseParserGrammar(source);

  /// `expr = LPAREN expr RPAREN | NUMBER ;` tokenized from [depth]
  /// nested parens around `1`.
  List<Token> nested(int depth) => [
        for (var i = 0; i < depth; i++) tok('LPAREN', '('),
        tok('NUMBER', '1'),
        for (var i = 0; i < depth; i++) tok('RPAREN', ')'),
        eof(),
      ];

  final nestedGrammar = grammar('expr = LPAREN expr RPAREN | NUMBER ;');

  group('maxDepth', () {
    test('defaults to uncapped', () {
      expect(const GrammarParserOptions().maxDepth, isNull);
      // Mild nesting parses with no options at all.
      final ast = GrammarParser(nested(5), nestedGrammar).parse();
      expect(ast.ruleName, equals('expr'));
    });

    test('under the cap parses', () {
      final ast = GrammarParser(
        nested(3),
        nestedGrammar,
        options: const GrammarParserOptions(maxDepth: 100),
      ).parse();
      expect(ast.ruleName, equals('expr'));
    });

    test('entry runs at depth zero, first nesting trips a cap of one', () {
      // A flat rule needs no nesting: the entry call at depth 0 is
      // always allowed.
      final flat = GrammarParser(
        [tok('NUMBER', '1'), eof()],
        grammar('value = NUMBER ;'),
        options: const GrammarParserOptions(maxDepth: 1),
      ).parse();
      expect(flat.ruleName, equals('value'));

      // One nesting level trips a cap of 1: the nested call runs at
      // depth 1, which is already refused.
      expect(
        () => GrammarParser(
          [tok('NUMBER', '1'), eof()],
          grammar('a = b ; b = NUMBER ;'),
          options: const GrammarParserOptions(maxDepth: 1),
        ).parse(),
        throwsA(
          isA<GrammarParseError>().having(
            (e) => e.message,
            'message',
            contains('supported limit (1)'),
          ),
        ),
      );
    });

    test('deep input fails cleanly with the priority message', () {
      // 500 levels against a cap of 100: the guard — not the stack —
      // stops the recursion, and the depth message wins over any
      // furthest-failure text.
      try {
        GrammarParser(
          nested(500),
          nestedGrammar,
          options: const GrammarParserOptions(maxDepth: 100),
        ).parse();
        fail('expected GrammarParseError');
      } on GrammarParseError catch (e) {
        expect(e.message, contains('supported limit (100)'));
      }
    });

    test('ordinary syntax errors keep their message under a high cap', () {
      // A genuine syntax error with headroom left reports normally —
      // no depth text leaks in.
      try {
        GrammarParser(
          [tok('LPAREN', '('), eof()],
          nestedGrammar,
          options: const GrammarParserOptions(maxDepth: 100),
        ).parse();
        fail('expected GrammarParseError');
      } on GrammarParseError catch (e) {
        expect(e.message, isNot(contains('supported limit')));
      }
    });
  });
}
