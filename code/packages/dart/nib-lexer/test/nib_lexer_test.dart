/// Unit tests for `coding_adventures_nib_lexer`: all 4 reference
/// tests, plus the corners the reference leaves uncovered — keyword
/// promotion across the whole table, error positions, empty input,
/// and factory-direct use.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('reference', () {
    test('simple function tokenizes', () {
      final tokens = tokenizeNib('fn main() { let x: u4 = 7; }');
      expect(tokens.map((t) => t.value), contains('fn'));
      expect(tokens.map((t) => t.value), contains('main'));
      expect(tokens.map((t) => t.value), contains('7'));
    });

    test('keyword types promote to literal values', () {
      final tokens = tokenizeNib('fn main() {}');
      final fnToken = tokens.firstWhere((t) => t.value == 'fn');
      expect(fnToken.type, 'fn');
    });

    test('shifts win over comparisons', () {
      final tokens = tokenizeNib('fn main() { let x: u8 = 1 << 2 >> 1; }');
      final kinds = [for (final t in tokens) t.type];
      expect(kinds, contains('SHL'));
      expect(kinds, contains('SHR'));
    });

    test('remainder operator tokenizes', () {
      final tokens = tokenizeNib('fn main() { let x: u8 = 86 % 7; }');
      expect([for (final t in tokens) t.type], contains('PERCENT'));
    });
  });

  group('extras', () {
    test('every keyword promotes across the table', () {
      for (final kw in [
        'fn',
        'let',
        'static',
        'const',
        'return',
        'for',
        'while',
        'in',
        'if',
        'else',
        'true',
        'false',
      ]) {
        final tokens = tokenizeNib(kw);
        expect(tokens[0].type, kw);
        expect(tokens[0].value, kw);
      }
    });

    test('non-keyword names keep the NAME type', () {
      final tokens = tokenizeNib('main');
      expect(tokens[0].type, 'NAME');
    });

    test('errors carry positions', () {
      // `@` has no token in this grammar.
      expect(() => tokenizeNib('@'), throwsA(isA<LexerError>()));
      try {
        tokenizeNib('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeNib('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createNibLexer('fn main() {}').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createNibLexer('@').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
