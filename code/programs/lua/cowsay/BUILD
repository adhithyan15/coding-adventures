#!/bin/bash
# BUILD -- Build and test Lua cowsay

set -e

# One-line execution to ensure REPO_ROOT and LUA_PATH stay in the same shell
REPO_ROOT=$(pwd -P); while [ "$REPO_ROOT" != "/" ] && [ ! -f "$REPO_ROOT/go.mod" ]; do REPO_ROOT=$(dirname "$REPO_ROOT"); done; export LUA_PATH="$REPO_ROOT/code/packages/lua/cli_builder/src/?.lua;$REPO_ROOT/code/packages/lua/cli_builder/src/?/init.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?/init.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?/init.lua;./?.lua;./?/init.lua;;"; lua main.lua "Hello Lua!" && lua main.lua -b "Resistance is futile" && lua main.lua -f tux "I am a penguin"
