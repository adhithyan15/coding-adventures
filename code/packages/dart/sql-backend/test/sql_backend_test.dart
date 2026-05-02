import 'package:coding_adventures_sql_backend/sql_backend.dart';
import 'package:test/test.dart';

void main() {
  group('SqlValues', () {
    test('classifies SQL values', () {
      expect(SqlValues.typeName(null), 'NULL');
      expect(SqlValues.typeName(true), 'BOOLEAN');
      expect(SqlValues.typeName(42), 'INTEGER');
      expect(SqlValues.typeName(1.5), 'REAL');
      expect(SqlValues.typeName('hello'), 'TEXT');
      expect(SqlValues.typeName([1, 2]), 'BLOB');
      expect(SqlValues.isSqlValue(Object()), isFalse);
      expect(() => SqlValues.typeName(Object()), throwsArgumentError);
    });
  });

  group('iterators', () {
    test('return row copies and track current cursor rows', () {
      final source = [
        {'id': 1, 'name': 'Alice'},
      ];
      final iterator = ListRowIterator(source);
      final row = iterator.next()!;
      row['name'] = 'mutated';
      expect(source.single['name'], 'Alice');
      expect(iterator.next(), isNull);

      final backend = users();
      final cursor = backend.openCursor('users');
      expect(cursor.currentRow(), isNull);
      expect(cursor.next()!['id'], 1);
      expect(cursor.currentRow()!['id'], 1);
      cursor.close();
      expect(cursor.next(), isNull);
    });
  });

  group('schema', () {
    test('exposes column metadata and schema provider adapter', () {
      const pk = ColumnDef('id', 'INTEGER', primaryKey: true);
      const unique = ColumnDef('email', 'TEXT', unique: true);
      final withNullDefault = ColumnDef.withDefault('middle', 'TEXT', null);

      expect(pk.effectiveNotNull, isTrue);
      expect(pk.effectiveUnique, isTrue);
      expect(unique.effectiveNotNull, isFalse);
      expect(unique.effectiveUnique, isTrue);
      expect(withNullDefault.hasDefault, isTrue);
      expect(withNullDefault.defaultValue, isNull);

      final schema = BackendAdapters.asSchemaProvider(users());
      expect(schema.columns('users'), ['id', 'name', 'age', 'email']);
      expect(() => schema.columns('missing'), throwsA(isA<TableNotFound>()));
    });
  });

  group('InMemoryBackend', () {
    test('lists tables columns and scans rows', () {
      final backend = users();
      expect(backend.tables(), contains('users'));
      expect(backend.columns('users')[1].name, 'name');
      expect(collect(backend.scan('users')).map((row) => row['id']), [1, 2, 3]);
      expect(() => backend.columns('missing'), throwsA(isA<TableNotFound>()));
    });

    test('insert applies defaults and validates rows', () {
      final backend = InMemoryBackend();
      backend.createTable('items', [
        const ColumnDef('id', 'INTEGER', primaryKey: true),
        ColumnDef.withDefault('status', 'TEXT', 'active'),
      ], false);

      backend.insert('items', {'id': 1});
      expect(
        () => backend.insert('items', {'id': 2, 'ghost': 'x'}),
        throwsA(isA<ColumnNotFound>()),
      );
      expect(collect(backend.scan('items')).single['status'], 'active');
    });

    test('enforces primary key not null and unique constraints', () {
      final backend = users();
      expect(
        () => backend.insert('users', {
          'id': 1,
          'name': 'Dup',
          'age': 9,
          'email': 'dup@example.com',
        }),
        throwsA(isA<ConstraintViolation>()),
      );
      expect(
        () => backend.insert('users', {
          'id': 4,
          'name': null,
          'age': 9,
          'email': 'dup@example.com',
        }),
        throwsA(isA<ConstraintViolation>()),
      );
      expect(
        () => backend.insert('users', {
          'id': 4,
          'name': 'Dup',
          'age': 9,
          'email': 'alice@example.com',
        }),
        throwsA(isA<ConstraintViolation>()),
      );
    });

    test('allows multiple nulls in unique columns', () {
      final backend = InMemoryBackend();
      backend.createTable('users', const [
        ColumnDef('id', 'INTEGER', primaryKey: true),
        ColumnDef('email', 'TEXT', unique: true),
      ], false);
      backend.insert('users', {'id': 1, 'email': null});
      backend.insert('users', {'id': 2, 'email': null});
      expect(collect(backend.scan('users')), hasLength(2));
    });

    test('updates and deletes positioned cursor rows', () {
      final backend = users();
      final cursor = backend.openCursor('users');
      expect(cursor.next()!['id'], 1);

      backend.update('users', cursor, {'name': 'ALICE'});
      expect(backend.openCursor('users').next()!['name'], 'ALICE');

      backend.delete('users', cursor);
      expect(backend.openCursor('users').next()!['id'], 2);
      expect(
        () => backend.update('users', cursor, {'name': 'x'}),
        throwsA(isA<Unsupported>()),
      );
    });

    test('creates drops and alters tables', () {
      final backend = InMemoryBackend();
      backend.createTable('t', const [ColumnDef('id', 'INTEGER')], false);
      backend.createTable('t', const [], true);
      expect(
        () => backend.createTable('t', const [], false),
        throwsA(isA<TableAlreadyExists>()),
      );

      backend.addColumn('t', ColumnDef.withDefault('status', 'TEXT', 'new'));
      backend.insert('t', {'id': 1});
      expect(collect(backend.scan('t')).single['status'], 'new');
      expect(
        () => backend.addColumn('t', const ColumnDef('status', 'TEXT')),
        throwsA(isA<ColumnAlreadyExists>()),
      );
      expect(
        () => backend.addColumn(
          't',
          const ColumnDef('required', 'TEXT', notNull: true),
        ),
        throwsA(isA<ConstraintViolation>()),
      );

      backend.dropTable('t', false);
      backend.dropTable('t', true);
      expect(() => backend.dropTable('t', false), throwsA(isA<TableNotFound>()));
    });

    test('commits rolls back and rejects stale transaction handles', () {
      final backend = users();
      final handle = backend.beginTransaction();
      backend.insert('users', {
        'id': 4,
        'name': 'Dave',
        'age': 41,
        'email': 'dave@example.com',
      });
      backend.rollback(handle);
      expect(
        collect(backend.scan('users')).any((row) => row['id'] == 4),
        isFalse,
      );

      final committed = backend.beginTransaction();
      backend.insert('users', {
        'id': 4,
        'name': 'Dave',
        'age': 41,
        'email': 'dave@example.com',
      });
      backend.commit(committed);
      expect(
        collect(backend.scan('users')).any((row) => row['id'] == 4),
        isTrue,
      );

      final active = backend.beginTransaction();
      expect(backend.currentTransaction(), active);
      expect(backend.beginTransaction, throwsA(isA<Unsupported>()));
      backend.commit(active);
      expect(() => backend.commit(active), throwsA(isA<Unsupported>()));
    });

    test('lists scans and drops indexes', () {
      final backend = users();
      backend.createIndex(
        const IndexDef('idx_age', 'users', columns: ['age']),
      );

      expect(backend.listIndexes('users').single.name, 'idx_age');
      final rowids = backend.scanIndex('idx_age', [25], [30]).toList();
      expect(rowids, [1, 0]);
      expect(
        collect(backend.scanByRowids('users', rowids)).map((row) => row['id']),
        [2, 1],
      );

      backend.dropIndex('idx_age');
      expect(backend.listIndexes(), isEmpty);
      expect(() => backend.dropIndex('idx_age'), throwsA(isA<IndexNotFound>()));
      backend.dropIndex('idx_age', ifExists: true);
    });

    test('validates index inputs', () {
      final backend = users();
      backend.createIndex(
        const IndexDef('idx_email', 'users', columns: ['email'], unique: true),
      );

      expect(
        () => backend.createIndex(
          const IndexDef('idx_email', 'users', columns: ['email']),
        ),
        throwsA(isA<IndexAlreadyExists>()),
      );
      expect(
        () => backend.createIndex(
          const IndexDef('idx_missing', 'missing', columns: ['id']),
        ),
        throwsA(isA<TableNotFound>()),
      );
      expect(
        () => backend.createIndex(
          const IndexDef('idx_bad', 'users', columns: ['missing']),
        ),
        throwsA(isA<ColumnNotFound>()),
      );
      expect(
        () => backend.scanIndex('missing', null, null).toList(),
        throwsA(isA<IndexNotFound>()),
      );
    });

    test('optional savepoints and triggers default cleanly', () {
      final backend = users();
      expect(() => backend.createSavepoint('s1'), throwsA(isA<Unsupported>()));
      expect(
        () => backend.createTrigger(
          const TriggerDef('tr', 'users', 'AFTER', 'INSERT', 'SELECT 1'),
        ),
        throwsA(isA<Unsupported>()),
      );
      expect(backend.listTriggers('users'), isEmpty);
    });
  });
}

InMemoryBackend users() {
  final backend = InMemoryBackend();
  backend.createTable('users', const [
    ColumnDef('id', 'INTEGER', primaryKey: true),
    ColumnDef('name', 'TEXT', notNull: true),
    ColumnDef('age', 'INTEGER'),
    ColumnDef('email', 'TEXT', unique: true),
  ], false);
  backend.insert('users', {
    'id': 1,
    'name': 'Alice',
    'age': 30,
    'email': 'alice@example.com',
  });
  backend.insert('users', {
    'id': 2,
    'name': 'Bob',
    'age': 25,
    'email': 'bob@example.com',
  });
  backend.insert('users', {
    'id': 3,
    'name': 'Carol',
    'age': null,
    'email': null,
  });
  return backend;
}

List<Row> collect(RowIterator iterator) {
  final rows = <Row>[];
  try {
    Row? row;
    while ((row = iterator.next()) != null) {
      rows.add(row!);
    }
  } finally {
    iterator.close();
  }
  return rows;
}
