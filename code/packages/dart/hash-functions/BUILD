dart pub get
dart run tool/generate_fixture_snapshot.dart
dart format --output=none --set-exit-if-changed tool test
dart analyze --fatal-infos
dart test -p vm
dart test -p node
