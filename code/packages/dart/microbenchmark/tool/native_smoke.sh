#!/bin/sh
set -eu
env GIT_REVISION= dart run bin/array_microbenchmark.dart > .dart_tool/native-report.json
dart run tool/replay_report.dart .dart_tool/native-report.json
dd if=/dev/zero of=.dart_tool/oversized-report.bin bs=1048576 count=4 status=none
printf x >> .dart_tool/oversized-report.bin
if dart run tool/replay_report.dart .dart_tool/oversized-report.bin; then
  echo 'replay adapter accepted a report above 4 MiB' >&2
  exit 1
fi
