luarocks show coding-adventures-compiler-ir >/dev/null 2>&1 || (cd ../compiler_ir && luarocks make --local coding-adventures-compiler-ir-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-leb128 >/dev/null 2>&1 || (cd ../wasm_leb128 && luarocks make --local coding-adventures-wasm-leb128-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-opcodes >/dev/null 2>&1 || (cd ../wasm_opcodes && luarocks make --local coding-adventures-wasm-opcodes-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-types >/dev/null 2>&1 || (cd ../wasm_types && luarocks make --local coding-adventures-wasm-types-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-ir-to-wasm-compiler-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
