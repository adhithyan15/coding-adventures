mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
(cd ../matrix && luarocks make --local coding-adventures-matrix-0.1.0-1.rockspec)
(cd ../loss_functions && luarocks make --local coding-adventures-loss-functions-0.1.0-1.rockspec)
cd tests && busted . --verbose --pattern=test_
