cd ../paint-instructions && dart pub get
cd ../barcode-layout-1d && dart pub get
cd ../pbt && dart pub get
cd ../fuzz && dart pub get
cd ../code128 && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart test
dart run test/code_128_conformance_test.dart
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
