#!/bin/bash
# BUILD -- Build and test Lua cowsay

set -e

# Use Physical path to avoid symlink issues on macOS
REPO_ROOT=$(cd ../../../.. && pwd -P)
echo "REPO_ROOT: $REPO_ROOT"

# Ensure LUA_PATH is correctly set with both the package source and the init.lua pattern
export LUA_PATH="$REPO_ROOT/code/packages/lua/cli_builder/src/?.lua;$REPO_ROOT/code/packages/lua/cli_builder/src/?/init.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?/init.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?/init.lua;./?.lua;./?/init.lua;;"
echo "LUA_PATH: $LUA_PATH"

# Test simple message
lua main.lua "Hello Lua!"

# Test with flag
lua main.lua -b "Resistance is futile"

# Test with cowfile
lua main.lua -f tux "I am a penguin"
