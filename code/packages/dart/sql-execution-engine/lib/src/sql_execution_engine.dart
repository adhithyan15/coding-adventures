import 'dart:convert';

typedef Row = Map<String, Object?>;

abstract class DataSource {
  List<String> schema(String tableName);
  List<Row> scan(String tableName);
}

class QueryResult {
  const QueryResult(this.columns, this.rows);

  final List<String> columns;
  final List<List<Object?>> rows;
}

class ExecutionResult {
  const ExecutionResult._(this.ok, this.result, this.error);

  factory ExecutionResult.success(QueryResult result) =>
      ExecutionResult._(true, result, null);

  factory ExecutionResult.failure(String error) =>
      ExecutionResult._(false, null, error);

  final bool ok;
  final QueryResult? result;
  final String? error;
}

class SqlExecutionException implements Exception {
  SqlExecutionException(this.message, [this.cause]);

  final String message;
  final Object? cause;

  @override
  String toString() => message;
}

class InMemoryDataSource implements DataSource {
  final _schemas = <String, List<String>>{};
  final _tables = <String, List<Row>>{};

  InMemoryDataSource addTable(
    String name,
    Iterable<String> schema,
    Iterable<Row> rows,
  ) {
    _schemas[name] = List<String>.from(schema);
    _tables[name] = [for (final row in rows) Map<String, Object?>.from(row)];
    return this;
  }

  @override
  List<String> schema(String tableName) {
    final columns = _schemas[tableName];
    if (columns == null)
      throw SqlExecutionException('table not found: $tableName');
    return List<String>.from(columns);
  }

  @override
  List<Row> scan(String tableName) {
    final rows = _tables[tableName];
    if (rows == null)
      throw SqlExecutionException('table not found: $tableName');
    return [for (final row in rows) Map<String, Object?>.from(row)];
  }
}

class SqlExecutionEngine {
  static const _keywords = {
    'SELECT',
    'FROM',
    'WHERE',
    'GROUP',
    'BY',
    'HAVING',
    'ORDER',
    'LIMIT',
    'OFFSET',
    'DISTINCT',
    'ALL',
    'JOIN',
    'INNER',
    'LEFT',
    'RIGHT',
    'FULL',
    'OUTER',
    'CROSS',
    'ON',
    'AS',
    'AND',
    'OR',
    'NOT',
    'IS',
    'NULL',
    'IN',
    'BETWEEN',
    'LIKE',
    'TRUE',
    'FALSE',
    'ASC',
    'DESC',
    'COUNT',
    'SUM',
    'AVG',
    'MIN',
    'MAX',
    'UPPER',
    'LOWER',
    'LENGTH',
  };

  static QueryResult execute(String sql, DataSource dataSource) {
    try {
      return _executeSelect(
        _Parser(_tokenize(sql)).parseStatement(),
        dataSource,
      );
    } on SqlExecutionException {
      rethrow;
    } catch (error) {
      throw SqlExecutionException(error.toString(), error);
    }
  }

  static ExecutionResult tryExecute(String sql, DataSource dataSource) {
    try {
      return ExecutionResult.success(execute(sql, dataSource));
    } catch (error) {
      return ExecutionResult.failure(error.toString());
    }
  }

  static QueryResult _executeSelect(
    _SelectStatement statement,
    DataSource dataSource,
  ) {
    var rows = _scanTable(
      dataSource,
      statement.from.name,
      statement.from.alias,
    );
    for (final join in statement.joins) {
      rows = _applyJoin(
        rows,
        _scanTable(dataSource, join.table.name, join.table.alias),
        join,
      );
    }

    final where = statement.where;
    if (where != null) {
      rows = [
        for (final row in rows)
          if (_truthy(_eval(where, row.values, null))) row,
      ];
    }

    var frames = _makeFrames(rows, statement);
    final having = statement.having;
    if (having != null) {
      frames = [
        for (final frame in frames)
          if (_truthy(_eval(having, frame.row.values, frame.groupRows))) frame,
      ];
    }

    if (statement.orderBy.isNotEmpty) {
      frames.sort(
        (left, right) => _compareOrder(left, right, statement.orderBy),
      );
    }

    final projected = _project(frames, statement);
    var projectedRows = projected.rows;

    if (statement.distinct) {
      final seen = <String>{};
      projectedRows = [
        for (final row in projectedRows)
          if (seen.add(jsonEncode(row))) row,
      ];
    }

    final from = (statement.offset ?? 0).clamp(0, projectedRows.length);
    final limit = statement.limit == null
        ? projectedRows.length - from
        : statement.limit!.clamp(0, projectedRows.length);
    projectedRows = projectedRows.sublist(
      from,
      (from + limit).clamp(from, projectedRows.length),
    );

    return QueryResult(projected.columns, projectedRows);
  }

