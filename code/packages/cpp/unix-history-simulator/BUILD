#!/usr/bin/env bash
# needs-toolchain: python
set -euo pipefail
build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT
c++ -std=c++17 -Wall -Wextra -Werror -Iinclude src/unix_history.cpp tests/model_test.cpp -o "$build_dir/model-test"
c++ -std=c++17 -Wall -Wextra -Werror -Iinclude src/unix_history.cpp ../../../specs/fixtures/historical-unix-v1/test-adapters/cpp/fixture_cli.cpp -o "$build_dir/fixture-cli"
"$build_dir/model-test"
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py "$build_dir/fixture-cli"
