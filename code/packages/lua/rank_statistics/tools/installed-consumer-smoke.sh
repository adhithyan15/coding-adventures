#!/bin/sh
set -eu

package_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$package_root"
tree=$(mktemp -d "${TMPDIR:-/tmp}/ca-lua-rank-consumer.XXXXXX")
trap 'rm -rf "$tree"' EXIT HUP INT TERM
luarocks --tree="$tree" make --deps-mode=none coding-adventures-rank-statistics-0.1.0-1.rockspec
LUA_PATH="$(luarocks --tree="$tree" path --lr-path)${LUA_PATH:+;$LUA_PATH}"
LUA_CPATH="$(luarocks --tree="$tree" path --lr-cpath)${LUA_CPATH:+;$LUA_CPATH}"
export LUA_PATH LUA_CPATH
lua tests/installed-consumer/Consumer.lua