  static List<_RowContext> _scanTable(
    DataSource dataSource,
    String tableName,
    String alias,
  ) {
    final schema = dataSource.schema(tableName);
    return [
      for (final raw in dataSource.scan(tableName))
        _RowContext({
          for (final column in schema) ...{
            column: raw[column],
            '$alias.$column': raw[column],
            '$tableName.$column': raw[column],
          },
        }),
    ];
  }

  static List<_RowContext> _applyJoin(
    List<_RowContext> leftRows,
    List<_RowContext> rightRows,
    _JoinDef join,
  ) {
    final joined = <_RowContext>[];
    if (join.type == 'CROSS') {
      for (final left in leftRows) {
        for (final right in rightRows) {
          joined.add(left.merge(right));
        }
      }
      return joined;
    }

    for (final left in leftRows) {
      var matched = false;
      for (final right in rightRows) {
        final merged = left.merge(right);
        if (join.on == null || _truthy(_eval(join.on!, merged.values, null))) {
          joined.add(merged);
          matched = true;
        }
      }
      if (!matched && join.type == 'LEFT') joined.add(left);
    }
    return joined;
  }

  static List<_RowFrame> _makeFrames(
    List<_RowContext> rows,
    _SelectStatement statement,
  ) {
    final grouped = statement.groupBy.isNotEmpty;
    final aggregated =
        statement.selectItems.any((item) => _hasAggregate(item.expression)) ||
        (statement.having != null && _hasAggregate(statement.having!));
    if (!grouped && !aggregated)
      return [for (final row in rows) _RowFrame(row, null)];

    if (!grouped) {
      final row = rows.isEmpty ? _RowContext({}) : rows.first;
      return [_RowFrame(row, rows)];
    }

    final groups = <String, List<_RowContext>>{};
    for (final row in rows) {
      final keyValues = [
        for (final expression in statement.groupBy)
          _eval(expression, row.values, null),
      ];
      groups.putIfAbsent(jsonEncode(keyValues), () => []).add(row);
    }
    return [
      for (final groupRows in groups.values)
        _RowFrame(groupRows.first, groupRows),
    ];
  }

  static _Projection _project(
    List<_RowFrame> frames,
    _SelectStatement statement,
  ) {
    if (statement.selectItems.length == 1 &&
        statement.selectItems.first.expression is _StarExpr) {
      final columns =
          frames.isEmpty
                ? <String>[]
                : frames.first.row.values.keys
                      .where((key) => !key.contains('.'))
                      .toList()
            ..sort();
      final rows = [
        for (final frame in frames)
          [for (final column in columns) frame.row.values[column]],
      ];
      return _Projection(columns, rows);
    }

    final columns = [
      for (final item in statement.selectItems)
        item.alias ?? _expressionLabel(item.expression),
    ];
    final rows = [
      for (final frame in frames)
        [
          for (final item in statement.selectItems)
            _eval(item.expression, frame.row.values, frame.groupRows),
        ],
    ];
    return _Projection(columns, rows);
  }

  static int _compareOrder(
    _RowFrame left,
    _RowFrame right,
    List<_OrderItem> orderBy,
  ) {
    for (final item in orderBy) {
      final comparison = _compareSql(
        _eval(item.expression, left.row.values, left.groupRows),
        _eval(item.expression, right.row.values, right.groupRows),
      );
      if (comparison != 0) return item.descending ? -comparison : comparison;
    }
    return 0;
  }

