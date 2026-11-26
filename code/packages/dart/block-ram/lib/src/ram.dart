/// Synchronous RAM modules: single-port and dual-port RAM with
/// configurable read modes.
///
/// A module adds address decoding and clock-edge operation over an
/// [SramArray]: each `tick` samples its inputs, and operations land on
/// the rising edge (clock 0 → 1). During a write, [ReadMode] decides
/// what the output shows.
library;

import 'sram.dart';

/// Controls what `data_out` shows during a write operation.
///
/// ```text
/// readFirst:  data_out = old value (read before write)
/// writeFirst: data_out = new value (write before read)
/// noChange:   data_out = previous read value (output unchanged)
/// ```
enum ReadMode {
  /// Read the old value before writing the new one.
  readFirst,

  /// Write first, then read back the new value.
  writeFirst,

  /// Output retains its previous value during writes.
  noChange,
}

/// Thrown when both dual-port RAM ports write to the same address.
///
/// Real hardware leaves this undefined (either value, or corruption);
/// the simulation reports it instead of guessing silently.
final class WriteCollisionError implements Exception {
  const WriteCollisionError({required this.address});

  /// The conflicting address.
  final int address;

  @override
  bool operator ==(Object other) =>
      other is WriteCollisionError && other.address == address;

  @override
  int get hashCode => address.hashCode;

  @override
  String toString() =>
      'Write collision: both ports writing to address $address';
}

/// Single-port synchronous RAM: one address port, one data bus.
///
/// Each cycle performs one operation (read or write, by `writeEnable`)
/// on the rising clock edge; otherwise the previous output repeats.
final class SinglePortRAM {
  /// Creates a [depth] x [width] RAM with [readMode], all zeros.
  ///
  /// Throws [ArgumentError] if either dimension is below 1.
  factory SinglePortRAM(int depth, int width, ReadMode readMode) {
    if (depth < 1) {
      throw ArgumentError.value(
          depth, 'depth', 'depth must be >= 1, got $depth');
    }
    if (width < 1) {
      throw ArgumentError.value(
          width, 'width', 'width must be >= 1, got $width');
    }
    return SinglePortRAM._(
        depth, width, readMode, SramArray(depth, width), List.filled(width, 0));
  }

  SinglePortRAM._(
      this._depth, this._width, this.readMode, this._array, this._lastRead);

  final int _depth;
  final int _width;

  /// What the output shows during writes.
  final ReadMode readMode;
  final SramArray _array;
  int _prevClock = 0;
  List<int> _lastRead;

  /// Executes one half-cycle, returning `data_out` (`width` bits).
  ///
  /// Throws [ArgumentError] if [address] is out of range, [dataIn]
  /// has the wrong length, or [clock], [writeEnable], or (on the write
  /// path) any bit of [dataIn] is not 0 or 1.
  List<int> tick(int clock, int address, List<int> dataIn, int writeEnable) {
    validateBit(clock, 'clock');
    validateBit(writeEnable, 'write_enable');
    if (address < 0 || address >= _depth) {
      throw ArgumentError.value(address, 'address',
          'address $address out of range [0, ${_depth - 1}]');
    }
    if (dataIn.length != _width) {
      throw ArgumentError.value(dataIn, 'dataIn',
          'data_in length ${dataIn.length} does not match width $_width');
    }

    final risingEdge = _prevClock == 0 && clock == 1;
    _prevClock = clock;
    if (!risingEdge) {
      return List.of(_lastRead);
    }
    if (writeEnable == 0) {
      _lastRead = _array.read(address);
      return List.of(_lastRead);
    }
    switch (readMode) {
      case ReadMode.readFirst:
        _lastRead = _array.read(address);
        _array.write(address, dataIn);
        return List.of(_lastRead);
      case ReadMode.writeFirst:
        _array.write(address, dataIn);
        _lastRead = List.of(dataIn);
        return List.of(_lastRead);
      case ReadMode.noChange:
        _array.write(address, dataIn);
        return List.of(_lastRead);
    }
  }

  /// Number of addressable words.
  int depth() => _depth;

  /// Bits per word.
  int width() => _width;

