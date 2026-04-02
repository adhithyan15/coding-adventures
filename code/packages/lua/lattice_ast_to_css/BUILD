mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
cd ../grammar_tools && luarocks make --local coding-adventures-grammar-tools-0.1.0-1.rockspec
(cd ../lattice_parser && luarocks make --local coding-adventures-lattice-parser-0.1.0-1.rockspec)
luarocks make --local coding-adventures-lattice-ast-to-css-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
