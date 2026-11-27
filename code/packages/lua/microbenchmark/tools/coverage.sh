#!/bin/sh
set -eu

package_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$package_root"

if [ -n "${CA_LUAROCKS_COVERAGE_TREE:-}" ]; then
  coverage_tree=$CA_LUAROCKS_COVERAGE_TREE
elif [ -d /workspace/.coding-adventures-tools/luarocks-coverage-pr20100 ]; then
  coverage_tree=/workspace/.coding-adventures-tools/luarocks-coverage-pr20100
else
  coverage_tree=$HOME/.luarocks
fi
luarocks --tree="$coverage_tree" install luacov 0.17.0-1
luarocks --tree="$coverage_tree" install luafilesystem 1.9.0-1
old_path=$PATH
old_lua_path=${LUA_PATH:-}
old_lua_cpath=${LUA_CPATH:-}
rock_paths=$(luarocks --tree="$coverage_tree" path)
eval "$rock_paths"
PATH="$PATH:$old_path"
LUA_PATH="$LUA_PATH;$old_lua_path"
LUA_CPATH="$LUA_CPATH;$old_lua_cpath"
export PATH LUA_PATH LUA_CPATH

coverage_dir=$(mktemp -d "${TMPDIR:-/tmp}/ca-microbenchmark-coverage.XXXXXX")
trap 'rm -rf "$coverage_dir"' EXIT HUP INT TERM
config="$coverage_dir/luacov.lua"
report="$coverage_dir/report.txt"
cat > "$config" <<EOF
return {
  statsfile = "$coverage_dir/stats.out",
  reportfile = "$report",
  include = { "src/coding_adventures/microbenchmark/init\$" },
  includeuntestedfiles = { "src/coding_adventures/microbenchmark/init.lua" },
  modules = { ["coding_adventures.microbenchmark"] = "src/coding_adventures/microbenchmark/init.lua" },
}
EOF

LUA_INIT='require("luacov")' LUACOV_CONFIG="$config" CA_COVERAGE_MODE=1 lua tests/run.lua
luacov -c "$config"
lua tests/check_coverage.lua "$report" src/coding_adventures/microbenchmark/init.lua
