mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
luarocks remove --force coding-adventures-cli-builder 2>/dev/null || true
luarocks make --local coding-adventures-device-driver-framework-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
