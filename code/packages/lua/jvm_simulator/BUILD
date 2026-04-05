luarocks remove --force coding-adventures-jvm-simulator 2>/dev/null || true
luarocks make --local coding-adventures-jvm-simulator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
