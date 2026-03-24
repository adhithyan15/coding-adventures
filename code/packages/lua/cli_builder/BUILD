#!/bin/bash
# BUILD -- Build and test Lua cli_builder

set -e

# One-line execution with diagnostic prints and CLAUDE.md marker
REPO_ROOT=$(pwd -P); while [ "$REPO_ROOT" != "/" ] && [ ! -f "$REPO_ROOT/CLAUDE.md" ]; do REPO_ROOT=$(dirname "$REPO_ROOT"); done; LUA_PATHS="$REPO_ROOT/code/packages/lua/cli_builder/src/?.lua;$REPO_ROOT/code/packages/lua/cli_builder/src/?/init.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?.lua;$REPO_ROOT/code/packages/lua/directed_graph/src/?/init.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?.lua;$REPO_ROOT/code/packages/lua/state_machine/src/?/init.lua;./?.lua;./?/init.lua;;"; echo "DEBUG: REPO_ROOT=$REPO_ROOT"; lua -e "print('DEBUG package.path: ' .. package.path); print('DEBUG LUA_PATHS: ' .. [[$LUA_PATHS]])"; busted --lpath "$LUA_PATHS" tests/test_errors.lua
