cd ../hash-functions && dart pub get
cd ../hash-map && dart pub get
cd ../hash-set && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart test
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
rm -rf coverage
