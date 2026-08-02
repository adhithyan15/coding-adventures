import 'dart:math' as math;

/// A row/column position returned by [Matrix.argmin] and [Matrix.argmax].
class MatrixIndex {
  const MatrixIndex(this.row, this.column);

  final int row;
  final int column;

  @override
  bool operator ==(Object other) =>
      other is MatrixIndex && row == other.row && column == other.column;

  @override
  int get hashCode => Object.hash(row, column);

  @override
  String toString() => 'MatrixIndex($row, $column)';
}

/// An immutable rectangular matrix of double-precision values.
///
/// A scalar becomes a 1x1 matrix, a one-dimensional list becomes a row vector,
/// and a nested list remains a two-dimensional grid. Construction deep-copies
/// the input so later caller mutation cannot change the matrix.
class Matrix {
  Matrix(Object data) : _data = _normalize(data);

  Matrix._fromRows(List<List<double>> rows) : _data = _freeze(rows);

  final List<List<double>> _data;

  /// Immutable row-major matrix data.
  List<List<double>> get data => _data;

  int get rows => _data.length;
  int get cols => _data.first.length;

  static Matrix zeros(int rows, int cols) {
    _validateDimensions(rows, cols);
    return Matrix._fromRows(
      List.generate(rows, (_) => List<double>.filled(cols, 0.0)),
    );
  }

  static Matrix identity(int size) {
    _validateDimensions(size, size);
    return Matrix._fromRows(
      List.generate(
        size,
        (row) => List.generate(size, (column) => row == column ? 1.0 : 0.0),
      ),
    );
  }

  static Matrix fromDiagonal(List<num> values) {
    if (values.isEmpty) {
      throw ArgumentError.value(values, 'values', 'must not be empty');
    }
    return Matrix._fromRows(
      List.generate(
        values.length,
        (row) => List.generate(
          values.length,
          (column) => row == column ? values[row].toDouble() : 0.0,
        ),
      ),
    );
  }

  Matrix add(Object other) =>
      _elementWise(other, (left, right) => left + right, 'addition');

  Matrix subtract(Object other) =>
      _elementWise(other, (left, right) => left - right, 'subtraction');

  Matrix operator +(Object other) => add(other);
  Matrix operator -(Object other) => subtract(other);

  Matrix operator *(num scalar) => scale(scalar);

  Matrix scale(num scalar) => map((value) => value * scalar.toDouble());

  Matrix transpose() => Matrix._fromRows(
        List.generate(
          cols,
          (column) => List.generate(rows, (row) => _data[row][column]),
        ),
      );

  /// True matrix multiplication: `(M x K) dot (K x N) = M x N`.
  Matrix dot(Matrix other) {
    if (cols != other.rows) {
      throw ArgumentError(
        'dot dimension mismatch: ${rows}x$cols versus ${other.rows}x${other.cols}',
      );
    }
    return Matrix._fromRows(
      List.generate(rows, (row) {
        return List.generate(other.cols, (column) {
          var total = 0.0;
          for (var inner = 0; inner < cols; inner++) {
            total += _data[row][inner] * other._data[inner][column];
          }
          return total;
        });
      }),
    );
  }

  double get(int row, int column) {
    _checkIndex(row, column);
    return _data[row][column];
  }

  Matrix set(int row, int column, num value) {
    _checkIndex(row, column);
    final next = _data.map((values) => values.toList()).toList();
    next[row][column] = value.toDouble();
    return Matrix._fromRows(next);
  }

  double sum() {
    var total = 0.0;
    for (final row in _data) {
      for (final value in row) {
        total += value;
      }
    }
    return total;
  }

  Matrix sumRows() => Matrix._fromRows(
        _data
            .map((row) => [row.fold(0.0, (total, value) => total + value)])
            .toList(),
      );

  Matrix sumCols() => Matrix._fromRows([
        List.generate(
          cols,
          (column) => _data.fold(0.0, (total, row) => total + row[column]),
        ),
      ]);

  double mean() => sum() / (rows * cols);

  double min() => _data.expand((row) => row).reduce(math.min);
  double max() => _data.expand((row) => row).reduce(math.max);

  MatrixIndex argmin() => _argExtreme((candidate, best) => candidate < best);
  MatrixIndex argmax() => _argExtreme((candidate, best) => candidate > best);

  Matrix map(double Function(double value) transform) => Matrix._fromRows(
        _data.map((row) => row.map(transform).toList()).toList(),
      );

  Matrix sqrt() {
    if (_data.expand((row) => row).any((value) => value < 0.0)) {
      throw ArgumentError(
        'square root is undefined for negative matrix values',
      );
    }
    return map(math.sqrt);
  }

  Matrix abs() => map((value) => value.abs());

  Matrix pow(num exponent) =>
      map((value) => math.pow(value, exponent).toDouble());

  Matrix flatten() => Matrix._fromRows([_data.expand((row) => row).toList()]);

  Matrix reshape(int rows, int cols) {
    _validateDimensions(rows, cols);
    if (rows * cols != this.rows * this.cols) {
      throw ArgumentError(
        'cannot reshape ${this.rows}x${this.cols} into ${rows}x$cols',
      );
    }
    final flat = _data.expand((row) => row).toList();
    return Matrix._fromRows(
      List.generate(rows, (row) => flat.sublist(row * cols, (row + 1) * cols)),
    );
  }

