(cd ../transistors && luarocks make --local --no-manifest --no-deps coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local --no-manifest --no-deps coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../block_ram && luarocks make --local --no-manifest --no-deps coding-adventures-block-ram-0.1.0-1.rockspec)
luarocks make --local --no-manifest --no-deps coding-adventures-fpga-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
