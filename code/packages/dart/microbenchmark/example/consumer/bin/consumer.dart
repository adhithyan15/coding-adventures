import 'package:coding_adventures_microbenchmark/coding_adventures_microbenchmark.dart';
import 'package:coding_adventures_rank_statistics/coding_adventures_rank_statistics.dart';

void main() {
  if (median([1.0, 3.0]) != 2.0 || makeValues().length != 4096) {
    throw StateError('package consumer API mismatch');
  }
}