  Matrix row(int index) {
    RangeError.checkValidIndex(index, _data, 'row');
    return Matrix._fromRows([_data[index].toList()]);
  }

  Matrix col(int index) {
    RangeError.checkValidIndex(index, _data.first, 'column');
    return Matrix._fromRows(_data.map((row) => [row[index]]).toList());
  }

  Matrix slice(int rowStart, int rowEnd, int columnStart, int columnEnd) {
    if (rowStart < 0 || rowEnd > rows) {
      throw RangeError.range(
        rowStart < 0 ? rowStart : rowEnd,
        0,
        rows,
        'row range',
      );
    }
    if (columnStart < 0 || columnEnd > cols) {
      throw RangeError.range(
        columnStart < 0 ? columnStart : columnEnd,
        0,
        cols,
        'column range',
      );
    }
    if (rowStart >= rowEnd || columnStart >= columnEnd) {
      throw ArgumentError('slice ranges must be non-empty and increasing');
    }
    return Matrix._fromRows(
      _data
          .sublist(rowStart, rowEnd)
          .map((row) => row.sublist(columnStart, columnEnd))
          .toList(),
    );
  }

  bool equals(Matrix other) {
    if (rows != other.rows || cols != other.cols) return false;
    for (var row = 0; row < rows; row++) {
      for (var column = 0; column < cols; column++) {
        if (_data[row][column] != other._data[row][column]) return false;
      }
    }
    return true;
  }

  bool close(Matrix other, {double tolerance = 1e-9}) {
    if (tolerance < 0 || rows != other.rows || cols != other.cols) return false;
    for (var row = 0; row < rows; row++) {
      for (var column = 0; column < cols; column++) {
        final left = _data[row][column];
        final right = other._data[row][column];
        if (left.isNaN || right.isNaN) return false;
        if (left == right) continue;
        if (!left.isFinite ||
            !right.isFinite ||
            (left - right).abs() > tolerance) {
          return false;
        }
      }
    }
    return true;
  }

  @override
  bool operator ==(Object other) => other is Matrix && equals(other);

  @override
  int get hashCode =>
      Object.hash(rows, cols, Object.hashAll(_data.expand((row) => row)));

  @override
  String toString() => 'Matrix($_data)';

  Matrix _elementWise(
    Object other,
    double Function(double left, double right) operation,
    String name,
  ) {
    if (other is num) {
      final scalar = other.toDouble();
      return map((value) => operation(value, scalar));
    }
    if (other is! Matrix) {
      throw ArgumentError.value(other, 'other', 'must be a Matrix or num');
    }
    if (rows != other.rows || cols != other.cols) {
      throw ArgumentError(
        '$name dimension mismatch: ${rows}x$cols versus ${other.rows}x${other.cols}',
      );
    }
    return Matrix._fromRows(
      List.generate(
        rows,
        (row) => List.generate(
          cols,
          (column) => operation(_data[row][column], other._data[row][column]),
        ),
      ),
    );
  }

  MatrixIndex _argExtreme(
    bool Function(double candidate, double best) isBetter,
  ) {
    var best = _data[0][0];
    var bestRow = 0;
    var bestColumn = 0;
    for (var row = 0; row < rows; row++) {
      for (var column = 0; column < cols; column++) {
        final candidate = _data[row][column];
        if (isBetter(candidate, best)) {
          best = candidate;
          bestRow = row;
          bestColumn = column;
        }
      }
    }
    return MatrixIndex(bestRow, bestColumn);
  }

  void _checkIndex(int row, int column) {
    RangeError.checkValidIndex(row, _data, 'row');
    RangeError.checkValidIndex(column, _data.first, 'column');
  }

  static List<List<double>> _normalize(Object data) {
    if (data is num) {
      return _freeze([
        [data.toDouble()],
      ]);
    }
    if (data is! List || data.isEmpty) {
      throw ArgumentError.value(
        data,
        'data',
        'must be a scalar or non-empty numeric list',
      );
    }
    if (data.every((value) => value is num)) {
      return _freeze([
        data.cast<num>().map((value) => value.toDouble()).toList(),
      ]);
    }
    if (!data.every((value) => value is List)) {
      throw ArgumentError.value(
        data,
        'data',
        'must not mix rows and scalar values',
      );
    }
    final rawRows = data.cast<List>();
    if (rawRows.first.isEmpty) {
      throw ArgumentError.value(data, 'data', 'rows must not be empty');
    }
    final width = rawRows.first.length;
    final rows = <List<double>>[];
    for (final row in rawRows) {
      if (row.length != width || !row.every((value) => value is num)) {
        throw ArgumentError.value(
          data,
          'data',
          'must be a rectangular numeric grid',
        );
      }
      rows.add(row.cast<num>().map((value) => value.toDouble()).toList());
    }
    return _freeze(rows);
  }

  static List<List<double>> _freeze(List<List<double>> rows) =>
      List<List<double>>.unmodifiable(rows.map(List<double>.unmodifiable));

  static void _validateDimensions(int rows, int cols) {
    if (rows <= 0 || cols <= 0) {
      throw ArgumentError('matrix dimensions must be positive');
    }
  }
}