  static Object? _eval(
    _Expr expression,
    Row row,
    List<_RowContext>? groupRows,
  ) {
    if (expression is _LiteralExpr) return expression.value;
    if (expression is _NullExpr) return null;
    if (expression is _ColumnExpr) {
      if (expression.table != null)
        return row['${expression.table}.${expression.name}'];
      if (row.containsKey(expression.name)) return row[expression.name];
      for (final entry in row.entries) {
        if (entry.key.endsWith('.${expression.name}')) return entry.value;
      }
      return null;
    }
    if (expression is _StarExpr) return row;
    if (expression is _UnaryExpr) {
      final value = _eval(expression.expression, row, groupRows);
      switch (expression.operator) {
        case 'NOT':
          return value == null ? null : !_truthy(value);
        case '-':
          return value == null ? null : -_asDouble(value);
        default:
          throw SqlExecutionException(
            'unknown unary operator: ${expression.operator}',
          );
      }
    }
    if (expression is _BinaryExpr)
      return _evalBinary(expression, row, groupRows);
    if (expression is _IsNullExpr) {
      final result = _eval(expression.expression, row, groupRows) == null;
      return expression.negated ? !result : result;
    }
    if (expression is _BetweenExpr) {
      final value = _eval(expression.expression, row, groupRows);
      final lower = _eval(expression.lower, row, groupRows);
      final upper = _eval(expression.upper, row, groupRows);
      if (value == null || lower == null || upper == null) return null;
      final result =
          _compareSql(value, lower) >= 0 && _compareSql(value, upper) <= 0;
      return expression.negated ? !result : result;
    }
    if (expression is _InListExpr) {
      final value = _eval(expression.expression, row, groupRows);
      if (value == null) return null;
      final found = expression.values.any((optionExpr) {
        final option = _eval(optionExpr, row, groupRows);
        return option != null && _sqlEquals(value, option);
      });
      return expression.negated ? !found : found;
    }
    if (expression is _LikeExpr) {
      final value = _eval(expression.expression, row, groupRows);
      final pattern = _eval(expression.pattern, row, groupRows);
      if (value == null || pattern == null) return null;
      final result = _like(value.toString(), pattern.toString());
      return expression.negated ? !result : result;
    }
    if (expression is _FunctionExpr)
      return _evalFunction(expression, row, groupRows);
    throw SqlExecutionException('unknown expression');
  }

  static Object? _evalBinary(
    _BinaryExpr binary,
    Row row,
    List<_RowContext>? groupRows,
  ) {
    if (binary.operator == 'AND') {
      final left = _eval(binary.left, row, groupRows);
      if (left != null && !_truthy(left)) return false;
      final right = _eval(binary.right, row, groupRows);
      if (right != null && !_truthy(right)) return false;
      return left == null || right == null ? null : true;
    }
    if (binary.operator == 'OR') {
      final left = _eval(binary.left, row, groupRows);
      if (left != null && _truthy(left)) return true;
      final right = _eval(binary.right, row, groupRows);
      if (right != null && _truthy(right)) return true;
      return left == null || right == null ? null : false;
    }

    final left = _eval(binary.left, row, groupRows);
    final right = _eval(binary.right, row, groupRows);
    if (left == null || right == null) return null;
    switch (binary.operator) {
      case '+':
        return _asDouble(left) + _asDouble(right);
      case '-':
        return _asDouble(left) - _asDouble(right);
      case '*':
        return _asDouble(left) * _asDouble(right);
      case '/':
        return _asDouble(left) / _asDouble(right);
      case '%':
        return _asDouble(left) % _asDouble(right);
      case '=':
        return _sqlEquals(left, right);
      case '!=':
      case '<>':
        return !_sqlEquals(left, right);
      case '<':
        return _compareSql(left, right) < 0;
      case '>':
        return _compareSql(left, right) > 0;
      case '<=':
        return _compareSql(left, right) <= 0;
      case '>=':
        return _compareSql(left, right) >= 0;
      default:
        throw SqlExecutionException('unknown operator: ${binary.operator}');
    }
  }

