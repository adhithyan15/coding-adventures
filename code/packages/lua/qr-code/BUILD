luarocks show coding-adventures-gf256 >/dev/null 2>&1 || (cd ../gf256 && luarocks make --local --deps-mode=none coding-adventures-gf256-0.1.0-1.rockspec)
luarocks show coding-adventures-paint-instructions >/dev/null 2>&1 || (cd ../paint_instructions && luarocks make --local --deps-mode=none coding-adventures-paint-instructions-0.1.0-1.rockspec)
luarocks show coding-adventures-barcode-2d >/dev/null 2>&1 || (cd ../barcode-2d && luarocks make --local --deps-mode=none coding-adventures-barcode-2d-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-qr-code-0.1.0-1.rockspec
cd spec && busted . --verbose
