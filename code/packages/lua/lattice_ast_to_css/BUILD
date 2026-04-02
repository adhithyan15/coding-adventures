cd ../grammar_tools && luarocks make --local --no-manifest coding-adventures-grammar-tools-0.1.0-1.rockspec
(cd ../lattice_parser && luarocks make --local --no-manifest coding-adventures-lattice-parser-0.1.0-1.rockspec)
luarocks make --local --no-manifest coding-adventures-lattice-ast-to-css-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
