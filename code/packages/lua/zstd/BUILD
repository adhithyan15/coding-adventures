luarocks show coding-adventures-lzss 1>/dev/null 2>/dev/null || (cd ../lzss && luarocks make --local --deps-mode=none coding-adventures-lzss-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-zstd-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;;" busted . --verbose --pattern=test_
