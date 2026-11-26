/// FPGA-style block RAM with reconfigurable aspect ratio.
///
/// A tile of fixed total storage whose width/depth ratio can change:
/// reconfiguring rewires the address decoder and column MUX (modeled by
/// rebuilding the underlying dual-port RAM, which clears stored data).
library;

import 'ram.dart';
import 'sram.dart';

/// Block RAM with configurable aspect ratio and dual-port access.
final class ConfigurableBRAM {
  /// Creates a tile of [totalBits] bits at [width] bits per word.
  ///
  /// Throws [ArgumentError] if either is below 1 or [width] does not
  /// divide [totalBits] evenly.
  factory ConfigurableBRAM(int totalBits, int width) {
    if (totalBits < 1) {
      throw ArgumentError.value(
          totalBits, 'totalBits', 'total_bits must be >= 1, got $totalBits');
    }
    if (width < 1) {
      throw ArgumentError.value(
          width, 'width', 'width must be >= 1, got $width');
    }
    if (totalBits % width != 0) {
      throw ArgumentError.value(width, 'width',
          'width $width does not evenly divide total_bits $totalBits');
    }
    return ConfigurableBRAM._(
        totalBits,
        width,
        totalBits ~/ width,
        DualPortRAM(
            totalBits ~/ width, width, ReadMode.readFirst, ReadMode.readFirst));
  }

  ConfigurableBRAM._(this._totalBits, this._width, this._depth, this._ram);

  final int _totalBits;
  int _width;
  int _depth;
  DualPortRAM _ram;

  /// Changes the aspect ratio, clearing all stored data.
  ///
  /// Throws [ArgumentError] if [width] is below 1 or does not divide
  /// the total bits evenly.
  void reconfigure(int width) {
    if (width < 1) {
      throw ArgumentError.value(
          width, 'width', 'width must be >= 1, got $width');
    }
    if (_totalBits % width != 0) {
      throw ArgumentError.value(width, 'width',
          'width $width does not evenly divide total_bits $_totalBits');
    }
    _width = width;
    _depth = _totalBits ~/ width;
    _ram = DualPortRAM(_depth, _width, ReadMode.readFirst, ReadMode.readFirst);
  }

  /// Port A operation (port B idles on a read of address 0).
  ///
  /// Throws [ArgumentError] for out-of-range addresses, mis-sized
  /// data, or non-bit signal values. A write collision is impossible
  /// with the other port idle.
  List<int> tickA(int clock, int address, List<int> dataIn, int writeEnable) {
    validateBit(clock, 'clock');
    final zeros = List.filled(_width, 0);
    final (outA, _) =
        _ram.tick(clock, address, dataIn, writeEnable, 0, zeros, 0);
    return outA;
  }

  /// Port B operation (port A idles on a read of address 0).
  ///
  /// Throws [ArgumentError] for out-of-range addresses, mis-sized
  /// data, or non-bit signal values. A write collision is impossible
  /// with the other port idle.
  List<int> tickB(int clock, int address, List<int> dataIn, int writeEnable) {
    validateBit(clock, 'clock');
    final zeros = List.filled(_width, 0);
    final (_, outB) =
        _ram.tick(clock, 0, zeros, 0, address, dataIn, writeEnable);
    return outB;
  }

  /// Addressable words at the current configuration.
  int depth() => _depth;

  /// Bits per word at the current configuration.
  int width() => _width;

  /// Total storage capacity in bits (fixed).
  int totalBits() => _totalBits;
}
