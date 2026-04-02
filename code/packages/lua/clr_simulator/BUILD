luarocks remove --force coding-adventures-clr-simulator 2>/dev/null || true
luarocks remove --force coding-adventures-cpu-pipeline 2>/dev/null || true
luarocks remove --force coding-adventures-cpu-simulator 2>/dev/null || true
luarocks make --local --no-manifest coding-adventures-clr-simulator-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
