luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
luarocks make --local --no-manifest --no-deps coding-adventures-branch-predictor-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
