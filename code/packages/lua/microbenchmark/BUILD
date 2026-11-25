#!/bin/sh
set -eu
for spec in \
  ../directed_graph/coding-adventures-directed-graph-0.1.0-1.rockspec \
  ../state_machine/coding-adventures-state-machine-0.1.0-1.rockspec \
  ../grammar_tools/coding-adventures-grammar-tools-0.1.0-1.rockspec \
  ../lexer/coding-adventures-lexer-0.1.0-1.rockspec \
  ../json_lexer/coding-adventures-json-lexer-0.1.0-1.rockspec \
  ../parser/coding-adventures-parser-0.1.0-1.rockspec \
  ../json_parser/coding-adventures-json-parser-0.1.0-1.rockspec \
  ../rank_statistics/coding-adventures-rank-statistics-0.1.0-1.rockspec; do
  package_dir=$(dirname "$spec")
  rockspec=$(basename "$spec")
  (cd "$package_dir" && luarocks make --local --deps-mode=none "$rockspec")
done
luarocks make --local --deps-mode=none coding-adventures-microbenchmark-0.1.0-1.rockspec
eval "$(luarocks path)"
lua tests/run.lua
