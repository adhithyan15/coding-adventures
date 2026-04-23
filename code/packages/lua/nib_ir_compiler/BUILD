luarocks show coding-adventures-compiler-ir >/dev/null 2>&1 || (cd ../compiler_ir && sh BUILD)
luarocks show coding-adventures-nib-type-checker >/dev/null 2>&1 || (cd ../nib_type_checker && sh BUILD)
luarocks make --local --deps-mode=none coding-adventures-nib-ir-compiler-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
