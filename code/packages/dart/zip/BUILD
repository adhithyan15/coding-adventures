cd ../lzss
dart pub get
cd ../zip
dart pub get
dart format --output=none --set-exit-if-changed lib/coding_adventures_zip.dart test/portable_conformance_test.dart
dart analyze --fatal-infos
dart test
