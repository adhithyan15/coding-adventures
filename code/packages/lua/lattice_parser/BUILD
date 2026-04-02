(cd ../grammar_tools && luarocks make --local coding-adventures-grammar-tools-0.1.0-1.rockspec)
(cd ../lattice_lexer && luarocks make --local coding-adventures-lattice-lexer-0.1.0-1.rockspec)
(cd ../parser && luarocks make --local coding-adventures-parser-0.1.0-1.rockspec)
luarocks make --local coding-adventures-lattice-parser-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