  static Object? _evalFunction(
    _FunctionExpr function,
    Row row,
    List<_RowContext>? groupRows,
  ) {
    final name = function.name.toUpperCase();
    if ({'COUNT', 'SUM', 'AVG', 'MIN', 'MAX'}.contains(name)) {
      final rows = groupRows;
      if (rows == null)
        throw SqlExecutionException(
          'aggregate used outside grouped context: $name',
        );
      if (name == 'COUNT') {
        if (function.args.length == 1 && function.args.first is _StarExpr)
          return rows.length;
        final arg = function.args.isEmpty ? null : function.args.first;
        if (arg == null) return rows.length;
        return rows.where((row) => _eval(arg, row.values, null) != null).length;
      }
      if (function.args.isEmpty)
        throw SqlExecutionException('aggregate requires an argument: $name');
      final values = [
        for (final row in rows)
          if (_eval(function.args.first, row.values, null) != null)
            _eval(function.args.first, row.values, null),
      ];
      if (values.isEmpty) return null;
      switch (name) {
        case 'SUM':
          return values.fold<double>(0, (sum, value) => sum + _asDouble(value));
        case 'AVG':
          return values.fold<double>(
                0,
                (sum, value) => sum + _asDouble(value),
              ) /
              values.length;
        case 'MIN':
          return values.reduce(
            (best, next) => _compareSql(next, best) < 0 ? next : best,
          );
        case 'MAX':
          return values.reduce(
            (best, next) => _compareSql(next, best) > 0 ? next : best,
          );
      }
    }

    final value = function.args.isEmpty
        ? null
        : _eval(function.args.first, row, groupRows);
    if (value == null) return null;
    switch (name) {
      case 'UPPER':
        return value.toString().toUpperCase();
      case 'LOWER':
        return value.toString().toLowerCase();
      case 'LENGTH':
        return value.toString().length;
      default:
        throw SqlExecutionException('unknown function: $name');
    }
  }

  static bool _hasAggregate(_Expr expression) {
    if (expression is _FunctionExpr) {
      return {
            'COUNT',
            'SUM',
            'AVG',
            'MIN',
            'MAX',
          }.contains(expression.name.toUpperCase()) ||
          expression.args.any(_hasAggregate);
    }
    if (expression is _BinaryExpr)
      return _hasAggregate(expression.left) || _hasAggregate(expression.right);
    if (expression is _UnaryExpr) return _hasAggregate(expression.expression);
    if (expression is _IsNullExpr) return _hasAggregate(expression.expression);
    if (expression is _BetweenExpr) {
      return _hasAggregate(expression.expression) ||
          _hasAggregate(expression.lower) ||
          _hasAggregate(expression.upper);
    }
    if (expression is _InListExpr) {
      return _hasAggregate(expression.expression) ||
          expression.values.any(_hasAggregate);
    }
    if (expression is _LikeExpr)
      return _hasAggregate(expression.expression) ||
          _hasAggregate(expression.pattern);
    return false;
  }

  static bool _truthy(Object? value) {
    if (value == null) return false;
    if (value is bool) return value;
    if (value is num) return value != 0;
    if (value is String) return value.isNotEmpty;
    return true;
  }

  static bool _sqlEquals(Object? left, Object? right) {
    if (left == null || right == null) return left == right;
    if (left is num && right is num) return left.compareTo(right) == 0;
    return left == right;
  }

  static int _compareSql(Object? left, Object? right) {
    final rankCompare = _rank(left).compareTo(_rank(right));
    if (rankCompare != 0) return rankCompare;
    if (left == null && right == null) return 0;
    if (left is bool && right is bool)
      return (left ? 1 : 0).compareTo(right ? 1 : 0);
    if (left is num && right is num) return left.compareTo(right);
    if (left is String && right is String) return left.compareTo(right);
    return left.toString().compareTo(right.toString());
  }

  static int _rank(Object? value) {
    if (value == null) return 0;
    if (value is bool) return 1;
    if (value is num) return 2;
    if (value is String) return 3;
    return 4;
  }

  static double _asDouble(Object? value) =>
      value is num ? value.toDouble() : double.parse(value.toString());

