luarocks remove --force coding-adventures-paint-instructions 2>/dev/null || true
luarocks make --local coding-adventures-paint-instructions-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
