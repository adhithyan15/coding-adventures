luarocks remove --force coding-adventures-clr-simulator 2>/dev/null || true
luarocks make --local coding-adventures-clr-simulator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
