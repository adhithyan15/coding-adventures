import 'dart:math' as math;

import 'package:coding_adventures_sql_lexer/sql_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('5-Class SQL Lexer Security Resilience Showcase', () {
    test('Class 1: CWE-400 Source Length DoS', () {
      // 1 MB ceiling
      final atLimit = ' ' * maxSourceLength;
      expect(() => tokenizeSql(atLimit), returnsNormally);

      final overLimit = ' ' * (maxSourceLength + 1);
      expect(
        () => tokenizeSql(overLimit),
        throwsA(isA<InvalidSqlSourceException>().having(
          (e) => e.message,
          'message',
          messageSourceTooLarge,
        )),
      );
      expect(() => createSqlLexer(overLimit),
          throwsA(isA<InvalidSqlSourceException>()));
    });

    test('Class 2: Surrogate Smuggling & Encoding Attacks', () {
      final loneLead = String.fromCharCode(0xd800);
      final loneTrail = String.fromCharCode(0xdc00);
      final reversedPair = String.fromCharCodes([0xdc00, 0xd800]);
      final midSurrogate = 'SELECT ${loneLead}1';

      for (final invalid in [loneLead, loneTrail, reversedPair, midSurrogate]) {
        expect(
          () => tokenizeSql(invalid),
          throwsA(isA<InvalidSqlSourceException>().having(
            (e) => e.message,
            'message',
            messageInvalidSurrogates,
          )),
        );
      }
    });

    test('Class 3: Case-Insensitive Keyword Normalization', () {
      for (final query in [
        'select id from users',
        'SELECT id FROM users',
        'SeLeCt id FrOm users',
      ]) {
        final tokens = tokenizeSql(query);
        expect(tokens[0].type, equals('KEYWORD'));
        expect(tokens[0].value, equals('SELECT'));
        expect(tokens[1].type, equals('NAME'));
        expect(tokens[1].value, equals('id')); // exact case preserved
        expect(tokens[2].type, equals('KEYWORD'));
        expect(tokens[2].value, equals('FROM'));
        expect(tokens[3].type, equals('NAME'));
        expect(tokens[3].value, equals('users'));
      }
    });

    test('Class 4: Operator & Identifier Aliasing Integrity', () {
      // Inequality operators != and <> both alias to NOT_EQUALS
      final t1 = tokenizeSql('x != y');
      final t2 = tokenizeSql('x <> y');
      expect(t1[1].type, equals('NOT_EQUALS'));
      expect(t1[1].value, equals('!='));
      expect(t2[1].type, equals('NOT_EQUALS'));
      expect(t2[1].value, equals('<>'));

      // Comparison operators <=, >=, =, <, >
      final ops = tokenizeSql('a <= b >= c = d < e > f');
      expect(ops[1].type, equals('LESS_EQUALS'));
      expect(ops[3].type, equals('GREATER_EQUALS'));
      expect(ops[5].type, equals('EQUALS'));
      expect(ops[7].type, equals('LESS_THAN'));
      expect(ops[9].type, equals('GREATER_THAN'));

      // Quoted identifiers: backticks and SQLite double quotes
      final quoted = tokenizeSql('`my table` "my col" "col""name"');
      expect(quoted[0].type, equals('NAME'));
      expect(quoted[0].value, equals('`my table`'));
      expect(quoted[1].type, equals('NAME'));
      expect(quoted[1].value, equals('my col')); // stripped outer quotes
      expect(quoted[2].type, equals('NAME'));
      expect(quoted[2].value, equals('col"name')); // unescaped double-quotes
    });

    test('Class 5: CWE-209 Payload-Blind Confidentiality', () {
      // Lexical error does not reflect caller SQL payload
      try {
        tokenizeSql('SELECT @unknown_symbol');
        fail('Expected SqlLexicalException');
      } on SqlLexicalException catch (e) {
        expect(e.errorCode, equals(SqlLexerErrorCode.lexicalError));
        expect(e.errorId, equals('lexical-error'));
        expect(e.message, equals(messageLexicalError));
        expect(e.toString(), contains('Lexical analysis failed'));
        expect(e.toString().contains('@unknown_symbol'), isFalse);
        expect(e.line, equals(1));
        expect(e.column, equals(8));
      }

      // Invalid version error
      try {
        tokenizeSql('SELECT 1', version: 'sql99_unsupported');
        fail('Expected InvalidSqlVersionException');
      } on InvalidSqlVersionException catch (e) {
        expect(e.errorCode, equals(SqlLexerErrorCode.invalidVersion));
        expect(e.errorId, equals('invalid-version'));
        expect(e.message, equals(messageInvalidVersion));
        expect(e.toString().contains('sql99'), isFalse);
      }
    });
  });

  group('SQL Lexer Additional Capabilities', () {
    test('skips line and block comments seamlessly', () {
      const sqlWithComments = '''
        -- Line comment before query
        SELECT 42 /* block comment in middle */ FROM tbl; -- trailing comment
      ''';
      final tokens = tokenizeSql(sqlWithComments);
      expect(
          tokens.map((t) => t.type).toList(),
          equals([
            'KEYWORD', // SELECT
            'NUMBER', // 42
            'KEYWORD', // FROM
            'NAME', // tbl
            'SEMICOLON',
            'EOF',
          ]));
    });

    test('tokenizes strings, arithmetic, and punctuation', () {
      const sql = "SELECT 'hello world', 3.14 + 2 * (10 / 5) % 1;";
      final tokens = tokenizeSql(sql);
      expect(tokens[0].value, equals('SELECT'));
      expect(tokens[1].type, equals('STRING'));
      expect(tokens[1].value, equals('hello world'));
      expect(tokens[2].type, equals('COMMA'));
      expect(tokens[3].type, equals('NUMBER'));
      expect(tokens[3].value, equals('3.14'));
      expect(tokens[4].type, equals('PLUS'));
      expect(tokens[5].type, equals('NUMBER'));
      expect(tokens[6].type, equals('STAR'));
      expect(tokens[7].type, equals('LPAREN'));
      expect(tokens[8].type, equals('NUMBER'));
      expect(tokens[9].type, equals('SLASH'));
      expect(tokens[10].type, equals('NUMBER'));
      expect(tokens[11].type, equals('RPAREN'));
      expect(tokens[12].type, equals('PERCENT'));
      expect(tokens[13].type, equals('NUMBER'));
      expect(tokens[14].type, equals('SEMICOLON'));
    });

    test('aliases and factory functions createSqlLexer / tokenizeSQL', () {
      final lexer = createSQLLexer('SELECT 1');
      final tokens = lexer.tokenize();
      expect(tokens.length, equals(3)); // SELECT, 1, EOF

      final tokens2 = tokenizeSQL('SELECT 1');
      expect(tokens2.length, equals(3));
    });

    test('loadSqlTokenGrammar and resolveSqlVersion validation', () {
      expect(resolveSqlVersion('sql'), equals('sql'));
      expect(resolveSqlVersion('SQL'), equals('sql'));
      expect(loadSqlTokenGrammar(), isNotNull);
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test ensures robust tokenization', () {
      final rng = math.Random(42);
      final keywords = [
        'SELECT',
        'FROM',
        'WHERE',
        'INSERT',
        'INTO',
        'UPDATE',
        'DELETE',
        'CREATE',
        'TABLE'
      ];
      final names = ['users', 'orders', 'id', 'price', 'name', 'col_a', 'data'];
      final numbers = ['0', '1', '42', '100', '99.9', '3.14159'];
      final ops = ['=', '!=', '<>', '<', '>', '<=', '>=', '+', '-', '*', '/'];

      for (var i = 0; i < 200; i++) {
        final parts = <String>[];
        final numTokens = rng.nextInt(10) + 2;
        for (var j = 0; j < numTokens; j++) {
          final choice = rng.nextInt(5);
          switch (choice) {
            case 0:
              final kw = keywords[rng.nextInt(keywords.length)];
              parts.add(rng.nextBool() ? kw.toLowerCase() : kw);
              break;
            case 1:
              parts.add(names[rng.nextInt(names.length)]);
              break;
            case 2:
              parts.add(numbers[rng.nextInt(numbers.length)]);
              break;
            case 3:
              parts.add(ops[rng.nextInt(ops.length)]);
              break;
            case 4:
              parts.add("'str_${rng.nextInt(100)}'");
              break;
          }
        }
        final sql = parts.join(' ');
        final tokens = tokenizeSql(sql);
        expect(tokens, isNotEmpty);
        expect(tokens.last.type, equals('EOF'));
      }
    });

    test('10,000-iteration mutation fuzz loop rejects or tokenizes safely', () {
      final rng = math.Random(1337);
      final alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_ \t\n;,()+-*/%=<>`\'"';

      for (var iter = 0; iter < 10000; iter++) {
        final length = rng.nextInt(20) + 1;
        final buffer = StringBuffer();
        for (var i = 0; i < length; i++) {
          buffer.write(alphabet[rng.nextInt(alphabet.length)]);
        }
        final candidate = buffer.toString();
        try {
          final tokens = tokenizeSql(candidate);
          expect(tokens.last.type, equals('EOF'));
        } on SqlLexicalException {
          // Expected on unrecognized syntax fragments
        }
      }
    });
  });
}
