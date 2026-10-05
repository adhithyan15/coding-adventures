cd ../paint-instructions && dart pub get
cd ../barcode-layout-1d && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run test/barcode_layout_1d_test.dart
dart run coverage:test_with_coverage --test=test/barcode_layout_1d_conformance_test.dart --branch-coverage --function-coverage --fail-under=90
