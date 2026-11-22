dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run test/radix_tree_test.dart
dart run coverage:test_with_coverage --test=test/radix_tree_test.dart --branch-coverage --function-coverage --fail-under=95
