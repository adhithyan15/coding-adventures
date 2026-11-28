#!/bin/sh
set -eu
cd "$(dirname "$0")/../example/consumer"
dart pub get
dart analyze --fatal-infos
dart run bin/consumer.dart
