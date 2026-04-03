package = "coding-adventures-starlark-ast-to-bytecode-compiler"
version = "0.1.0-1"
source = {
    url = "https://github.com/coding-adventures/coding-adventures",
}
description = {
    summary     = "Compiles Starlark ASTs to stack-based bytecode",
    detailed    = [[
        Translates a Starlark Abstract Syntax Tree (AST) produced by the
        starlark_parser into a flat sequence of bytecode instructions ready
        for execution in a stack-based virtual machine.
    ]],
    homepage    = "https://github.com/coding-adventures/coding-adventures",
    license     = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-bytecode-compiler",
}
build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.starlark_ast_to_bytecode_compiler"] =
            "src/coding_adventures/starlark_ast_to_bytecode_compiler/init.lua",
    },
}
