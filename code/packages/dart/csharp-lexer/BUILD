cd ../grammar-tools && dart pub get
cd ../lexer && dart pub get
cd ../csharp-lexer && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run test/csharp_lexer_conformance_test.dart
dart run coverage:test_with_coverage --test=test/csharp_lexer_test.dart --branch-coverage --function-coverage --fail-under=95
