import 'dart:math' as math;

import 'package:coding_adventures_lattice_lexer/coding_adventures_lattice_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('5-Class Lattice Lexer Security Resilience Showcase', () {
    test('Class 1: CWE-400 Source Length DoS Defense', () {
      final atLimit = ' ' * maxSourceLength;
      expect(() => tokenizeLattice(atLimit), returnsNormally);

      final overLimit = ' ' * (maxSourceLength + 1);
      expect(
        () => tokenizeLattice(overLimit),
        throwsA(isA<InvalidLatticeSourceException>().having(
          (e) => e.message,
          'message',
          messageSourceTooLarge,
        )),
      );
      expect(() => createLatticeLexer(overLimit),
          throwsA(isA<InvalidLatticeSourceException>()));
    });

    test('Class 2: Surrogate Smuggling & Encoding Attacks', () {
      final loneLead = String.fromCharCode(0xd800);
      final loneTrail = String.fromCharCode(0xdc00);
      final reversedPair = String.fromCharCodes([0xdc00, 0xd800]);
      final midSurrogate = '\$primary: ${loneLead}#fff;';

      for (final invalid in [loneLead, loneTrail, reversedPair, midSurrogate]) {
        expect(
          () => tokenizeLattice(invalid),
          throwsA(isA<InvalidLatticeSourceException>().having(
            (e) => e.message,
            'message',
            messageInvalidSurrogates,
          )),
        );
      }
    });

    test('Class 3: Token Domain Integrity (Lattice Extensions)', () {
      const source = r'''
        $color: #333 !default;
        $global-flag: true !global;
        %placeholder-card { border: 1px solid red; }
        @if $a == $b and $c != $d and $e >= 10 and $f <= 20 { }
      ''';
      final tokens = tokenizeLattice(source);
      final types = tokens.map((t) => t.type).toSet();

      expect(types, contains('VARIABLE'));
      expect(types, contains('HASH'));
      expect(types, contains('BANG_DEFAULT'));
      expect(types, contains('BANG_GLOBAL'));
      expect(types, contains('PLACEHOLDER'));
      expect(types, contains('EQUALS_EQUALS'));
      expect(types, contains('NOT_EQUALS'));
      expect(types, contains('GREATER_EQUALS'));
      expect(types, contains('LESS_EQUALS'));
      expect(tokens.last.type, equals('EOF'));
    });

    test('Class 4: Comment & String Literal Resilience', () {
      const sourceWithComments = '''
        // Line comment 1
        \$font: "Helvetica", 'Arial'; // Line comment 2
        /* Block comment in middle */
        h1 { /* multi-line
                block comment */
          font-family: \$font;
        }
      ''';
      final tokens = tokenizeLattice(sourceWithComments);
      expect(tokens.any((t) => t.type == 'LINE_COMMENT'), isFalse);
      expect(tokens.any((t) => t.type == 'COMMENT'), isFalse);
      expect(tokens.first.type, equals('VARIABLE'));
      expect(tokens.first.value, equals(r'$font'));
    });

    test('Class 5: CWE-209 Payload-Blind Confidentiality', () {
      try {
        tokenizeLattice('\$secret: `unrecognized_backtick_literal`;');
        fail('Expected LatticeLexicalException');
      } on LatticeLexicalException catch (e) {
        expect(e.errorCode, equals(LatticeLexerErrorCode.lexicalError));
        expect(e.errorId, equals('lexical-error'));
        expect(e.message, equals(messageLexicalError));
        final str = e.toString();
        expect(str, contains(messageLexicalError));
        expect(str.contains('unrecognized'), isFalse);
        expect(str.contains('secret'), isFalse);
        expect(e.line, isNotNull);
        expect(e.column, isNotNull);
      }

      // Invalid version error
      try {
        tokenizeLattice('h1 { color: red; }', version: 'lattice-v99');
        fail('Expected InvalidLatticeVersionException');
      } on InvalidLatticeVersionException catch (e) {
        expect(e.errorCode, equals(LatticeLexerErrorCode.invalidVersion));
        expect(e.errorId, equals('invalid-version'));
        expect(e.message, equals(messageInvalidVersion));
        expect(e.toString().contains('lattice-v99'), isFalse);
      }
    });
  });

  group('Lattice Lexer Additional Syntax Coverage', () {
    test('Tokenizes directives, functions, dimensions, percentages', () {
      const source = r'''
        @use "theme" as t;
        @mixin rounded($radius: 4px) {
          border-radius: $radius;
        }
        .box {
          @include rounded(10px);
          width: 50%;
          opacity: 0.85;
          margin: 1.5em;
        }
      ''';
      final tokens = tokenizeLatticeLexer(source);
      final types = tokens.map((t) => t.type).toList();

      expect(types, contains('AT_KEYWORD'));
      expect(types, contains('STRING'));
      expect(types, contains('IDENT'));
      expect(types, contains('FUNCTION'));
      expect(types, contains('DIMENSION'));
      expect(types, contains('PERCENTAGE'));
      expect(types, contains('NUMBER'));
    });

    test('Resolves version strings and aliases', () {
      expect(resolveLatticeVersion('lattice'), equals('lattice'));
      expect(resolveLatticeVersion('LATTICE'), equals('lattice'));
      expect(resolveLatticeVersion('  Lattice  '), equals('lattice'));
      expect(supportedLatticeVersions, equals(['lattice']));

      final lexer = createLatticeLexer('h1 { color: red; }');
      expect(lexer.version, equals('lattice'));
      expect(lexer.source, equals('h1 { color: red; }'));
      final tokens = lexer.tokenize();
      expect(tokens, isNotEmpty);
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test ensures robust tokenization', () {
      final rng = math.Random(42);
      final vars = [r'$color', r'$bg', r'$size', r'$font', r'$gap', r'$radius'];
      final idents = [
        'h1',
        'div',
        'p',
        'span',
        'color',
        'margin',
        'padding',
        'red',
        'blue'
      ];
      final values = [
        '10px',
        '2rem',
        '100%',
        '42',
        '0.5',
        '"hello"',
        "'world'",
        '#fff',
        '#1a2b3c'
      ];
      final ops = ['==', '!=', '>=', '<=', ':', ';', '{', '}', '(', ')', ','];

      for (var i = 0; i < 200; i++) {
        final parts = <String>[];
        final numTokens = rng.nextInt(12) + 2;
        for (var j = 0; j < numTokens; j++) {
          final choice = rng.nextInt(4);
          switch (choice) {
            case 0:
              parts.add(vars[rng.nextInt(vars.length)]);
              break;
            case 1:
              parts.add(idents[rng.nextInt(idents.length)]);
              break;
            case 2:
              parts.add(values[rng.nextInt(values.length)]);
              break;
            case 3:
              parts.add(ops[rng.nextInt(ops.length)]);
              break;
          }
        }
        final css = parts.join(' ');
        final tokens = tokenizeLattice(css);
        expect(tokens, isNotEmpty);
        expect(tokens.last.type, equals('EOF'));
      }
    });

    test('10,000-iteration mutation fuzz loop rejects or tokenizes safely', () {
      final rng = math.Random(1337);
      final alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-\$@%:;{}(),.\'"/# \t\n';

      for (var iter = 0; iter < 10000; iter++) {
        final length = rng.nextInt(20) + 1;
        final buffer = StringBuffer();
        for (var i = 0; i < length; i++) {
          buffer.write(alphabet[rng.nextInt(alphabet.length)]);
        }
        final candidate = buffer.toString();
        try {
          final tokens = tokenizeLattice(candidate);
          expect(tokens.last.type, equals('EOF'));
        } on LatticeLexicalException {
          // Expected on unrecognized syntax fragments
        }
      }
    });
  });
}
