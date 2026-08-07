luarocks show coding-adventures-avl-tree >/dev/null 2>&1 || (cd ../avl-tree && luarocks make --local --deps-mode=none coding-adventures-avl-tree-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-tree-set-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../avl-tree/src/?.lua;../../avl-tree/src/?/init.lua;;" busted . --verbose --pattern=test_
