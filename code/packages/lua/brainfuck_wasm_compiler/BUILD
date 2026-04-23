luarocks show coding-adventures-brainfuck-ir-compiler >/dev/null 2>&1 || (cd ../brainfuck_ir_compiler && luarocks make --local coding-adventures-brainfuck-ir-compiler-0.1.0-1.rockspec)
luarocks show coding-adventures-ir-to-wasm-compiler >/dev/null 2>&1 || (cd ../ir_to_wasm_compiler && luarocks make --local --deps-mode=none coding-adventures-ir-to-wasm-compiler-0.1.0-1.rockspec)
luarocks show coding-adventures-ir-to-wasm-validator >/dev/null 2>&1 || (cd ../ir_to_wasm_validator && luarocks make --local --deps-mode=none coding-adventures-ir-to-wasm-validator-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-module-encoder >/dev/null 2>&1 || (cd ../wasm_module_encoder && luarocks make --local --deps-mode=none coding-adventures-wasm-module-encoder-0.1.0-1.rockspec)
luarocks show coding-adventures-wasm-validator >/dev/null 2>&1 || (cd ../wasm_validator && luarocks make --local --deps-mode=none coding-adventures-wasm-validator-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-brainfuck-wasm-compiler-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
