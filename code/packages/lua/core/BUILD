mkdir -p "$HOME/.luarocks/lib/luarocks/rocks-5.4"
[ -f "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest" ] || printf 'commands={}
dependencies={}
modules={}
repository={}' > "$HOME/.luarocks/lib/luarocks/rocks-5.4/manifest"
(cd ../cpu_pipeline && luarocks make --local coding-adventures-cpu-pipeline-0.1.0-1.rockspec)
(cd ../cpu_simulator && luarocks make --local coding-adventures-cpu-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-core-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
