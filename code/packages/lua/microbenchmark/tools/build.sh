#!/bin/sh
set -eu

package_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$package_root"

make_rock() {
  package_dir=$1
  rockspec=$2
  if [ -n "${CA_LUAROCKS_TREE:-}" ]; then
    (cd "$package_dir" && luarocks --tree="$CA_LUAROCKS_TREE" make --deps-mode=none "$rockspec")
  else
    (cd "$package_dir" && luarocks make --local --deps-mode=none "$rockspec")
  fi
}

make_rock ../directed_graph coding-adventures-directed-graph-0.1.0-1.rockspec
make_rock ../state_machine coding-adventures-state-machine-0.1.0-1.rockspec
make_rock ../grammar_tools coding-adventures-grammar-tools-0.1.0-1.rockspec
make_rock ../lexer coding-adventures-lexer-0.1.0-1.rockspec
make_rock ../json_lexer coding-adventures-json-lexer-0.1.0-1.rockspec
make_rock ../parser coding-adventures-parser-0.1.0-1.rockspec
make_rock ../json_parser coding-adventures-json-parser-0.1.0-1.rockspec
make_rock ../rank_statistics coding-adventures-rank-statistics-0.1.0-1.rockspec

if [ -n "${CA_LUAROCKS_TREE:-}" ]; then
  luarocks --tree="$CA_LUAROCKS_TREE" make --deps-mode=none coding-adventures-microbenchmark-0.1.0-1.rockspec
  export LUA_PATH="$(luarocks --tree="$CA_LUAROCKS_TREE" path --lr-path)"
  export LUA_CPATH="$(luarocks --tree="$CA_LUAROCKS_TREE" path --lr-cpath)"
else
  luarocks make --local --deps-mode=none coding-adventures-microbenchmark-0.1.0-1.rockspec
  export LUA_PATH="$(luarocks path --lr-path)"
  export LUA_CPATH="$(luarocks path --lr-cpath)"
fi
lua tests/run.lua
report=$(mktemp)
trap 'rm -f "$report"' EXIT HUP INT TERM
env GIT_REVISION= GITHUB_SHA= lua script/array-microbenchmark --pairs 40 > "$report"
lua tests/installed-consumer/Consumer.lua "$report"
