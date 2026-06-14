(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../grammar_tools && luarocks make --local coding-adventures-grammar-tools-0.1.0-1.rockspec)
(cd ../lexer && luarocks make --local coding-adventures-lexer-0.1.0-1.rockspec)
(cd ../interpreter_ir && luarocks make --local coding-adventures-interpreter-ir-0.1.0-1.rockspec)
(cd ../vm_core && luarocks make --local --deps-mode=none coding-adventures-vm-core-0.1.0-1.rockspec)
(cd ../codegen_core && luarocks make --local --deps-mode=none coding-adventures-codegen-core-0.1.0-1.rockspec)
(cd ../jit_core && luarocks make --local --deps-mode=none coding-adventures-jit-core-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-brainfuck-0.1.0-1.rockspec
cd tests && busted . --verbose --pattern=test_
