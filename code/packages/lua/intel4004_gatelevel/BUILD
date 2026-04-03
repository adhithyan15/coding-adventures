luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks show coding-adventures-logic-gates >/dev/null 2>&1 || (cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
luarocks show coding-adventures-arithmetic >/dev/null 2>&1 || (cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-intel4004-gatelevel-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
