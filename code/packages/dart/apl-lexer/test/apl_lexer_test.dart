/// Unit tests for `coding_adventures_apl_lexer`: all 10 reference
/// integration tests, plus the corners the reference leaves uncovered
/// — single-glyph pins, CRLF versus LF newline shapes, NEWLINE values,
/// high-minus exponent forms, error positions, and empty input. Every
/// input below was additionally verified token-for-token against the
/// Rust implementation via a differential probe.
library;

import 'package:coding_adventures_apl_lexer/apl_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

List<String> types(String source) => [
      for (final t in tokenizeApl(source))
        if (t.type != 'EOF') t.type
    ];

List<String> values(String source) => [
      for (final t in tokenizeApl(source))
        if (t.type != 'EOF') t.value
    ];

List<(String, String)> pairs(String source) => [
      for (final t in tokenizeApl(source))
        if (t.type != 'EOF') (t.type, t.value)
    ];

void main() {
  group('reference', () {
    test('every primitive function glyph', () {
      expect(types('+ - × ÷ ⌈ ⌊ ⍴ ⍳ , = ≠ < ≤ ≥ >'), [
        'PLUS',
        'MINUS',
        'TIMES',
        'DIVIDE',
        'CEILING',
        'FLOOR',
        'RHO',
        'IOTA',
        'RAVEL',
        'EQ',
        'NE',
        'LT',
        'LE',
        'GE',
        'GT',
      ]);
    });

    test('operators, grouping, and assignment', () {
      expect(types('A←(+/ B)'),
          ['NAME', 'ARROW', 'LPAREN', 'PLUS', 'REDUCE', 'NAME', 'RPAREN']);
      expect(types('+\\ B'), ['PLUS', 'SCAN', 'NAME']);
      expect(types('∘.×'), ['OUTER', 'TIMES']);
    });

    test('outer never splits into jot and dot', () {
      expect(types('∘.='), ['OUTER', 'EQ']);
    });

    test('dense arrays strand as number sequences', () {
      expect(types('1 2 3'), ['NUMBER', 'NUMBER', 'NUMBER']);
      expect(values('1 2 3'), ['1', '2', '3']);
    });

    test('high minus is a number sign, dash is a function', () {
      expect(types('¯3'), ['NUMBER']);
      expect(values('¯3'), ['¯3']);
      expect(types('A-B'), ['NAME', 'MINUS', 'NAME']);
      expect(types('A -¯3'), ['NAME', 'MINUS', 'NUMBER']);
    });

    test('decimals and high-minus exponents', () {
      expect(types('1.5E¯3'), ['NUMBER']);
      expect(values('1.5E¯3'), ['1.5E¯3']);
      expect(values('3.14'), ['3.14']);
    });

    test('plain ASCII names', () {
      expect(types('VAR1 x Result'), ['NAME', 'NAME', 'NAME']);
    });

    test('whitespace and line comments vanish', () {
      expect(types('⍝ a whole comment line\nA←1'),
          ['NEWLINE', 'NAME', 'ARROW', 'NUMBER']);
      expect(types('A←1 ⍝ trailing comment\nB←2'),
          ['NAME', 'ARROW', 'NUMBER', 'NEWLINE', 'NAME', 'ARROW', 'NUMBER']);
    });

    test('newline is its own significant token', () {
      expect(types('A←1\nB←2'),
          ['NAME', 'ARROW', 'NUMBER', 'NEWLINE', 'NAME', 'ARROW', 'NUMBER']);
    });

    test('MA05 kickoff example end to end', () {
      expect(types('A←⍳5\nB←+/A'), [
        'NAME',
        'ARROW',
        'IOTA',
        'NUMBER',
        'NEWLINE',
        'NAME',
        'ARROW',
        'PLUS',
        'REDUCE',
        'NAME',
      ]);
    });
  });

  group('extras', () {
    test('single glyphs carry their own values', () {
      const glyphs = {
        '+': 'PLUS',
        '-': 'MINUS',
        '×': 'TIMES',
        '÷': 'DIVIDE',
        '⌈': 'CEILING',
        '⌊': 'FLOOR',
        '⍴': 'RHO',
        '⍳': 'IOTA',
        ',': 'RAVEL',
        '/': 'REDUCE',
        '\\': 'SCAN',
        '←': 'ARROW',
        '(': 'LPAREN',
        ')': 'RPAREN',
      };
      for (final entry in glyphs.entries) {
        expect(pairs(entry.key), [(entry.value, entry.key)]);
      }
    });

    test('LF and CRLF newline shapes differ honestly', () {
      // Bare LF hits the engine's built-in NEWLINE (literal backslash-n
      // value); CRLF matches the grammar's NEWLINE rule (raw value) —
      // both lanes, by shared engine construction.
      expect(pairs('A\nB'), [('NAME', 'A'), ('NEWLINE', r'\n'), ('NAME', 'B')]);
      expect(
          pairs('A\r\nB'), [('NAME', 'A'), ('NEWLINE', '\r\n'), ('NAME', 'B')]);
    });

    test('negative exponent and scale forms', () {
      expect(pairs('-3'), [('MINUS', '-'), ('NUMBER', '3')]);
      expect(pairs('2e4'), [('NUMBER', '2e4')]);
      expect(pairs('x9'), [('NAME', 'x9')]);
    });

    test('reduce and scan idioms', () {
      expect(pairs('+/⍳⍴A'), [
        ('PLUS', '+'),
        ('REDUCE', '/'),
        ('IOTA', '⍳'),
        ('RHO', '⍴'),
        ('NAME', 'A'),
      ]);
      expect(pairs('M←(+/A)÷⍴A'), [
        ('NAME', 'M'),
        ('ARROW', '←'),
        ('LPAREN', '('),
        ('PLUS', '+'),
        ('REDUCE', '/'),
        ('NAME', 'A'),
        ('RPAREN', ')'),
        ('DIVIDE', '÷'),
        ('RHO', '⍴'),
        ('NAME', 'A'),
      ]);
    });

    test('out-of-alphabet characters fail closed', () {
      for (final src in ['@', 'A@B', '_', '9A \$']) {
        expect(() => tokenizeApl(src), throwsA(isA<LexerError>()));
      }
    });

    test('errors carry positions', () {
      try {
        tokenizeApl('AB@CD');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeApl('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createAplLexer('A+B').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createAplLexer('@').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
