# needs-toolchain: python
set -e
build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT
javac -Xlint:all -Werror -d "$build_dir" src/codingadventures/unixhistory/UnixHistory.java test/codingadventures/unixhistory/ModelTest.java ../../../specs/fixtures/historical-unix-v1/test-adapters/java/FixtureCli.java
java -cp "$build_dir" codingadventures.unixhistory.ModelTest
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py java -cp "$build_dir" codingadventures.unixhistory.FixtureCli
