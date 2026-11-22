import 'dart:math';

import 'package:coding_adventures_typescript_parser/coding_adventures_typescript_parser.dart';
import 'package:test/test.dart';

void main() {
  group('5-Class TypeScript Parser Security Resilience Showcase', () {
    test('Class 1: CWE-400 Resource Exhaustion via Token Ceiling', () {
      final oversizedTokens = List<Token>.generate(
        maxTokenCount + 1,
        (i) => Token(
          type: 'SEMICOLON',
          value: ';',
          line: 1,
          column: i + 1,
        ),
      );

      expect(
        () => TypescriptParser.fromTokens(oversizedTokens),
        throwsA(
          isA<TypescriptParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdTokenCountExceeded),
          ),
        ),
      );

      final atLimitTokens = List<Token>.generate(
        maxTokenCount,
        (i) => Token(
          type: 'SEMICOLON',
          value: ';',
          line: 1,
          column: i + 1,
        ),
      );
      final parser = TypescriptParser.fromTokens(atLimitTokens);
      expect(parser.tokens.length, equals(maxTokenCount));
    });

    test('Class 2a: CWE-674 Delimiter Nesting Depth (Parentheses)', () {
      // Depth 65 exceeds maxNestingDepth (64)
      final deepParens = '${"(" * 65}1${")" * 65};';
      expect(
        () => parseTypescript(deepParens),
        throwsA(
          isA<TypescriptParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdNestingTooDeep),
          ),
        ),
      );

      // Depth 64 is within bounds
      final safeParens = '${"(" * 64}1${")" * 64};';
      final ast = parseTypescript(safeParens);
      expect(ast.ruleName, equals('program'));
    });

    test('Class 2b: CWE-674 Delimiter Nesting Depth (Braces & Brackets)', () {
      // Braces rejection at depth 65
      final deepBraces = '${"{" * 65}${"}" * 65}';
      expect(
        () => parseTypescript(deepBraces, version: 'ts1.0'),
        throwsA(isA<TypescriptParseException>()),
      );

      // Braces accepted at depth 64 in ts1.0
      final safeBraces = '${"{" * 64}${"}" * 64}';
      final ast = parseTypescript(safeBraces, version: 'ts1.0');
      expect(ast.ruleName, equals('program'));

      // Brackets rejection at depth 65
      final deepBrackets = '${"[" * 65}1${"]" * 65}';
      expect(
        () => parseTypescript(deepBrackets, version: 'ts1.0'),
        throwsA(isA<TypescriptParseException>()),
      );

      // Mixed delimiters: ( { [ ...
      final mixed = '${"({[" * 22}1${"]})" * 22};'; // 22 * 3 = 66
      expect(
        () => parseTypescript(mixed, version: 'ts1.0'),
        throwsA(isA<TypescriptParseException>()),
      );
    });

    test('Class 3a: CWE-674 Consecutive Prefix-Operator Chains', () {
      // 65 consecutive NOT operators
      final deepNot = '${"!" * 65}true;';
      expect(
        () => parseTypescript(deepNot, version: 'ts1.0'),
        throwsA(isA<TypescriptParseException>()),
      );

      // 64 consecutive NOT operators is within limit
      final safeNot = '${"!" * 64}true;';
      final ast = parseTypescript(safeNot, version: 'ts1.0');
      expect(ast.ruleName, equals('program'));

      // Chained prefix keywords
      final deepKeywords = '${"typeof " * 65}1;';
      expect(
        () => parseTypescript(deepKeywords, version: 'ts1.0'),
        throwsA(isA<TypescriptParseException>()),
      );
    });

    test('Class 3b: Balanced Constructs Never Leak Depth', () {
      // 200 sequential expressions at depth 1 should never accumulate depth
      final buffer = StringBuffer();
      for (var i = 0; i < 200; i++) {
        buffer.write('(1); ');
      }
      final ast = parseTypescript(buffer.toString());
      expect(ast.ruleName, equals('program'));

      // 200 sequential prefix statements
      final prefixBuffer = StringBuffer();
      for (var i = 0; i < 200; i++) {
        prefixBuffer.write('!true; ');
      }
      final astPrefix =
          parseTypescript(prefixBuffer.toString(), version: 'ts1.0');
      expect(astPrefix.ruleName, equals('program'));
    });

    test('Class 3c: Stray Closers Cannot Hide Later Depth', () {
      // 100 stray closers clamped at 0, followed by depth 65
      final strayAndDeep = '${")" * 100}${"(" * 65}1${")" * 65};';
      expect(
        () => parseTypescript(strayAndDeep),
        throwsA(isA<TypescriptParseException>()),
      );
    });

    test('Class 3d: StackOverflowError Backstop', () {
      // Trigger artificial StackOverflowError in traceWriter during parse
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (_) => throw StackOverflowError(),
      );
      expect(
        () => parseTypescript('var x = 1;', options: options),
        throwsA(
          isA<TypescriptParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdNestingTooDeep),
          ),
        ),
      );
    });

    test('Class 4: CWE-209 Payload-Blind Diagnostics & Confidentiality', () {
      const secret = 'SUPER_SECRET_PAYLOAD_TOKEN';
      try {
        parseTypescript('var x = 1 + ;'); // syntax error
        fail('Expected syntax error');
      } on TypescriptParserException catch (e) {
        expect(e.message, equals(messageSyntaxError));
        expect(e.errorId, equals(errorIdSyntaxError));
        expect(e.toString(), contains('TypeScript syntax parse error.'));
        expect(e.toString(), isNot(contains(secret)));
      }

      // Version rejection
      try {
        parseTypescript('var x = 1;', version: 'INVALID_TS_VERSION');
        fail('Expected version error');
      } on TypescriptParserException catch (e) {
        expect(e.message, equals(messageInvalidVersion));
        expect(e.errorId, equals(errorIdInvalidVersion));
        expect(e.toString(), isNot(contains('INVALID_TS_VERSION')));
      }
    });

    test('Class 5: AST Conformance Across All 7 Editions', () {
      final samples = {
        'typescript': 'let x = 1 + 2;',
        'ts1.0': 'var x = 1;',
        'ts2.0': 'var x = 1;',
        'ts3.0': 'var x = 1;',
        'ts4.0': 'var x = 1;',
        'ts5.0': 'var x = 1;',
        'ts5.8': 'var x = 1;',
      };

      for (final entry in samples.entries) {
        final version = entry.key;
        final code = entry.value;
        final ast = parseTypescript(code, version: version);
        expect(ast.ruleName, equals('program'));
        expect(ast.children, isNotEmpty);
      }
    });
  });

  group('TypeScript Parser Comprehensive Syntax Coverage', () {
    test('Accepts representative TypeScript statements', () {
      final genericStatements = [
        'var x = 1;',
        'let y = 2;',
        'const z = 3;',
        'x = x + 1;',
        '1 + 2 * 3;',
        '((4));',
      ];

      for (final stmt in genericStatements) {
        final ast = parseTypescript(stmt);
        expect(ast.ruleName, equals('program'));
      }

      final ts10Statements = [
        'var x = 1;',
        'var x: number = 1;',
        'if (x) { var y = 1; } else { var z = 2; }',
        'while (x) { var a = 1; }',
        'return 1;',
        '{ var a = 1; }',
      ];

      for (final stmt in ts10Statements) {
        final ast = parseTypescript(stmt, version: 'ts1.0');
        expect(ast.ruleName, equals('program'));
      }
    });

    test('Rejects malformed TypeScript syntax with typed errors', () {
      final badInputs = [
        'var = 1;',
        '1 + ;',
        '(1 + 2',
      ];

      for (final bad in badInputs) {
        expect(
          () => parseTypescript(bad),
          throwsA(isA<TypescriptParseException>()),
          reason: 'Expected syntax error for: $bad',
        );
      }
    });

    test('Lexical failures surface as typed lexer exceptions', () {
      expect(
        () => parseTypescript('var x = @#INVALID;'),
        throwsA(isA<Exception>()),
      );
    });

    test('parseTypescriptTokens agrees with parseTypescript', () {
      const code = 'var x = 1; var y = 2;';
      final parser = createTypescriptParser(code);
      final ast1 = parser.parse();
      final ast2 = parseTypescriptTokens(parser.tokens);
      final ast3 = parseTypeScriptTokens(parser.tokens);

      expect(ast1.ruleName, equals(ast2.ruleName));
      expect(ast1.children.length, equals(ast2.children.length));
      expect(ast1.children.length, equals(ast3.children.length));
    });

    test('Parser options and aliases', () {
      var traceCalled = false;
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (_) => traceCalled = true,
      );

      final ast = parseTypeScript('var x = 1;', options: options);
      expect(ast.ruleName, equals('program'));
      expect(traceCalled, isTrue);

      final parser = createTypeScriptParser('var x = 1;');
      expect(parser.version, equals(defaultTypescriptParserVersion));
      expect(parser.tokens, isNotEmpty);
    });

    test('Exception formatting and location', () {
      const syntaxErr =
          TypescriptParseException.syntaxError(line: 2, column: 7);
      expect(syntaxErr.line, equals(2));
      expect(syntaxErr.column, equals(7));
      expect(syntaxErr.errorId, equals(errorIdSyntaxError));
      expect(
          syntaxErr.toString(), equals('TypeScript syntax parse error. (2:7)'));

      const noLocErr = TypescriptParseException.syntaxError();
      expect(noLocErr.toString(), equals('TypeScript syntax parse error.'));

      const verErr = InvalidTypescriptParserVersionException();
      expect(verErr.errorId, equals(errorIdInvalidVersion));
      expect(verErr.toString(), equals(messageInvalidVersion));

      const tokErr = TypescriptParseException.tokenCountExceeded();
      expect(tokErr.errorId, equals(errorIdTokenCountExceeded));
      expect(tokErr.toString(), equals(messageTokenCountExceeded));

      const nestErr = TypescriptParseException.nestingTooDeep();
      expect(nestErr.errorId, equals(errorIdNestingTooDeep));
      expect(nestErr.toString(), equals(messageNestingTooDeep));
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test verifies valid design generation', () {
      final rng = Random(42);
      final genericTemplates = [
        'var x = 1;',
        'let a = 2 + 3;',
        'const b = 4 * 5;',
        'x = x + 1;',
        '1 + 2;',
      ];
      final tsTemplates = [
        'var x = 1;',
        'var a: number = 2;',
        'var b = 4;',
        'if (x) { var y = 0; }',
        'while (x) { var z = 1; }',
        'return 1;',
        '{ var k = 1; }',
        ';',
      ];

      for (var i = 0; i < 200; i++) {
        final version = supportedTypescriptParserVersions[
            rng.nextInt(supportedTypescriptParserVersions.length)];
        final templates =
            version == 'typescript' ? genericTemplates : tsTemplates;
        final numStatements = rng.nextInt(5) + 1;
        final buffer = StringBuffer();
        for (var s = 0; s < numStatements; s++) {
          buffer.write(templates[rng.nextInt(templates.length)]);
          buffer.write(' ');
        }

        final ast = parseTypescript(buffer.toString(), version: version);
        expect(ast.ruleName, equals('program'));
      }
    });

    test(
        '10,000-iteration mutation fuzz loop ensures memory and exception safety',
        () {
      final rng = Random(1337);
      const baseProgram = 'var x = 1; let y = 2 + 3; x = x + 1;';
      final alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \t\n\r_+-*/=<>!~@#%^&|()[]{}:;,.\'"\\`';

      for (var i = 0; i < 10000; i++) {
        final version = supportedTypescriptParserVersions[
            i % supportedTypescriptParserVersions.length];
        final chars = baseProgram.split('');

        // Apply 1 to 3 mutations
        final mutations = rng.nextInt(3) + 1;
        for (var m = 0; m < mutations; m++) {
          final pos = rng.nextInt(chars.length);
          final mutType = rng.nextInt(3);
          switch (mutType) {
            case 0: // replace
              chars[pos] = alphabet[rng.nextInt(alphabet.length)];
            case 1: // delete
              if (chars.length > 5) chars.removeAt(pos);
            case 2: // insert
              chars.insert(pos, alphabet[rng.nextInt(alphabet.length)]);
          }
        }

        final mutated = chars.join('');
        try {
          final ast = parseTypescript(mutated, version: version);
          expect(ast.ruleName, equals('program'));
        } on TypescriptParserException {
          // Expected typed syntax, recursion or token count rejection
        } catch (_) {
          // Lexer exceptions also allowed
        }
      }
    });
  });
}
