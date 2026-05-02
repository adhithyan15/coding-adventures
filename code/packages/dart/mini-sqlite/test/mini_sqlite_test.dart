import 'package:coding_adventures_mini_sqlite/mini_sqlite.dart';
import 'package:test/test.dart';

void main() {
  group('MiniSqlite', () {
    test('exposes DB API style constants', () {
      expect(MiniSqlite.apiLevel, '2.0');
      expect(MiniSqlite.threadSafety, 1);
      expect(MiniSqlite.paramStyle, 'qmark');
    });

    test('creates inserts and selects rows', () {
      final conn = MiniSqlite.connect(':memory:');
      conn.execute(
        'CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)',
      );
      conn.executeMany('INSERT INTO users VALUES (?, ?, ?)', [
        [1, 'Alice', true],
        [2, 'Bob', false],
        [3, 'Carol', true],
      ]);

      final cursor = conn.execute(
        'SELECT name FROM users WHERE active = ? ORDER BY id ASC',
        [true],
      );

      expect(cursor.description.single.name, 'name');
      final rows = cursor.fetchAll();
      expect(rows[0][0], 'Alice');
      expect(rows[1][0], 'Carol');
    });

    test('fetches incrementally', () {
      final conn = MiniSqlite.connect(':memory:');
      conn.execute('CREATE TABLE nums (n INTEGER)');
      conn.executeMany('INSERT INTO nums VALUES (?)', [
        [1],
        [2],
        [3],
      ]);

      final cursor = conn.execute('SELECT n FROM nums ORDER BY n ASC');
      expect(cursor.fetchOne()![0], 1);
      expect(cursor.fetchMany(1)[0][0], 2);
      expect(cursor.fetchAll()[0][0], 3);
      expect(cursor.fetchOne(), isNull);
    });

    test('updates and deletes rows', () {
      final conn = MiniSqlite.connect(':memory:');
      conn.execute('CREATE TABLE users (id INTEGER, name TEXT)');
      conn.executeMany('INSERT INTO users VALUES (?, ?)', [
        [1, 'Alice'],
        [2, 'Bob'],
        [3, 'Carol'],
      ]);

      final updated = conn.execute('UPDATE users SET name = ? WHERE id = ?', [
        'Bobby',
        2,
      ]);
      expect(updated.rowCount, 1);

      final deleted = conn.execute('DELETE FROM users WHERE id IN (?, ?)', [
        1,
        3,
      ]);
      expect(deleted.rowCount, 2);

      final rows = conn.execute('SELECT id, name FROM users').fetchAll();
      expect(rows[0][0], 2);
      expect(rows[0][1], 'Bobby');
    });

    test('rolls back and commits snapshots', () {
      final conn = MiniSqlite.connect(':memory:');
      conn.execute('CREATE TABLE users (id INTEGER, name TEXT)');
      conn.commit();
      conn.execute('INSERT INTO users VALUES (?, ?)', [1, 'Alice']);
      conn.rollback();
      expect(conn.execute('SELECT * FROM users').fetchAll(), isEmpty);

      conn.execute('INSERT INTO users VALUES (?, ?)', [1, 'Alice']);
      conn.commit();
      conn.rollback();
      expect(conn.execute('SELECT * FROM users').fetchAll(), hasLength(1));
    });

    test('supports predicates ordering and drop', () {
      final conn = MiniSqlite.connect(':memory:');
      conn.execute(
        'CREATE TABLE things (id INTEGER, label TEXT, score REAL, enabled BOOLEAN)',
      );
      conn.execute("INSERT INTO things VALUES (1, NULL, 1.5, TRUE)");
      conn.execute("INSERT INTO things VALUES (2, 'middle', 2.5, FALSE)");
      conn.execute("INSERT INTO things VALUES (3, 'tail', 3.5, TRUE)");

      final nullOrHigh = conn
          .execute(
            'SELECT id FROM things WHERE label IS NULL OR score >= 3 ORDER BY id DESC',
          )
          .fetchAll();
      expect(nullOrHigh.map((row) => row[0]), [3, 1]);

      final filtered = conn
          .execute(
            'SELECT id FROM things WHERE label IS NOT NULL AND id <> 2 ORDER BY id ASC',
          )
          .fetchAll();
      expect(filtered.single[0], 3);

      conn.execute('DROP TABLE things');
      expect(
        () => conn.execute('SELECT * FROM things'),
        throwsA(
          isA<MiniSqliteException>().having(
            (error) => error.kind,
            'kind',
            'OperationalError',
          ),
        ),
      );
    });

    test('validates parameters cursor lifecycle and unsupported SQL', () {
      final conn = MiniSqlite.connect(':memory:');
      conn.execute('CREATE TABLE notes (id INTEGER, text TEXT)');

      final inserted = conn.execute(
        "INSERT INTO notes VALUES (?, 'literal ? with ''quote''')",
        [1],
      );
      expect(inserted.rowCount, 1);
      expect(inserted.lastRowId, 1);

      final cursor = conn.execute('SELECT text FROM notes');
      cursor.arraySize = 1;
      final batch = cursor.fetchMany();
      expect(batch, hasLength(1));
      expect(batch.single.single, "literal ? with 'quote'");
      cursor.close();
      expect(
        cursor.fetchAll,
        throwsA(
          isA<MiniSqliteException>().having(
            (error) => error.kind,
            'kind',
            'ProgrammingError',
          ),
        ),
      );

      expect(
        () => conn.execute('SELECT * FROM notes WHERE id = ?', []),
        throwsA(
          isA<MiniSqliteException>().having(
            (error) => error.kind,
            'kind',
            'ProgrammingError',
          ),
        ),
      );
      expect(
        () => conn.execute('SELECT * FROM notes', [1]),
        throwsA(
          isA<MiniSqliteException>().having(
            (error) => error.kind,
            'kind',
            'ProgrammingError',
          ),
        ),
      );
      expect(
        () => conn.execute('PRAGMA user_version'),
        throwsA(
          isA<MiniSqliteException>().having(
            (error) => error.kind,
            'kind',
            'OperationalError',
          ),
        ),
      );
    });

    test('supports SQL transaction commands and autocommit', () {
      final conn = MiniSqlite.connect(':memory:');
      conn.execute('CREATE TABLE events (id INTEGER)');
      conn.commit();
      conn.execute('BEGIN');
      conn.execute('INSERT INTO events VALUES (1)');
      conn.execute('ROLLBACK');
      expect(conn.execute('SELECT * FROM events').fetchAll(), isEmpty);

      conn.execute('BEGIN');
      conn.execute('INSERT INTO events VALUES (2)');
      conn.execute('COMMIT');
      conn.execute('ROLLBACK');
      expect(conn.execute('SELECT * FROM events').fetchAll(), hasLength(1));

      final autocommit = MiniSqlite.connect(
        ':memory:',
        options: const ConnectionOptions(autocommit: true),
      );
      autocommit.execute('CREATE TABLE events (id INTEGER)');
      autocommit.execute('INSERT INTO events VALUES (1)');
      autocommit.rollback();
      expect(
        autocommit.execute('SELECT * FROM events').fetchAll(),
        hasLength(1),
      );
    });

    test('rejects file backed connections', () {
      expect(
        () => MiniSqlite.connect('app.db'),
        throwsA(
          isA<MiniSqliteException>().having(
            (error) => error.kind,
            'kind',
            'NotSupportedError',
          ),
        ),
      );
    });
  });
}
