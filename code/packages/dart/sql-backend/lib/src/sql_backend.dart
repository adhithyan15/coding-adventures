import 'dart:collection';
import 'dart:convert';

typedef Row = Map<String, Object?>;

class TransactionHandle {
  const TransactionHandle(this.value);
  final int value;

  @override
  bool operator ==(Object other) =>
      other is TransactionHandle && other.value == value;

  @override
  int get hashCode => value.hashCode;
}

class SqlValues {
  const SqlValues._();

  static bool isSqlValue(Object? value) =>
      value == null ||
      value is bool ||
      value is int ||
      value is double ||
      value is String ||
      value is List<int>;

  static String typeName(Object? value) {
    if (value == null) return 'NULL';
    if (value is bool) return 'BOOLEAN';
    if (value is int) return 'INTEGER';
    if (value is double) return 'REAL';
    if (value is String) return 'TEXT';
    if (value is List<int>) return 'BLOB';
    throw ArgumentError.value(value, 'value', 'not a SQL value');
  }

  static int compareValues(Object? left, Object? right) {
    final rankCompare = _rank(left).compareTo(_rank(right));
    if (rankCompare != 0) return rankCompare;
    if (left == null) return 0;
    if (left is bool && right is bool) {
      return (left ? 1 : 0).compareTo(right ? 1 : 0);
    }
    if (left is num && right is num) return left.compareTo(right);
    if (left is String && right is String) return left.compareTo(right);
    if (left is List<int> && right is List<int>) {
      for (var i = 0; i < left.length && i < right.length; i++) {
        final cmp = left[i].compareTo(right[i]);
        if (cmp != 0) return cmp;
      }
      return left.length.compareTo(right.length);
    }
    return left.toString().compareTo(right.toString());
  }

  static int _rank(Object? value) {
    if (value == null) return 0;
    if (value is bool) return 1;
    if (value is num) return 2;
    if (value is String) return 3;
    if (value is List<int>) return 4;
    return 5;
  }
}

abstract class RowIterator {
  Row? next();
  void close();
}

abstract class Cursor implements RowIterator {
  Row? currentRow();
}

class ListRowIterator implements RowIterator {
  ListRowIterator(Iterable<Row> rows)
      : _rows = rows.map((row) => Map<String, Object?>.from(row)).toList();

  final List<Row> _rows;
  var _index = 0;
  var _closed = false;

  @override
  Row? next() {
    if (_closed || _index >= _rows.length) return null;
    return Map<String, Object?>.from(_rows[_index++]);
  }

  @override
  void close() {
    _closed = true;
  }
}

class ListCursor implements Cursor {
  ListCursor(this._rows);

  final List<Row> _rows;
  var _index = -1;
  Row? _current;
  var _closed = false;

  int get currentIndex => _index;
  bool isBackedBy(List<Row> rows) => identical(_rows, rows);

  void adjustAfterDelete() {
    _index -= 1;
    _current = null;
  }

  @override
  Row? currentRow() =>
      _current == null ? null : Map<String, Object?>.from(_current!);

  @override
  Row? next() {
    if (_closed) return null;
    _index += 1;
    if (_index >= _rows.length) {
      _current = null;
      return null;
    }
    _current = _rows[_index];
    return Map<String, Object?>.from(_current!);
  }

  @override
  void close() {
    _closed = true;
  }
}

class ColumnDef {
  const ColumnDef(
    this.name,
    this.typeName, {
    this.notNull = false,
    this.primaryKey = false,
    this.unique = false,
    this.autoincrement = false,
    this.defaultValue,
    this.hasDefault = false,
    this.checkExpression,
    this.foreignKey,
  });

  factory ColumnDef.withDefault(
    String name,
    String typeName,
    Object? defaultValue, {
    bool notNull = false,
    bool primaryKey = false,
    bool unique = false,
    bool autoincrement = false,
  }) =>
      ColumnDef(
        name,
        typeName,
        notNull: notNull,
        primaryKey: primaryKey,
        unique: unique,
        autoincrement: autoincrement,
        defaultValue: defaultValue,
        hasDefault: true,
      );

  final String name;
  final String typeName;
  final bool notNull;
  final bool primaryKey;
  final bool unique;
  final bool autoincrement;
  final Object? defaultValue;
  final bool hasDefault;
  final Object? checkExpression;
  final Object? foreignKey;

  bool get effectiveNotNull => notNull || primaryKey;
  bool get effectiveUnique => unique || primaryKey;
}

class TriggerDef {
  const TriggerDef(this.name, this.table, this.timing, this.event, this.body);
  final String name;
  final String table;
  final String timing;
  final String event;
  final String body;
}

class IndexDef {
  const IndexDef(
    this.name,
    this.table, {
    this.columns = const [],
    this.unique = false,
    this.auto = false,
  });

