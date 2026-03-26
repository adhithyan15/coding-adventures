package = "coding-adventures-bytecode-compiler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Compiler translating ASTs to stack-based bytecode instructions",
    detailed = [[
        Three compilers in one package:
        1. BytecodeCompiler — hardcoded AST-to-bytecode compiler for our VM.
        2. JVMCompiler — targets JVM-style bytecode (ICONST, BIPUSH, LDC, etc.).
        3. GenericCompiler — pluggable framework with handler registration,
           scope management, jump patching, and nested code object compilation.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-virtual-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.bytecode_compiler"] = "src/coding_adventures/bytecode_compiler/init.lua",
    },
}
