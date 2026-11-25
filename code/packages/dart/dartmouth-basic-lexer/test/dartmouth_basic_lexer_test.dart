/// Port of the reference `rust/dartmouth-basic-lexer` unit suite
/// (52 tests).
///
/// Like the reference helpers, [pairs] drops EOF only, while
/// [pairsNoNewline] also drops NEWLINE for single-expression tests.
library;

import 'package:coding_adventures_dartmouth_basic_lexer/dartmouth_basic_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

/// (type, value) for every non-EOF token.
List<(String, String)> pairs(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') (t.type, t.value),
    ];

/// (type, value) for every non-EOF, non-NEWLINE token.
List<(String, String)> pairsNoNewline(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF' && t.type != 'NEWLINE') (t.type, t.value),
    ];

void main() {
  group('dartmouth-basic-lexer', () {
    test('let statement', () {
      // `10 LET X = 5` is the "hello world" of BASIC. After the hooks
      // `10` is LINE_NUM and `5` is NUMBER.
      final p = pairs(tokenizeDartmouthBasic('10 LET X = 5\n'));
      expect(p[0], equals(('LINE_NUM', '10')));
      expect(p[1], equals(('KEYWORD', 'LET')));
      expect(p[2], equals(('NAME', 'X')));
      expect(p[3].$1, equals('EQ'));
      expect(p[4], equals(('NUMBER', '5')));
      expect(p[5].$1, equals('NEWLINE'));
    });

    test('print comma', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('20 PRINT X, Y\n'));
      expect(p[0], equals(('LINE_NUM', '20')));
      expect(p[1], equals(('KEYWORD', 'PRINT')));
      expect(p[2], equals(('NAME', 'X')));
      expect(p[3].$1, equals('COMMA'));
      expect(p[4], equals(('NAME', 'Y')));
    });

    test('goto target is number', () {
      // The key test for the relabel hook: `30` at line start is
      // LINE_NUM, the `10` target is NUMBER.
      final p = pairsNoNewline(tokenizeDartmouthBasic('30 GOTO 10\n'));
      expect(p[0], equals(('LINE_NUM', '30')));
      expect(p[1], equals(('KEYWORD', 'GOTO')));
      expect(
        p[2],
        equals(('NUMBER', '10')),
        reason: 'GOTO target must be NUMBER, not LINE_NUM',
      );
    });

    test('if then', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('40 IF X > 0 THEN 100\n'));
      expect(p[0], equals(('LINE_NUM', '40')));
      expect(p[1], equals(('KEYWORD', 'IF')));
      expect(p[2], equals(('NAME', 'X')));
      expect(p[3].$1, equals('GT'));
      expect(p[4], equals(('NUMBER', '0')));
      expect(p[5], equals(('KEYWORD', 'THEN')));
      expect(p[6], equals(('NUMBER', '100')));
    });

    test('for loop', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('50 FOR I = 1 TO 10 STEP 2\n'));
      expect(p[0], equals(('LINE_NUM', '50')));
      expect(p[1], equals(('KEYWORD', 'FOR')));
      expect(p[2], equals(('NAME', 'I')));
      expect(p[3].$1, equals('EQ'));
      expect(p[4], equals(('NUMBER', '1')));
      expect(p[5], equals(('KEYWORD', 'TO')));
      expect(p[6], equals(('NUMBER', '10')));
      expect(p[7], equals(('KEYWORD', 'STEP')));
      expect(p[8], equals(('NUMBER', '2')));
    });

    test('def user function', () {
      // USER_FN is "FN" followed by exactly one letter.
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('60 DEF FNA(X) = X * X\n'));
      expect(p[0], equals(('LINE_NUM', '60')));
      expect(p[1], equals(('KEYWORD', 'DEF')));
      expect(p[2], equals(('USER_FN', 'FNA')));
      expect(p[3].$1, equals('LPAREN'));
      expect(p[4], equals(('NAME', 'X')));
      expect(p[5].$1, equals('RPAREN'));
      expect(p[6].$1, equals('EQ'));
      expect(p[7], equals(('NAME', 'X')));
      expect(p[8].$1, equals('STAR'));
      expect(p[9], equals(('NAME', 'X')));
    });

    test('builtin functions sin cos', () {
      final p = pairsNoNewline(
          tokenizeDartmouthBasic('70 LET Y = SIN(X) + COS(X)\n'));
      expect(p[0], equals(('LINE_NUM', '70')));
      expect(p[1], equals(('KEYWORD', 'LET')));
      expect(p[2], equals(('NAME', 'Y')));
      expect(p[3].$1, equals('EQ'));
      expect(p[4], equals(('BUILTIN_FN', 'SIN')));
      expect(p[5].$1, equals('LPAREN'));
      expect(p[6], equals(('NAME', 'X')));
      expect(p[7].$1, equals('RPAREN'));
      expect(p[8].$1, equals('PLUS'));
      expect(p[9], equals(('BUILTIN_FN', 'COS')));
      expect(p[10].$1, equals('LPAREN'));
      expect(p[11], equals(('NAME', 'X')));
      expect(p[12].$1, equals('RPAREN'));
    });

    test('case insensitive print', () {
      // `10 print x` tokenizes like `10 PRINT X`: types match, and
      // keywords come out uppercased.
      final pLower = pairsNoNewline(tokenizeDartmouthBasic('10 print x\n'));
      final pUpper = pairsNoNewline(tokenizeDartmouthBasic('10 PRINT X\n'));
      expect(pLower.length, equals(pUpper.length));
      for (var i = 0; i < pLower.length; i++) {
        expect(pLower[i].$1, equals(pUpper[i].$1));
      }
      expect(pLower[1], equals(('KEYWORD', 'PRINT')));
    });

    test('case insensitive let', () {
      // `20 Let A = 1` — mixed-case keyword.
      final p = pairsNoNewline(tokenizeDartmouthBasic('20 Let A = 1\n'));
      expect(p[1], equals(('KEYWORD', 'LET')));
      expect(p[2], equals(('NAME', 'A')));
    });

    test('case insensitive goto', () {
      // `30 goto 20` — lowercase keyword and number.
      final p = pairsNoNewline(tokenizeDartmouthBasic('30 goto 20\n'));
      expect(p[1], equals(('KEYWORD', 'GOTO')));
      expect(p[2], equals(('NUMBER', '20')));
    });

    test('operator le not lt eq', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('10 IF X <= Y THEN 50\n'));
      expect(p[3], equals(('LE', '<=')));
      expect(p.length, equals(7));
    });

    test('operator ge not gt eq', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('10 IF X >= Y THEN 50\n'));
      expect(p[3], equals(('GE', '>=')));
      expect(p.length, equals(7));
    });

    test('operator ne not lt gt', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('10 IF X <> Y THEN 50\n'));
      expect(p[3], equals(('NE', '<>')));
      expect(p.length, equals(7));
    });

    test('number integer', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 42\n'));
      expect(p[4], equals(('NUMBER', '42')));
    });

    test('number decimal', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 3.14\n'));
      expect(p[4], equals(('NUMBER', '3.14')));
    });

    test('number leading dot', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = .5\n'));
      expect(p[4], equals(('NUMBER', '.5')));
    });

    test('number scientific', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 1.5E3\n'));
      expect(p[4], equals(('NUMBER', '1.5E3')));
    });

    test('number negative exponent', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 1.5E-3\n'));
      expect(p[4], equals(('NUMBER', '1.5E-3')));
    });

    test('number integer exponent', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 1E10\n'));
      expect(p[4], equals(('NUMBER', '1E10')));
    });

    test('string literal', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('10 PRINT "HELLO WORLD"\n'));
      expect(p[0], equals(('LINE_NUM', '10')));
      expect(p[1], equals(('KEYWORD', 'PRINT')));
      expect(p[2].$1, equals('STRING'));
      expect(p[2].$2, contains('HELLO WORLD'));
    });

    test('empty string literal', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 PRINT ""\n'));
      expect(p[2].$1, equals('STRING'));
    });

    test('rem comment suppressed', () {
      // LINE_NUM("10"), KEYWORD("REM"), NEWLINE — the words THIS, IS,
      // A, COMMENT are all gone.
      final p = pairs(tokenizeDartmouthBasic('10 REM THIS IS A COMMENT\n'));
      expect(p.length, equals(3));
      expect(p[0], equals(('LINE_NUM', '10')));
      expect(p[1], equals(('KEYWORD', 'REM')));
      expect(p[2].$1, equals('NEWLINE'));
    });

    test('rem then let', () {
      final p = pairs(tokenizeDartmouthBasic('10 REM\n20 LET X = 1\n'));
      expect(p[0], equals(('LINE_NUM', '10')));
      expect(p[1], equals(('KEYWORD', 'REM')));
      expect(p[2].$1, equals('NEWLINE'));
      expect(p[3], equals(('LINE_NUM', '20')));
      expect(p[4], equals(('KEYWORD', 'LET')));
      expect(p[5], equals(('NAME', 'X')));
      expect(p[7], equals(('NUMBER', '1')));
    });

    test('multi line program', () {
      final p =
          pairs(tokenizeDartmouthBasic('10 LET X = 1\n20 PRINT X\n30 END\n'));
      expect(p[0], equals(('LINE_NUM', '10')));
      expect(p[1], equals(('KEYWORD', 'LET')));
      expect(p[4], equals(('NUMBER', '1')));
      expect(p[6], equals(('LINE_NUM', '20')));
      expect(p[7], equals(('KEYWORD', 'PRINT')));
      expect(p[10], equals(('LINE_NUM', '30')));
      expect(p[11], equals(('KEYWORD', 'END')));
    });

    test('variable single letter', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 1\n'));
      expect(p[2], equals(('NAME', 'X')));
    });

    test('variable letter digit', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET A1 = 2\n'));
      expect(p[2], equals(('NAME', 'A1')));
    });

    test('variable string suffix', () {
      final p = pairsNoNewline(
          tokenizeDartmouthBasic('10 LET A\$ = "HI"\n20 PRINT A\$\n'));
      expect(p[2], equals(('NAME', 'A\$')));
      expect(
        p.where((t) => t.$1 == 'NAME' && t.$2 == 'A\$').length,
        equals(2),
      );
    });

    test('variable z9', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET Z9 = 3\n'));
      expect(p[2], equals(('NAME', 'Z9')));
    });

    test('print semicolon', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 PRINT X; Y\n'));
      expect(p[3].$1, equals('SEMICOLON'));
    });

    test('print comma is comma', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 PRINT X, Y\n'));
      expect(p[3].$1, equals('COMMA'));
    });

    test('arithmetic operators', () {
      final p = pairsNoNewline(
          tokenizeDartmouthBasic('10 LET X = A + B - C * D / E ^ F\n'));
      final typeNames = [for (final t in p) t.$1];
      expect(typeNames, contains('PLUS'));
      expect(typeNames, contains('MINUS'));
      expect(typeNames, contains('STAR'));
      expect(typeNames, contains('SLASH'));
      expect(typeNames, contains('CARET'));
    });

    test('all keywords', () {
      // The 20 keywords from the 1964 Dartmouth BASIC spec.
      for (final kw in [
        'LET',
        'PRINT',
        'INPUT',
        'IF',
        'THEN',
        'GOTO',
        'GOSUB',
        'RETURN',
        'FOR',
        'TO',
        'STEP',
        'NEXT',
        'END',
        'STOP',
        'REM',
        'READ',
        'DATA',
        'RESTORE',
        'DIM',
        'DEF',
      ]) {
        // REM would suppress subsequent tokens, so it gets a bare line.
        final src = kw == 'REM' ? '10 $kw\n' : '10 $kw X\n';
        final keywordTokens = tokenizeDartmouthBasic(src)
            .where((t) => t.type == 'KEYWORD')
            .toList();
        expect(
          keywordTokens.any((t) => t.value == kw),
          isTrue,
          reason: 'Expected KEYWORD token for $kw',
        );
      }
    });

    test('all builtin functions', () {
      for (final func in [
        'SIN',
        'COS',
        'TAN',
        'ATN',
        'EXP',
        'LOG',
        'ABS',
        'SQR',
        'INT',
        'RND',
        'SGN',
      ]) {
        final p =
            pairsNoNewline(tokenizeDartmouthBasic('10 LET X = $func(X)\n'));
        expect(
          p.any((t) => t.$1 == 'BUILTIN_FN' && t.$2 == func),
          isTrue,
          reason: 'Expected BUILTIN_FN($func)',
        );
      }
    });

    test('user function fna', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = FNA(Y)\n'));
      expect(
        p.any((t) => t.$1 == 'USER_FN' && t.$2 == 'FNA'),
        isTrue,
      );
    });

    test('user function fnz', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = FNZ(Y)\n'));
      expect(
        p.any((t) => t.$1 == 'USER_FN' && t.$2 == 'FNZ'),
        isTrue,
      );
    });

    test('error recovery unknown char', () {
      // `@` is not a valid BASIC character; like the reference
      // engine (the grammar's `errors:` section is dormant in both
      // lanes), the lexer throws rather than emitting UNKNOWN.
      expect(
        () => createDartmouthBasicLexer('10 LET @ = 1\n').tokenize(),
        throwsA(isA<LexerError>()),
      );
    });

    test('eq token in let', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 5\n'));
      expect(p[3].$1, equals('EQ'));
    });

    test('eq token in if', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('10 IF X = 5 THEN 100\n'));
      expect(p.any((t) => t.$1 == 'EQ'), isTrue);
    });

    test('gosub return', () {
      final p =
          pairsNoNewline(tokenizeDartmouthBasic('10 GOSUB 500\n20 RETURN\n'));
      expect(p[1], equals(('KEYWORD', 'GOSUB')));
      expect(p[2], equals(('NUMBER', '500')));
      expect(p[4], equals(('KEYWORD', 'RETURN')));
    });

    test('stop and end keywords', () {
      final pStop = pairsNoNewline(tokenizeDartmouthBasic('10 STOP\n'));
      expect(pStop[1], equals(('KEYWORD', 'STOP')));
      final pEnd = pairsNoNewline(tokenizeDartmouthBasic('10 END\n'));
      expect(pEnd[1], equals(('KEYWORD', 'END')));
    });

    test('data read restore', () {
      final p = pairsNoNewline(
          tokenizeDartmouthBasic('10 DATA 1, 2, 3\n20 READ X\n30 RESTORE\n'));
      expect(
        p.any((t) => t.$1 == 'KEYWORD' && t.$2 == 'DATA'),
        isTrue,
      );
      expect(
        p.any((t) => t.$1 == 'KEYWORD' && t.$2 == 'READ'),
        isTrue,
      );
      expect(
        p.any((t) => t.$1 == 'KEYWORD' && t.$2 == 'RESTORE'),
        isTrue,
      );
    });

    test('dim keyword', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 DIM A(10)\n'));
      expect(p[1], equals(('KEYWORD', 'DIM')));
      expect(p[2], equals(('NAME', 'A')));
      expect(p[3].$1, equals('LPAREN'));
      expect(p[4], equals(('NUMBER', '10')));
      expect(p[5].$1, equals('RPAREN'));
    });

    test('next keyword', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 NEXT I\n'));
      expect(p[1], equals(('KEYWORD', 'NEXT')));
      expect(p[2], equals(('NAME', 'I')));
    });

    test('input keyword', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 INPUT X\n'));
      expect(p[1], equals(('KEYWORD', 'INPUT')));
      expect(p[2], equals(('NAME', 'X')));
    });

    test('whitespace ignored', () {
      final pCompact = pairs(tokenizeDartmouthBasic('10 LET X=5\n'));
      final pSpaced = pairs(tokenizeDartmouthBasic('10  LET  X  =  5\n'));
      final pTabbed = pairs(tokenizeDartmouthBasic('10\tLET\tX\t=\t5\n'));
      expect(pCompact.length, equals(pSpaced.length));
      expect(pCompact.length, equals(pTabbed.length));
      for (var i = 0; i < pCompact.length; i++) {
        expect(pCompact[i].$1, equals(pSpaced[i].$1));
        expect(pCompact[i].$2, equals(pSpaced[i].$2));
      }
    });

    test('create lexer factory', () {
      final tokens = createDartmouthBasicLexer('10 LET X = 42\n').tokenize();
      expect(tokens.last.type, equals('EOF'));
      expect(
        tokens.any((t) => t.type == 'LINE_NUM' && t.value == '10'),
        isTrue,
      );
    });

    test('newline tokens present', () {
      final newlineCount = tokenizeDartmouthBasic('10 LET X = 1\n20 PRINT X\n')
          .where((t) => t.type == 'NEWLINE')
          .length;
      expect(newlineCount, equals(2));
    });

    test('eof always present', () {
      expect(
        tokenizeDartmouthBasic('10 END\n').last.type,
        equals('EOF'),
      );
    });

    test('lt gt individual', () {
      final pLt =
          pairsNoNewline(tokenizeDartmouthBasic('10 IF X < Y THEN 50\n'));
      expect(pLt[3].$1, equals('LT'));
      final pGt =
          pairsNoNewline(tokenizeDartmouthBasic('10 IF X > Y THEN 50\n'));
      expect(pGt[3].$1, equals('GT'));
    });

    test('caret exponentiation', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('10 LET X = 2 ^ 3\n'));
      expect(
        p.any((t) => t.$1 == 'CARET' && t.$2 == '^'),
        isTrue,
      );
    });

    test('line number zero', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('0 END\n'));
      expect(p[0], equals(('LINE_NUM', '0')));
    });

    test('large line number', () {
      final p = pairsNoNewline(tokenizeDartmouthBasic('9999 END\n'));
      expect(p[0], equals(('LINE_NUM', '9999')));
    });

    test('rem at eof without newline drops eof', () {
      // Suppression swallows the trailing EOF too — proven identical
      // against the Rust reference (which yields the same streams).
      List<String> types(String src) =>
          [for (final t in tokenizeDartmouthBasic(src)) t.type];
      expect(types('10 REM foo'), equals(['LINE_NUM', 'KEYWORD']));
      expect(
        types('10 REM foo\n'),
        equals(['LINE_NUM', 'KEYWORD', 'NEWLINE', 'EOF']),
      );
      expect(
        types('10 LET X = 1\n20 REM bar'),
        equals([
          'LINE_NUM',
          'KEYWORD',
          'NAME',
          'EQ',
          'NUMBER',
          'NEWLINE',
          'LINE_NUM',
          'KEYWORD'
        ]),
      );
    });
  });
}
