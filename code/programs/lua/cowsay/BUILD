#!/bin/bash
# BUILD -- Build and test Lua cowsay

set -e

# Very robust root detection: git > GITHUB_WORKSPACE > dirname walk
if git rev-parse --show-toplevel >/dev/null 2>&1; then
    REPO_ROOT=$(git rev-parse --show-toplevel)
elif [ -n "$GITHUB_WORKSPACE" ]; then
    REPO_ROOT="$GITHUB_WORKSPACE"
else
    REPO_ROOT=$(dirname $(dirname $(dirname $(dirname $(pwd -P)))))
fi

echo "DEBUG: REPO_ROOT=$REPO_ROOT"
echo "DEBUG: PWD=$(pwd -P)"

# Ensure LUA_PATH is correctly set with absolute paths
export LUA_PATH="$REPO_ROOT/code/packages/lua/cli_builder/src/?.lua;$REPO_ROOT/code/packages/lua/cli_builder/src/?/init.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?/init.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?/init.lua;./?.lua;./?/init.lua;;"
echo "DEBUG: LUA_PATH=$LUA_PATH"

# Test simple message
lua main.lua "Hello Lua!"

# Test with flag
lua main.lua -b "Resistance is futile"

# Test with cowfile
lua main.lua -f tux "I am a penguin"
