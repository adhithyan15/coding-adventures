(cd ../sha1 && luarocks make --local coding-adventures-sha1-0.1.0-1.rockspec)
cd tests && LUA_PATH="../sha1/src/?.lua;../sha1/src/?/init.lua;../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
