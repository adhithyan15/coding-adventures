/// Unit tests for `coding_adventures_r_lexer`: all 12 reference
/// tests, plus the corners the reference leaves uncovered — hook edge
/// cases (unbalanced closers, bracket variants), error positions, the
/// nullable API shape, and empty input.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_r_lexer/r_lexer.dart';
import 'package:test/test.dart';

List<(String, String)> lex(String source) => [
      for (final t in tokenizeR(source))
        if (t.type != 'EOF' && t.type != 'NEWLINE') (t.type, t.value)
    ];

int newlineCount(String source) =>
    tokenizeR(source).where((t) => t.type == 'NEWLINE').length;

void main() {
  group('reference', () {
    test('underscore is part of a name', () {
      final p = lex('data_frame <- 1\n');
      expect(p[0], ('NAME', 'data_frame'));
      expect(p[1], ('ASSIGN', '<-'));
      expect(p[2], ('NUMBER', '1'));
    });

    test('no underscore assignment token', () {
      final p = lex('a_b\n');
      expect(p, hasLength(1));
      expect(p[0], ('NAME', 'a_b'));
    });

    test('typed NA constants are keywords', () {
      for (final kw in ['NA_integer_', 'NA_real_', 'NA_character_']) {
        final toks = tokenizeR('$kw\n');
        expect(toks.any((t) => t.type == 'KEYWORD' && t.value == kw), isTrue,
            reason: 'expected KEYWORD($kw)');
      }
    });

    test('all assignment operators', () {
      expect(lex('x <- 1\n')[1], ('ASSIGN', '<-'));
      expect(lex('x <<- 1\n')[1], ('SUPER_ASSIGN', '<<-'));
      expect(lex('1 -> x\n')[1], ('RIGHT_ASSIGN', '->'));
      expect(lex('1 ->> x\n')[1], ('RIGHT_SUPER_ASSIGN', '->>'));
      expect(lex('x = 1\n').any((t) => t.$1 == 'EQ' && t.$2 == '='), isTrue);
    });

    test('operators numbers strings keywords', () {
      final p = lex('a + b * c ^ d : e %in% f\n');
      final names = [for (final (t, _) in p) t];
      for (final op in ['PLUS', 'STAR', 'CARET', 'COLON', 'PERCENT_OP']) {
        expect(names, contains(op));
      }
      expect(
          tokenizeR('if (x) 1 else 2\n')
              .any((t) => t.type == 'KEYWORD' && t.value == 'if'),
          isTrue);
      expect(
          tokenizeR('TRUE\n')
              .any((t) => t.type == 'KEYWORD' && t.value == 'TRUE'),
          isTrue);
      expect(lex('"hi"\n')[0].$1, 'STRING');
      expect(lex("'hi'\n")[0].$1, 'STRING');
      expect(lex('3.14\n')[0], ('NUMBER', '3.14'));
    });

    test('dollar and brackets', () {
      final p = lex('df\$col[1]\n');
      final names = [for (final (t, _) in p) t];
      for (final t in ['DOLLAR', 'LBRACKET', 'RBRACKET']) {
        expect(names, contains(t));
      }
    });

    test('parens drop newlines, braces keep them', () {
      expect(newlineCount('sum(1,\n2)\n'), 1);
      expect(newlineCount('{\nx <- 1\ny <- 2\n}\n'), greaterThanOrEqualTo(3));
    });

    test('dotted names and dots', () {
      expect(lex('data.frame\n')[0], ('NAME', 'data.frame'));
      expect(lex('.GlobalEnv\n')[0], ('NAME', '.GlobalEnv'));
    });

    test('unknown char is an error', () {
      expect(tryTokenizeR('x <- @\n'), isNull);
    });

    test('integer hex and complex literals', () {
      expect(lex('10L\n')[0], ('INT_LIT', '10L'));
      expect(lex('0xFF\n')[0], ('HEX_LIT', '0xFF'));
      expect(lex('0x1FL\n')[0], ('HEX_LIT', '0x1FL'));
      expect(lex('2.5i\n')[0], ('COMPLEX_LIT', '2.5i'));
      expect(lex('1e3L\n')[0], ('INT_LIT', '1e3L'));
    });

    test('plain numbers and names unaffected', () {
      expect(lex('10\n')[0], ('NUMBER', '10'));
      expect(lex('1e3\n')[0], ('NUMBER', '1e3'));
      expect(lex('Length\n')[0], ('NAME', 'Length'));
      expect(lex('i\n')[0], ('NAME', 'i'));
      expect(lex('0x1FL\n'), hasLength(1));
    });

    test('pipe operator and backslash lambda', () {
      final p = lex('x |> f()\n');
      expect(p[0], ('NAME', 'x'));
      expect(p[1], ('PIPE_OP', '|>'));
      final l = lex('\\(x) x\n');
      expect(l[0], ('BACKSLASH', '\\'));
      expect(l[1], ('LPAREN', '('));
    });
  });

  group('extras', () {
    test('hook drops bracket-interior newlines too', () {
      expect(newlineCount('a[1,\n2]\n'), 1);
    });

    test('unbalanced closers never unbalance the hook', () {
      // Stray `)` drives depth negative-ward, but the `> 0` gate only
      // drops inside real opens — counts proven identical in Rust via
      // differential probe.
      expect(newlineCount(')\n)\n'), 2);
      expect(newlineCount('(\n)\n)\n'), 2);
    });

    test('hook preserves positions and EOF', () {
      final tokens = tokenizeR('sum(1,\n2)\n');
      expect(tokens.last.type, 'EOF');
      expect(tokens.last.line, 3);
    });

    test('errors carry positions', () {
      try {
        tokenizeR('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeR('x <- 1\n'), isNotNull);
      expect(() => tokenizeR('x <- @\n'), throwsA(isA<LexerError>()));
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeR('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createRLexer('x <- 1\n').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createRLexer('@').tokenize(), throwsA(isA<LexerError>()));
    });
  });
}
