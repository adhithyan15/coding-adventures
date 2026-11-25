cd ../sha256 && dart pub get
cd ../pbt && dart pub get
cd ../fuzz && dart pub get
cd ../correlation-vector && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart test
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
