# needs-toolchain: python
set -e
dart run test/model_test.dart
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py dart run ../../../specs/fixtures/historical-unix-v1/test-adapters/dart/fixture_cli.dart
