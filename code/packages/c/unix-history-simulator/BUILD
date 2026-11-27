#!/usr/bin/env bash
# needs-toolchain: python
set -euo pipefail
build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT
sources=(src/filesystem.c src/descriptors.c src/kernel.c)
cc -std=c11 -Wall -Wextra -Werror -Iinclude "${sources[@]}" tests/model_test.c -o "$build_dir/model-test"
cc -std=c11 -Wall -Wextra -Werror -Iinclude "${sources[@]}" ../../../specs/fixtures/historical-unix-v1/test-adapters/c/fixture_cli.c -o "$build_dir/fixture-cli"
"$build_dir/model-test"
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py "$build_dir/fixture-cli"
