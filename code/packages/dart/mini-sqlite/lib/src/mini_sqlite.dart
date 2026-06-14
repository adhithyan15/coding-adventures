class MiniSqlite {
  static const apiLevel = '2.0';
  static const threadSafety = 1;
  static const paramStyle = 'qmark';

  static Connection connect(
    String database, {
    ConnectionOptions options = const ConnectionOptions(),
  }) {
    if (database != ':memory:') {
      throw MiniSqliteException(
        'NotSupportedError',
        'Dart mini-sqlite supports only :memory: in Level 0',
      );
    }
    return Connection._(options);
  }
}

class ConnectionOptions {
  const ConnectionOptions({this.autocommit = false});

  final bool autocommit;
}

class Column {
  const Column(this.name);

  final String name;
}

class MiniSqliteException implements Exception {
  MiniSqliteException(this.kind, this.message);

  final String kind;
  final String message;

  @override
  String toString() => '$kind: $message';
}

class Connection {
  Connection._(ConnectionOptions options) : _autocommit = options.autocommit;

  _Database _db = _Database();
  final bool _autocommit;
  _Database? _snapshot;
  bool _closed = false;

  Cursor cursor() {
    _assertOpen();
    return Cursor._(this);
  }

  Cursor execute(String sql, [List<Object?> parameters = const []]) {
    return cursor().execute(sql, parameters);
  }

  Cursor executeMany(String sql, Iterable<List<Object?>> parameterSets) {
    return cursor().executeMany(sql, parameterSets);
  }

  void commit() {
    _assertOpen();
    _snapshot = null;
  }

  void rollback() {
    _assertOpen();
    final snapshot = _snapshot;
    if (snapshot != null) {
      _db = snapshot.copy();
      _snapshot = null;
    }
  }

  void close() {
    if (_closed) {
      return;
    }
    final snapshot = _snapshot;
    if (snapshot != null) {
      _db = snapshot.copy();
    }
    _snapshot = null;
    _closed = true;
  }

  _ExecutionResult _executeBound(String sql, List<Object?> parameters) {
    _assertOpen();
    final bound = _SqlText.bindParameters(sql, parameters);
    try {
      switch (_SqlText.firstKeyword(bound)) {
        case 'BEGIN':
          _ensureSnapshot();
          return _ExecutionResult.empty(0);
        case 'COMMIT':
          _snapshot = null;
          return _ExecutionResult.empty(0);
        case 'ROLLBACK':
          final snapshot = _snapshot;
          if (snapshot != null) {
            _db = snapshot.copy();
          }
          _snapshot = null;
          return _ExecutionResult.empty(0);
        case 'SELECT':
          return _db.select(_Statements.parseSelect(bound));
        case 'CREATE':
          return _withSnapshot(
            () => _db.create(_Statements.parseCreate(bound)),
          );
        case 'DROP':
          return _withSnapshot(() => _db.drop(_Statements.parseDrop(bound)));
        case 'INSERT':
          return _withSnapshot(
            () => _db.insert(_Statements.parseInsert(bound)),
          );
        case 'UPDATE':
          return _withSnapshot(
            () => _db.update(_Statements.parseUpdate(bound)),
          );
        case 'DELETE':
          return _withSnapshot(
            () => _db.delete(_Statements.parseDelete(bound)),
          );
        default:
          throw ArgumentError('unsupported SQL statement');
      }
    } on MiniSqliteException {
      rethrow;
    } catch (error) {
      throw MiniSqliteException('OperationalError', '$error');
    }
  }

  _ExecutionResult _withSnapshot(_ExecutionResult Function() action) {
    _ensureSnapshot();
    return action();
  }

  void _ensureSnapshot() {
    if (!_autocommit && _snapshot == null) {
      _snapshot = _db.copy();
    }
  }

  void _assertOpen() {
    if (_closed) {
      throw MiniSqliteException('ProgrammingError', 'connection is closed');
    }
  }
}

