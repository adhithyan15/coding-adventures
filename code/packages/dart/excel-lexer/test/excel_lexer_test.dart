import 'dart:math' as math;

import 'package:coding_adventures_excel_lexer/coding_adventures_excel_lexer.dart';
import 'package:test/test.dart';

List<String> _types(String source) =>
    tokenizeExcel(source).map((t) => t.type).toList();

void main() {
  group('5-Class Excel Lexer Security Resilience Showcase', () {
    test('Class 1: CWE-400 Source Length DoS Defense', () {
      final atLimit = ' ' * maxSourceLength;
      expect(() => tokenizeExcel(atLimit), returnsNormally);

      final overLimit = ' ' * (maxSourceLength + 1);
      expect(
        () => tokenizeExcel(overLimit),
        throwsA(isA<InvalidExcelSourceException>().having(
          (e) => e.message,
          'message',
          messageSourceTooLarge,
        )),
      );
      expect(() => createExcelLexer(overLimit),
          throwsA(isA<InvalidExcelSourceException>()));
    });

    test('Class 2: Surrogate Smuggling & Encoding Attacks', () {
      final loneLead = String.fromCharCode(0xd800);
      final loneTrail = String.fromCharCode(0xdc00);
      final reversedPair = String.fromCharCodes([0xdc00, 0xd800]);
      final midSurrogate = '=SUM(A1,$loneLead)';

      for (final invalid in [loneLead, loneTrail, reversedPair, midSurrogate]) {
        expect(
          () => tokenizeExcel(invalid),
          throwsA(isA<InvalidExcelSourceException>().having(
            (e) => e.message,
            'message',
            messageInvalidSurrogates,
          )),
        );
      }
    });

    test('Class 3: Context-Sensitive Name Disambiguation', () {
      final call = tokenizeExcel('SUM(A1)');
      expect(call.first.type, equals('FUNCTION_NAME'));
      expect(call.first.value, equals('SUM'));

      // ASCII spaces between the name and `(` still promote to a function.
      expect(_types('SUM  (A1)').first, equals('FUNCTION_NAME'));

      // Structured table references promote to TABLE_NAME.
      final table = tokenizeExcel('DeptSales[Sales Amount]');
      expect(table.first.type, equals('TABLE_NAME'));
      expect(table[1].type, equals('STRUCTURED_COLUMN'));
      expect(_types('DeptSales [Sales Amount]').first, equals('TABLE_NAME'));

      // A bare name stays a NAME, including when separated by a tab/newline
      // (skipped by the grammar, so it breaks contiguity like a char peek).
      expect(_types('Rate').first, equals('NAME'));
      expect(_types('Rate\t(1)').first, equals('NAME'));
      expect(_types('Rate\n(1)').first, equals('NAME'));
      expect(_types('Rate + (1)').first, equals('NAME'));
      expect(_types('Rate ').first, equals('NAME'));
    });

    test('Class 4: Literal & Reference Token Integrity', () {
      const source = r'''=IF($A$1<>"x""y",#REF!,TRUE)&'My Sheet'!B2:C3''';
      final tokens = tokenizeExcel(source);
      final types = tokens.map((t) => t.type).toSet();

      expect(
        types,
        containsAll(<String>[
          'EQUALS',
          'FUNCTION_NAME',
          'LPAREN',
          'CELL',
          'NOT_EQUALS',
          'STRING',
          'COMMA',
          'ERROR_CONSTANT',
          'KEYWORD',
          'RPAREN',
          'AMP',
          'REF_PREFIX',
          'COLON',
        ]),
      );
      expect(tokens.last.type, equals('EOF'));

      // The sign of `1+2` must never be swallowed into the NUMBER token.
      final sum = tokenizeExcel('1+2');
      expect(sum.map((t) => t.value).toList(), equals(['1', '+', '2', '']));
      expect(tokenizeExcel('1.5E+3').first.value, equals('1.5E+3'));
    });

    test('Class 5: CWE-209 Payload-Blind Confidentiality', () {
      try {
        tokenizeExcel('=SECRET_PAYLOAD ~ 1');
        fail('Expected ExcelLexicalException');
      } on ExcelLexicalException catch (e) {
        expect(e.errorCode, equals(ExcelLexerErrorCode.lexicalError));
        expect(e.errorId, equals('lexical-error'));
        expect(e.message, equals(messageLexicalError));
        final str = e.toString();
        expect(str, contains(messageLexicalError));
        expect(str.contains('SECRET'), isFalse);
        expect(str.contains('PAYLOAD'), isFalse);
        expect(e.line, isNotNull);
        expect(e.column, isNotNull);
      }

      try {
        tokenizeExcel('=A1', version: 'excel-v99');
        fail('Expected InvalidExcelVersionException');
      } on InvalidExcelVersionException catch (e) {
        expect(e.errorCode, equals(ExcelLexerErrorCode.invalidVersion));
        expect(e.errorId, equals('invalid-version'));
        expect(e.message, equals(messageInvalidVersion));
        expect(e.toString().contains('excel-v99'), isFalse);
      }
    });
  });

  group('Excel Lexer Additional Syntax Coverage', () {
    test('Tokenizes array constants, operators and workbook prefixes', () {
      final types = _types('={1,2;3,4}+[Book1]Sheet1!A1%^2<=3>=2');
      expect(
        types,
        containsAll(<String>[
          'LBRACE',
          'RBRACE',
          'SEMICOLON',
          'PLUS',
          'REF_PREFIX',
          'PERCENT',
          'CARET',
          'LESS_EQUALS',
          'GREATER_EQUALS',
        ]),
      );
    });

    test('Preserves non-ASCII characters in column accounting', () {
      // The accented glyph inside the string counts as one column, keeping the
      // contiguity check for the trailing function call exact.
      final tokens = tokenizeExcel('"\u00e9"&SUM(1)');
      final fn = tokens.firstWhere((t) => t.value == 'SUM');
      expect(fn.type, equals('FUNCTION_NAME'));
    });

    test('Resolves version strings and aliases', () {
      expect(resolveExcelVersion('excel'), equals('excel'));
      expect(resolveExcelVersion('EXCEL'), equals('excel'));
      expect(resolveExcelVersion('  Excel  '), equals('excel'));
      expect(supportedExcelVersions, equals(['excel']));
      expect(() => resolveExcelVersion('lotus'),
          throwsA(isA<InvalidExcelVersionException>()));

      final lexer = createExcelLexer('=SUM(A1)');
      expect(lexer.version, equals('excel'));
      expect(lexer.source, equals('=SUM(A1)'));
      expect(lexer.tokenize(), equals(tokenizeExcelFormula('=SUM(A1)')));
      expect(tokenizeExcelLexer('A1'), equals(tokenizeExcel('A1')));
    });

    test('Exception diagnostics format locations when available', () {
      expect(const InvalidExcelVersionException().toString(),
          equals(messageInvalidVersion));
      expect(const ExcelLexicalException(line: 1, column: 2).toString(),
          equals('$messageLexicalError (1:2)'));
      expect(formatLocation('msg', 3, null), equals('msg'));
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test ensures robust tokenization', () {
      final rng = math.Random(42);
      final refs = ['A1', r'$B$2', 'C10', 'Sheet1!D4', 'A:B', '1:10'];
      final names = ['Rate', 'SUM', 'IF', 'Total', 'DeptSales'];
      final values = ['10', '2.5', '1E3', '"hello"', 'TRUE', '#N/A'];
      final ops = [
        '+',
        '-',
        '*',
        '/',
        '^',
        '&',
        '=',
        '<>',
        '<=',
        '(',
        ')',
        ','
      ];

      for (var i = 0; i < 200; i++) {
        final parts = <String>[];
        final numTokens = rng.nextInt(12) + 2;
        for (var j = 0; j < numTokens; j++) {
          switch (rng.nextInt(4)) {
            case 0:
              parts.add(refs[rng.nextInt(refs.length)]);
            case 1:
              parts.add(names[rng.nextInt(names.length)]);
            case 2:
              parts.add(values[rng.nextInt(values.length)]);
            default:
              parts.add(ops[rng.nextInt(ops.length)]);
          }
        }
        final tokens = tokenizeExcel(parts.join(' '));
        expect(tokens, isNotEmpty);
        expect(tokens.last.type, equals('EOF'));
      }
    });

    test('10,000-iteration mutation fuzz loop rejects or tokenizes safely', () {
      final rng = math.Random(1337);
      const alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.\$@%:;{}()[],"\'#!&+-*/^<>= \t\n';

      for (var iter = 0; iter < 10000; iter++) {
        final length = rng.nextInt(20) + 1;
        final buffer = StringBuffer();
        for (var i = 0; i < length; i++) {
          buffer.write(alphabet[rng.nextInt(alphabet.length)]);
        }
        try {
          final tokens = tokenizeExcel(buffer.toString());
          expect(tokens.last.type, equals('EOF'));
        } on ExcelLexicalException {
          // Expected on unrecognized syntax fragments.
        }
      }
    });
  });
}
