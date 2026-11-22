import 'dart:convert';
import 'dart:math' as math;

import 'package:coding_adventures_hash_functions/coding_adventures_hash_functions.dart';

import 'constants.dart';
import 'errors.dart';

double _alpha(int registerCount) {
  switch (registerCount) {
    case 16:
      return alpha16;
    case 32:
      return alpha32;
    case 64:
      return alpha64;
    default:
      return alphaInfNum / (1.0 + alphaInfDenom / registerCount);
  }
}

BigInt _fmix64(BigInt input) {
  var value = input & mask64;
  value = (value ^ (value >> fmix64Shift)) & mask64;
  value = (value * fmix64Mul1) & mask64;
  value = (value ^ (value >> fmix64Shift)) & mask64;
  value = (value * fmix64Mul2) & mask64;
  value = (value ^ (value >> fmix64Shift)) & mask64;
  return value;
}

void _validateString(String text) {
  final units = text.codeUnits;
  for (var i = 0; i < units.length; i++) {
    final cu = units[i];
    if (cu >= highSurrogateMin && cu <= highSurrogateMax) {
      if (i + 1 >= units.length) {
        throw HyperLogLogException.invalidInput();
      }
      final next = units[++i];
      if (next < lowSurrogateMin || next > lowSurrogateMax) {
        throw HyperLogLogException.invalidInput();
      }
    } else if (cu >= lowSurrogateMin && cu <= lowSurrogateMax) {
      throw HyperLogLogException.invalidInput();
    }
  }
}

/// A persistent HyperLogLog cardinality sketch.
///
/// Values are hashed with DT17 FNV-1a 64-bit followed by the fixed MurmurHash3
/// 64-bit finalizer. Every update and merge returns a new value, leaving the
/// receiver unchanged.
final class HyperLogLog {
  /// The precision parameter p (between 4 and 16).
  final int precision;

  final List<int> _registers;

  /// Creates a new empty HyperLogLog sketch with the given [precision].
  factory HyperLogLog([int precision = defaultPrecision]) {
    _validatePrecision(precision);
    final count = 1 << precision;
    return HyperLogLog._(
      precision: precision,
      registers: List<int>.filled(count, 0, growable: false),
    );
  }

  /// Creates a sketch from existing register values.
  factory HyperLogLog.fromRegisters({
    required int precision,
    required List<int> registers,
  }) {
    _validatePrecision(precision);
    final count = 1 << precision;
    if (registers.length != count) {
      throw HyperLogLogException.invalidInput();
    }
    for (final r in registers) {
      if (r < 0 || r > hashBitWidth) {
        throw HyperLogLogException.invalidInput();
      }
    }
    return HyperLogLog._(
      precision: precision,
      registers: List<int>.unmodifiable(registers),
    );
  }

  /// Internal constructor for testing with arbitrary registers.
  HyperLogLog.testingPrecision({
    required this.precision,
    required List<int> registers,
  }) : _registers = List<int>.unmodifiable(registers);

  const HyperLogLog._({
    required this.precision,
    required List<int> registers,
  }) : _registers = registers;

  /// Return a new sketch that has observed these exact [bytes].
  HyperLogLog addingBytes(List<int> bytes) {
    for (final byte in bytes) {
      if (byte < 0 || byte > 255) {
        throw HyperLogLogException.invalidInput();
      }
    }
    final hash = _fmix64(fnv1a64(bytes));
    final bucket = (hash >> (hashBitWidth - precision)).toInt();
    final remainingBits = hashBitWidth - precision;
    final mask = (BigInt.one << remainingBits) - BigInt.one;
    final remaining = hash & mask;
    final leadingZeros = remaining == BigInt.zero
        ? remainingBits
        : remainingBits - remaining.bitLength;
    final rank = leadingZeros + 1;

    if (rank <= _registers[bucket]) {
      return this;
    }
    final updated = List<int>.from(_registers, growable: false);
    updated[bucket] = rank;
    return HyperLogLog._(
      precision: precision,
      registers: List<int>.unmodifiable(updated),
    );
  }

