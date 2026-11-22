cd ../grammar-tools && dart pub get
cd ../lexer && dart pub get
cd ../parser && dart pub get
cd ../excel-lexer && dart pub get
cd ../excel-parser && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run test/excel_parser_conformance_test.dart
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
