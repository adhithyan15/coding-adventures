luarocks remove --force coding-adventures-cli-builder 2>/dev/null || true
luarocks make --local --no-manifest coding-adventures-device-driver-framework-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
