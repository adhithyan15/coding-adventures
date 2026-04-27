luarocks show coding-adventures-nib-parser >/dev/null 2>&1 || (cd ../nib_parser && sh BUILD)
luarocks show coding-adventures-type-checker-protocol >/dev/null 2>&1 || (cd ../type_checker_protocol && sh BUILD)
luarocks make --local --deps-mode=none coding-adventures-nib-type-checker-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
