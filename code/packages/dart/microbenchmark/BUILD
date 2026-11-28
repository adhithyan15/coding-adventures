set -eu
dart pub get
dart run tool/generate_fixture_snapshot.dart
dart format --output=none --set-exit-if-changed lib test bin tool
dart analyze --fatal-infos
dart test -p vm --coverage=.dart_tool/coverage --branch-coverage
dart run coverage:format_coverage --lcov --in=.dart_tool/coverage --out=.dart_tool/coverage/lcov.info --report-on=lib
dart run tool/check_coverage.dart --lcov=.dart_tool/coverage/lcov.info
dart test -p node
sh tool/native_smoke.sh
sh tool/consumer_smoke.sh
