luarocks show coding-adventures-paint-instructions >/dev/null 2>&1 || (cd ../paint_instructions && luarocks make --local --deps-mode=none coding-adventures-paint-instructions-0.1.0-1.rockspec)
luarocks show coding-adventures-barcode-layout-1d >/dev/null 2>&1 || (cd ../barcode_layout_1d && luarocks make --local --deps-mode=none coding-adventures-barcode-layout-1d-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-code39-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
