(cd ../pixel-container && luarocks make --local --deps-mode=none coding-adventures-pixel-container-0.1.0-1.rockspec)
(cd ../lzss && luarocks make --local --deps-mode=none coding-adventures-lzss-0.1.0-1.rockspec)
(cd ../zip && luarocks make --local --deps-mode=none coding-adventures-zip-0.2.0-1.rockspec)
luarocks install --local busted
luarocks install --local dkjson
luarocks install --local libdeflate 1.0.2-1
luarocks install --local luacheck
luarocks install --local luacov
luarocks make --local --deps-mode=none coding-adventures-image-codec-png-0.1.0-1.rockspec
luac -p src/coding_adventures/image_codec_png/init.lua tests/test_image_codec_png.lua tests/test_portable_conformance.lua tests/check_coverage.lua
eval "$(luarocks path)" && luacheck src tests --std lua54+busted
cd tests && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua;../../zip/src/?.lua;../../zip/src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted . --verbose --pattern=test_
cd tests && rm -f luacov.stats.out luacov.report.out && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua;../../zip/src/?.lua;../../zip/src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted . --coverage --coverage-config-file=../.luacov --pattern=test_ && luacov -c ../.luacov
cd tests && lua check_coverage.lua
