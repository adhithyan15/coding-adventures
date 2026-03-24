#!/bin/bash
# BUILD -- Build and test Lua cowsay

set -e

# Definitive root detection: walk up until go.mod and code/ are found
REPO_ROOT=$(pwd -P)
while [ "$REPO_ROOT" != "/" ]; do
    if [ -f "$REPO_ROOT/go.mod" ] && [ -d "$REPO_ROOT/code" ]; then
        break
    fi
    REPO_ROOT=$(dirname "$REPO_ROOT")
done

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
