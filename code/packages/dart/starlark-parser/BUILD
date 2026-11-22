cd ../grammar-tools && dart pub get
cd ../lexer && dart pub get
cd ../parser && dart pub get
cd ../starlark-lexer && dart pub get
cd ../starlark-parser && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart test
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
rm -rf coverage
