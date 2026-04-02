luarocks remove --force coding-adventures-network-stack 2>/dev/null || true
luarocks make --local --no-manifest --no-deps coding-adventures-network-stack-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
