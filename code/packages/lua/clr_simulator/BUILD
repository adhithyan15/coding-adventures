mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
luarocks remove --force coding-adventures-clr-simulator 2>/dev/null || true
luarocks remove --force coding-adventures-cpu-pipeline 2>/dev/null || true
luarocks remove --force coding-adventures-cpu-simulator 2>/dev/null || true
luarocks remove --force coding-adventures-hazard-detection 2>/dev/null || true
luarocks make --local coding-adventures-clr-simulator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
