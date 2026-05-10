import 'package:coding_adventures_sql_execution_engine/sql_execution_engine.dart';
import 'package:test/test.dart';

void main() {
  DataSource dataSource() => InMemoryDataSource()
    ..addTable(
      'employees',
      ['id', 'name', 'dept', 'salary', 'active'],
      [
        {
          'id': 1,
          'name': 'Alice',
          'dept': 'Engineering',
          'salary': 95000,
          'active': true,
        },
        {
          'id': 2,
          'name': 'Bob',
          'dept': 'Marketing',
          'salary': 72000,
          'active': true,
        },
        {
          'id': 3,
          'name': 'Carol',
          'dept': 'Engineering',
          'salary': 88000,
          'active': false,
        },
        {
          'id': 4,
          'name': 'Dave',
          'dept': null,
          'salary': 60000,
          'active': true,
        },
        {
          'id': 5,
          'name': 'Eve',
          'dept': 'HR',
          'salary': 70000,
          'active': false,
        },
      ],
    )
    ..addTable(
      'departments',
      ['dept', 'budget'],
      [
        {'dept': 'Engineering', 'budget': 500000},
        {'dept': 'Marketing', 'budget': 200000},
        {'dept': 'HR', 'budget': 150000},
      ],
    );

  group('SqlExecutionEngine', () {
    test('scans in-memory tables', () {
      final source = dataSource();
      expect(source.schema('employees'), [
        'id',
        'name',
        'dept',
        'salary',
        'active',
      ]);
      expect(source.scan('employees'), hasLength(5));
      expect(
        () => source.schema('missing'),
        throwsA(isA<SqlExecutionException>()),
      );
    });

    test('selects and filters rows', () {
      final result = SqlExecutionEngine.execute(
        'SELECT name, salary FROM employees WHERE active = true AND salary >= 70000 ORDER BY salary DESC',
        dataSource(),
      );

      expect(result.columns, ['name', 'salary']);
      expect(result.rows[0], ['Alice', 95000]);
      expect(result.rows[1], ['Bob', 72000]);
    });

    test('supports null predicates and like', () {
      final nullResult = SqlExecutionEngine.execute(
        'SELECT name FROM employees WHERE dept IS NULL',
        dataSource(),
      );
      expect(nullResult.rows[0], ['Dave']);

      final likeResult = SqlExecutionEngine.execute(
        "SELECT name FROM employees WHERE name LIKE 'A%'",
        dataSource(),
      );
      expect(likeResult.rows[0], ['Alice']);
    });

    test('supports joins', () {
      final result = SqlExecutionEngine.execute(
        'SELECT e.name, d.budget FROM employees AS e INNER JOIN departments AS d ON e.dept = d.dept ORDER BY e.id',
        dataSource(),
      );

      expect(result.columns, ['name', 'budget']);
      expect(result.rows, hasLength(4));
      expect(result.rows[0], ['Alice', 500000]);
      expect(result.rows[3], ['Eve', 150000]);
    });

    test('supports grouping aggregates and having', () {
      final result = SqlExecutionEngine.execute(
        'SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total FROM employees WHERE dept IS NOT NULL GROUP BY dept HAVING COUNT(*) >= 1 ORDER BY dept',
        dataSource(),
      );

      expect(result.columns, ['dept', 'cnt', 'total']);
      expect(result.rows[0], ['Engineering', 2, 183000.0]);
      expect(result.rows[1], ['HR', 1, 70000.0]);
      expect(result.rows[2], ['Marketing', 1, 72000.0]);
    });

    test('supports distinct limit and offset', () {
      final result = SqlExecutionEngine.execute(
        'SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL ORDER BY dept LIMIT 2 OFFSET 1',
        dataSource(),
      );

      expect(result.columns, ['dept']);
      expect(result.rows[0], ['HR']);
      expect(result.rows[1], ['Marketing']);
    });

    test('reports errors through tryExecute', () {
      final result = SqlExecutionEngine.tryExecute(
        'SELECT * FROM ghosts',
        dataSource(),
      );

      expect(result.ok, isFalse);
      expect(result.error, contains('table not found: ghosts'));
    });

    test('select star uses bare columns', () {
      final result = SqlExecutionEngine.execute(
        'SELECT * FROM employees WHERE id = 1',
        dataSource(),
      );

      expect(result.columns, ['active', 'dept', 'id', 'name', 'salary']);
      expect(result.rows[0], [true, 'Engineering', 1, 'Alice', 95000]);
    });
  });
}
