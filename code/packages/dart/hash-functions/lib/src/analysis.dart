typedef HashCallback = BigInt Function(List<int> data);

/// Computes the deterministic bit-flip avalanche score for explicit samples.
double avalancheScore({
  required HashCallback hashFn,
  required int outputBits,
  required List<List<int>> samples,
}) {
  if (outputBits < 1 || outputBits > 64) {
    throw ArgumentError('outputBits must be in 1..64');
  }
  if (samples.isEmpty) {
    throw ArgumentError('samples must not be empty');
  }
  for (final sample in samples) {
    if (sample.isEmpty) {
      throw ArgumentError('avalanche samples must not be empty');
    }
    _validateBytes(sample);
  }

  final upperBound = BigInt.one << outputBits;
  var flippedOutputBits = BigInt.zero;
  var comparedOutputBits = BigInt.zero;
  for (final sample in samples) {
    final originalHash = _validateCallbackResult(hashFn(sample), upperBound);
    for (var bit = 0; bit < sample.length * 8; bit++) {
      final changed = List<int>.of(sample);
      changed[bit ~/ 8] ^= 1 << (bit % 8);
      final changedHash = _validateCallbackResult(hashFn(changed), upperBound);
      flippedOutputBits += _popCount(originalHash ^ changedHash);
      comparedOutputBits += BigInt.from(outputBits);
    }
  }
  return flippedOutputBits.toDouble() / comparedOutputBits.toDouble();
}

/// Returns the chi-squared statistic for caller-supplied inputs and buckets.
double distributionTest({
  required HashCallback hashFn,
  required List<List<int>> inputs,
  required int numBuckets,
}) {
  if (inputs.isEmpty) {
    throw ArgumentError('inputs must not be empty');
  }
  if (numBuckets < 1 || numBuckets > 9007199254740991) {
    throw ArgumentError('numBuckets must be in 1..2^53-1');
  }
  for (final input in inputs) {
    _validateBytes(input);
  }

  final bucketCount = BigInt.from(numBuckets);
  final counts = <BigInt, int>{};
  for (final input in inputs) {
    final value = hashFn(input);
    if (value < BigInt.zero || value > _mask64) {
      throw ArgumentError('callback result must be in 0..2^64-1');
    }
    final bucket = value % bucketCount;
    counts.update(bucket, (count) => count + 1, ifAbsent: () => 1);
  }

  final total = BigInt.from(inputs.length);
  var sumSquares = BigInt.zero;
  for (final count in counts.values) {
    final exactCount = BigInt.from(count);
    sumSquares += exactCount * exactCount;
  }
  final numerator = bucketCount * sumSquares - total * total;
  final denominator = total;
  final divisor = _greatestCommonDivisor(numerator, denominator);
  return (numerator ~/ divisor).toDouble() /
      (denominator ~/ divisor).toDouble();
}

void _validateBytes(List<int> data) {
  for (final byte in data) {
    if (byte < 0 || byte > 255) {
      throw ArgumentError('Byte input values must be in 0..255');
    }
  }
}

BigInt _validateCallbackResult(BigInt value, BigInt upperBound) {
  if (value < BigInt.zero || value >= upperBound) {
    throw ArgumentError('callback result does not fit outputBits');
  }
  return value;
}

BigInt _popCount(BigInt value) {
  var remaining = value;
  var count = BigInt.zero;
  while (remaining != BigInt.zero) {
    remaining &= remaining - BigInt.one;
    count += BigInt.one;
  }
  return count;
}

BigInt _greatestCommonDivisor(BigInt left, BigInt right) {
  var a = left;
  var b = right;
  while (b != BigInt.zero) {
    final remainder = a % b;
    a = b;
    b = remainder;
  }
  return a;
}

final BigInt _mask64 = BigInt.parse('ffffffffffffffff', radix: 16);
