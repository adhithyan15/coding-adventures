luarocks remove --force coding-adventures-cli-builder 2>/dev/null || true
luarocks make --local --no-manifest --no-deps coding-adventures-device-driver-framework-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
