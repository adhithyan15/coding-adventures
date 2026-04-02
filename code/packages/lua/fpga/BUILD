luarocks remove --force coding-adventures-fpga 2>/dev/null || true
luarocks make --local --no-manifest coding-adventures-fpga-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
