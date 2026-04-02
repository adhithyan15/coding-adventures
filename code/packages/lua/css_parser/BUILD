mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
(cd ../grammar_tools && luarocks make --local coding-adventures-grammar-tools-0.1.0-1.rockspec)
(cd ../lexer && luarocks make --local coding-adventures-lexer-0.1.0-1.rockspec)
(cd ../css_lexer && luarocks make --local coding-adventures-css-lexer-0.1.0-1.rockspec)
(cd ../parser && luarocks make --local coding-adventures-parser-0.1.0-1.rockspec)
luarocks make --local coding-adventures-css-parser-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
