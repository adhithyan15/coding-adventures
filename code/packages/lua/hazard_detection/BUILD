luarocks remove --force coding-adventures-hazard-detection 2>/dev/null || true
luarocks make --local coding-adventures-hazard-detection-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