  /// All contents for inspection, one row per word.
  List<List<int>> dump() =>
      [for (var row = 0; row < _depth; row++) _array.read(row)];
}

/// True dual-port synchronous RAM: two independent ports.
///
/// Both ports operate simultaneously on different addresses. A
/// same-address, same-cycle double write throws [WriteCollisionError].
final class DualPortRAM {
  /// Creates a [depth] x [width] RAM with per-port read modes.
  ///
  /// Throws [ArgumentError] if either dimension is below 1.
  factory DualPortRAM(
      int depth, int width, ReadMode readModeA, ReadMode readModeB) {
    if (depth < 1) {
      throw ArgumentError.value(
          depth, 'depth', 'depth must be >= 1, got $depth');
    }
    if (width < 1) {
      throw ArgumentError.value(
          width, 'width', 'width must be >= 1, got $width');
    }
    return DualPortRAM._(depth, width, readModeA, readModeB,
        SramArray(depth, width), List.filled(width, 0), List.filled(width, 0));
  }

  DualPortRAM._(this._depth, this._width, this.readModeA, this.readModeB,
      this._array, this._lastReadA, this._lastReadB);

  final int _depth;
  final int _width;

  /// What port A shows during its writes.
  final ReadMode readModeA;

  /// What port B shows during its writes.
  final ReadMode readModeB;
  final SramArray _array;
  int _prevClock = 0;
  List<int> _lastReadA;
  List<int> _lastReadB;

  /// Executes one half-cycle on both ports.
  ///
  /// Returns `(dataOutA, dataOutB)`. Throws [WriteCollisionError] when
  /// both ports write to the same address on a rising edge, and
  /// [ArgumentError] for out-of-range addresses, mis-sized data, or
  /// non-bit clock, write-enable, or written data values.
  (List<int>, List<int>) tick(
    int clock,
    int addressA,
    List<int> dataInA,
    int writeEnableA,
    int addressB,
    List<int> dataInB,
    int writeEnableB,
  ) {
    validateBit(clock, 'clock');
    validateBit(writeEnableA, 'write_enable_a');
    validateBit(writeEnableB, 'write_enable_b');
    if (addressA < 0 || addressA >= _depth) {
      throw ArgumentError.value(addressA, 'addressA',
          'address_a $addressA out of range [0, ${_depth - 1}]');
    }
    if (addressB < 0 || addressB >= _depth) {
      throw ArgumentError.value(addressB, 'addressB',
          'address_b $addressB out of range [0, ${_depth - 1}]');
    }
    if (dataInA.length != _width) {
      throw ArgumentError.value(dataInA, 'dataInA',
          'data_in_a length ${dataInA.length} does not match width $_width');
    }
    if (dataInB.length != _width) {
      throw ArgumentError.value(dataInB, 'dataInB',
          'data_in_b length ${dataInB.length} does not match width $_width');
    }

    final risingEdge = _prevClock == 0 && clock == 1;
    _prevClock = clock;
    if (!risingEdge) {
      return (List.of(_lastReadA), List.of(_lastReadB));
    }
    if (writeEnableA == 1 && writeEnableB == 1 && addressA == addressB) {
      throw WriteCollisionError(address: addressA);
    }

    final outA =
        _processPort(addressA, dataInA, writeEnableA, readModeA, _lastReadA);
    _lastReadA = List.of(outA);
    final outB =
        _processPort(addressB, dataInB, writeEnableB, readModeB, _lastReadB);
    _lastReadB = List.of(outB);
    return (outA, outB);
  }

  /// Runs one port's operation against the shared array.
  List<int> _processPort(int address, List<int> dataIn, int writeEnable,
      ReadMode readMode, List<int> lastRead) {
    if (writeEnable == 0) {
      return _array.read(address);
    }
    switch (readMode) {
      case ReadMode.readFirst:
        final result = _array.read(address);
        _array.write(address, dataIn);
        return result;
      case ReadMode.writeFirst:
        _array.write(address, dataIn);
        return List.of(dataIn);
      case ReadMode.noChange:
        _array.write(address, dataIn);
        return lastRead;
    }
  }

  /// Number of addressable words.
  int depth() => _depth;

  /// Bits per word.
  int width() => _width;
}
