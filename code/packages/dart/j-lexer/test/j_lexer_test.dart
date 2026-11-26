/// Unit tests for `coding_adventures_j_lexer`: all 15 reference
/// tests, plus the corners the reference leaves uncovered — adverb
/// values, error positions, and empty input. Every input below was
/// additionally verified token-for-token against the Rust
/// implementation via a differential probe.
library;

import 'package:coding_adventures_j_lexer/j_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

List<String> types(String source) => [
      for (final t in tokenizeJ(source))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String source) => [
      for (final t in tokenizeJ(source))
        if (t.type != 'EOF') t.value
    ];

void main() {
  group('reference', () {
    test('slash is reduce, percent is divide', () {
      expect(types('+/'), ['PLUS', 'REDUCE']);
      expect(types('A%B'), ['NAME', 'PERCENT', 'NAME']);
    });

    test('every primitive verb glyph', () {
      expect(types('+ - * % ^ <. >. \$ i. , #'), [
        'PLUS',
        'MINUS',
        'STAR',
        'PERCENT',
        'CARET',
        'FLOOR',
        'CEILING',
        'DOLLAR',
        'IDOT',
        'RAVEL',
        'HASH',
      ]);
    });

    test('all six comparison glyphs', () {
      expect(types('= ~: < > <: >:'), ['EQ', 'NE', 'LT', 'GT', 'LE', 'GE']);
    });

    test('digraphs beat their single-char prefixes', () {
      expect(types('<.'), ['FLOOR']);
      expect(types('<:'), ['LE']);
      expect(types('<'), ['LT']);
      expect(types('>.'), ['CEILING']);
      expect(types('>:'), ['GE']);
      expect(types('>'), ['GT']);
      expect(types('~:'), ['NE']);
      expect(types('=.'), ['ASSIGN_LOCAL']);
      expect(types('=:'), ['ASSIGN_GLOBAL']);
      expect(types('='), ['EQ']);
      expect(types('i.'), ['IDOT']);
    });

    test('idot never swallows ordinary i-names', () {
      expect(types('iota'), ['NAME']);
      expect(types('i'), ['NAME']);
      expect(types('i.5'), ['IDOT', 'NUMBER']);
    });

    test('adverbs, conjunction, grouping, assignment', () {
      expect(types('A=.(+/ B)'), [
        'NAME',
        'ASSIGN_LOCAL',
        'LPAREN',
        'PLUS',
        'REDUCE',
        'NAME',
        'RPAREN',
      ]);
      expect(types('A=:B'), ['NAME', 'ASSIGN_GLOBAL', 'NAME']);
      expect(types('+\\ B'), ['PLUS', 'SCAN', 'NAME']);
      expect(types('f@g'), ['NAME', 'AT', 'NAME']);
    });

    test('dense arrays strand as number sequences', () {
      expect(types('1 2 3'), ['NUMBER', 'NUMBER', 'NUMBER']);
      expect(values('1 2 3'), ['1', '2', '3']);
    });

    test('underscore negatives versus the minus verb', () {
      expect(types('_3'), ['NUMBER']);
      expect(values('_3'), ['_3']);
      expect(types('A-B'), ['NAME', 'MINUS', 'NAME']);
      expect(types('A -_3'), ['NAME', 'MINUS', 'NUMBER']);
    });

    test('bare underscore is an error, not infinity', () {
      expect(() => tokenizeJ('_'), throwsA(isA<LexerError>()));
    });

    test('decimals and underscore exponents', () {
      expect(types('1.5E_3'), ['NUMBER']);
      expect(values('1.5E_3'), ['1.5E_3']);
      expect(values('3.14'), ['3.14']);
    });

    test('plain ASCII names', () {
      expect(types('VAR1 x Result'), ['NAME', 'NAME', 'NAME']);
    });

    test('whitespace and NB comments vanish', () {
      expect(types('NB. a whole comment line\nA=.1'),
          ['NEWLINE', 'NAME', 'ASSIGN_LOCAL', 'NUMBER']);
      expect(types('A=.1 NB. trailing comment\nB=.2'), [
        'NAME',
        'ASSIGN_LOCAL',
        'NUMBER',
        'NEWLINE',
        'NAME',
        'ASSIGN_LOCAL',
        'NUMBER',
      ]);
    });

    test('NB comment never swallows NB-led names', () {
      expect(types('NBA=.1'), ['NAME', 'ASSIGN_LOCAL', 'NUMBER']);
    });

    test('newline is its own significant token', () {
      expect(types('A=.1\nB=.2'), [
        'NAME',
        'ASSIGN_LOCAL',
        'NUMBER',
        'NEWLINE',
        'NAME',
        'ASSIGN_LOCAL',
        'NUMBER'
      ]);
    });

    test('index generator and sum reduce end to end', () {
      expect(types('A=.i.5\nB=.+/A'), [
        'NAME',
        'ASSIGN_LOCAL',
        'IDOT',
        'NUMBER',
        'NEWLINE',
        'NAME',
        'ASSIGN_LOCAL',
        'PLUS',
        'REDUCE',
        'NAME',
      ]);
    });
  });

  group('extras', () {
    test('adverb values are the bare glyphs', () {
      expect(values('+/'), ['+', '/']);
      expect(values('+\\ B'), ['+', '\\', 'B']);
    });

    test('dollar and hash are single tokens', () {
      expect(types('\$'), ['DOLLAR']);
      expect(types('#'), ['HASH']);
    });

    test('errors carry positions', () {
      try {
        tokenizeJ('AB`CD');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeJ('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createJLexer('A=.1').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createJLexer('`').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
