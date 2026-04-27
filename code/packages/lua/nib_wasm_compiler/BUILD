luarocks show coding-adventures-nib-parser >/dev/null 2>&1 || (cd ../nib_parser && sh BUILD)
luarocks show coding-adventures-nib-type-checker >/dev/null 2>&1 || (cd ../nib_type_checker && sh BUILD)
luarocks show coding-adventures-nib-ir-compiler >/dev/null 2>&1 || (cd ../nib_ir_compiler && sh BUILD)
luarocks show coding-adventures-ir-to-wasm-compiler >/dev/null 2>&1 || (cd ../ir_to_wasm_compiler && sh BUILD)
luarocks show coding-adventures-ir-to-wasm-validator >/dev/null 2>&1 || (cd ../ir_to_wasm_validator && sh BUILD)
luarocks show coding-adventures-wasm-module-encoder >/dev/null 2>&1 || (cd ../wasm_module_encoder && sh BUILD)
luarocks show coding-adventures-wasm-validator >/dev/null 2>&1 || (cd ../wasm_validator && sh BUILD)
luarocks make --local --deps-mode=none coding-adventures-nib-wasm-compiler-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