  final String name;
  final String table;
  final List<String> columns;
  final bool unique;
  final bool auto;
}

class BackendError implements Exception {
  BackendError(this.message);
  final String message;
  @override
  String toString() => message;
}

class TableNotFound extends BackendError {
  TableNotFound(String table) : super("table not found: '$table'");
}

class TableAlreadyExists extends BackendError {
  TableAlreadyExists(String table) : super("table already exists: '$table'");
}

class ColumnNotFound extends BackendError {
  ColumnNotFound(String table, String column)
      : super("column not found: '$table.$column'");
}

class ColumnAlreadyExists extends BackendError {
  ColumnAlreadyExists(String table, String column)
      : super("column already exists: '$table.$column'");
}

class ConstraintViolation extends BackendError {
  ConstraintViolation(String table, String column, String message)
      : super(message);
}

class Unsupported extends BackendError {
  Unsupported(String operation) : super('unsupported operation: $operation');
}

class Internal extends BackendError {
  Internal(String message) : super(message);
}

class IndexAlreadyExists extends BackendError {
  IndexAlreadyExists(String index) : super("index already exists: '$index'");
}

class IndexNotFound extends BackendError {
  IndexNotFound(String index) : super("index not found: '$index'");
}

class TriggerAlreadyExists extends BackendError {
  TriggerAlreadyExists(String trigger)
      : super("trigger already exists: '$trigger'");
}

class TriggerNotFound extends BackendError {
  TriggerNotFound(String trigger) : super("trigger not found: '$trigger'");
}

abstract class Backend {
  List<String> tables();
  List<ColumnDef> columns(String table);
  RowIterator scan(String table);
  void insert(String table, Row row);
  void update(String table, Cursor cursor, Map<String, Object?> assignments);
  void delete(String table, Cursor cursor);
  void createTable(String table, List<ColumnDef> columns, bool ifNotExists);
  void dropTable(String table, bool ifExists);
  void addColumn(String table, ColumnDef column);
  void createIndex(IndexDef index);
  void dropIndex(String name, {bool ifExists = false});
  List<IndexDef> listIndexes([String? table]);
  Iterable<int> scanIndex(
    String indexName,
    List<Object?>? lo,
    List<Object?>? hi, {
    bool loInclusive = true,
    bool hiInclusive = true,
  });
  RowIterator scanByRowids(String table, List<int> rowids);
  TransactionHandle beginTransaction();
  void commit(TransactionHandle handle);
  void rollback(TransactionHandle handle);
  TransactionHandle? currentTransaction() => null;
  void createSavepoint(String name) => throw Unsupported('savepoints');
  void releaseSavepoint(String name) => throw Unsupported('savepoints');
  void rollbackToSavepoint(String name) => throw Unsupported('savepoints');
  void createTrigger(TriggerDef defn) => throw Unsupported('triggers');
  void dropTrigger(String name, {bool ifExists = false}) =>
      throw Unsupported('triggers');
  List<TriggerDef> listTriggers(String table) => const [];
}

abstract class SchemaProvider {
  List<String> columns(String table);
}

class BackendAdapters {
  const BackendAdapters._();

  static SchemaProvider asSchemaProvider(Backend backend) =>
      _BackendSchemaProvider(backend);
}

class _BackendSchemaProvider implements SchemaProvider {
  _BackendSchemaProvider(this.backend);
  final Backend backend;

  @override
  List<String> columns(String table) =>
      backend.columns(table).map((column) => column.name).toList();
}

class InMemoryBackend extends Backend {
  final Map<String, _Table> _tables = SplayTreeMap();
  final Map<String, IndexDef> _indexes = SplayTreeMap();
  _Snapshot? _snapshot;
  TransactionHandle? _activeHandle;
  var _nextHandle = 1;

  @override
  List<String> tables() => _tables.values.map((table) => table.name).toList();

  @override
  List<ColumnDef> columns(String table) => List.of(_requireTable(table).columns);

  @override
  RowIterator scan(String table) => ListRowIterator(_requireTable(table).rows);

  ListCursor openCursor(String table) => ListCursor(_requireTable(table).rows);

  @override
  void insert(String table, Row row) {
    final state = _requireTable(table);
    final normalized = _applyDefaults(table, state, row);
    _checkNotNull(table, state, normalized);
    _checkUnique(table, state, normalized, null);
    state.rows.add(normalized);
  }

  @override
  void update(String table, Cursor cursor, Map<String, Object?> assignments) {
    final state = _requireTable(table);
    final listCursor = _requireListCursor(table, state, cursor);
    final index = listCursor.currentIndex;
    if (index < 0 || index >= state.rows.length) {
      throw Unsupported('cursor has no current row');
    }

    final updated = Map<String, Object?>.from(state.rows[index]);
    for (final entry in assignments.entries) {
      updated[_canonicalColumn(table, state, entry.key)] = entry.value;
    }
    _checkNotNull(table, state, updated);
    _checkUnique(table, state, updated, index);
    state.rows[index] = updated;
  }

