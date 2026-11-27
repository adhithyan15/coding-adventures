# needs-toolchain: python
set -euo pipefail
build_dir=$(mktemp -d)
trap 'rm -rf "$build_dir"' EXIT
lua -e 'assert(_VERSION >= "Lua 5.3", "Lua 5.3 or newer required")'
lua test.lua
cp ../../../specs/fixtures/historical-unix-v1/test-adapters/lua/fixture_cli.lua "$build_dir/fixture_cli.lua"
python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py lua "$build_dir/fixture_cli.lua"
