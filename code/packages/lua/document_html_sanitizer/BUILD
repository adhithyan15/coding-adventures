mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
eval $(luarocks path --bin)
luarocks make coding-adventures-document-html-sanitizer-0.1.0-1.rockspec
busted spec/ --verbose
