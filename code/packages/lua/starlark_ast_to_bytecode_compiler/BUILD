(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../grammar_tools && luarocks make --local coding-adventures-grammar-tools-0.1.0-1.rockspec)
(cd ../lexer && luarocks make --local coding-adventures-lexer-0.1.0-1.rockspec)
(cd ../parser && luarocks make --local coding-adventures-parser-0.1.0-1.rockspec)
(cd ../virtual_machine && luarocks make --local coding-adventures-virtual-machine-0.1.0-1.rockspec)
(cd ../bytecode_compiler && luarocks make --local coding-adventures-bytecode-compiler-0.1.0-1.rockspec)
luarocks make --local coding-adventures-starlark-ast-to-bytecode-compiler-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
