cd ../ct-compare && dart pub get
cd ../md5 && dart pub get
cd ../sha1 && dart pub get
cd ../sha256 && dart pub get
cd ../sha384 && dart pub get
cd ../sha512 && dart pub get
cd ../zeroize && dart pub get
cd ../pbt && dart pub get
cd ../fuzz && dart pub get
cd ../hmac && dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart test
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=95
