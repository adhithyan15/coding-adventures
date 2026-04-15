(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../grammar_tools && luarocks make --local coding-adventures-grammar-tools-0.1.0-1.rockspec)
(cd ../lexer && luarocks make --local coding-adventures-lexer-0.1.0-1.rockspec)
(cd ../nib_lexer && luarocks make --local coding-adventures-nib-lexer-0.1.0-1.rockspec)
(cd ../parser && luarocks make --local coding-adventures-parser-0.1.0-1.rockspec)
luarocks make --local coding-adventures-nib-parser-0.1.0-1.rockspec
cd tests && LUA_PATH="../../nib_lexer/src/?.lua;../../nib_lexer/src/?/init.lua;../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_
