luarocks show coding-adventures-http-core >/dev/null 2>&1 || (cd ../http_core && luarocks make --local --deps-mode=none coding-adventures-http-core-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-http1-0.1.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../http_core/src/?.lua;../../http_core/src/?/init.lua;;" busted . --verbose --pattern=test_
