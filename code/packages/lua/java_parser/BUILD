luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks show coding-adventures-grammar-tools >/dev/null 2>&1 || (cd ../grammar_tools && luarocks make --local coding-adventures-grammar-tools-0.1.0-1.rockspec)
luarocks show coding-adventures-lexer >/dev/null 2>&1 || (cd ../lexer && luarocks make --local --deps-mode=none coding-adventures-lexer-0.1.0-1.rockspec)
luarocks show coding-adventures-java-lexer >/dev/null 2>&1 || (cd ../java_lexer && luarocks make --local --deps-mode=none coding-adventures-java-lexer-0.1.0-1.rockspec)
luarocks show coding-adventures-parser >/dev/null 2>&1 || (cd ../parser && luarocks make --local --deps-mode=none coding-adventures-parser-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-java-parser-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
