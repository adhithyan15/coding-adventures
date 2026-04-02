cd ../wasm_leb128 && luarocks make --local --no-manifest --no-deps coding-adventures-wasm-leb128-0.1.0-1.rockspec
cd ../wasm_types && luarocks make --local --no-manifest --no-deps coding-adventures-wasm-types-0.1.0-1.rockspec
cd ../wasm_opcodes && luarocks make --local --no-manifest --no-deps coding-adventures-wasm-opcodes-0.1.0-1.rockspec
cd ../wasm_module_parser && luarocks make --local --no-manifest --no-deps coding-adventures-wasm-module-parser-0.1.0-1.rockspec
luarocks make --local --no-manifest --no-deps coding-adventures-wasm-simulator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