class Cursor {
  Cursor._(this._connection);

  final Connection _connection;
  List<List<Object?>> _rows = const [];
  int _offset = 0;
  bool _closed = false;

  List<Column> description = const [];
  int rowCount = -1;
  Object? lastRowId;
  int arraySize = 1;

  Cursor execute(String sql, [List<Object?> parameters = const []]) {
    _assertOpen();
    final result = _connection._executeBound(sql, parameters);
    _rows = result.rows;
    _offset = 0;
    description = [for (final name in result.columns) Column(name)];
    rowCount = result.rowCount;
    lastRowId = result.lastRowId;
    return this;
  }

  Cursor executeMany(String sql, Iterable<List<Object?>> parameterSets) {
    var last = this;
    for (final parameters in parameterSets) {
      last = execute(sql, parameters);
    }
    return last;
  }

  List<Object?>? fetchOne() {
    _assertOpen();
    if (_offset >= _rows.length) {
      return null;
    }
    return _rows[_offset++];
  }

  List<List<Object?>> fetchMany([int? size]) {
    _assertOpen();
    final limit = size ?? arraySize;
    final output = <List<Object?>>[];
    while (output.length < limit && _offset < _rows.length) {
      output.add(_rows[_offset++]);
    }
    return output;
  }

  List<List<Object?>> fetchAll() {
    _assertOpen();
    final output = <List<Object?>>[];
    while (_offset < _rows.length) {
      output.add(_rows[_offset++]);
    }
    return output;
  }

  void close() {
    _closed = true;
  }

  void _assertOpen() {
    if (_closed) {
      throw MiniSqliteException('ProgrammingError', 'cursor is closed');
    }
  }
}

class _ExecutionResult {
  const _ExecutionResult({
    required this.columns,
    required this.rows,
    required this.rowCount,
    this.lastRowId,
  });

  factory _ExecutionResult.empty(int rowCount) {
    return _ExecutionResult(
      columns: const [],
      rows: const [],
      rowCount: rowCount,
    );
  }

  final List<String> columns;
  final List<List<Object?>> rows;
  final int rowCount;
  final Object? lastRowId;
}

class _Table {
  _Table(this.columns);

  final List<String> columns;
  final rows = <Map<String, Object?>>[];
  var nextRowId = 1;

  _Table copy() {
    final table = _Table(List<String>.from(columns));
    table.rows.addAll(rows.map((row) => Map<String, Object?>.from(row)));
    table.nextRowId = nextRowId;
    return table;
  }
}

class _Database {
  final tables = <String, _Table>{};

  _Database copy() {
    final db = _Database();
    for (final entry in tables.entries) {
      db.tables[entry.key] = entry.value.copy();
    }
    return db;
  }

  _ExecutionResult create(_CreateStatement statement) {
    final key = _identifierKey(statement.tableName);
    if (tables.containsKey(key)) {
      throw MiniSqliteException(
        'OperationalError',
        'table already exists: ${statement.tableName}',
      );
    }
    tables[key] = _Table(statement.columns);
    return _ExecutionResult.empty(0);
  }

  _ExecutionResult drop(String tableName) {
    final removed = tables.remove(_identifierKey(tableName));
    if (removed == null) {
      throw MiniSqliteException(
        'OperationalError',
        'no such table: $tableName',
      );
    }
    return _ExecutionResult.empty(0);
  }

  _ExecutionResult insert(_InsertStatement statement) {
    final table = _requireTable(statement.tableName);
    final columns = statement.columns ?? table.columns;
    if (columns.length != statement.values.length) {
      throw MiniSqliteException(
        'ProgrammingError',
        'column/value count mismatch',
      );
    }

    final row = <String, Object?>{
      for (final column in table.columns) _identifierKey(column): null,
    };
    for (var i = 0; i < columns.length; i++) {
      row[_identifierKey(columns[i])] = _SqlValue.parseLiteral(
        statement.values[i],
      );
    }

    final rowId = table.nextRowId++;
    table.rows.add(row);
    return _ExecutionResult(
      columns: const [],
      rows: const [],
      rowCount: 1,
      lastRowId: rowId,
    );
  }

