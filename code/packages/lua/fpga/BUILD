(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local --deps-mode=none coding-adventures-logic-gates-0.6.0-1.rockspec)
luarocks show coding-adventures-block-ram >/dev/null 2>&1 || (cd ../block_ram && luarocks make --local --deps-mode=none coding-adventures-block-ram-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-fpga-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
