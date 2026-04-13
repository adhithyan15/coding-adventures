luarocks show coding-adventures-huffman-tree 1>/dev/null 2>/dev/null || (cd ../huffman-tree && luarocks make --local --deps-mode=none coding-adventures-huffman-tree-0.1.0-1.rockspec)
luarocks show coding-adventures-lzss 1>/dev/null 2>/dev/null || (cd ../lzss && luarocks make --local --deps-mode=none coding-adventures-lzss-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-deflate-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../huffman-tree/src/?.lua;../../huffman-tree/src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;../../heap/src/?.lua;../../heap/src/?/init.lua;;" busted . --verbose --pattern=test_