  /// Return a new sketch that has observed the string's exact UTF-8 bytes.
  HyperLogLog addingString(String text) {
    _validateString(text);
    return addingBytes(utf8.encode(text));
  }

  /// Return a new sketch that has observed [value] (either String or List<int>).
  HyperLogLog adding(Object value) {
    if (value is String) {
      return addingString(value);
    } else if (value is List<int>) {
      return addingBytes(value);
    } else {
      throw HyperLogLogException.invalidInput();
    }
  }

  /// Estimate the number of distinct values observed by this sketch.
  int count() {
    final m = numRegisters.toDouble();
    var harmonicSum = 0.0;
    for (final r in _registers) {
      harmonicSum += math.pow(2.0, -r.toDouble());
    }
    var estimate = _alpha(numRegisters) * m * m / harmonicSum;

    if (estimate <= smallRangeFactor * m) {
      var emptyRegisters = 0;
      for (final r in _registers) {
        if (r == 0) emptyRegisters++;
      }
      if (emptyRegisters > 0) {
        estimate = m * math.log(m / emptyRegisters.toDouble());
      }
    }

    if (estimate > twoTo64 / largeRangeDivisor) {
      final ratio = 1.0 - estimate / twoTo64;
      if (ratio > 0) {
        estimate = -twoTo64 * math.log(ratio);
      }
    }

    if (!estimate.isFinite || estimate <= 0) {
      return 0;
    }
    if (estimate >= twoTo64 || estimate >= maxSafeInt) {
      return maxSafeInt;
    }
    return estimate.round();
  }

  /// Return the register-wise union of two equal-precision sketches.
  HyperLogLog merged(HyperLogLog other) {
    if (precision != other.precision) {
      throw HyperLogLogException.precisionMismatch(precision, other.precision);
    }
    final mergedRegisters = List<int>.generate(
      _registers.length,
      (i) => math.max(_registers[i], other._registers[i]),
      growable: false,
    );
    return HyperLogLog._(
      precision: precision,
      registers: List<int>.unmodifiable(mergedRegisters),
    );
  }

  /// Total number of registers in this sketch (2^precision).
  int get numRegisters => _registers.length;

  /// Whether no elements have been observed.
  bool get isEmpty => _registers.every((r) => r == 0);

  /// Theoretical relative standard error (1.04 / sqrt(m)).
  double get errorRate =>
      errorRateNumerator / math.sqrt(numRegisters.toDouble());

  /// Defensively copied register values.
  List<int> get registers => List<int>.unmodifiable(_registers);

  /// Register values for testing equivalence with reference implementations.
  List<int> get registerValuesForTesting => registers;

  /// Error rate for a given precision.
  static double errorRateFor(int precision) {
    _validatePrecision(precision);
    return errorRateNumerator / math.sqrt((1 << precision).toDouble());
  }

  /// Memory in bytes required to store registers at 6 bits per register.
  static int memoryBytesFor(int precision) {
    _validatePrecision(precision);
    return ((1 << precision) * registerBitWidth) ~/ bitsPerByte;
  }

  /// Optimal precision for a target error rate.
  static int optimalPrecisionFor(double desiredError) {
    if (!desiredError.isFinite || desiredError <= 0) {
      throw HyperLogLogException.invalidDesiredError(desiredError);
    }
    final requiredRegisters = math.pow(errorRateNumerator / desiredError, 2.0);
    if (!requiredRegisters.isFinite) {
      return maximumPrecision;
    }
    final precision = (math.log(requiredRegisters) / math.ln2).ceil();
    return math.min(
      maximumPrecision,
      math.max(minimumPrecision, precision),
    );
  }

  static void _validatePrecision(int precision) {
    if (precision < minimumPrecision || precision > maximumPrecision) {
      throw HyperLogLogException.invalidPrecision(precision);
    }
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! HyperLogLog || precision != other.precision) return false;
    for (var i = 0; i < _registers.length; i++) {
      if (_registers[i] != other._registers[i]) return false;
    }
    return true;
  }

  @override
  int get hashCode {
    var hash = precision.hashCode;
    for (final r in _registers) {
      hash = (hash * 31 + r) & maxSafeInt;
    }
    return hash;
  }
}
