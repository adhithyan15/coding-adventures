import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_fenwick_tree/coding_adventures_fenwick_tree.dart';
import 'package:test/test.dart';

// The reference uses only direct indexing and flat loops, with no Fenwick cells,
// bit operations, cached prefixes or calls back into the implementation.
int flatSum(List<int> values, int start, int end) {
  var result = 0;
  for (var index = start; index < end; index++) {
    result += values[index];
  }
  return result;
}

int? flatKth(List<int> values, int target) {
  var sum = 0;
  for (var index = 0; index < values.length; index++) {
    sum += values[index];
    if (sum >= target) return index + 1;
  }
  return null;
}

Iterable<List<int>> arrays(int length, List<int> alphabet) sync* {
  if (length == 0) {
    yield <int>[];
    return;
  }
  for (final prefix in arrays(length - 1, alphabet)) {
    for (final value in alphabet) {
      yield [...prefix, value];
    }
  }
}

int audit(FenwickTree tree, List<int> values) {
  expect(tree.size, values.length);
  var queries = 0;
  for (var index = 0; index <= values.length; index++) {
    expect(tree.prefixSum(index), flatSum(values, 0, index));
    queries++;
  }
  for (var left = 1; left <= values.length; left++) {
    expect(tree.pointQuery(left), values[left - 1]);
    queries++;
    for (var right = left; right <= values.length; right++) {
      expect(tree.rangeSum(left, right), flatSum(values, left - 1, right));
      queries++;
    }
  }
  return queries;
}

Matcher errorCode(String code) => throwsA(
    isA<FenwickException>().having((error) => error.code, 'code', code));

Object? invoke(FenwickTree tree, Map<String, dynamic> op) {
  switch (op['op']) {
    case 'prefix_sum':
      return tree.prefixSum(op['i'] as int);
    case 'range_sum':
      return tree.rangeSum(op['l'] as int, op['r'] as int);
    case 'point_query':
      return tree.pointQuery(op['i'] as int);
    case 'update':
      tree.update(op['i'] as int, op['delta'] as int);
      return List.generate(tree.size, (index) => tree.pointQuery(index + 1));
    case 'find_kth':
      return tree.findKth(op['k'] as int);
    default:
      throw StateError('Unrecognized fixture operation');
  }
}

