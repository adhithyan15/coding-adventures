#!/bin/sh
set -eu
dart run bin/array_microbenchmark.dart > .dart_tool/native-report.json
dart run tool/replay_report.dart .dart_tool/native-report.json
