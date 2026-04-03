luarocks remove --force coding-adventures-cpu-simulator 2>/dev/null || true
luarocks make --local coding-adventures-cpu-simulator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
