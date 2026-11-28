import 'dart:io';

void main() {
  final source =
      File('testdata/microbenchmark-pairs-v1.json').readAsStringSync();
  final destination = File('.dart_tool/microbenchmark_pairs_snapshot.dart')
    ..parent.createSync(recursive: true);
  destination.writeAsStringSync(
      "const microbenchmarkPairsFixture = r'''$source''';\n");
}
