#!/bin/bash
# BUILD -- Build and test Lua cowsay

REPO_ROOT=$(cd ../../../.. && pwd)
export LUA_PATH="$REPO_ROOT/code/packages/lua/cli_builder/src/?.lua;$REPO_ROOT/code/packages/lua/cli_builder/src/?/init.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?/init.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?/init.lua;./?.lua;./?/init.lua;;"

# Test simple message
lua main.lua "Hello Lua!"

# Test with flag
lua main.lua -b "Resistance is futile"

# Test with cowfile
lua main.lua -f tux "I am a penguin"
