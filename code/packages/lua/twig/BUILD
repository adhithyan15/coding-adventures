(cd ../interpreter_ir && luarocks make --local coding-adventures-interpreter-ir-0.1.0-1.rockspec)
(cd ../vm_core && luarocks make --local --deps-mode=none coding-adventures-vm-core-0.1.0-1.rockspec)
(cd ../codegen_core && luarocks make --local --deps-mode=none coding-adventures-codegen-core-0.1.0-1.rockspec)
(cd ../jit_core && luarocks make --local --deps-mode=none coding-adventures-jit-core-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-twig-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
