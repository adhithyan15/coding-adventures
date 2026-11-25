/// A closed order-statistic confidence interval.
class MedianInterval {
  const MedianInterval(this.low, this.high);
  final double low;
  final double high;
}

List<double> _validated(List<double> values) {
  if (values.isEmpty) throw ArgumentError('sample must not be empty');
  final result = List<double>.of(values);
  if (result.any((value) => !value.isFinite)) {
    throw ArgumentError('sample values must be finite');
  }
  result.sort();
  return result;
}

double _midpoint(double low, double high) {
  final sum = low + high;
  return sum.isFinite ? sum / 2 : low / 2 + high / 2;
}

/// Returns the sample median without changing [values].
double median(List<double> values) {
  final sorted = _validated(values);
  final middle = sorted.length ~/ 2;
  if (sorted.length.isOdd) return sorted[middle];
  return _midpoint(sorted[middle - 1], sorted[middle]);
}

int _rank(int n, int confidencePercent) {
  final limit = BigInt.from(100 - confidencePercent) << n;
  var coefficient = BigInt.one;
  var tail = BigInt.zero;
  var rank = 0;
  for (var j = 0; j <= n ~/ 2; j++) {
    tail += coefficient;
    if (tail * BigInt.from(200) > limit) break;
    rank = j + 1;
    coefficient = coefficient * BigInt.from(n - j) ~/ BigInt.from(j + 1);
  }
  return rank;
}

/// Returns an exact-cutoff interval, or null when no rank qualifies.
MedianInterval? medianInterval(List<double> values, int confidencePercent) {
  if (confidencePercent < 1 || confidencePercent > 99) {
    throw ArgumentError.value(confidencePercent, 'confidencePercent');
  }
  final sorted = _validated(values);
  final rank = _rank(sorted.length, confidencePercent);
  if (rank == 0) return null;
  return MedianInterval(sorted[rank - 1], sorted[sorted.length - rank]);
}

/// Independent bounded sign enumeration used by consumers' tests.
bool oracleRankQualifies(int n, int rank, int confidencePercent) {
  if (n < 1 ||
      n > 20 ||
      rank < 1 ||
      confidencePercent < 1 ||
      confidencePercent > 99) {
    throw ArgumentError('oracle bounds exceeded');
  }
  var tail = BigInt.zero;
  for (var mask = 0; mask < (1 << n); mask++) {
    if (mask.bitCount <= rank - 1) tail += BigInt.one;
  }
  return tail * BigInt.from(200) <= (BigInt.from(100 - confidencePercent) << n);
}

extension on int {
  int get bitCount {
    var bits = this;
    var count = 0;
    while (bits != 0) {
      count += bits & 1;
      bits >>= 1;
    }
    return count;
  }
}
