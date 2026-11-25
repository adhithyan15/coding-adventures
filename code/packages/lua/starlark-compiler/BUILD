luac -p src/coding_adventures/starlark_compiler/constants.lua src/coding_adventures/starlark_compiler/messages.lua src/coding_adventures/starlark_compiler/errors.lua src/coding_adventures/starlark_compiler/opcodes.lua src/coding_adventures/starlark_compiler/init.lua tests/test_starlark_compiler.lua tests/check_coverage.lua
cd tests && busted . --verbose --pattern=test_ && lua check_coverage.lua
