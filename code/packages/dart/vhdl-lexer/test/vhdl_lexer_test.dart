import 'dart:math' as math;

import 'package:coding_adventures_vhdl_lexer/coding_adventures_vhdl_lexer.dart';
import 'package:test/test.dart';

List<String> _types(String source, {String? version}) =>
    tokenizeVhdl(source, version: version).map((t) => t.type).toList();

List<String> _values(String source, {String? version}) =>
    tokenizeVhdl(source, version: version).map((t) => t.value).toList();

void main() {
  group('5-Class VHDL Lexer Security Resilience Showcase', () {
    test('Class 1: CWE-400 Source Length DoS Defense', () {
      final atLimit = ' ' * maxSourceLength;
      expect(() => tokenizeVhdl(atLimit), returnsNormally);

      final overLimit = ' ' * (maxSourceLength + 1);
      expect(
        () => tokenizeVhdl(overLimit),
        throwsA(isA<InvalidVhdlSourceException>().having(
          (e) => e.message,
          'message',
          messageSourceTooLarge,
        )),
      );
      expect(() => createVhdlLexer(overLimit),
          throwsA(isA<InvalidVhdlSourceException>()));
    });

    test('Class 2: Surrogate Smuggling & Version-Precedence', () {
      final loneLead = String.fromCharCode(0xd800);
      final loneTrail = String.fromCharCode(0xdc00);
      final reversedPair = String.fromCharCodes([0xdc00, 0xd800]);
      final midSurrogate = 'entity e$loneLead is end;';

      for (final invalid in [loneLead, loneTrail, reversedPair, midSurrogate]) {
        expect(
          () => tokenizeVhdl(invalid),
          throwsA(isA<InvalidVhdlSourceException>().having(
            (e) => e.message,
            'message',
            messageInvalidSurrogates,
          )),
        );
      }

      // Source bounds are validated before the version is resolved.
      expect(
        () => tokenizeVhdl(loneLead, version: 'not-a-version'),
        throwsA(isA<InvalidVhdlSourceException>()),
      );
    });

    test('Class 3: Edition Selection & Domain Isolation', () {
      expect(resolveVhdlVersion(), equals('2008'));
      expect(resolveVhdlVersion(null), equals('2008'));
      expect(resolveVhdlVersion(''), equals('2008'));
      expect(resolveVhdlVersion('   '), equals('2008'));
      expect(supportedVhdlVersions,
          equals(['1987', '1993', '2002', '2008', '2019']));

      for (final year in supportedVhdlVersions) {
        expect(resolveVhdlVersion(year), equals(year));
        expect(resolveVhdlVersion('vhdl$year'), equals(year));
        expect(resolveVhdlVersion('  VHDL$year  '), equals(year));
        expect(loadVhdlTokenGrammar(version: year).keywords, isNotEmpty);
      }

      for (final invalid in ['vhdl', '2005', 'verilog', 'vhdl-2008', '08']) {
        expect(
          () => resolveVhdlVersion(invalid),
          throwsA(isA<InvalidVhdlVersionException>().having(
            (e) => e.errorId,
            'errorId',
            errorIdInvalidVersion,
          )),
        );
        expect(() => tokenizeVhdl('entity', version: invalid),
            throwsA(isA<InvalidVhdlVersionException>()));
      }
    });

    test('Class 4: Case-Insensitive Identity & Keyword Retyping', () {
      expect(
          _types('ENTITY Foo IS END;'),
          equals(
              ['KEYWORD', 'NAME', 'KEYWORD', 'KEYWORD', 'SEMICOLON', 'EOF']));
      expect(_values('ENTITY Foo IS END;'),
          equals(['entity', 'foo', 'is', 'end', ';', '']));
      expect(_values('Entity ENTITY entity'),
          equals(['entity', 'entity', 'entity', '']));

      // Non-identifier tokens are passed through unchanged.
      final tokens = tokenizeVhdl('16#FF# X"AF" 1.5e3 \'A\' clk\'event');
      expect(
        tokens.map((t) => t.type).toList(),
        equals([
          'BASED_LITERAL',
          'BIT_STRING',
          'REAL_NUMBER',
          'CHAR_LITERAL',
          'NAME',
          'TICK',
          'NAME',
          'EOF',
        ]),
      );
      expect(tokens.first.value, equals('16#FF#'));
      expect(tokens[1].value, equals('X"AF"'));

      // Operators that overload meaning stay single tokens.
      expect(
          _types('a <= b; x := a ** 2 & b; c /= d'), contains('LESS_EQUALS'));
      expect(_types('x := a ** 2 & b;'),
          containsAll(['VAR_ASSIGN', 'POWER', 'AMPERSAND']));

      // Every edition lowers identifiers the same way.
      for (final year in supportedVhdlVersions) {
        expect(_values('ENTITY E', version: year), equals(['entity', 'e', '']));
      }
    });

    test('Class 5: CWE-209 Payload-Blind Confidentiality', () {
      try {
        tokenizeVhdl('entity secret_payload is end; ~');
        fail('Expected VhdlLexicalException');
      } on VhdlLexicalException catch (e) {
        expect(e.errorCode, equals(VhdlLexerErrorCode.lexicalError));
        expect(e.errorId, equals('lexical-error'));
        expect(e.message, equals(messageLexicalError));
        final str = e.toString();
        expect(str, contains(messageLexicalError));
        expect(str.contains('secret'), isFalse);
        expect(e.line, equals(1));
        expect(e.column, equals(31));
      }

      try {
        tokenizeVhdl('entity e is end;', version: 'secret-edition');
        fail('Expected InvalidVhdlVersionException');
      } on InvalidVhdlVersionException catch (e) {
        expect(e.errorCode, equals(VhdlLexerErrorCode.invalidVersion));
        expect(e.errorId, equals('invalid-version'));
        expect(e.message, equals(messageInvalidVersion));
        expect(e.toString().contains('secret'), isFalse);
      }
    });
  });

  group('VHDL Lexer Additional Coverage', () {
    test('Comments, strings and extended identifiers', () {
      final tokens = tokenizeVhdl('a <= b; -- trailing comment\n'
          r'\Ext Id\ "str""q"');
      final types = tokens.map((t) => t.type).toList();
      expect(types, containsAll(['EXTENDED_IDENT', 'STRING']));
      expect(types.contains('COMMENT'), isFalse);
    });

    test('Lexer object, aliases and exception formatting', () {
      final lexer = createVhdlLexer('ENTITY e IS END;', version: 'vhdl1993');
      expect(lexer.version, equals('1993'));
      expect(lexer.source, equals('ENTITY e IS END;'));
      expect(lexer.tokenize(),
          equals(tokenizeVhdlLexer('ENTITY e IS END;', version: '1993')));
      expect(createVhdlLexer('x').version, equals('2008'));

      expect(const InvalidVhdlVersionException().toString(),
          equals(messageInvalidVersion));
      expect(const VhdlLexicalException(line: 2, column: 3).toString(),
          equals('$messageLexicalError (2:3)'));
      expect(formatLocation('msg', 3, null), equals('msg'));
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test ensures robust, case-stable lexing', () {
      final rng = math.Random(42);
      final words = [
        'entity',
        'ARCHITECTURE',
        'Signal',
        'clk',
        'Data_Bus',
        'begin',
        'PROCESS',
        'if',
      ];
      final symbols = ['<=', ':=', ';', ':', '(', ')', '+', '&', ','];
      final literals = ['0', '42', '16#FF#', 'X"AF"', "'1'", '"text"'];

      for (var i = 0; i < 200; i++) {
        final parts = <String>[];
        final count = rng.nextInt(12) + 2;
        for (var j = 0; j < count; j++) {
          switch (rng.nextInt(3)) {
            case 0:
              parts.add(words[rng.nextInt(words.length)]);
            case 1:
              parts.add(symbols[rng.nextInt(symbols.length)]);
            default:
              parts.add(literals[rng.nextInt(literals.length)]);
          }
        }
        final source = parts.join(' ');
        final tokens = tokenizeVhdl(source);
        expect(tokens.last.type, equals('EOF'));

        // Case folding must not change the token structure.
        final lowered = tokenizeVhdl(source.toLowerCase());
        final shapes = tokens.map((t) => t.type).toList();
        final loweredShapes = lowered.map((t) => t.type).toList();
        if (!source.contains('X"') && !source.contains('16#')) {
          expect(loweredShapes, equals(shapes), reason: source);
        }
      }
    });

    test('10,000-iteration mutation fuzz loop rejects or tokenizes safely', () {
      final rng = math.Random(1337);
      const alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_#"\'\\-<=>:;,.()&+*/ \t\n';

      for (var iter = 0; iter < 10000; iter++) {
        final length = rng.nextInt(24) + 1;
        final buffer = StringBuffer();
        for (var i = 0; i < length; i++) {
          buffer.write(alphabet[rng.nextInt(alphabet.length)]);
        }
        try {
          expect(tokenizeVhdl(buffer.toString()).last.type, equals('EOF'));
        } on VhdlLexicalException {
          // Expected on unrecognized syntax fragments.
        }
      }
    });
  });
}