  _ExecutionResult update(_UpdateStatement statement) {
    final table = _requireTable(statement.tableName);
    var count = 0;
    for (final row in table.rows) {
      if (_Conditions.matches(statement.whereSql, row)) {
        for (final assignment in statement.assignments) {
          row[_identifierKey(assignment.column)] = _SqlValue.parseLiteral(
            assignment.valueSql,
          );
        }
        count++;
      }
    }
    return _ExecutionResult.empty(count);
  }

  _ExecutionResult delete(_DeleteStatement statement) {
    final table = _requireTable(statement.tableName);
    var count = 0;
    table.rows.removeWhere((row) {
      final shouldRemove = _Conditions.matches(statement.whereSql, row);
      if (shouldRemove) {
        count++;
      }
      return shouldRemove;
    });
    return _ExecutionResult.empty(count);
  }

  _ExecutionResult select(_SelectStatement statement) {
    final table = _requireTable(statement.tableName);
    final projection =
        statement.projection.length == 1 && statement.projection.single == '*'
            ? table.columns
            : statement.projection;
    final matchingRows = [
      for (final row in table.rows)
        if (_Conditions.matches(statement.whereSql, row)) row,
    ];
    _applyOrder(statement.orderBySql, matchingRows);

    return _ExecutionResult(
      columns: projection,
      rows: [
        for (final row in matchingRows)
          [for (final column in projection) row[_identifierKey(column)]],
      ],
      rowCount: -1,
    );
  }

  _Table _requireTable(String tableName) {
    final table = tables[_identifierKey(tableName)];
    if (table == null) {
      throw MiniSqliteException(
        'OperationalError',
        'no such table: $tableName',
      );
    }
    return table;
  }
}

class _SqlText {
  static String trimSql(String sql) {
    final trimmed = sql.trim();
    if (trimmed.endsWith(';')) {
      return trimmed.substring(0, trimmed.length - 1).trim();
    }
    return trimmed;
  }

  static String firstKeyword(String sql) {
    final match = RegExp(r'^[A-Za-z]+').firstMatch(sql.trimLeft());
    return match?.group(0)?.toUpperCase() ?? '';
  }

  static String bindParameters(String sql, List<Object?> parameters) {
    final output = StringBuffer();
    var parameterIndex = 0;
    String? quote;
    var i = 0;

    while (i < sql.length) {
      final ch = sql[i];
      if (quote != null) {
        output.write(ch);
        if (ch == quote) {
          if (i + 1 < sql.length && sql[i + 1] == quote) {
            i++;
            output.write(sql[i]);
          } else {
            quote = null;
          }
        }
      } else if (ch == "'" || ch == '"') {
        quote = ch;
        output.write(ch);
      } else if (ch == '?') {
        if (parameterIndex >= parameters.length) {
          throw MiniSqliteException(
            'ProgrammingError',
            'not enough query parameters',
          );
        }
        output.write(_SqlValue.formatParameter(parameters[parameterIndex++]));
      } else {
        output.write(ch);
      }
      i++;
    }

    if (parameterIndex != parameters.length) {
      throw MiniSqliteException(
        'ProgrammingError',
        'too many query parameters',
      );
    }
    return output.toString();
  }

  static List<String> splitTopLevel(String text, String separator) {
    final parts = <String>[];
    var start = 0;
    var depth = 0;
    String? quote;
    var i = 0;

    while (i < text.length) {
      final ch = text[i];
      if (quote != null) {
        if (ch == quote) {
          if (i + 1 < text.length && text[i + 1] == quote) {
            i++;
          } else {
            quote = null;
          }
        }
      } else if (ch == "'" || ch == '"') {
        quote = ch;
      } else if (ch == '(') {
        depth++;
      } else if (ch == ')' && depth > 0) {
        depth--;
      } else if (ch == separator && depth == 0) {
        parts.add(text.substring(start, i).trim());
        start = i + 1;
      }
      i++;
    }

    parts.add(text.substring(start).trim());
    return parts.where((part) => part.isNotEmpty).toList();
  }

