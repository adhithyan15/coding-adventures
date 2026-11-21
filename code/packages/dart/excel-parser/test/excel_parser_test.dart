import 'dart:math' as math;

import 'package:coding_adventures_excel_lexer/coding_adventures_excel_lexer.dart'
    show ExcelLexerException, tokenizeExcel;
import 'package:coding_adventures_excel_parser/coding_adventures_excel_parser.dart';
import 'package:test/test.dart';

Token _token(String type, String value, int column) =>
    Token(type: type, value: value, line: 1, column: column);

List<String> _types(List<Token> tokens) => tokens.map((t) => t.type).toList();

Matcher _nestingTooDeep() => throwsA(
      isA<ExcelParseException>()
          .having(
            (e) => e.errorCode,
            'errorCode',
            ExcelParserErrorCode.nestingTooDeep,
          )
          .having((e) => e.errorId, 'errorId', errorIdNestingTooDeep),
    );

void main() {
  group('5-Class Excel Parser Security Resilience Showcase', () {
    test('Class 1: Version Resolution & Domain Isolation', () {
      expect(resolveExcelParserVersion('excel'), equals('excel'));
      expect(resolveExcelParserVersion('EXCEL'), equals('excel'));
      expect(resolveExcelParserVersion('  Excel  '), equals('excel'));
      expect(supportedExcelParserVersions, equals(['excel']));
      expect(loadExcelParserGrammar().rules.first.name, equals('formula'));

      for (final invalid in ['lotus', 'sheets', 'calc', 'xlsx', '']) {
        expect(
          () => resolveExcelParserVersion(invalid),
          throwsA(
            isA<InvalidExcelParserVersionException>().having(
              (e) => e.errorId,
              'errorId',
              errorIdInvalidVersion,
            ),
          ),
        );
        expect(
          () => parseExcel('=A1', version: invalid),
          throwsA(isA<InvalidExcelParserVersionException>()),
        );
      }
    });

    test('Class 2: CWE-400 Resource Exhaustion via Token Ceiling', () {
      final oversizedTokens = List.generate(
        maxTokenCount + 1,
        (i) => _token('NAME', 'n$i', i + 1),
      );

      expect(
        () => parseExcelTokens(oversizedTokens),
        throwsA(
          isA<ExcelParseException>()
              .having(
                (e) => e.errorCode,
                'errorCode',
                ExcelParserErrorCode.tokenCountExceeded,
              )
              .having((e) => e.errorId, 'errorId', errorIdTokenCountExceeded),
        ),
      );
    });

    test(
      'Class 3: CWE-674 Recursion Depth Defense (Call Stack Protection)',
      () {
        final over = maxNestingDepth + 1;

        // Parentheses.
        expect(
          () => parseExcel('=${'(' * over}1${')' * over}'),
          _nestingTooDeep(),
        );

        // Function calls: FUNCTION_NAME is followed by a real LPAREN token.
        expect(
          () => parseExcel('=${'SUM(' * over}1${')' * over}'),
          _nestingTooDeep(),
        );

        // Array-constant braces.
        expect(
          () => parseExcel('=${'{' * over}1${'}' * over}'),
          _nestingTooDeep(),
        );

        // Nested structured-reference brackets arrive as LBRACKET tokens.
        final bracketTokens = [
          for (var i = 0; i < over; i++) _token('LBRACKET', '[', i + 1),
          for (var i = 0; i < over; i++) _token('RBRACKET', ']', over + i + 1),
        ];
        expect(() => parseExcelTokens(bracketTokens), _nestingTooDeep());

        // Mixed delimiters share a single ceiling.
        final mixed = '(' * 22 + '{' * 21 + '[' * 22;
        final mixedTokens = [
          for (var i = 0; i < mixed.length; i++)
            _token(
              mixed[i] == '('
                  ? 'LPAREN'
                  : mixed[i] == '{'
                      ? 'LBRACE'
                      : 'LBRACKET',
              mixed[i],
              i + 1,
            ),
        ];
        expect(mixedTokens.length, equals(over));
        expect(() => parseExcelTokens(mixedTokens), _nestingTooDeep());

        // Nesting at the ceiling parses without exhausting the stack.
        final atLimit = maxNestingDepth;
        expect(
          () => parseExcel('=${'(' * atLimit}1${')' * atLimit}'),
          returnsNormally,
        );

        // Unbalanced closers never drive the depth counter negative.
        expect(
          () => parseExcel('=${')' * over}1'),
          throwsA(isA<ExcelParseException>()),
        );
      },
    );

    test('Class 4: CWE-209 Payload-Blind Diagnostics & Confidentiality', () {
      try {
        parseExcel('=SECRET_PAYLOAD + * 1');
        fail('Expected ExcelParseException on invalid syntax');
      } on ExcelParseException catch (e) {
        expect(e.errorCode, equals(ExcelParserErrorCode.syntaxError));
        expect(e.errorId, equals(errorIdSyntaxError));
        expect(e.message, equals(messageSyntaxError));
        final str = e.toString();
        expect(str, contains(messageSyntaxError));
        expect(str.contains('SECRET'), isFalse);
        expect(str.contains('PAYLOAD'), isFalse);
        expect(e.line, isNotNull);
        expect(e.column, isNotNull);
      }

      expect(
        const ExcelParseException.tokenCountExceeded().toString(),
        equals(messageTokenCountExceeded),
      );
      expect(formatLocation('msg', 3, null), equals('msg'));
    });

    test('Class 5: Reference Normalization & AST Structure', () {
      final ast = parseExcel('=SUM(A:B, 1:10)');
      expect(ast.ruleName, equals('formula'));
      expect(ast.children, isNotEmpty);
      expect((ast.children.first as ASTNode).ruleName, equals('ws'));

      final parser = createExcelParser('=SUM(A:B, 1:10)');
      expect(
        _types(parser.tokens),
        equals([
          'EQUALS',
          'FUNCTION_NAME',
          'LPAREN',
          'COLUMN_REF',
          'COLON',
          'COLUMN_REF',
          'COMMA',
          'SPACE',
          'ROW_REF',
          'COLON',
          'ROW_REF',
          'RPAREN',
          'EOF',
        ]),
      );
    });
  });

  group('Reference Normalization Rules', () {
    test('Names and numbers adjacent to a colon are reclassified', () {
      final tokens = normalizeExcelReferenceTokens(tokenizeExcel('A : B'));
      expect(
        _types(tokens),
        equals([
          'COLUMN_REF',
          'SPACE',
          'COLON',
          'SPACE',
          'COLUMN_REF',
          'EOF',
        ]),
      );
    });

    test('Tokens not adjacent to a colon are unchanged', () {
      final tokens = normalizeExcelReferenceTokens(tokenizeExcel('Rate+2'));
      expect(_types(tokens), equals(['NAME', 'PLUS', 'NUMBER', 'EOF']));
    });

    test('Cells and ranges keep their CELL tokens', () {
      final tokens = normalizeExcelReferenceTokens(tokenizeExcel('A1:B2'));
      expect(_types(tokens), equals(['CELL', 'COLON', 'CELL', 'EOF']));
    });

    test('Leading and trailing positions have no neighbour', () {
      expect(
        _types(normalizeExcelReferenceTokens([_token('NAME', 'x', 1)])),
        equals(['NAME']),
      );
      expect(
        _types(normalizeExcelReferenceTokens([_token('NUMBER', '1', 1)])),
        equals(['NUMBER']),
      );
    });
  });

  group('Excel Parser Comprehensive Syntax Coverage', () {
    test('Accepts representative formulas across the grammar surface', () {
      const formulas = <String>[
        'A1',
        '=A1+B2*3',
        '=-A1^2',
        '=50%',
        '={1,2;3,4}',
        "='My Sheet'!A1",
        '=Sheet1!A1:B2',
        '=[Book1]Sheet1!A1',
        '=DeptSales[Sales Amount]',
        '=DeptSales[[#Headers],[Sales]]',
        '=IF(A1>1,"y","n")&"x"',
        '=#REF!',
        '=TRUE',
        '=Rate*2',
        '=A1:B2 C3:D4',
        '=(A1,B2)',
        '= A1 + 1 ',
        r'=$A$1:$B$2',
        '=SUM(A:B)',
        '=SUM(1:10)',
      ];
      for (final formula in formulas) {
        expect(
          parseExcel(formula).ruleName,
          equals('formula'),
          reason: 'Formula should parse: $formula',
        );
      }
    });

    test('Rejects malformed formulas with typed syntax errors', () {
      for (final formula in ['', '=1+', '=SUM(', '=)', '=@A1']) {
        expect(
          () => parseExcel(formula),
          throwsA(isA<ExcelParseException>()),
          reason: 'Formula should fail: $formula',
        );
      }
    });

    test('Lexical failures surface as typed lexer exceptions', () {
      expect(() => parseExcel('=A1#'), throwsA(isA<ExcelLexerException>()));
    });

    test('parseExcelFormula and parseExcelTokens agree with parseExcel', () {
      const source = '=SUM(A1:B2)';
      final expected = parseExcel(source).toString();
      expect(parseExcelFormula(source).toString(), equals(expected));
      expect(
        parseExcelTokens(tokenizeExcel(source)).ruleName,
        equals('formula'),
      );
    });

    test('Parser options and parser factory', () {
      final traceOutput = <String>[];
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (msg) => traceOutput.add(msg),
      );

      final parser = createExcelParser('=A1', options: options);
      expect(parser.version, equals('excel'));
      expect(parser.tokens, isNotEmpty);
      final ast = parser.parse();
      expect(ast.ruleName, equals('formula'));
      expect(traceOutput, isNotEmpty);
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test(
      '200-case PRNG property test verifies valid formula generation',
      () {
        final rng = math.Random(42);
        final cells = ['A1', r'$B$2', 'C10', 'Sheet1!D4'];
        final ranges = ['A:B', '1:10', 'A1:B2', r'$A$1:$C$3'];
        final ops = ['+', '-', '*', '/', '^', '&', '=', '<>', '<='];
        final funcs = ['SUM', 'MAX', 'MIN', 'AVERAGE', 'IF'];

        for (var i = 0; i < 200; i++) {
          final cell = cells[rng.nextInt(cells.length)];
          final range = ranges[rng.nextInt(ranges.length)];
          final func = funcs[rng.nextInt(funcs.length)];
          final op = ops[rng.nextInt(ops.length)];
          final formula = switch (rng.nextInt(3)) {
            0 => '=$func($cell,$range)',
            1 => '=$cell$op$cell',
            _ => '=$func($range)',
          };
          expect(
            parseExcel(formula).ruleName,
            equals('formula'),
            reason: 'Failed case $i: $formula',
          );
        }
      },
    );

    test(
      '10,000-iteration mutation fuzz loop ensures memory and exception safety',
      () {
        final rng = math.Random(1337);
        final seeds = [
          '=SUM(A1:B2)',
          '=IF(A1>1,"y","n")',
          '=DeptSales[Sales Amount]',
          '={1,2;3,4}',
          "='My Sheet'!A1",
          '=A:B',
        ];
        const alphabet =
            'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.\$@%:;{}()[],"\'#!&+-*/^<>= \t\n';

        for (var iter = 0; iter < 10000; iter++) {
          final chars = seeds[rng.nextInt(seeds.length)].split('');

          final numMutations = rng.nextInt(3) + 1;
          for (var m = 0; m < numMutations && chars.isNotEmpty; m++) {
            final pos = rng.nextInt(chars.length);
            switch (rng.nextInt(4)) {
              case 0:
                chars[pos] = alphabet[rng.nextInt(alphabet.length)];
              case 1:
                chars.removeAt(pos);
              case 2:
                chars.insert(pos, alphabet[rng.nextInt(alphabet.length)]);
              default:
                chars.insert(pos, rng.nextBool() ? '(' : ')');
            }
          }

          try {
            expect(parseExcel(chars.join()).ruleName, equals('formula'));
          } on ExcelParseException {
            // Expected typed syntax error or limit violation.
          } on ExcelLexerException {
            // Expected typed lexical error.
          }
        }
      },
    );
  });
}
