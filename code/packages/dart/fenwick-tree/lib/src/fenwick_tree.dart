/// Stable DT06 error identifiers shared with the language-neutral fixtures.
class FenwickException implements Exception {
  final String code;
  const FenwickException(this.code);

  @override
  String toString() => 'FenwickException($code)';
}

/// A one-based Fenwick tree with signed values and atomic checked updates.
///
/// Cell i covers the interval ending at i whose length is its lowest set bit.
/// Querying removes that bit to visit disjoint intervals; updating adds it to
/// visit every containing interval. The separate point array lets validation
/// finish before any tree cell changes.
class FenwickTree {
  static const int maxMagnitude = 9007199254740991;

  final List<int> _points;
  final List<int> _tree;
  int _absoluteTotal = 0;
  int _negativeCount = 0;

  factory FenwickTree(int size) {
    if (size < 0) throw const FenwickException('negative_size');
    return FenwickTree.fromList(List<int>.filled(size, 0));
  }

  FenwickTree.fromList(List<int> values)
      : _points = List<int>.of(values),
        _tree = List<int>.filled(values.length + 1, 0) {
    // Spend a remaining budget so cancellation in the signed total can never
    // conceal a violation of the sum-of-absolute-values contract.
    for (final value in _points) {
      if (value < -maxMagnitude || value > maxMagnitude) {
        throw const FenwickException('out_of_domain');
      }
      final magnitude = value.abs();
      if (magnitude > maxMagnitude - _absoluteTotal) {
        throw const FenwickException('out_of_domain');
      }
      _absoluteTotal += magnitude;
      if (value < 0) _negativeCount++;
    }
    // Linear construction sends each completed interval to its parent once.
    for (var index = 1; index <= size; index++) {
      _tree[index] += _points[index - 1];
      final parent = index + (index & -index);
      if (parent <= size) _tree[parent] += _tree[index];
    }
  }

  int get size => _points.length;

  void _checkPoint(int index) {
    if (index < 1 || index > size) {
      throw const FenwickException('index_out_of_range');
    }
  }

  /// Sum of the first [index] values; zero is the empty prefix.
  int prefixSum(int index) {
    if (index < 0 || index > size) {
      throw const FenwickException('index_out_of_range');
    }
    var sum = 0;
    for (var cursor = index; cursor > 0; cursor -= cursor & -cursor) {
      sum += _tree[cursor];
    }
    return sum;
  }

  /// Inclusive sum, with reversed-range validation before endpoint validation.
  int rangeSum(int left, int right) {
    if (left > right) throw const FenwickException('invalid_range');
    _checkPoint(left);
    _checkPoint(right);
    return prefixSum(right) - prefixSum(left - 1);
  }

  int pointQuery(int index) {
    _checkPoint(index);
    return _points[index - 1];
  }

  /// Add [delta] to one point, checking index, delta, and L1 budget in order.
  void update(int index, int delta) {
    _checkPoint(index);
    if (delta < -maxMagnitude || delta > maxMagnitude) {
      throw const FenwickException('out_of_domain');
    }
    final oldValue = _points[index - 1];
    final newValue = oldValue + delta;
    final otherMagnitude = _absoluteTotal - oldValue.abs();
    if (newValue.abs() > maxMagnitude - otherMagnitude) {
      throw const FenwickException('out_of_domain');
    }

    // Every potentially rejecting check is above this point.
    _absoluteTotal = otherMagnitude + newValue.abs();
    if (oldValue < 0) _negativeCount--;
    if (newValue < 0) _negativeCount++;
    _points[index - 1] = newValue;
    for (var cursor = index; cursor <= size; cursor += cursor & -cursor) {
      _tree[cursor] += delta;
    }
  }

  /// Least index whose prefix reaches [target], for nonnegative frequencies.
  ///
  /// Validate empty, nonpositive, and above-total targets first. This package
  /// also rejects signed-frequency trees with the code negative_frequency.
  int findKth(int target) {
    if (size == 0) throw const FenwickException('empty');
    if (target <= 0) throw const FenwickException('nonpositive_target');
    if (target > prefixSum(size)) {
      throw const FenwickException('exceeds_total');
    }
    if (_negativeCount != 0) {
      throw const FenwickException('negative_frequency');
    }
    // Binary lifting finds the longest prefix whose sum is still below target.
    var step = 1;
    while (step <= size ~/ 2) {
      step *= 2;
    }
    var index = 0;
    var remaining = target;
    for (; step > 0; step ~/= 2) {
      final next = index + step;
      if (next <= size && _tree[next] < remaining) {
        remaining -= _tree[next];
        index = next;
      }
    }
    return index + 1;
  }
}