  static List<String> splitByKeyword(String text, String keyword) {
    final parts = <String>[];
    var start = 0;
    var depth = 0;
    String? quote;
    var i = 0;

    while (i < text.length) {
      final ch = text[i];
      if (quote != null) {
        if (ch == quote) {
          quote = null;
        }
      } else if (ch == "'" || ch == '"') {
        quote = ch;
      } else if (ch == '(') {
        depth++;
      } else if (ch == ')' && depth > 0) {
        depth--;
      } else if (depth == 0 && _matchesKeyword(text, keyword, i)) {
        parts.add(text.substring(start, i).trim());
        i += keyword.length - 1;
        start = i + 1;
      }
      i++;
    }

    parts.add(text.substring(start).trim());
    return parts.where((part) => part.isNotEmpty).toList();
  }

  static bool _matchesKeyword(String text, String keyword, int index) {
    if (index + keyword.length > text.length) {
      return false;
    }
    final candidate = text.substring(index, index + keyword.length);
    final before = index == 0 ? '' : text[index - 1];
    final after = index + keyword.length == text.length
        ? ''
        : text[index + keyword.length];
    return candidate.toUpperCase() == keyword.toUpperCase() &&
        !_isIdentifierChar(before) &&
        !_isIdentifierChar(after);
  }

  static bool _isIdentifierChar(String ch) {
    return ch.isNotEmpty && RegExp(r'[A-Za-z0-9_]').hasMatch(ch);
  }
}

class _SqlValue {
  static String formatParameter(Object? value) {
    if (value == null) {
      return 'NULL';
    }
    if (value is String) {
      return "'${value.replaceAll("'", "''")}'";
    }
    if (value is bool) {
      return value ? 'TRUE' : 'FALSE';
    }
    if (value is num) {
      return value.toString();
    }
    return "'${value.toString().replaceAll("'", "''")}'";
  }

  static Object? parseLiteral(String token) {
    final text = token.trim();
    if (text.length >= 2 &&
        ((text.startsWith("'") && text.endsWith("'")) ||
            (text.startsWith('"') && text.endsWith('"')))) {
      final quote = text[0];
      return text
          .substring(1, text.length - 1)
          .replaceAll(quote + quote, quote);
    }
    if (text.toUpperCase() == 'NULL') {
      return null;
    }
    if (text.toUpperCase() == 'TRUE') {
      return true;
    }
    if (text.toUpperCase() == 'FALSE') {
      return false;
    }
    return int.tryParse(text) ?? double.tryParse(text) ?? text;
  }

  static Object? resolve(Map<String, Object?> row, String token) {
    final text = token.trim();
    final key = _identifierKey(text);
    return row.containsKey(key) ? row[key] : parseLiteral(text);
  }

  static bool equals(Object? left, Object? right) {
    if (left == null || right == null) {
      return left == null && right == null;
    }
    if (left is num && right is num) {
      return left == right;
    }
    return left == right;
  }

  static int compare(Object? left, Object? right) {
    if (equals(left, right)) {
      return 0;
    }
    if (left == null) {
      return -1;
    }
    if (right == null) {
      return 1;
    }
    if (left is num && right is num) {
      return left.compareTo(right);
    }
    return left.toString().compareTo(right.toString());
  }
}

class _Conditions {
  static bool matches(String? whereSql, Map<String, Object?> row) {
    if (whereSql == null || whereSql.trim().isEmpty) {
      return true;
    }
    return _SqlText.splitByKeyword(whereSql, 'OR').any(
      (disjunct) => _SqlText.splitByKeyword(
        disjunct,
        'AND',
      ).every((atom) => _matchesAtom(atom, row)),
    );
  }

