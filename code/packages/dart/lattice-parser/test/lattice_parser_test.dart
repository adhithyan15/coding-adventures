import 'dart:math' as math;

import 'package:coding_adventures_lattice_lexer/coding_adventures_lattice_lexer.dart'
    show LatticeLexerException;
import 'package:coding_adventures_lattice_parser/coding_adventures_lattice_parser.dart';
import 'package:test/test.dart';

void main() {
  group('5-Class Lattice Parser Security Resilience Showcase', () {
    test('Class 1: Version Resolution & Domain Isolation', () {
      expect(resolveLatticeParserVersion('lattice'), equals('lattice'));
      expect(resolveLatticeParserVersion('LATTICE'), equals('lattice'));
      expect(resolveLatticeParserVersion('  Lattice  '), equals('lattice'));
      expect(supportedLatticeParserVersions, equals(['lattice']));

      for (final invalid in ['scss', 'sass', 'less', 'css', '']) {
        expect(
          () => resolveLatticeParserVersion(invalid),
          throwsA(
            isA<InvalidLatticeParserVersionException>().having(
              (e) => e.errorId,
              'errorId',
              errorIdInvalidVersion,
            ),
          ),
        );
        expect(
          () => parseLattice('h1 { color: red; }', version: invalid),
          throwsA(isA<InvalidLatticeParserVersionException>()),
        );
      }
    });

    test('Class 2: CWE-400 Resource Exhaustion via Token Ceiling', () {
      final oversizedTokens = List.generate(
        maxTokenCount + 1,
        (i) => Token(type: 'IDENT', value: 'cls$i', line: 1, column: i + 1),
      );

      expect(
        () => parseLatticeTokens(oversizedTokens),
        throwsA(
          isA<LatticeParseException>()
              .having(
                (e) => e.errorCode,
                'errorCode',
                LatticeParserErrorCode.tokenCountExceeded,
              )
              .having((e) => e.errorId, 'errorId', errorIdTokenCountExceeded),
        ),
      );
    });

    test(
      'Class 3: CWE-674 Recursion Depth Defense (Call Stack Protection)',
      () {
        // Braces nesting depth > 256
        final excessiveLeftBraces = '{' * (maxNestingDepth + 1);
        final excessiveRightBraces = '}' * (maxNestingDepth + 1);
        final deepBraceInput =
            'div $excessiveLeftBraces color: red; $excessiveRightBraces';

        expect(
          () => parseLattice(deepBraceInput),
          throwsA(
            isA<LatticeParseException>()
                .having(
                  (e) => e.errorCode,
                  'errorCode',
                  LatticeParserErrorCode.nestingTooDeep,
                )
                .having((e) => e.errorId, 'errorId', errorIdNestingTooDeep),
          ),
        );

        // Parentheses nesting depth > 256
        final excessiveLeftParens = '(' * (maxNestingDepth + 1);
        final excessiveRightParens = ')' * (maxNestingDepth + 1);
        final deepParenInput =
            r'$val: ' + excessiveLeftParens + '1' + excessiveRightParens + ';';

        expect(
          () => parseLattice(deepParenInput),
          throwsA(
            isA<LatticeParseException>().having(
              (e) => e.errorCode,
              'errorCode',
              LatticeParserErrorCode.nestingTooDeep,
            ),
          ),
        );

        // Function call nesting depth > 256 (FUNCTION token opens paren)
        final excessiveLeftFuncs = 'calc(' * (maxNestingDepth + 1);
        final excessiveRightFuncs = ')' * (maxNestingDepth + 1);
        final deepFuncInput =
            'div { width: ${excessiveLeftFuncs}1px$excessiveRightFuncs; }';

        expect(
          () => parseLattice(deepFuncInput),
          throwsA(
            isA<LatticeParseException>().having(
              (e) => e.errorCode,
              'errorCode',
              LatticeParserErrorCode.nestingTooDeep,
            ),
          ),
        );

        // Safe deep nesting (<= 256) succeeds
        final safeLeft = '(' * 16;
        final safeRight = ')' * 16;
        final safeInput =
            '@if ' + safeLeft + '1' + safeRight + ' { body { color: red; } }';
        expect(() => parseLattice(safeInput), returnsNormally);
      },
    );

    test('Class 4: CWE-209 Payload-Blind Diagnostics & Confidentiality', () {
      try {
        parseLattice('h1 { \$super_secret_property: %invalid_value }');
        fail('Expected LatticeParseException on invalid syntax');
      } on LatticeParseException catch (e) {
        expect(e.errorCode, equals(LatticeParserErrorCode.syntaxError));
        expect(e.errorId, equals(errorIdSyntaxError));
        expect(e.message, equals(messageSyntaxError));
        final str = e.toString();
        expect(str, contains(messageSyntaxError));
        expect(str.contains('super_secret_property'), isFalse);
        expect(str.contains('invalid_value'), isFalse);
        expect(e.line, isNotNull);
        expect(e.column, isNotNull);
      }
    });

    test('Class 5: AST Structural Integrity & Hierarchy', () {
      final ast = parseLattice(r'''
        $theme: dark;
        h1 { color: red; }
      ''');
      expect(ast.ruleName, equals('stylesheet'));
      expect(ast.children, isNotEmpty);
      final firstRule = ast.children.first as ASTNode;
      expect(firstRule.ruleName, equals('rule'));
    });
  });

  group('Lattice Parser Comprehensive Syntax Coverage', () {
    test('Variables with !default and !global flags', () {
      const source = r'''
        $font-size: 16px !default;
        $primary-color: #007bff !global;
      ''';
      final ast = parseLattice(source);
      expect(ast.ruleName, equals('stylesheet'));
    });

    test('Mixin definitions and include directives', () {
      const source = r'''
        @mixin button-styles($bg: red, $padding: 10px) {
          background: $bg;
          padding: $padding;
        }
        @mixin simple-reset {
          margin: 0;
          padding: 0;
        }
        .btn {
          @include button-styles(#fff, 12px);
          @include simple-reset;
        }
      ''';
      final ast = parseLattice(source);
      expect(ast.ruleName, equals('stylesheet'));
    });

    test('Control flow: @if, @else if, @else, @for, @each, @while', () {
      const source = r'''
        @if $theme == dark and $brightness <= 50 {
          body { background: #111; }
        } @else if $theme == light or $inverted == false {
          body { background: #fff; }
        } @else {
          body { background: #888; }
        }

        @for $i from 1 through 3 {
          .col-$i { width: 33%; }
        }

        @each $item in a, b, c {
          .icon-$item { display: inline-block; }
        }

        @while $val > 0 {
          $val: $val - 1;
        }
      ''';
      final ast = parseLattice(source);
      expect(ast.ruleName, equals('stylesheet'));
    });

    test('Functions and return directives', () {
      const source = r'''
        @function double-size($size) {
          @return $size * 2;
        }
        div {
          font-size: double-size(14px);
        }
      ''';
      final ast = parseLattice(source);
      expect(ast.ruleName, equals('stylesheet'));
    });

    test('Use directives and map literals', () {
      const source = r'''
        @use "colors" as c;
        $palette: (primary: #007bff, secondary: #6c757d);
      ''';
      final ast = parseLattice(source);
      expect(ast.ruleName, equals('stylesheet'));
    });

    test('Parser options and parser factory', () {
      final traceOutput = <String>[];
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (msg) => traceOutput.add(msg),
      );

      final parser = createLatticeParser(
        'h1 { color: red; }',
        options: options,
      );
      expect(parser.version, equals('lattice'));
      expect(parser.tokens, isNotEmpty);
      final ast = parser.parse();
      expect(ast.ruleName, equals('stylesheet'));
      expect(traceOutput, isNotEmpty);
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test(
      '200-case PRNG property test verifies valid stylesheet generation',
      () {
        final rng = math.Random(42);
        final selectors = ['h1', 'p', '.card', '#main', 'button', 'a:hover'];
        final props = ['color', 'margin', 'padding', 'width', 'display'];
        final values = ['red', '10px', '2rem', 'none', 'block', '#fff'];
        final vars = [r'$primary', r'$size', r'$font', r'$gap'];

        for (var i = 0; i < 200; i++) {
          final template = rng.nextInt(4);
          String lattice;
          switch (template) {
            case 0:
              final sel = selectors[rng.nextInt(selectors.length)];
              final prop = props[rng.nextInt(props.length)];
              final val = values[rng.nextInt(values.length)];
              lattice = '$sel { $prop: $val; }';
              break;
            case 1:
              final v = vars[rng.nextInt(vars.length)];
              final val = values[rng.nextInt(values.length)];
              lattice = '$v: $val;';
              break;
            case 2:
              final name = 'mix_${rng.nextInt(100)}';
              lattice = '@mixin $name { margin: 0; }';
              break;
            default:
              final sel = selectors[rng.nextInt(selectors.length)];
              lattice = '$sel { @include button; }';
              break;
          }

          final ast = parseLattice(lattice);
          expect(
            ast.ruleName,
            equals('stylesheet'),
            reason: 'Failed case $i: $lattice',
          );
        }
      },
    );

    test(
        '10,000-iteration mutation fuzz loop ensures memory and exception safety',
        () {
      final rng = math.Random(1337);
      final seeds = [
        r'$color: red;',
        'h1 { color: red; }',
        '@mixin btn { padding: 10px; }',
        '.card { @include btn; }',
        r'@if $a == $b { body { color: blue; } }',
        '@use "theme";',
      ];
      final alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-\$@%:;{}(),.\'"/# \t\n';

      for (var iter = 0; iter < 10000; iter++) {
        final seed = seeds[rng.nextInt(seeds.length)];
        final chars = seed.split('');

        // Apply 1 to 3 mutations
        final numMutations = rng.nextInt(3) + 1;
        for (var m = 0; m < numMutations; m++) {
          final mutType = rng.nextInt(4);
          if (chars.isEmpty) break;
          final pos = rng.nextInt(chars.length);
          switch (mutType) {
            case 0:
              chars[pos] = alphabet[rng.nextInt(alphabet.length)];
              break;
            case 1:
              chars.removeAt(pos);
              break;
            case 2:
              chars.insert(pos, alphabet[rng.nextInt(alphabet.length)]);
              break;
            case 3:
              chars.insert(pos, rng.nextBool() ? '{' : '}');
              break;
          }
        }

        final mutant = chars.join();
        try {
          final ast = parseLattice(mutant);
          expect(ast.ruleName, equals('stylesheet'));
        } on LatticeParseException {
          // Expected typed syntax error or limit violation
        } on LatticeLexerException {
          // Expected typed lexical error
        }
      }
    });
  });
}