void main() {
  test(
      'shared DT06 fixtures compare implementation results and unchanged state',
      () {
    final file = File(Platform.environment['FENWICK_FIXTURE'] ??
        '../../../specs/fixtures/fenwick-tree-v1/cases.json');
    final document =
        jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
    expect(document['version'], 1);
    final cases = document['cases'] as List<dynamic>;
    expect(cases.length, 19);
    for (final raw in cases) {
      final fixture = raw as Map<String, dynamic>;
      final values = (fixture['values'] as List<dynamic>).cast<int>();
      FenwickTree construct() => fixture.containsKey('size')
          ? FenwickTree(fixture['size'] as int)
          : FenwickTree.fromList(values);
      if (fixture.containsKey('constructor_error')) {
        expect(construct, errorCode(fixture['constructor_error'] as String),
            reason: fixture['name'] as String);
        continue;
      }
      final tree = construct();
      var current = List<int>.of(values);
      audit(tree, current);
      for (final rawOp in fixture['ops'] as List<dynamic>) {
        final op = rawOp as Map<String, dynamic>;
        if (op.containsKey('error')) {
          expect(() => invoke(tree, op), errorCode(op['error'] as String),
              reason: fixture['name'] as String);
          expect(op['after'] ?? current, current);
        } else {
          expect(invoke(tree, op), op['result'],
              reason: fixture['name'] as String);
          if (op['op'] == 'update') {
            current = (op['result'] as List<dynamic>).cast<int>();
          }
        }
        audit(tree, current);
      }
    }
  });

  test('bounded signed arrays and every one-step update use flat oracle', () {
    var sourceArrays = 0;
    var updates = 0;
    var states = 0;
    var queries = 0;
    for (var length = 0; length <= 6; length++) {
      for (final values in arrays(length, [-1, 0, 1])) {
        sourceArrays++;
        queries += audit(FenwickTree.fromList(values), values);
        states++;
        for (var index = 1; index <= length; index++) {
          for (final delta in [-1, 0, 1]) {
            final tree = FenwickTree.fromList(values);
            final expected = List<int>.of(values);
            expected[index - 1] += delta;
            tree.update(index, delta);
            queries += audit(tree, expected);
            states++;
            updates++;
          }
        }
      }
    }
    expect(
        [sourceArrays, updates, states, queries], [1093, 18045, 19138, 596092]);
  });

  test('bounded nonnegative arrays use independent linear kth oracle', () {
    var sourceArrays = 0;
    var targets = 0;
    for (var length = 0; length <= 6; length++) {
      for (final values in arrays(length, [0, 1, 2])) {
        sourceArrays++;
        final tree = FenwickTree.fromList(values);
        final total = flatSum(values, 0, values.length);
        for (var target = 1; target <= total + 1; target++) {
          final expected = flatKth(values, target);
          if (expected == null) {
            expect(() => tree.findKth(target),
                errorCode(length == 0 ? 'empty' : 'exceeds_total'));
          } else {
            expect(tree.findKth(target), expected);
          }
          targets++;
        }
      }
    }
    expect([sourceArrays, targets], [1093, 7108]);
  });

  test('focused lengths 7 8 9 and cumulative updates cross powers of two', () {
    for (final length in [7, 8, 9]) {
      final values = List<int>.filled(length, 0);
      final tree = FenwickTree(length);
      audit(tree, values);
      for (var index = 1; index <= length; index++) {
        for (final delta in [index, -index - 1, 2]) {
          tree.update(index, delta);
          values[index - 1] += delta;
          audit(tree, values);
        }
      }
      final total = flatSum(values, 0, length);
      for (var target = 1; target <= total; target++) {
        expect(tree.findKth(target), flatKth(values, target));
      }
    }
  });

  test('bounds precedence portable endpoints and rejection atomicity', () {
    const limit = FenwickTree.maxMagnitude;
    expect(const FenwickException('x').toString(), 'FenwickException(x)');
    for (final value in [limit + 1, -limit - 1, -9223372036854775808]) {
      expect(() => FenwickTree.fromList([value]), errorCode('out_of_domain'));
    }
    expect(() => FenwickTree.fromList([limit, -limit]),
        errorCode('out_of_domain'));
    expect(() => FenwickTree(-1), errorCode('negative_size'));
    final tree = FenwickTree.fromList([limit - 1, 0, -1]);
    final original = [limit - 1, 0, -1];
    for (final index in [-1, 0, 4]) {
      expect(
          () => tree.update(index, limit + 1), errorCode('index_out_of_range'));
      expect(() => tree.pointQuery(index), errorCode('index_out_of_range'));
      audit(tree, original);
    }
    expect(() => tree.prefixSum(-1), errorCode('index_out_of_range'));
    expect(() => tree.prefixSum(4), errorCode('index_out_of_range'));
    expect(() => tree.rangeSum(0, -1), errorCode('invalid_range'));
    expect(() => tree.rangeSum(0, 1), errorCode('index_out_of_range'));
    expect(() => tree.rangeSum(1, 4), errorCode('index_out_of_range'));
    for (final delta in [1, -1, limit + 1, -limit - 1, -9223372036854775808]) {
      expect(() => tree.update(2, delta), errorCode('out_of_domain'));
      audit(tree, original);
    }
    tree.update(1, -limit);
    audit(tree, [-1, 0, -1]);
    tree.update(1, 1);
    tree.update(3, limit);
    audit(tree, [0, 0, limit - 1]);
    expect(tree.findKth(limit - 1), 3);
    final negative = FenwickTree.fromList([-limit]);
    negative.update(1, limit);
    audit(negative, [0]);
    final positive = FenwickTree.fromList([limit]);
    expect(() => positive.update(1, 1), errorCode('out_of_domain'));
    audit(positive, [limit]);
    expect(positive.findKth(limit), 1);
  });

  test('negative frequency tracking restores kth and owns caller input', () {
    final values = [2, 0, 1];
    final tree = FenwickTree.fromList(values);
    values[0] = 999;
    audit(tree, [2, 0, 1]);
    tree.update(2, -1);
    expect(() => tree.findKth(1), errorCode('negative_frequency'));
    expect(() => tree.findKth(0), errorCode('nonpositive_target'));
    expect(() => tree.findKth(3), errorCode('exceeds_total'));
    tree.update(2, 1);
    expect(tree.findKth(3), 3);
    expect(() => tree.update(0, -1), errorCode('index_out_of_range'));
    expect(tree.findKth(3), 3);
    final empty = FenwickTree(0);
    expect(() => empty.findKth(0), errorCode('empty'));
    expect(() => empty.update(1, 0), errorCode('index_out_of_range'));
    expect(() => empty.rangeSum(1, 1), errorCode('index_out_of_range'));
  });
}
