import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

void main() {
  List<String> types(List<Token> tokens) => tokens.map((token) => token.type).toList();
  List<String> values(List<Token> tokens) => tokens.map((token) => token.value).toList();

  group('tokenize', () {
    test('tokenizes a simple assignment', () {
      final tokens = tokenize('x = 1 + 2');
      expect(types(tokens), ['NAME', 'EQUALS', 'NUMBER', 'PLUS', 'NUMBER', 'EOF']);
      expect(values(tokens), ['x', '=', '1', '+', '2', '']);
    });

    test('tokenizes strings and escapes', () {
      final tokens = tokenize(r'"He said \"hi\""');
      expect(tokens.first, const Token(type: 'STRING', value: 'He said "hi"', line: 1, column: 1));
    });

    test('recognizes keywords from config', () {
      final tokens = tokenize(
        'if value == 1',
        const LexerConfig(keywords: ['if', 'else']),
      );
      expect(types(tokens), ['KEYWORD', 'NAME', 'EQUALS_EQUALS', 'NUMBER', 'EOF']);
    });

    test('tracks newlines and columns', () {
      final tokens = tokenize('x = 1\ny = 2');
      expect(tokens[0], const Token(type: 'NAME', value: 'x', line: 1, column: 1));
      expect(tokens[3].type, 'NEWLINE');
      expect(tokens[4], const Token(type: 'NAME', value: 'y', line: 2, column: 1));
    });

    test('throws on unterminated strings and unexpected characters', () {
      expect(() => tokenize('"hello'), throwsA(isA<LexerError>()));
      expect(() => tokenize('@'), throwsA(isA<LexerError>()));
    });
  });

  group('tokenizer dfa', () {
    test('classifies characters', () {
      expect(classifyChar(null), 'eof');
      expect(classifyChar('7'), 'digit');
      expect(classifyChar('A'), 'alpha');
      expect(classifyChar('_'), 'underscore');
      expect(classifyChar('+'), 'operator');
      expect(classifyChar('{'), 'open_brace');
      expect(classifyChar('@'), 'other');
    });

    test('dispatches from start to the correct handler states', () {
      final cases = <String, String>{
        'digit': 'in_number',
        'alpha': 'in_name',
        'underscore': 'in_name',
        'quote': 'in_string',
        'newline': 'at_newline',
        'whitespace': 'at_whitespace',
        'operator': 'in_operator',
        'equals': 'in_equals',
        'open_paren': 'in_operator',
        'eof': 'done',
        'other': 'error',
      };

      for (final entry in cases.entries) {
        final dfa = newTokenizerDfa();
        expect(dfa.process(entry.key), entry.value);
      }
    });

    test('is complete and terminal states loop', () {
      final dfa = newTokenizerDfa();
      expect(dfa.isComplete(), isTrue);

      dfa.process('eof');
      expect(dfa.currentState, 'done');
      expect(dfa.process('digit'), 'done');

      final failing = newTokenizerDfa();
      failing.process('other');
      expect(failing.currentState, 'error');
      expect(failing.process('digit'), 'error');
    });
  });
}