  @override
  void delete(String table, Cursor cursor) {
    final state = _requireTable(table);
    final listCursor = _requireListCursor(table, state, cursor);
    final index = listCursor.currentIndex;
    if (index < 0 || index >= state.rows.length) {
      throw Unsupported('cursor has no current row');
    }
    state.rows.removeAt(index);
    listCursor.adjustAfterDelete();
  }

  @override
  void createTable(String table, List<ColumnDef> columns, bool ifNotExists) {
    final key = _key(table);
    if (_tables.containsKey(key)) {
      if (ifNotExists) return;
      throw TableAlreadyExists(table);
    }

    final seen = <String>{};
    for (final column in columns) {
      if (!seen.add(_key(column.name))) {
        throw ColumnAlreadyExists(table, column.name);
      }
    }

    _tables[key] = _Table(table, columns, const []);
  }

  @override
  void dropTable(String table, bool ifExists) {
    if (_tables.remove(_key(table)) == null) {
      if (ifExists) return;
      throw TableNotFound(table);
    }
    _indexes.removeWhere((_, index) => _same(index.table, table));
  }

  @override
  void addColumn(String table, ColumnDef column) {
    final state = _requireTable(table);
    if (state.columns.any((existing) => _same(existing.name, column.name))) {
      throw ColumnAlreadyExists(table, column.name);
    }
    if (column.effectiveNotNull && !column.hasDefault) {
      throw ConstraintViolation(
        table,
        column.name,
        'NOT NULL constraint failed: $table.${column.name}',
      );
    }
    state.columns.add(column);
    for (final row in state.rows) {
      row[column.name] = column.hasDefault ? column.defaultValue : null;
    }
  }

  @override
  void createIndex(IndexDef index) {
    if (_indexes.containsKey(_key(index.name))) {
      throw IndexAlreadyExists(index.name);
    }
    final state = _requireTable(index.table);
    for (final column in index.columns) {
      _canonicalColumn(index.table, state, column);
    }
    _indexes[_key(index.name)] = _cloneIndex(index);
  }

  @override
  void dropIndex(String name, {bool ifExists = false}) {
    if (_indexes.remove(_key(name)) == null && !ifExists) {
      throw IndexNotFound(name);
    }
  }

  @override
  List<IndexDef> listIndexes([String? table]) => _indexes.values
      .where((index) => table == null || _same(index.table, table))
      .map(_cloneIndex)
      .toList();

  @override
  Iterable<int> scanIndex(
    String indexName,
    List<Object?>? lo,
    List<Object?>? hi, {
    bool loInclusive = true,
    bool hiInclusive = true,
  }) sync* {
    final index = _indexes[_key(indexName)];
    if (index == null) throw IndexNotFound(indexName);

    final state = _requireTable(index.table);
    final keyed = <_KeyedRow>[
      for (var rowid = 0; rowid < state.rows.length; rowid++)
        _KeyedRow(_indexKey(state, state.rows[rowid], index.columns), rowid),
    ]..sort((left, right) {
        final cmp = _compareKey(left.key, right.key);
        return cmp != 0 ? cmp : left.rowid.compareTo(right.rowid);
      });

    for (final row in keyed) {
      if (lo != null) {
        final cmp = _comparePrefix(row.key, lo);
        if (cmp < 0 || (cmp == 0 && !loInclusive)) continue;
      }
      if (hi != null) {
        final cmp = _comparePrefix(row.key, hi);
        if (cmp > 0 || (cmp == 0 && !hiInclusive)) break;
      }
      yield row.rowid;
    }
  }

  @override
  RowIterator scanByRowids(String table, List<int> rowids) {
    final state = _requireTable(table);
    return ListRowIterator([
      for (final rowid in rowids)
        if (rowid >= 0 && rowid < state.rows.length) state.rows[rowid],
    ]);
  }

  @override
  TransactionHandle beginTransaction() {
    if (_activeHandle != null) {
      throw Unsupported('nested transactions');
    }
    final handle = TransactionHandle(_nextHandle++);
    _snapshot = _capture();
    _activeHandle = handle;
    return handle;
  }

  @override
  void commit(TransactionHandle handle) {
    _requireActive(handle);
    _snapshot = null;
    _activeHandle = null;
  }

  @override
  void rollback(TransactionHandle handle) {
    _requireActive(handle);
    final snapshot = _snapshot;
    if (snapshot != null) _restore(snapshot);
    _snapshot = null;
    _activeHandle = null;
  }

  @override
  TransactionHandle? currentTransaction() => _activeHandle;

