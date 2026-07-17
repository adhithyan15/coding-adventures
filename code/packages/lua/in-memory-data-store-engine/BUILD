luarocks show coding-adventures-hyperloglog >/dev/null 2>&1 || (cd ../hyperloglog && luarocks make --local --deps-mode=none coding-adventures-hyperloglog-0.1.0-1.rockspec)
luarocks show coding-adventures-in-memory-data-store-protocol >/dev/null 2>&1 || (cd ../in-memory-data-store-protocol && luarocks make --local --deps-mode=none coding-adventures-in-memory-data-store-protocol-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-in-memory-data-store-engine-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../hyperloglog/src/?.lua;../../hyperloglog/src/?/init.lua;../../in-memory-data-store-protocol/src/?.lua;../../in-memory-data-store-protocol/src/?/init.lua;;" busted . --verbose --pattern=test_
