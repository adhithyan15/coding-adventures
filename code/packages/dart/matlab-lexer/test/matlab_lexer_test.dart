/// Port of the reference `rust/matlab-lexer` unit suite (20 tests).
///
/// Like the reference `lex` helper, [lex] drops both EOF and NEWLINE;
/// the hook's own test counts NEWLINEs on the full stream instead.
library;

import 'package:coding_adventures_matlab_lexer/matlab_lexer.dart';
import 'package:test/test.dart';

/// Non-EOF, non-NEWLINE (type, value) pairs of [source].
List<(String, String)> lex(String source) => [
      for (final t in tokenizeMatlab(source))
        if (t.type != 'EOF' && t.type != 'NEWLINE') (t.type, t.value),
    ];

List<String> types(String source) => [for (final p in lex(source)) p.$1];

void main() {
  group('matlab-lexer', () {
    test('transpose after a value', () {
      final p = lex("A'\n");
      expect(p[1], equals(('TRANSPOSE', "'")));
      // No space: `(A)'`, `x(1)'`, and `A''` (double transpose) are
      // transposes.
      expect(
        types("(A)'\n"),
        equals(['LPAREN', 'NAME', 'RPAREN', 'TRANSPOSE']),
      );
      expect(types("A''\n"), equals(['NAME', 'TRANSPOSE', 'TRANSPOSE']));
    });

    test('char array when not after a value', () {
      // At the start, after `=`, after an operator → string. STRING
      // values are the decoded content (quotes stripped), matching
      // double-quoted strings.
      expect(lex("'abc'\n")[0], equals(('STRING', 'abc')));
      expect(lex("x = 'hi'\n")[2], equals(('STRING', 'hi')));
      // The classic trap: `A' * B'` is two transposes, NOT a string.
      expect(
        types("A' * B'\n"),
        equals(['NAME', 'TRANSPOSE', 'STAR', 'NAME', 'TRANSPOSE']),
      );
    });

    test('whitespace resets to string inside brackets', () {
      // `[1 'a']`: the space before `'` means it starts a string, not
      // a transpose of `1`.
      expect(
        types("[1 'a']\n"),
        equals(['LBRACKET', 'NUMBER', 'STRING', 'RBRACKET']),
      );
    });

    test('doubled quote is an escaped apostrophe', () {
      // `'it''s'` is the single char array  it's  — `''` decodes to
      // one `'`.
      expect(lex("x = 'it''s'\n")[2], equals(('STRING', "it's")));
    });

    test('double quoted strings', () {
      // The grammar lexer resolves double-quoted strings to their
      // content.
      expect(lex('"hello"\n')[0], equals(('STRING', 'hello')));
      // A `'` inside a double-quoted string is just text, not a
      // transpose.
      expect(lex('"it\'s"\n')[0], equals(('STRING', "it's")));
    });

    test('double-quoted string resembling a placeholder is not restored', () {
      // Regression: a crafted double-quoted string whose content looks
      // like the internal `` `N` `` placeholder must survive verbatim
      // — restoration keys off the distinct CHARARRAY type, not the
      // value. (Earlier some real char arrays are present so index 0
      // exists in the table.)
      final p = lex('a = \'x\'; b = "`0`"\n');
      // The last STRING token is the double-quoted one; it must stay
      // `0`, NOT be replaced by the char array 'x'.
      final lastString = p.reversed.firstWhere((t) => t.$1 == 'STRING');
      expect(lastString.$2, equals('`0`'));
    });

    test('elementwise operators', () {
      expect(
        types('A .* B ./ C .^ D .\\ E\n'),
        equals([
          'NAME',
          'ELEM_MUL',
          'NAME',
          'ELEM_RDIV',
          'NAME',
          'ELEM_POW',
          'NAME',
          'ELEM_LDIV',
          'NAME',
        ]),
      );
      // `.'` is the non-conjugate transpose.
      expect(types("A.'\n"), equals(['NAME', 'ELEM_TRANSPOSE']));
    });

    test('number dot star is elementwise not a trailing dot', () {
      // `3.*4` must be 3 .* 4, never `3.` `*` `4`.
      expect(types('3.*4\n'), equals(['NUMBER', 'ELEM_MUL', 'NUMBER']));
    });

    test('matrix operators and backslash', () {
      expect(
        types('A * B / C \\ D ^ 2\n'),
        equals([
          'NAME',
          'STAR',
          'NAME',
          'SLASH',
          'NAME',
          'BACKSLASH',
          'NAME',
          'CARET',
          'NUMBER',
        ]),
      );
    });

    test('double quoted escape and no trailing newline', () {
      // The `""` escape inside a double-quoted string is preserved
      // through the pre-pass (the grammar lexer resolves it). And a
      // source with no trailing newline tokenizes fine.
      expect(types('"a""b"\n'), equals(['STRING']));
      expect(types('x'), equals(['NAME']));
      // A `...` continuation at end-of-input is harmless.
      expect(types('1 + ...'), equals(['NUMBER', 'PLUS']));
    });

    test('numbers', () {
      expect(lex('3.5\n')[0], equals(('NUMBER', '3.5')));
      expect(lex('.5\n')[0], equals(('NUMBER', '.5')));
      expect(lex('1e3\n')[0], equals(('NUMBER', '1e3')));
      expect(lex('2.5e-3\n')[0], equals(('NUMBER', '2.5e-3')));
    });

    test('comparison and logical operators', () {
      expect(
        types('a == b ~= c <= d >= e && f || g\n'),
        equals([
          'NAME',
          'EQ_EQ',
          'NAME',
          'NE',
          'NAME',
          'LE',
          'NAME',
          'GE',
          'NAME',
          'AND_AND',
          'NAME',
          'OR_OR',
          'NAME',
        ]),
      );
      expect(types('~x\n'), equals(['TILDE', 'NAME']));
    });

    test('matrix literal and range', () {
      expect(
        types('[1 2; 3 4]\n'),
        equals([
          'LBRACKET',
          'NUMBER',
          'NUMBER',
          'SEMICOLON',
          'NUMBER',
          'NUMBER',
          'RBRACKET',
        ]),
      );
      expect(
        types('1:2:10\n'),
        equals(['NUMBER', 'COLON', 'NUMBER', 'COLON', 'NUMBER']),
      );
    });

    test('keywords and end', () {
      for (final kw in [
        'if',
        'elseif',
        'else',
        'end',
        'for',
        'while',
        'function',
        'switch',
        'return',
      ]) {
        expect(
          tokenizeMatlab('$kw\n')
              .any((t) => t.type == 'KEYWORD' && t.value == kw),
          isTrue,
          reason: 'expected KEYWORD($kw)',
        );
      }
    });

    test('newlines dropped in parens kept in brackets', () {
      final inParens =
          tokenizeMatlab('f(1,\n2)\n').where((t) => t.type == 'NEWLINE').length;
      expect(inParens, equals(1),
          reason: 'paren-interior newline should be dropped');
      // Inside [ ], a newline IS a row separator — it must be kept.
      final inBrackets = tokenizeMatlab('[1 2\n3 4]\n')
          .where((t) => t.type == 'NEWLINE')
          .length;
      expect(inBrackets, equals(2),
          reason: 'bracket-interior newline (row sep) should be kept');
    });

    test('line comment is skipped', () {
      expect(
        types('x = 1 % a comment\n'),
        equals(['NAME', 'EQ', 'NUMBER']),
      );
    });

    test('block comment is stripped', () {
      const src = "x = 1\n%{\nthis is ignored\n'and so is this'\n%}\ny = 2\n";
      expect(
        types(src),
        equals(['NAME', 'EQ', 'NUMBER', 'NAME', 'EQ', 'NUMBER']),
      );
    });

    test('line continuation joins lines', () {
      // `1 + ...` then `2` is the single expression `1 + 2` — no
      // newline between.
      expect(types('1 + ...\n2\n'), equals(['NUMBER', 'PLUS', 'NUMBER']));
    });

    test('unknown character is an error', () {
      expect(tryTokenizeMatlab('x = #\n'), isNull);
    });

    test('handles eof mid string without panic', () {
      // An unterminated char array must not panic the scanner.
      tryTokenizeMatlab("x = 'oops\n");
    });
  });
}
