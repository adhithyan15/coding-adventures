/// SRAM cells and arrays: single-bit storage and 2D grids with
/// row/column addressing.
///
/// A 6-transistor SRAM cell (cross-coupled inverters + access
/// transistors) is modeled by its steady-state behavior: isolated cells
/// hold their value, selected cells read out, and driven bit lines
/// overwrite. Arrays organize cells into rows sharing word lines.
library;

/// Rejects non-bit values with [ArgumentError], always.
///
/// Deliberate fail-closed strengthening over the reference's
/// `debug_assert`: release-mode Rust silently stores garbage bits, which
/// is exactly the silent-corruption class the publish bar forbids. Every
/// other lane upgrades asserts to [ArgumentError] the same way; the
/// *placement* of the checks (which paths validate, and in what order)
/// still mirrors the reference bit for bit.
/// Throws [ArgumentError] if [value] is not 0 or 1.
void validateBit(int value, String name) {
  if (value != 0 && value != 1) {
    throw ArgumentError.value(value, name, '$name must be 0 or 1, got $value');
  }
}

/// Single-bit storage element modeled at the gate level.
///
/// Internally a pair of cross-coupled inverters gated by access
/// transistors; the simulation models steady-state behavior directly:
/// word line 0 isolates (reads yield `null`, writes are ignored),
/// word line 1 selects (reads yield the bit, writes overwrite it).
final class SramCell {
  /// Creates a cell initialized to 0 (power-on reset state).
  SramCell() : _value = 0;

  int _value;

  /// Reads the stored bit when selected, else `null`.
  ///
  /// Throws [ArgumentError] if [wordLine] is not 0 or 1.
  int? read(int wordLine) {
    validateBit(wordLine, 'word_line');
    return wordLine == 0 ? null : _value;
  }

  /// Writes [bitLine] when selected; ignored otherwise.
  ///
  /// Throws [ArgumentError] if either line is not 0 or 1.
  void write(int wordLine, int bitLine) {
    validateBit(wordLine, 'word_line');
    validateBit(bitLine, 'bit_line');
    if (wordLine == 1) {
      _value = bitLine;
    }
  }

  /// The current stored value, for inspection.
  int value() => _value;

  @override
  bool operator ==(Object other) => other is SramCell && other._value == _value;

  @override
  int get hashCode => _value.hashCode;

  @override
  String toString() => 'SramCell($_value)';
}

/// 2D grid of SRAM cells with row/column addressing.
///
/// Each row shares a word line; each column shares a bit line. Reading a
/// row outputs every cell in it; writing a row stores one bit per cell.
final class SramArray {
  /// Creates a [rows] x [cols] array initialized to all zeros.
  ///
  /// Throws [ArgumentError] if either dimension is below 1.
  factory SramArray(int rows, int cols) {
    if (rows < 1) {
      throw ArgumentError.value(rows, 'rows', 'rows must be >= 1, got $rows');
    }
    if (cols < 1) {
      throw ArgumentError.value(cols, 'cols', 'cols must be >= 1, got $cols');
    }
    return SramArray._(rows, cols,
        List.generate(rows, (_) => List.generate(cols, (_) => SramCell())));
  }

  SramArray._(this._rows, this._cols, this._cells);

  final int _rows;
  final int _cols;
  final List<List<SramCell>> _cells;

  /// Reads every column of [row].
  ///
  /// Throws [ArgumentError] if [row] is out of range.
  List<int> read(int row) {
    if (row < 0 || row >= _rows) {
      throw ArgumentError.value(
          row, 'row', 'row $row out of range [0, ${_rows - 1}]');
    }
    return [for (final cell in _cells[row]) cell.read(1)!];
  }

  /// Writes [data] (exactly [cols] bits) to [row].
  ///
  /// Throws [ArgumentError] if [row] is out of range, [data] has the
  /// wrong length, or any element of [data] is not 0 or 1.
  void write(int row, List<int> data) {
    if (row < 0 || row >= _rows) {
      throw ArgumentError.value(
          row, 'row', 'row $row out of range [0, ${_rows - 1}]');
    }
    if (data.length != _cols) {
      throw ArgumentError.value(data, 'data',
          'data length ${data.length} does not match cols $_cols');
    }
    for (var i = 0; i < data.length; i++) {
      validateBit(data[i], 'data[$i]');
    }
    for (var col = 0; col < data.length; col++) {
      _cells[row][col].write(1, data[col]);
    }
  }

  /// Dimensions as (rows, cols).
  (int, int) shape() => (_rows, _cols);
}
