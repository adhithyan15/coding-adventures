luarocks show coding-adventures-lzss 1>/dev/null 2>/dev/null || (cd ../lzss && luarocks make --local --deps-mode=none coding-adventures-lzss-0.1.0-1.rockspec)
luarocks install --local busted
luarocks install --local dkjson
luarocks install --local libdeflate 1.0.2-1
luarocks install --local luacheck
luarocks install --local luacov
luarocks make --local --deps-mode=none coding-adventures-zip-0.2.0-1.rockspec
luac -p src/coding_adventures/zip/init.lua tests/test_zip.lua tests/test_portable_conformance.lua tests/check_coverage.lua
eval "$(luarocks path)" && luacheck src tests --std lua54+busted
cd tests && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted . --verbose --pattern=test_
cd tests && rm -f luacov.stats.out luacov.report.out && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted test_portable_conformance.lua --coverage --coverage-config-file=../.luacov --exclude-tags=slow --pattern=test_ && luacov -c ../.luacov
cd tests && lua check_coverage.lua
