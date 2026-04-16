luarocks show coding-adventures-wasm-leb128 >/dev/null 2>&1 || (cd ../wasm_leb128 && luarocks make --local coding-adventures-wasm-leb128-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-wasm-module-encoder-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