  static bool _matchesAtom(String atom, Map<String, Object?> row) {
    final text = atom.trim();
    final isNull = RegExp(
      r'^(.+?)\s+IS\s+(NOT\s+)?NULL$',
      caseSensitive: false,
      dotAll: true,
    ).firstMatch(text);
    if (isNull != null) {
      final value = _SqlValue.resolve(row, isNull.group(1)!);
      final negate = isNull.group(2) != null;
      return negate ? value != null : value == null;
    }

    final inMatch = RegExp(
      r'^(.+?)\s+IN\s*\((.*)\)$',
      caseSensitive: false,
      dotAll: true,
    ).firstMatch(text);
    if (inMatch != null) {
      final left = _SqlValue.resolve(row, inMatch.group(1)!);
      return _SqlText.splitTopLevel(inMatch.group(2)!, ',').any(
        (valueSql) => _SqlValue.equals(left, _SqlValue.resolve(row, valueSql)),
      );
    }

    final comparison = RegExp(
      r'^\s*(.+?)\s*(=|!=|<>|<=|>=|<|>)\s*(.+?)\s*$',
      dotAll: true,
    ).firstMatch(text);
    if (comparison == null) {
      final value = _SqlValue.resolve(row, text);
      return value is bool ? value : value != null;
    }

    final left = _SqlValue.resolve(row, comparison.group(1)!);
    final operator = comparison.group(2)!;
    final right = _SqlValue.resolve(row, comparison.group(3)!);

    switch (operator) {
      case '=':
        return _SqlValue.equals(left, right);
      case '!=':
      case '<>':
        return !_SqlValue.equals(left, right);
      case '<':
        return _SqlValue.compare(left, right) < 0;
      case '<=':
        return _SqlValue.compare(left, right) <= 0;
      case '>':
        return _SqlValue.compare(left, right) > 0;
      case '>=':
        return _SqlValue.compare(left, right) >= 0;
      default:
        return false;
    }
  }
}

class _Statements {
  static _CreateStatement parseCreate(String sql) {
    final match = _match(
      r'^\s*CREATE\s+TABLE\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*$',
      sql,
    );
    return _CreateStatement(
      match.group(1)!,
      _SqlText.splitTopLevel(
        match.group(2)!,
        ',',
      ).map(_identifierFromColumn).toList(),
    );
  }

  static String parseDrop(String sql) {
    return _match(
      r'^\s*DROP\s+TABLE\s+([A-Za-z_][A-Za-z0-9_]*)\s*$',
      sql,
    ).group(1)!;
  }

  static _InsertStatement parseInsert(String sql) {
    final match = _match(
      r'^\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*?)\))?\s+VALUES\s*\((.*)\)\s*$',
      sql,
    );
    return _InsertStatement(
      match.group(1)!,
      match.group(2) == null
          ? null
          : _SqlText.splitTopLevel(
              match.group(2)!,
              ',',
            ).map(_identifierFromColumn).toList(),
      _SqlText.splitTopLevel(match.group(3)!, ','),
    );
  }

  static _UpdateStatement parseUpdate(String sql) {
    final match = _match(
      r'^\s*UPDATE\s+([A-Za-z_][A-Za-z0-9_]*)\s+SET\s+(.+?)(?:\s+WHERE\s+(.+))?\s*$',
      sql,
    );
    return _UpdateStatement(
        match.group(1)!,
        [
          for (final assignmentSql
              in _SqlText.splitTopLevel(match.group(2)!, ','))
            _parseAssignment(assignmentSql),
        ],
        match.group(3));
  }

  static _DeleteStatement parseDelete(String sql) {
    final match = _match(
      r'^\s*DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+WHERE\s+(.+))?\s*$',
      sql,
    );
    return _DeleteStatement(match.group(1)!, match.group(2));
  }