  static String _expressionLabel(_Expr expression) {
    if (expression is _ColumnExpr) return expression.name;
    if (expression is _FunctionExpr) {
      return expression.args.length == 1 && expression.args.first is _StarExpr
          ? '${expression.name.toUpperCase()}(*)'
          : '${expression.name.toUpperCase()}(...)';
    }
    if (expression is _LiteralExpr) return expression.value.toString();
    return '?';
  }

  static bool _like(String value, String sqlPattern) {
    final regex = StringBuffer();
    for (var index = 0; index < sqlPattern.length; index++) {
      final ch = sqlPattern[index];
      if (ch == '%') {
        regex.write('.*');
      } else if (ch == '_') {
        regex.write('.');
      } else {
        regex.write(RegExp.escape(ch));
      }
    }
    return RegExp('^${regex.toString()}\$').hasMatch(value);
  }

  static List<_Token> _tokenize(String sql) {
    final tokens = <_Token>[];
    var index = 0;
    while (index < sql.length) {
      final ch = sql[index];
      if (_isWhitespace(ch)) {
        index += 1;
      } else if (ch == '-' && index + 1 < sql.length && sql[index + 1] == '-') {
        index += 2;
        while (index < sql.length && sql[index] != '\n') {
          index += 1;
        }
      } else if (ch == "'") {
        final value = StringBuffer();
        index += 1;
        while (index < sql.length) {
          final current = sql[index];
          if (current == "'" &&
              index + 1 < sql.length &&
              sql[index + 1] == "'") {
            value.write("'");
            index += 2;
          } else if (current == "'") {
            index += 1;
            break;
          } else {
            value.write(current);
            index += 1;
          }
        }
        tokens.add(_Token(_TokenKind.string, value.toString()));
      } else if (_isDigit(ch) ||
          (ch == '.' && index + 1 < sql.length && _isDigit(sql[index + 1]))) {
        final start = index;
        index += 1;
        while (index < sql.length &&
            (_isDigit(sql[index]) || sql[index] == '.')) {
          index += 1;
        }
        tokens.add(_Token(_TokenKind.number, sql.substring(start, index)));
      } else if (_isIdentStart(ch)) {
        final start = index;
        index += 1;
        while (index < sql.length && _isIdentPart(sql[index])) {
          index += 1;
        }
        final value = sql.substring(start, index);
        final upper = value.toUpperCase();
        tokens.add(
          _Token(
            _keywords.contains(upper) ? _TokenKind.keyword : _TokenKind.ident,
            value,
          ),
        );
      } else if (ch == '"' || ch == '`') {
        final quote = ch;
        final start = index + 1;
        index = start;
        while (index < sql.length && sql[index] != quote) {
          index += 1;
        }
        tokens.add(_Token(_TokenKind.ident, sql.substring(start, index)));
        if (index < sql.length) index += 1;
      } else {
        if (index + 1 < sql.length) {
          final two = sql.substring(index, index + 2);
          if ({'!=', '<>', '<=', '>='}.contains(two)) {
            tokens.add(_Token(_TokenKind.symbol, two));
            index += 2;
            continue;
          }
        }
        if ('=<>+-*/%(),.;'.contains(ch))
          tokens.add(_Token(_TokenKind.symbol, ch));
        index += 1;
      }
    }
    tokens.add(_Token(_TokenKind.eof, ''));
    return tokens;
  }

  static bool _isWhitespace(String ch) => ch.trim().isEmpty;
  static bool _isDigit(String ch) =>
      ch.codeUnitAt(0) >= 48 && ch.codeUnitAt(0) <= 57;
  static bool _isIdentStart(String ch) {
    final code = ch.codeUnitAt(0);
    return (code >= 65 && code <= 90) ||
        (code >= 97 && code <= 122) ||
        ch == '_';
  }

  static bool _isIdentPart(String ch) => _isIdentStart(ch) || _isDigit(ch);
}

class _Parser {
  _Parser(this.tokens);

  final List<_Token> tokens;
  var position = 0;

  _Token get _peek => tokens[position];

  _Token _advance() {
    final token = tokens[position];
    if (token.kind != _TokenKind.eof) position += 1;
    return token;
  }

