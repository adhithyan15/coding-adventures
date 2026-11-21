cd ../grammar-tools && dart pub get
cd ../lexer && dart pub get
cd ../parser && dart pub get
cd ../lattice-lexer && dart pub get
cd ../lattice-parser && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run test/lattice_parser_conformance_test.dart
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
