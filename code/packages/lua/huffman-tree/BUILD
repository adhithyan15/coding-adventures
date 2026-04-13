luarocks show coding-adventures-heap 1>/dev/null 2>/dev/null || (cd ../heap && luarocks make --local --deps-mode=none coding-adventures-heap-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-huffman-tree-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../heap/src/?.lua;../../heap/src/?/init.lua;;" busted . --verbose --pattern=test_