  SqlExecutionException _error(String message) =>
      SqlExecutionException('$message near token $position');

  _SelectStatement parseStatement() {
    _expectKeyword('SELECT');
    final distinct = _matchKeyword('DISTINCT');
    _matchKeyword('ALL');
    final selectItems = _parseSelectList();
    _expectKeyword('FROM');
    final from = _parseTableRef();
    final joins = _parseJoins();
    final where = _matchKeyword('WHERE') ? _parseExpression() : null;
    final groupBy = _matchKeyword('GROUP')
        ? (() {
            _expectKeyword('BY');
            return _parseExpressionList();
          })()
        : <_Expr>[];
    final having = _matchKeyword('HAVING') ? _parseExpression() : null;
    final orderBy = _matchKeyword('ORDER')
        ? (() {
            _expectKeyword('BY');
            return _parseOrderList();
          })()
        : <_OrderItem>[];
    final limit = _matchKeyword('LIMIT') ? _numberAsInt(_advance()) : null;
    final offset = _matchKeyword('OFFSET') ? _numberAsInt(_advance()) : null;
    _matchSymbol(';');
    _expect(_TokenKind.eof);
    return _SelectStatement(
      distinct,
      selectItems,
      from,
      joins,
      where,
      groupBy,
      having,
      orderBy,
      limit,
      offset,
    );
  }

  List<_SelectItem> _parseSelectList() {
    final items = <_SelectItem>[];
    do {
      if (_matchSymbol('*')) {
        items.add(_SelectItem(const _StarExpr(), null));
      } else {
        final expression = _parseExpression();
        final alias = _matchKeyword('AS')
            ? _expectIdentifier()
            : _peek.kind == _TokenKind.ident
            ? _advance().value
            : null;
        items.add(_SelectItem(expression, alias));
      }
    } while (_matchSymbol(','));
    return items;
  }

  _TableRef _parseTableRef() {
    final name = _expectIdentifier();
    final alias = _matchKeyword('AS')
        ? _expectIdentifier()
        : _peek.kind == _TokenKind.ident
        ? _advance().value
        : name;
    return _TableRef(name, alias);
  }

  List<_JoinDef> _parseJoins() {
    final joins = <_JoinDef>[];
    while (true) {
      String? type;
      if (_matchKeyword('INNER')) {
        _expectKeyword('JOIN');
        type = 'INNER';
      } else if (_matchKeyword('LEFT')) {
        _matchKeyword('OUTER');
        _expectKeyword('JOIN');
        type = 'LEFT';
      } else if (_matchKeyword('CROSS')) {
        _expectKeyword('JOIN');
        type = 'CROSS';
      } else if (_matchKeyword('JOIN')) {
        type = 'INNER';
      }
      if (type == null) break;
      final table = _parseTableRef();
      final on = type == 'CROSS'
          ? null
          : (() {
              _expectKeyword('ON');
              return _parseExpression();
            })();
      joins.add(_JoinDef(type, table, on));
    }
    return joins;
  }

  List<_Expr> _parseExpressionList() {
    final expressions = <_Expr>[];
    do {
      expressions.add(_parseExpression());
    } while (_matchSymbol(','));
    return expressions;
  }

  List<_OrderItem> _parseOrderList() {
    final items = <_OrderItem>[];
    do {
      final expression = _parseExpression();
      final descending = _matchKeyword('ASC') ? false : _matchKeyword('DESC');
      items.add(_OrderItem(expression, descending));
    } while (_matchSymbol(','));
    return items;
  }

  _Expr _parseExpression() => _parseOr();

  _Expr _parseOr() {
    var left = _parseAnd();
    while (_matchKeyword('OR')) {
      left = _BinaryExpr('OR', left, _parseAnd());
    }
    return left;
  }

  _Expr _parseAnd() {
    var left = _parseNot();
    while (_matchKeyword('AND')) {
      left = _BinaryExpr('AND', left, _parseNot());
    }
    return left;
  }

  _Expr _parseNot() => _matchKeyword('NOT')
      ? _UnaryExpr('NOT', _parseNot())
      : _parseComparison();

