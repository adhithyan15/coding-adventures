luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
luarocks make --local coding-adventures-gpu-core-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
