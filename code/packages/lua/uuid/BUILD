mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
(cd ../md5 && luarocks make --local coding-adventures-md5-0.1.0-1.rockspec)
(cd ../sha1 && luarocks make --local coding-adventures-sha1-0.1.0-1.rockspec)
cd tests && busted . --verbose --pattern=test_
