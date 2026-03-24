#!/bin/bash
# BUILD -- Build and test Lua cli_builder

set -e

REPO_ROOT=$(cd ../../../.. && pwd -P)
export LUA_PATH="$REPO_ROOT/code/packages/lua/cli_builder/src/?.lua;$REPO_ROOT/code/packages/lua/cli_builder/src/?/init.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?/init.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?/init.lua;;"

cd tests && busted . --verbose --pattern=test_