  _Expr _parseComparison() {
    final left = _parseAdditive();
    if (_matchKeyword('IS')) {
      final negated = _matchKeyword('NOT');
      _expectKeyword('NULL');
      return _IsNullExpr(left, negated);
    }
    if (_matchKeyword('NOT')) {
      if (_matchKeyword('BETWEEN')) {
        final lower = _parseAdditive();
        _expectKeyword('AND');
        return _BetweenExpr(left, lower, _parseAdditive(), true);
      }
      if (_matchKeyword('IN')) return _InListExpr(left, _parseInValues(), true);
      if (_matchKeyword('LIKE')) return _LikeExpr(left, _parseAdditive(), true);
      throw _error('expected BETWEEN, IN, or LIKE after NOT');
    }
    if (_matchKeyword('BETWEEN')) {
      final lower = _parseAdditive();
      _expectKeyword('AND');
      return _BetweenExpr(left, lower, _parseAdditive(), false);
    }
    if (_matchKeyword('IN')) return _InListExpr(left, _parseInValues(), false);
    if (_matchKeyword('LIKE')) return _LikeExpr(left, _parseAdditive(), false);
    if (_peek.kind == _TokenKind.symbol &&
        {'=', '!=', '<>', '<', '>', '<=', '>='}.contains(_peek.value)) {
      final operator = _advance().value;
      return _BinaryExpr(operator, left, _parseAdditive());
    }
    return left;
  }

  List<_Expr> _parseInValues() {
    _expectSymbol('(');
    final values = _parseExpressionList();
    _expectSymbol(')');
    return values;
  }

  _Expr _parseAdditive() {
    var left = _parseMultiplicative();
    while (_peek.kind == _TokenKind.symbol &&
        {'+', '-'}.contains(_peek.value)) {
      final operator = _advance().value;
      left = _BinaryExpr(operator, left, _parseMultiplicative());
    }
    return left;
  }

  _Expr _parseMultiplicative() {
    var left = _parseUnary();
    while (_peek.kind == _TokenKind.symbol &&
        {'*', '/', '%'}.contains(_peek.value)) {
      final operator = _advance().value;
      left = _BinaryExpr(operator, left, _parseUnary());
    }
    return left;
  }

  _Expr _parseUnary() =>
      _matchSymbol('-') ? _UnaryExpr('-', _parseUnary()) : _parsePrimary();

  _Expr _parsePrimary() {
    final token = _peek;
    if (_matchSymbol('(')) {
      final expression = _parseExpression();
      _expectSymbol(')');
      return expression;
    }
    if (token.kind == _TokenKind.number) {
      _advance();
      return _LiteralExpr(
        token.value.contains('.')
            ? double.parse(token.value)
            : int.parse(token.value),
      );
    }
    if (token.kind == _TokenKind.string) {
      _advance();
      return _LiteralExpr(token.value);
    }
    if (_matchKeyword('NULL')) return const _NullExpr();
    if (_matchKeyword('TRUE')) return const _LiteralExpr(true);
    if (_matchKeyword('FALSE')) return const _LiteralExpr(false);
    if (_matchSymbol('*')) return const _StarExpr();
    if (token.kind == _TokenKind.ident || token.kind == _TokenKind.keyword) {
      final name = _advance().value;
      if (_matchSymbol('(')) {
        final args = <_Expr>[];
        if (!_matchSymbol(')')) {
          if (_matchSymbol('*')) {
            args.add(const _StarExpr());
          } else {
            args.add(_parseExpression());
            while (_matchSymbol(',')) {
              args.add(_parseExpression());
            }
          }
          _expectSymbol(')');
        }
        return _FunctionExpr(name, args);
      }
      if (_matchSymbol('.')) return _ColumnExpr(name, _expectIdentifier());
      return _ColumnExpr(null, name);
    }
    throw _error('unexpected token: ${token.value}');
  }

  String _expectIdentifier() {
    final token = _advance();
    if (token.kind == _TokenKind.ident || token.kind == _TokenKind.keyword)
      return token.value;
    throw _error('expected identifier');
  }

  int _numberAsInt(_Token token) {
    if (token.kind != _TokenKind.number) throw _error('expected number');
    return int.parse(token.value);
  }

