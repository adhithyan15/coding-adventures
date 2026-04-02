mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
cd ../wasm_leb128 && luarocks make --local coding-adventures-wasm-leb128-0.1.0-1.rockspec
cd ../wasm_types && luarocks make --local coding-adventures-wasm-types-0.1.0-1.rockspec
luarocks make --local coding-adventures-wasm-module-parser-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
