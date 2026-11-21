import 'dart:math' as math;

import 'package:coding_adventures_sql_lexer/sql_lexer.dart'
    show SqlLexerException;
import 'package:coding_adventures_sql_parser/sql_parser.dart';
import 'package:test/test.dart';

void main() {
  group('5-Class SQL Parser Security Resilience Showcase', () {
    test('Class 1: Version Resolution & Domain Isolation', () {
      expect(resolveSqlVersion('sql'), equals('sql'));
      expect(resolveSqlVersion('SQL'), equals('sql'));
      expect(resolveSqlVersion('  Sql  '), equals('sql'));

      for (final invalid in ['sql99', 'postgres', 'mysql', 'sqlite', '']) {
        expect(
          () => resolveSqlVersion(invalid),
          throwsA(
            isA<InvalidSqlParserVersionException>().having(
              (e) => e.errorId,
              'errorId',
              errorIdInvalidVersion,
            ),
          ),
        );
        expect(
          () => parseSql('SELECT 1', version: invalid),
          throwsA(isA<InvalidSqlParserVersionException>()),
        );
      }
    });

    test('Class 2: CWE-400 Resource Exhaustion via Token Ceiling', () {
      // Stream exceeding maxTokenCount (65,536) is rejected before grammar parsing
      final oversizedTokens = List.generate(
        maxTokenCount + 1,
        (i) => Token(type: 'NAME', value: 'col$i', line: 1, column: i + 1),
      );

      expect(
        () => parseSqlTokens(oversizedTokens),
        throwsA(
          isA<SqlParseException>()
              .having(
                (e) => e.errorCode,
                'errorCode',
                SqlParserErrorCode.tokenCountExceeded,
              )
              .having((e) => e.errorId, 'errorId', errorIdTokenCountExceeded),
        ),
      );
    });

    test(
      'Class 3: CWE-674 Recursion Depth Defense (Call Stack Protection)',
      () {
        // Nesting depth > 256 is rejected defensively before recursive descent
        final excessiveLeft = '(' * (maxNestingDepth + 1);
        final excessiveRight = ')' * (maxNestingDepth + 1);
        final dangerousQuery = 'SELECT $excessiveLeft 1 $excessiveRight FROM t';

        expect(
          () => parseSql(dangerousQuery),
          throwsA(
            isA<SqlParseException>()
                .having(
                  (e) => e.errorCode,
                  'errorCode',
                  SqlParserErrorCode.nestingTooDeep,
                )
                .having((e) => e.errorId, 'errorId', errorIdNestingTooDeep),
          ),
        );

        // Deep but safe nesting (<= 256) is accepted
        final safeLeft = '(' * 16;
        final safeRight = ')' * 16;
        final safeQuery = 'SELECT $safeLeft 1 $safeRight FROM t';
        expect(() => parseSql(safeQuery), returnsNormally);
      },
    );

    test('Class 4: CWE-209 Payload-Blind Diagnostics & Confidentiality', () {
      try {
        parseSql('SELECT WHERE secret_col = 1234');
        fail('Expected SqlParseException on invalid syntax');
      } on SqlParseException catch (e) {
        expect(e.errorCode, equals(SqlParserErrorCode.syntaxError));
        expect(e.errorId, equals(errorIdSyntaxError));
        expect(e.message, equals(messageSyntaxError));
        // Error string must NEVER contain caller text or payload values
        final str = e.toString();
        expect(str, contains(messageSyntaxError));
        expect(str.contains('secret_col'), isFalse);
        expect(str.contains('1234'), isFalse);
        expect(str.contains('WHERE'), isFalse);
        expect(e.line, isNotNull);
        expect(e.column, isNotNull);
      }
    });

    test('Class 5: AST Structural Integrity & Hierarchy', () {
      final ast = parseSql('SELECT id, name FROM users WHERE id = 1');
      expect(ast.ruleName, equals('program'));
      expect(ast.children, isNotEmpty);
      final stmt = ast.children.first as ASTNode;
      expect(stmt.ruleName, equals('statement'));
    });
  });

  group('SQL Grammar Comprehensive Statement Coverage', () {
    test('SELECT with WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET', () {
      const sql = '''
        SELECT DISTINCT category, count(*)
        FROM products
        WHERE price > 50 AND in_stock = 1
        GROUP BY category
        HAVING count(*) >= 2
        ORDER BY category ASC, price DESC
        LIMIT 10 OFFSET 5;
      ''';
      final ast = parseSql(sql);
      expect(ast.ruleName, equals('program'));
    });

    test('JOIN clauses: INNER, LEFT, RIGHT, FULL, CROSS', () {
      for (final joinSql in [
        'SELECT * FROM a JOIN b ON a.id = b.a_id',
        'SELECT * FROM a INNER JOIN b ON a.id = b.a_id',
        'SELECT * FROM a LEFT JOIN b ON a.id = b.a_id',
        'SELECT * FROM a LEFT OUTER JOIN b ON a.id = b.a_id',
        'SELECT * FROM a RIGHT JOIN b ON a.id = b.a_id',
        'SELECT * FROM a RIGHT OUTER JOIN b ON a.id = b.a_id',
        'SELECT * FROM a FULL JOIN b ON a.id = b.a_id',
        'SELECT * FROM a FULL OUTER JOIN b ON a.id = b.a_id',
        'SELECT * FROM a CROSS JOIN b',
      ]) {
        final ast = parseSql(joinSql);
        expect(ast.ruleName, equals('program'), reason: 'Failed for $joinSql');
      }
    });

    test('INSERT statements: with column list and without column list', () {
      final ast1 = parseSql("INSERT INTO users (id, name) VALUES (1, 'Alice')");
      expect(ast1.ruleName, equals('program'));

      final ast2 = parseSql("INSERT INTO users VALUES (2, 'Bob')");
      expect(ast2.ruleName, equals('program'));
    });

    test('UPDATE statements: with and without WHERE clause', () {
      final ast1 = parseSql(
        "UPDATE users SET name = 'Bob', age = 30 WHERE id = 1",
      );
      expect(ast1.ruleName, equals('program'));

      final ast2 = parseSql('UPDATE counters SET value = value + 1');
      expect(ast2.ruleName, equals('program'));
    });

    test('DELETE statements: with and without WHERE clause', () {
      final ast1 = parseSql('DELETE FROM users WHERE age < 18');
      expect(ast1.ruleName, equals('program'));

      final ast2 = parseSql('DELETE FROM temp_logs');
      expect(ast2.ruleName, equals('program'));
    });

    test('CREATE TABLE and DROP TABLE statements', () {
      final createSql = '''
        CREATE TABLE IF NOT EXISTS accounts (
          id INT PRIMARY KEY,
          username VARCHAR(50) NOT NULL,
          balance FLOAT DEFAULT 0.0
        );
      ''';
      expect(parseSql(createSql).ruleName, equals('program'));

      expect(parseSql('DROP TABLE accounts;').ruleName, equals('program'));
      expect(
        parseSql('DROP TABLE IF EXISTS accounts;').ruleName,
        equals('program'),
      );
    });

    test(
      'Rich Expressions: BETWEEN, IN, LIKE, IS NULL, IS NOT NULL, arithmetic',
      () {
        for (final exprSql in [
          'SELECT * FROM t WHERE val BETWEEN 10 AND 100',
          "SELECT * FROM t WHERE status IN ('active', 'pending', 'review')",
          "SELECT * FROM t WHERE name LIKE 'Alice%'",
          'SELECT * FROM t WHERE deleted_at IS NULL',
          'SELECT * FROM t WHERE restored_at IS NOT NULL',
          'SELECT -1 + 2 * (3 / 4) % 5 FROM t',
          'SELECT a AND b OR NOT c FROM t',
          'SELECT x != y AND a <> b AND c <= d AND e >= f FROM t',
        ]) {
          expect(
            parseSql(exprSql).ruleName,
            equals('program'),
            reason: 'Failed expr: $exprSql',
          );
        }
      },
    );

    test('Case-insensitive keywords and casing variations', () {
      final lower = parseSQL('select id from users where id = 1');
      final upper = parseSQL('SELECT id FROM users WHERE id = 1');
      final mixed = parseSQL('SeLeCt id FrOm users WhErE id = 1');

      expect(lower.ruleName, equals('program'));
      expect(upper.ruleName, equals('program'));
      expect(mixed.ruleName, equals('program'));
    });

    test('Trace options and parser factory', () {
      final traceOutput = <String>[];
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (msg) => traceOutput.add(msg),
      );

      final parser = createSQLParser('SELECT 1', options: options);
      final ast = parser.parse();
      expect(ast.ruleName, equals('program'));
      expect(traceOutput, isNotEmpty);
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test verifies valid query generation', () {
      final rng = math.Random(42);
      final tables = ['users', 'orders', 'items', 'logs', 'metrics'];
      final columns = ['id', 'name', 'status', 'score', 'created_at', 'total'];
      final ops = ['=', '!=', '<>', '<', '>', '<=', '>='];

      for (var i = 0; i < 200; i++) {
        final table = tables[rng.nextInt(tables.length)];
        final col = columns[rng.nextInt(columns.length)];
        final op = ops[rng.nextInt(ops.length)];
        final val = rng.nextInt(1000);

        String sql;
        final template = rng.nextInt(6);
        switch (template) {
          case 0:
            sql = 'SELECT $col FROM $table WHERE $col $op $val';
            break;
          case 1:
            sql = 'SELECT DISTINCT $col FROM $table ORDER BY $col ASC LIMIT 10';
            break;
          case 2:
            sql = "INSERT INTO $table ($col) VALUES ('val_$val')";
            break;
          case 3:
            sql = 'UPDATE $table SET $col = $val WHERE id = 1';
            break;
          case 4:
            sql = 'DELETE FROM $table WHERE $col $op $val';
            break;
          default:
            sql =
                'CREATE TABLE IF NOT EXISTS ${table}_$i ($col INT PRIMARY KEY)';
            break;
        }

        final ast = parseSql(sql);
        expect(ast.ruleName, equals('program'), reason: 'Failed case $i: $sql');
      }
    });

    test(
        '10,000-iteration mutation fuzz loop ensures memory and exception safety',
        () {
      final rng = math.Random(1337);
      final seeds = [
        'SELECT 1 FROM t',
        'SELECT id, name FROM users WHERE id = 10',
        "INSERT INTO t (col) VALUES ('val')",
        'UPDATE t SET col = 1 WHERE id = 2',
        'DELETE FROM t WHERE id = 3',
        'CREATE TABLE t (id INT)',
        'DROP TABLE t',
      ];
      final alphabet =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_ \t\n;,()+-*/%=<>`\'"';

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
            case 0: // Replace char
              chars[pos] = alphabet[rng.nextInt(alphabet.length)];
              break;
            case 1: // Delete char
              chars.removeAt(pos);
              break;
            case 2: // Insert char
              chars.insert(pos, alphabet[rng.nextInt(alphabet.length)]);
              break;
            case 3: // Insert parenthesis
              chars.insert(pos, rng.nextBool() ? '(' : ')');
              break;
          }
        }

        final mutant = chars.join();
        try {
          final ast = parseSql(mutant);
          expect(ast.ruleName, equals('program'));
        } on SqlParseException {
          // Expected typed syntax error or limit violation
        } on SqlLexerException {
          // Expected typed lexical error
        }
      }
    });
  });
}
