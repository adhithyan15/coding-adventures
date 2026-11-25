/// Port of the reference `rust/scilab-lexer` unit suite (35 tests).
///
/// Like the reference `lex` helper, [lex] drops both EOF and NEWLINE —
/// engine-level names (`EQ`, `KEYWORD`, `NE_ALT`, ...) are asserted, not
/// per-keyword spellings: keyword promotion is generic in both engines.
library;

import 'package:test/test.dart';

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_scilab_lexer/scilab_lexer.dart';

/// Non-EOF, non-NEWLINE tokens of [source].
List<Token> lex(String source) => tokenizeScilab(source)
    .where((t) => t.type != 'EOF' && t.type != 'NEWLINE')
    .toList();

/// The non-EOF, non-NEWLINE token types of [source].
List<String> types(String source) => lex(source).map((t) => t.type).toList();

void main() {
  group('scilab-lexer', () {
    test('transpose after a value', () {
      final p = lex("A'\n");
      expect(p[1].type, equals('TRANSPOSE'));
      expect(p[1].value, equals("'"));
      expect(
        types("(A)'\n"),
        equals(['LPAREN', 'NAME', 'RPAREN', 'TRANSPOSE']),
      );
      expect(types("A''\n"), equals(['NAME', 'TRANSPOSE', 'TRANSPOSE']));
    });

    test('string when not after a value', () {
      final p0 = lex("'abc'\n");
      expect(p0[0].type, equals('STRING'));
      expect(p0[0].value, equals('abc'));
      final p1 = lex("x = 'hi'\n");
      expect(p1[2].type, equals('STRING'));
      expect(p1[2].value, equals('hi'));
      // The classic trap: `A' * B'` is two transposes, NOT a string.
      expect(
        types("A' * B'\n"),
        equals(['NAME', 'TRANSPOSE', 'STAR', 'NAME', 'TRANSPOSE']),
      );
    });

    test('whitespace resets to string inside brackets', () {
      expect(
        types("[1 'a']\n"),
        equals(['LBRACKET', 'NUMBER', 'STRING', 'RBRACKET']),
      );
    });

    test('doubled quote is an escaped apostrophe', () {
      final p = lex("x = 'it''s'\n");
      expect(p[2].type, equals('STRING'));
      expect(p[2].value, equals("it's"));
    });

    test('last index dollar is a value before transpose', () {
      // `$` denotes the last index (MA10 §1 finding 5) -- a value, so a
      // following `'` with no intervening whitespace is a transpose.
      expect(
        types("A(\$)'\n"),
        equals(['NAME', 'LPAREN', 'DOLLAR', 'RPAREN', 'TRANSPOSE']),
      );
    });

    test('single and double quoted strings are both string type', () {
      final p0 = lex('"hello"\n');
      expect(p0[0].type, equals('STRING'));
      expect(p0[0].value, equals('hello'));
      final p1 = lex("'hello'\n");
      expect(p1[0].type, equals('STRING'));
      expect(p1[0].value, equals('hello'));
      // A `'` inside a double-quoted string is just text, not a transpose.
      final p2 = lex('"it\'s"\n');
      expect(p2[0].type, equals('STRING'));
      expect(p2[0].value, equals("it's"));
    });

    test('double quoted string escape', () {
      // The `""` escape inside a double-quoted string.
      expect(types('"a""b"\n'), equals(['STRING']));
      final p = lex('"a""b"\n');
      expect(p[0].type, equals('STRING'));
      expect(p[0].value, equals('a"b'));
    });

    test('double-quoted string resembling a placeholder is not restored', () {
      // Regression: a crafted double-quoted string whose content looks like
      // the internal `` `N` `` placeholder must survive verbatim --
      // restoration keys off the distinct STRING_PLACEHOLDER type, not the
      // value. (An earlier real string is present so index 0 exists.)
      final p = lex('a = \'x\'; b = "`0`"\n');
      final lastString = p.reversed.firstWhere((t) => t.type == 'STRING');
      expect(lastString.value, equals('`0`'));
    });

    test('bare placeholder-shaped text is not hijacked', () {
      // Security regression: a literal `` `0` `` typed as ordinary
      // (otherwise invalid) source text must NOT resolve against an
      // unrelated real string literal's table entry just because its
      // digits happen to match that entry's index.
      final p = lex("x = 'HELLO'\ny = `0`\n");
      final placeholders =
          p.reversed.where((t) => t.type == 'STRING_PLACEHOLDER').toList();
      expect(placeholders, isNotEmpty,
          reason: 'the bare `0` on line 2 must not resolve to STRING');
      expect(placeholders.first.value, equals('`0`'),
          reason: 'value must stay the raw literal text, not HELLO');
    });

    test('out of range placeholder index is not silently accepted', () {
      // Security regression: a `` `N` `` sequence whose index has no
      // corresponding real placeholder stays STRING_PLACEHOLDER, an
      // honest failure for a future parser to reject.
      expect(
        types("z = `42`\n"),
        equals(['NAME', 'EQ', 'STRING_PLACEHOLDER']),
      );
    });

    test('line comment is skipped', () {
      expect(
        types('x = 1 // a comment\n'),
        equals(['NAME', 'EQ', 'NUMBER']),
      );
    });

    test('block comment is skipped', () {
      expect(
        types('x = /* ignored */ 1\n'),
        equals(['NAME', 'EQ', 'NUMBER']),
      );
    });

    test('block comment may span multiple lines', () {
      const src = "x = 1\n/*\nthis is ignored\n'and so is this'\n*/\ny = 2\n";
      expect(
        types(src),
        equals(['NAME', 'EQ', 'NUMBER', 'NAME', 'EQ', 'NUMBER']),
      );
    });

    test('block comment is not required to be alone on its line', () {
      // MA10 §3's key divergence from MATLAB's `%{`/`%}`: a block comment
      // may sit INLINE, sharing a line with real code both before and
      // after it.
      expect(
        types('x = 1 /* inline */ + 2 /* also inline */ * 3\n'),
        equals(['NAME', 'EQ', 'NUMBER', 'PLUS', 'NUMBER', 'STAR', 'NUMBER']),
      );
    });

    test('all eight percent constants lex as one token each', () {
      for (final name in [
        '%pi',
        '%e',
        '%i',
        '%inf',
        '%nan',
        '%eps',
        '%t',
        '%f',
      ]) {
        final p = lex('$name\n');
        expect(p.length, equals(1),
            reason: 'expected exactly one token for $name: $p');
        expect(p[0].type, equals('PERCENT_CONST'));
        expect(p[0].value, equals(name));
      }
    });

    test('percent constants do not swallow a trailing identifier', () {
      // `%inf` must win whole -- not split into `%i` + `nf` -- because the
      // regex alternation is ordered longest-spelling-first.
      expect(types('%inf\n'), equals(['PERCENT_CONST']));
      expect(types('%eps\n'), equals(['PERCENT_CONST']));
    });

    test('percent constant used in an expression', () {
      expect(
        types('r = %pi * 2\n'),
        equals(['NAME', 'EQ', 'PERCENT_CONST', 'STAR', 'NUMBER']),
      );
    });

    test('dollar last index', () {
      expect(
        types('A(\$)\n'),
        equals(['NAME', 'LPAREN', 'DOLLAR', 'RPAREN']),
      );
      expect(
        types('A(\$-1)\n'),
        equals(['NAME', 'LPAREN', 'DOLLAR', 'MINUS', 'NUMBER', 'RPAREN']),
      );
    });

    test('not equal digraph wins over bare lt gt', () {
      expect(types('a <> b\n'), equals(['NAME', 'NE_ALT', 'NAME']));
      expect(types('a < b\n'), equals(['NAME', 'LT', 'NAME']));
      expect(types('a > b\n'), equals(['NAME', 'GT', 'NAME']));
    });

    test('both not equal spellings are valid and distinct', () {
      expect(types('a ~= b\n'), equals(['NAME', 'NE', 'NAME']));
      expect(types('a <> b\n'), equals(['NAME', 'NE_ALT', 'NAME']));
    });

    test('le and ge still win over bare lt gt', () {
      expect(types('a <= b\n'), equals(['NAME', 'LE', 'NAME']));
      expect(types('a >= b\n'), equals(['NAME', 'GE', 'NAME']));
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
      expect(types("A.'\n"), equals(['NAME', 'ELEM_TRANSPOSE']));
    });

    test('number dot star is elementwise not a trailing dot', () {
      expect(types('3.*4\n'), equals(['NUMBER', 'ELEM_MUL', 'NUMBER']));
    });

    test('legacy double star is not a power operator', () {
      // MA10 §4: the deprecated `**` spelling for `^` is deferred by
      // simple omission -- `a ** b` lexes as two bare STAR tokens.
      expect(
        types('a ** b\n'),
        equals(['NAME', 'STAR', 'STAR', 'NAME']),
      );
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

    test('newlines dropped in parens kept in brackets', () {
      final inParens =
          tokenizeScilab('f(1,\n2)\n').where((t) => t.type == 'NEWLINE').length;
      expect(inParens, equals(1),
          reason: 'paren-interior newline should be dropped');
      final inBrackets = tokenizeScilab('[1 2\n3 4]\n')
          .where((t) => t.type == 'NEWLINE')
          .length;
      expect(inBrackets, equals(2),
          reason: 'bracket-interior newline (row sep) should be kept');
    });

    test('keywords promote from name', () {
      for (final kw in [
        'if',
        'elseif',
        'else',
        'end',
        'select',
        'case',
        'then',
        'while',
        'do',
        'for',
        'break',
        'continue',
        'function',
        'endfunction',
      ]) {
        expect(
          tokenizeScilab('$kw\n')
              .any((t) => t.type == 'KEYWORD' && t.value == kw),
          isTrue,
          reason: 'expected KEYWORD($kw)',
        );
      }
    });

    test('endfunction is distinct from generic end', () {
      // MA10 §1 finding 7 / §3: `endfunction` is Scilab's own historically
      // mandated function terminator -- lexed as its OWN keyword value,
      // never conflated with the generic block-closer `end`.
      final p = lex('function y=f(x)\n y = x\nendfunction\n');
      final lastKw = p.reversed.firstWhere((t) => t.type == 'KEYWORD');
      expect(lastKw.value, equals('endfunction'));
      // `end` and `endfunction` are different KEYWORD values, not aliases.
      expect(
        tokenizeScilab('end\n')[0].value,
        isNot(equals(tokenizeScilab('endfunction\n')[0].value)),
      );
    });

    test('switch and otherwise are not keywords in scilab', () {
      // Scilab has no `switch`/`otherwise` at all (its own construct is
      // `select`/`case`/`else`, MA10 §1 finding 4) -- these MUST remain
      // ordinary NAMEs, unlike matlab.tokens which reserves them.
      expect(types('switch\n'), equals(['NAME']));
      expect(types('otherwise\n'), equals(['NAME']));
    });

    test('select case then else end snippet', () {
      final p = lex('select x\n case 1 then y = 1\n else y = 0\n end\n');
      expect(
        p.map((t) => t.type).toList(),
        equals([
          'KEYWORD',
          'NAME',
          'KEYWORD',
          'NUMBER',
          'KEYWORD',
          'NAME',
          'EQ',
          'NUMBER',
          'KEYWORD',
          'NAME',
          'EQ',
          'NUMBER',
          'KEYWORD',
        ]),
      );
    });

    test('at sign is an unrecognized character', () {
      // Neither MATLAB's function-handle meaning nor Scilab's own
      // deprecated legacy `~`-spelling is in scope this cut (MA10 §4) --
      // `@` is simply absent from the grammar.
      expect(tryTokenizeScilab('a = @b\n'), isNull);
    });

    test('unknown character is an error', () {
      expect(tryTokenizeScilab('x = #\n'), isNull);
    });

    test('unrecognized percent word falls through honestly', () {
      // `%xyz` starts with a letter that is not a prefix of any of the
      // closed eight-word vocabulary (MA10 §4), so PERCENT_CONST does not
      // match it at all -- the bare `%` is an unrecognized character (an
      // honest failure, not a silent guess at what `%xyz` might mean).
      expect(tryTokenizeScilab('x = %xyz\n'), isNull);
    });

    test('percent word sharing a prefix still splits', () {
      // `%foo` is NOT one of the eight (only `%f` is real) -- because `f`
      // is a legitimate one-letter constant, `%foo` lexes as
      // PERCENT_CONST("%f") followed by NAME("oo") rather than failing
      // outright. A known, accepted edge case (MA10 §4's closed set is
      // enforced by the pattern, not by extra lookahead) -- rejected at
      // parse time (MA-10c) since two adjacent value tokens with no
      // operator between them is a syntax error in any sane grammar.
      expect(types('%foo\n'), equals(['PERCENT_CONST', 'NAME']));
    });

    test('handles eof mid string without panic', () {
      tryTokenizeScilab("x = 'oops\n");
    });
  });
}
