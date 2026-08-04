luarocks show coding-adventures-hash-functions >/dev/null 2>&1 || (cd ../hash-functions && luarocks make --local --deps-mode=none coding-adventures-hash-functions-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-bloom-filter-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../hash-functions/src/?.lua;../../hash-functions/src/?/init.lua;;" busted . --verbose --pattern=test_