  void _expect(_TokenKind kind) {
    final token = _advance();
    if (token.kind != kind) throw _error('expected $kind, got ${token.kind}');
  }

  void _expectKeyword(String value) {
    if (!_matchKeyword(value)) throw _error('expected $value');
  }

  bool _matchKeyword(String value) {
    if (_peek.kind == _TokenKind.keyword &&
        _peek.value.toUpperCase() == value) {
      _advance();
      return true;
    }
    return false;
  }

  void _expectSymbol(String value) {
    if (!_matchSymbol(value)) throw _error('expected $value');
  }

  bool _matchSymbol(String value) {
    if (_peek.kind == _TokenKind.symbol && _peek.value == value) {
      _advance();
      return true;
    }
    return false;
  }
}

enum _TokenKind { ident, keyword, number, string, symbol, eof }

class _Token {
  const _Token(this.kind, this.value);

  final _TokenKind kind;
  final String value;
}

class _SelectStatement {
  const _SelectStatement(
    this.distinct,
    this.selectItems,
    this.from,
    this.joins,
    this.where,
    this.groupBy,
    this.having,
    this.orderBy,
    this.limit,
    this.offset,
  );

  final bool distinct;
  final List<_SelectItem> selectItems;
  final _TableRef from;
  final List<_JoinDef> joins;
  final _Expr? where;
  final List<_Expr> groupBy;
  final _Expr? having;
  final List<_OrderItem> orderBy;
  final int? limit;
  final int? offset;
}

class _SelectItem {
  const _SelectItem(this.expression, this.alias);
  final _Expr expression;
  final String? alias;
}

class _TableRef {
  const _TableRef(this.name, this.alias);
  final String name;
  final String alias;
}

class _JoinDef {
  const _JoinDef(this.type, this.table, this.on);
  final String type;
  final _TableRef table;
  final _Expr? on;
}

class _OrderItem {
  const _OrderItem(this.expression, this.descending);
  final _Expr expression;
  final bool descending;
}

class _RowContext {
  _RowContext(Map<String, Object?> values)
    : values = Map<String, Object?>.from(values);

  final Row values;

  _RowContext merge(_RowContext other) =>
      _RowContext({...values, ...other.values});
}

class _RowFrame {
  const _RowFrame(this.row, this.groupRows);
  final _RowContext row;
  final List<_RowContext>? groupRows;
}

class _Projection {
  const _Projection(this.columns, this.rows);
  final List<String> columns;
  final List<List<Object?>> rows;
}

abstract class _Expr {
  const _Expr();
}

class _LiteralExpr extends _Expr {
  const _LiteralExpr(this.value);
  final Object? value;
}

class _NullExpr extends _Expr {
  const _NullExpr();
}

class _ColumnExpr extends _Expr {
  const _ColumnExpr(this.table, this.name);
  final String? table;
  final String name;
}

class _StarExpr extends _Expr {
  const _StarExpr();
}

class _UnaryExpr extends _Expr {
  const _UnaryExpr(this.operator, this.expression);
  final String operator;
  final _Expr expression;
}

class _BinaryExpr extends _Expr {
  const _BinaryExpr(this.operator, this.left, this.right);
  final String operator;
  final _Expr left;
  final _Expr right;
}

class _IsNullExpr extends _Expr {
  const _IsNullExpr(this.expression, this.negated);
  final _Expr expression;
  final bool negated;
}

class _BetweenExpr extends _Expr {
  const _BetweenExpr(this.expression, this.lower, this.upper, this.negated);
  final _Expr expression;
  final _Expr lower;
  final _Expr upper;
  final bool negated;
}

class _InListExpr extends _Expr {
  const _InListExpr(this.expression, this.values, this.negated);
  final _Expr expression;
  final List<_Expr> values;
  final bool negated;
}

class _LikeExpr extends _Expr {
  const _LikeExpr(this.expression, this.pattern, this.negated);
  final _Expr expression;
  final _Expr pattern;
  final bool negated;
}

class _FunctionExpr extends _Expr {
  const _FunctionExpr(this.name, this.args);
  final String name;
  final List<_Expr> args;
}
