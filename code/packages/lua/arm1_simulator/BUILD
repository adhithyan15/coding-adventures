luarocks remove --force coding-adventures-arm1-simulator 2>/dev/null || true
luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
