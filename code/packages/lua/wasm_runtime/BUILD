luarocks show coding-adventures-wasm-leb128 >/dev/null 2>&1 || (cd ../wasm_leb128 && luarocks make --local coding-adventures-wasm-leb128-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-types >/dev/null 2>&1 || (cd ../wasm_types && luarocks make --local coding-adventures-wasm-types-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-opcodes >/dev/null 2>&1 || (cd ../wasm_opcodes && luarocks make --local coding-adventures-wasm-opcodes-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-module-parser >/dev/null 2>&1 || (cd ../wasm_module_parser && luarocks make --local coding-adventures-wasm-module-parser-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-execution >/dev/null 2>&1 || (cd ../wasm_execution && sh BUILD)
luarocks show coding-adventures-virtual-machine >/dev/null 2>&1 || (cd ../virtual_machine && luarocks make --local coding-adventures-virtual-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-wasm-runtime-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
