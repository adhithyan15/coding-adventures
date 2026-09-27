luarocks show coding-adventures-der-asn1 1>/dev/null 2>/dev/null || (cd ../der_asn1 && luarocks make --local --deps-mode=none coding-adventures-der-asn1-0.1.0-1.rockspec)
luarocks install --local busted
luarocks install --local dkjson
luarocks install --local luacheck
luarocks install --local luacov
luarocks make --local --deps-mode=none coding-adventures-x509-extension-0.1.0-1.rockspec
luac -p src/coding_adventures/x509_extension/init.lua tests/test_x509_extension.lua tests/check_coverage.lua
eval "$(luarocks path)" && luacheck src tests --std lua54+busted
cd tests && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../der_asn1/src/?.lua;../../der_asn1/src/?/init.lua;../../der_tlv/src/?.lua;../../der_tlv/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted . --verbose --pattern=test_
cd tests && rm -f luacov.stats.out luacov.report.out && export PATH="$(luarocks path --lr-bin):$PATH" && export LUA_PATH="../src/?.lua;../src/?/init.lua;../../der_asn1/src/?.lua;../../der_asn1/src/?/init.lua;../../der_tlv/src/?.lua;../../der_tlv/src/?/init.lua;$(luarocks path --lr-path);;" && export LUA_CPATH="$(luarocks path --lr-cpath);;" && busted test_x509_extension.lua --coverage --coverage-config-file=../.luacov --pattern=test_ && luacov -c ../.luacov
cd tests && lua check_coverage.lua
