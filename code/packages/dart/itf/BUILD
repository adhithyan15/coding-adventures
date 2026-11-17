cd ../paint-instructions && dart pub get
cd ../barcode-layout-1d && dart pub get
cd ../itf && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run test/itf_conformance_test.dart
dart run coverage:test_with_coverage --test=test/itf_test.dart --branch-coverage --function-coverage --fail-under=95
