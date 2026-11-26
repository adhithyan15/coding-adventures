/// Port of the reference `rust/cobol-lexer` unit suite (16 tests).
///
/// Like the reference helpers, [pairs] drops EOF only, and [card]
/// builds a one-line card image so tests read without hand-counting
/// columns (the hook drops the first 7 chars of every line, so test
/// inputs are written as card images).
library;

import 'package:coding_adventures_cobol_lexer/cobol_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

/// (type, value) for every non-EOF token.
List<(String, String)> pairs(String src) => [
      for (final t in tokenizeCobol(src))
        if (t.type != 'EOF') (t.type, t.value),
    ];

List<String> kinds(String src) => [for (final p in pairs(src)) p.$1];

/// A one-line card: 7 leading columns (6 sequence + 1 space
/// indicator) then the code.
String card(String code) => '000000 $code';

void main() {
  group('column strip hook', () {
    test('strip drops sequence and keeps code', () {
      // cols 1–6 = "000100", col 7 = ' ', code begins at col 8.
      expect(
        stripCobolColumns('000100 PROGRAM-ID. PAYROLL.'),
        equals('PROGRAM-ID. PAYROLL.'),
      );
    });

    test('strip drops identification area', () {
      // An 80-col card: 6 seq + 1 indicator + code padded to col 72 +
      // an 8-char identification tail (cols 73–80) that must be dropped.
      const code = 'DISPLAY X.'; // 10 chars of code starting col 8
      final pad = ' ' * (72 - 7 - code.length); // fill up to col 72
      final cardImage = '000100 $code$pad' 'IDENTTAG';
      expect(cardImage.runes.length, equals(80));
      expect(stripCobolColumns(cardImage).trimRight(), equals('DISPLAY X.'));
    });

    test('strip removes comment lines', () {
      // col 7 = '*' → whole line dropped; '/' likewise.
      const src = '000100 DATA DIVISION.\n'
          '000200*THIS IS A COMMENT\n'
          '000300/PAGE EJECT\n'
          '000400 PROCEDURE DIVISION.';
      expect(
        stripCobolColumns(src),
        equals('DATA DIVISION.\nPROCEDURE DIVISION.'),
      );
    });

    test('strip splices continuation lines', () {
      // col 7 = '-' → this line's code appends to the previous line.
      const src = '000100 MOVE FIRST-\n000200-PART TO X.';
      expect(
        stripCobolColumns(src),
        equals('MOVE FIRST-PART TO X.'),
      );
    });
  });

  group('cobol-lexer', () {
    test('division headers are keywords', () {
      expect(
        pairs(card('IDENTIFICATION DIVISION.')),
        equals([
          ('KEYWORD', 'IDENTIFICATION'),
          ('KEYWORD', 'DIVISION'),
          ('DOT', '.'),
        ]),
      );
    });

    test('hyphenated names and reserved words', () {
      // GROSS-PAY is a NAME; WORKING-STORAGE and PROGRAM-ID are reserved.
      expect(
        pairs(card('GROSS-PAY')),
        equals([('NAME', 'GROSS-PAY')]),
      );
      expect(
        pairs(card('WORKING-STORAGE')),
        equals([('KEYWORD', 'WORKING-STORAGE')]),
      );
      expect(
        pairs(card('PROGRAM-ID')),
        equals([('KEYWORD', 'PROGRAM-ID')]),
      );
    });

    test('level numbers lex as number', () {
      // 01, 77, 88 are just NUMBERs here; the parser recognises the
      // level.
      expect(
        pairs(card('01  EMP-NAME')),
        equals([('NUMBER', '01'), ('NAME', 'EMP-NAME')]),
      );
      expect(pairs(card('77'))[0], equals(('NUMBER', '77')));
    });

    test('numeric literal vs terminating dot', () {
      // `3.14` is one NUMBER; the trailing `.` (VALUE 100.) is a
      // separate DOT.
      expect(
        pairs(card('VALUE 3.14.')),
        equals([
          ('KEYWORD', 'VALUE'),
          ('NUMBER', '3.14'),
          ('DOT', '.'),
        ]),
      );
      expect(
        pairs(card('VALUE 100.')),
        equals([
          ('KEYWORD', 'VALUE'),
          ('NUMBER', '100'),
          ('DOT', '.'),
        ]),
      );
    });

    test('string literals both quote styles', () {
      expect(
        kinds(card('DISPLAY "HELLO" \'X\'')),
        equals(['KEYWORD', 'STRING', 'STRING']),
      );
    });

    test('optional separators are skipped', () {
      // COBOL commas/semicolons are optional separators — pure noise.
      expect(
        kinds(card('DISPLAY A, B; C')),
        equals(['KEYWORD', 'NAME', 'NAME', 'NAME']),
      );
    });

    test('picture clause yields one pic string', () {
      expect(
        pairs(card('PICTURE X(20)')),
        equals([
          ('KEYWORD', 'PICTURE'),
          ('PIC_STRING', 'X(20)'),
        ]),
      );
      expect(
        pairs(card('PIC 9(3)V99')),
        equals([
          ('KEYWORD', 'PIC'),
          ('PIC_STRING', '9(3)V99'),
        ]),
      );
    });

    test('picture terminates at entry period', () {
      // The core picture pattern excludes `.`, so `PIC X(20).` splits
      // the picture from the entry-terminating DOT.
      expect(
        pairs(card('PIC X(20).')),
        equals([
          ('KEYWORD', 'PIC'),
          ('PIC_STRING', 'X(20)'),
          ('DOT', '.'),
        ]),
      );
    });

    test('mode returns to default after picture', () {
      // After the picture, `VALUE ZERO` must lex as ordinary keywords
      // — proof the lexer left `picture` mode.
      expect(
        pairs(card('PIC S9(4)V99 VALUE ZERO.')),
        equals([
          ('KEYWORD', 'PIC'),
          ('PIC_STRING', 'S9(4)V99'),
          ('KEYWORD', 'VALUE'),
          ('KEYWORD', 'ZERO'),
          ('DOT', '.'),
        ]),
      );
    });

    test('a bare x outside picture is a name', () {
      // Critically: `X` is only a picture symbol *after* PIC. In a
      // MOVE it is an ordinary data name.
      expect(
        pairs(card('MOVE ZERO TO X.')),
        equals([
          ('KEYWORD', 'MOVE'),
          ('KEYWORD', 'ZERO'),
          ('KEYWORD', 'TO'),
          ('NAME', 'X'),
          ('DOT', '.'),
        ]),
      );
    });

    test('full carded program tokenizes', () {
      // A complete four-division program as 80-column card images,
      // including a comment line, level entries with PICTUREs, and a
      // PROCEDURE paragraph.
      const src = '000100 IDENTIFICATION DIVISION.\n'
          '000200 PROGRAM-ID. PAYROLL.\n'
          "000300*COMPUTE ONE EMPLOYEE'S PAY\n"
          '000400 DATA DIVISION.\n'
          '000500 WORKING-STORAGE SECTION.\n'
          '000600 01  EMP-NAME     PICTURE X(20).\n'
          '000700 77  GROSS-PAY    PIC 9(6)V99 VALUE ZERO.\n'
          '000800 PROCEDURE DIVISION.\n'
          '000900 MAIN-PARAGRAPH.\n'
          '001000     MOVE ZERO TO GROSS-PAY.\n'
          '001100     DISPLAY EMP-NAME GROSS-PAY.\n'
          '001200     STOP RUN.';
      final toks = tokenizeCobol(src);

      // Ends in exactly one EOF.
      expect(toks.last.type, equals('EOF'));
      expect(toks.where((t) => t.type == 'EOF').length, equals(1));

      // The comment line was dropped: no token has value "COMPUTE".
      expect(toks.any((t) => t.value == 'COMPUTE'), isFalse,
          reason: 'comment line leaked');

      // Two PICTURE clauses → two PIC_STRING tokens, with the expected
      // values.
      final pics = [
        for (final t in toks)
          if (t.type == 'PIC_STRING') t.value,
      ];
      expect(pics, equals(['X(20)', '9(6)V99']));

      // All four division headers are present as keywords.
      for (final div in ['IDENTIFICATION', 'DATA', 'PROCEDURE']) {
        expect(
          toks.any((t) => t.type == 'KEYWORD' && t.value == div),
          isTrue,
          reason: 'missing division keyword $div',
        );
      }
      // ENVIRONMENT is optional and omitted here — confirm we didn't
      // invent it.
      expect(toks.any((t) => t.value == 'ENVIRONMENT'), isFalse);
    });

    test('unknown character is an error', () {
      // `@` is not a COBOL character (in the code area).
      expect(tryTokenizeCobol(card('MOVE @ TO X.')), isNull);
    });
  });

  group('extras', () {
    test('short lines carry no code', () {
      // Fewer than 7 chars: no indicator column, no code — an empty
      // line that tokenizes to just EOF.
      expect(stripCobolColumns('abc'), equals(''));
      expect(tokenizeCobol('abc').map((t) => t.type), equals(['EOF']));
    });

    test('continuation without predecessor keeps code', () {
      // A `-` line with nothing before it falls through and keeps its
      // code as-is.
      expect(
        stripCobolColumns('000100-PART TO X.'),
        equals('PART TO X.'),
      );
    });

    test('crlf cards strip cleanly', () {
      expect(
        stripCobolColumns('000100 MOVE X.\r\n000200 DISPLAY Y.\r\n'),
        equals('MOVE X.\nDISPLAY Y.\n'),
      );
    });

    test('errors carry positions', () {
      expect(
        () => tokenizeCobol(card('MOVE @ TO X.')),
        throwsA(isA<LexerError>()),
      );
      try {
        tokenizeCobol(card('AB@CD'));
        fail('expected LexerError');
      } on LexerError catch (e) {
        // Positions refer to the STRIPPED source: `AB@CD` starts at
        // column 1, so `@` is at column 3.
        expect((e.line, e.column), equals((1, 3)));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeCobol('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, equals('EOF'));
    });

    test('nullable twin agrees with the throwing API', () {
      final hard = tokenizeCobol(card('PIC X(20).'));
      expect(tryTokenizeCobol(card('PIC X(20).')), equals(hard));
    });
  });
}
