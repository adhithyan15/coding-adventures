#!/bin/bash
# BUILD -- Build and test Lua cowsay

set -e

# Use GITHUB_WORKSPACE if available (GitHub Actions), otherwise calculate relative to this script
REPO_ROOT=${GITHUB_WORKSPACE:-$(cd ../../../.. && pwd -P)}
echo "DEBUG: GITHUB_WORKSPACE=$GITHUB_WORKSPACE"
echo "DEBUG: Calculated REPO_ROOT=$REPO_ROOT"
echo "DEBUG: Current Directory=$(pwd)"

# Ensure LUA_PATH is correctly set with absolute paths
export LUA_PATH="$REPO_ROOT/code/packages/lua/cli_builder/src/?.lua;$REPO_ROOT/code/packages/lua/cli_builder/src/?/init.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?/init.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?/init.lua;./?.lua;./?/init.lua;;"
echo "DEBUG: LUA_PATH=$LUA_PATH"

# Test simple message
lua main.lua "Hello Lua!"

# Test with flag
lua main.lua -b "Resistance is futile"

# Test with cowfile
lua main.lua -f tux "I am a penguin"
