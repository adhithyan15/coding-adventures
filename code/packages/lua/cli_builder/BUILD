luarocks remove --force coding-adventures-cli-builder 2>/dev/null || true
luarocks make --local coding-adventures-cli-builder-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