  _Table _requireTable(String table) {
    final state = _tables[_key(table)];
    if (state == null) throw TableNotFound(table);
    return state;
  }

  ListCursor _requireListCursor(String table, _Table state, Cursor cursor) {
    if (cursor is ListCursor && cursor.isBackedBy(state.rows)) {
      return cursor;
    }
    throw Unsupported('foreign cursor for table $table');
  }

  Row _applyDefaults(String table, _Table state, Row row) {
    final normalized = Map<String, Object?>.from(row);
    for (final column in state.columns) {
      if (!_containsColumn(normalized, column.name)) {
        normalized[column.name] = column.hasDefault ? column.defaultValue : null;
      }
    }
    for (final column in normalized.keys) {
      if (!state.columns.any((existing) => _same(existing.name, column))) {
        throw ColumnNotFound(table, column);
      }
    }
    return normalized;
  }

  void _checkNotNull(String table, _Table state, Row row) {
    for (final column in state.columns) {
      if (column.effectiveNotNull && row[column.name] == null) {
        throw ConstraintViolation(
          table,
          column.name,
          'NOT NULL constraint failed: $table.${column.name}',
        );
      }
    }
  }

  void _checkUnique(String table, _Table state, Row row, int? ignoreIndex) {
    for (final column in state.columns) {
      if (!column.effectiveUnique) continue;
      final value = row[column.name];
      if (value == null) continue;
      for (var i = 0; i < state.rows.length; i++) {
        if (ignoreIndex == i) continue;
        if (state.rows[i][column.name] == value) {
          final label = column.primaryKey ? 'PRIMARY KEY' : 'UNIQUE';
          throw ConstraintViolation(
            table,
            column.name,
            '$label constraint failed: $table.${column.name}',
          );
        }
      }
    }
  }

  String _canonicalColumn(String table, _Table state, String column) {
    for (final candidate in state.columns) {
      if (_same(candidate.name, column)) return candidate.name;
    }
    throw ColumnNotFound(table, column);
  }

  List<Object?> _indexKey(_Table state, Row row, List<String> columns) => [
        for (final column in columns) row[_canonicalColumn('', state, column)],
      ];

  _Snapshot _capture() => _Snapshot(
        {
          for (final entry in _tables.entries) entry.key: entry.value.copy(),
        },
        {
          for (final entry in _indexes.entries) entry.key: _cloneIndex(entry.value),
        },
      );

  void _restore(_Snapshot snapshot) {
    _tables
      ..clear()
      ..addAll({
        for (final entry in snapshot.tables.entries) entry.key: entry.value.copy(),
      });
    _indexes
      ..clear()
      ..addAll({
        for (final entry in snapshot.indexes.entries)
          entry.key: _cloneIndex(entry.value),
      });
  }

  void _requireActive(TransactionHandle handle) {
    if (_activeHandle == null) throw Unsupported('no active transaction');
    if (_activeHandle != handle) throw Unsupported('stale transaction handle');
  }
}

class _Table {
  _Table(this.name, Iterable<ColumnDef> columns, Iterable<Row> rows)
      : columns = List.of(columns),
        rows = rows.map((row) => Map<String, Object?>.from(row)).toList();

  final String name;
  final List<ColumnDef> columns;
  final List<Row> rows;

  _Table copy() => _Table(name, columns, rows);
}

class _Snapshot {
  const _Snapshot(this.tables, this.indexes);
  final Map<String, _Table> tables;
  final Map<String, IndexDef> indexes;
}

class _KeyedRow {
  const _KeyedRow(this.key, this.rowid);
  final List<Object?> key;
  final int rowid;
}

String _key(String value) => value.toLowerCase();
bool _same(String left, String right) => left.toLowerCase() == right.toLowerCase();

bool _containsColumn(Row row, String column) =>
    row.keys.any((key) => _same(key, column));

IndexDef _cloneIndex(IndexDef index) => IndexDef(
      index.name,
      index.table,
      columns: List.of(index.columns),
      unique: index.unique,
      auto: index.auto,
    );

int _compareKey(List<Object?> left, List<Object?> right) {
  for (var i = 0; i < left.length && i < right.length; i++) {
    final cmp = SqlValues.compareValues(left[i], right[i]);
    if (cmp != 0) return cmp;
  }
  return left.length.compareTo(right.length);
}

int _comparePrefix(List<Object?> key, List<Object?> bound) {
  for (var i = 0; i < bound.length; i++) {
    final value = i < key.length ? key[i] : null;
    final cmp = SqlValues.compareValues(value, bound[i]);
    if (cmp != 0) return cmp;
  }
  return 0;
}

String _serializeKey(List<Object?> key) => key
    .map((value) {
      if (value == null) return 'NULL';
      if (value is List<int>) return base64Encode(value);
      return value.toString();
    })
    .join('\u001f');
