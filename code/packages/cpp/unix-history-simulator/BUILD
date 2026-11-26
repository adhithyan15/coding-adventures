#!/usr/bin/env bash
set -euo pipefail
build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT
c++ -std=c++17 -Wall -Wextra -Werror -Iinclude src/unix_history.cpp tests/model_test.cpp -o "$build_dir/model-test"
c++ -std=c++17 -Wall -Wextra -Werror -Iinclude src/unix_history.cpp src/cli.cpp -o "$build_dir/unix-history"
"$build_dir/model-test"
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py "$build_dir/unix-history"
