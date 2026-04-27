luarocks show coding-adventures-paint-instructions >/dev/null 2>&1 || (cd ../paint_instructions && luarocks make --local --deps-mode=none coding-adventures-paint-instructions-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-paint-vm-ascii-0.1.0-1.rockspec
busted tests --verbose --pattern=test_
