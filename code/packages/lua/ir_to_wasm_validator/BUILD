luarocks show coding-adventures-ir-to-wasm-compiler >/dev/null 2>&1 || (cd ../ir_to_wasm_compiler && luarocks make --local --deps-mode=none coding-adventures-ir-to-wasm-compiler-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-ir-to-wasm-validator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