  static _SelectStatement parseSelect(String sql) {
    final match = _match(
      r'^\s*SELECT\s+(.+?)\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)\s*(.*)\s*$',
      sql,
    );
    final rest = match.group(3)!.trim();
    String? whereSql;
    String? orderBySql;

    final whereAndOrder = RegExp(
      r'^WHERE\s+(.+?)\s+ORDER\s+BY\s+(.+)$',
      caseSensitive: false,
      dotAll: true,
    ).firstMatch(rest);
    final whereOnly = RegExp(
      r'^WHERE\s+(.+)$',
      caseSensitive: false,
      dotAll: true,
    ).firstMatch(rest);
    final orderOnly = RegExp(
      r'^ORDER\s+BY\s+(.+)$',
      caseSensitive: false,
      dotAll: true,
    ).firstMatch(rest);
    if (whereAndOrder != null) {
      whereSql = whereAndOrder.group(1);
      orderBySql = whereAndOrder.group(2);
    } else if (whereOnly != null) {
      whereSql = whereOnly.group(1);
    } else if (orderOnly != null) {
      orderBySql = orderOnly.group(1);
    } else if (rest.isNotEmpty) {
      throw ArgumentError('could not parse SELECT suffix');
    }

    return _SelectStatement(
      match.group(2)!,
      _SqlText.splitTopLevel(
        match.group(1)!,
        ',',
      ).map(_identifierFromColumn).toList(),
      whereSql,
      orderBySql,
    );
  }

  static RegExpMatch _match(String pattern, String sql) {
    final match = RegExp(
      pattern,
      caseSensitive: false,
      dotAll: true,
    ).firstMatch(_SqlText.trimSql(sql));
    if (match == null) {
      throw ArgumentError('could not parse SQL statement');
    }
    return match;
  }

  static _Assignment _parseAssignment(String assignmentSql) {
    final index = assignmentSql.indexOf('=');
    if (index < 0) {
      throw ArgumentError('invalid assignment');
    }
    return _Assignment(
      _identifierFromColumn(assignmentSql.substring(0, index)),
      assignmentSql.substring(index + 1).trim(),
    );
  }
}

class _CreateStatement {
  _CreateStatement(this.tableName, this.columns);

  final String tableName;
  final List<String> columns;
}

class _InsertStatement {
  _InsertStatement(this.tableName, this.columns, this.values);

  final String tableName;
  final List<String>? columns;
  final List<String> values;
}

class _UpdateStatement {
  _UpdateStatement(this.tableName, this.assignments, this.whereSql);

  final String tableName;
  final List<_Assignment> assignments;
  final String? whereSql;
}

class _DeleteStatement {
  _DeleteStatement(this.tableName, this.whereSql);

  final String tableName;
  final String? whereSql;
}

class _SelectStatement {
  _SelectStatement(
    this.tableName,
    this.projection,
    this.whereSql,
    this.orderBySql,
  );

  final String tableName;
  final List<String> projection;
  final String? whereSql;
  final String? orderBySql;
}

class _Assignment {
  _Assignment(this.column, this.valueSql);

  final String column;
  final String valueSql;
}

void _applyOrder(String? orderBySql, List<Map<String, Object?>> rows) {
  if (orderBySql == null || orderBySql.trim().isEmpty) {
    return;
  }
  final parts = orderBySql.trim().split(RegExp(r'\s+'));
  final column = parts.first;
  final descending = parts.length > 1 && parts[1].toUpperCase() == 'DESC';
  rows.sort(
    (left, right) => _SqlValue.compare(
      _SqlValue.resolve(left, column),
      _SqlValue.resolve(right, column),
    ),
  );
  if (descending) {
    rows.setAll(0, rows.reversed.toList());
  }
}

String _identifierFromColumn(String columnSql) {
  final parts = columnSql.trim().split(RegExp(r'\s+'));
  if (parts.isEmpty || parts.first.isEmpty) {
    throw ArgumentError('empty column definition');
  }
  return parts.first.replaceAll('"', '').replaceAll("'", '');
}

String _identifierKey(String name) => name.trim().toLowerCase();
