(cd ../transistors && luarocks make --local --no-manifest coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local --no-manifest coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local --no-manifest coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local --no-manifest coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local --no-manifest coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
