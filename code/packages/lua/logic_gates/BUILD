(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local --deps-mode=none coding-adventures-logic-gates-0.6.0-1.rockspec)
luarocks install --local luacov 0.17.0
cd tests && rm -f luacov.stats.out luacov.report.out && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../transistors/src/?.lua;../../transistors/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted . --verbose --coverage --coverage-config-file=../.luacov --pattern=test_ --exclude-pattern=test_global_capture && luacov -c ../.luacov
cd tests && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../transistors/src/?.lua;../../transistors/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted test_global_capture.lua --verbose --pattern=test_
cd tests && lua check_coverage.lua
