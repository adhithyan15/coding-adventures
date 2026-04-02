cd ../grammar_tools && luarocks make --local --no-manifest --no-deps coding-adventures-grammar-tools-0.1.0-1.rockspec
(cd ../lattice_ast_to_css && luarocks make --local --no-manifest --no-deps coding-adventures-lattice-ast-to-css-0.1.0-1.rockspec)
luarocks make --local --no-manifest --no-deps coding-adventures-lattice-transpiler-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
