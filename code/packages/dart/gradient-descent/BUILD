dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run test/gradient_descent_test.dart
dart run coverage:test_with_coverage --test=test/gradient_descent_test.dart --branch-coverage --function-coverage --fail-under=95
