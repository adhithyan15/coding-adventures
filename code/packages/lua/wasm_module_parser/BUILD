cd ../wasm_leb128 && luarocks make --local --no-manifest coding-adventures-wasm-leb128-0.1.0-1.rockspec
cd ../wasm_types && luarocks make --local --no-manifest coding-adventures-wasm-types-0.1.0-1.rockspec
luarocks make --local --no-manifest coding-adventures-wasm-module-parser-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
