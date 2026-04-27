package = "coding-adventures-brainfuck-ir-compiler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Brainfuck AOT compiler frontend: compiles Brainfuck ASTs to general-purpose IR",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-compiler-ir >= 0.1.0",
    "coding-adventures-compiler-source-map >= 0.1.0",
    "coding-adventures-brainfuck >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.brainfuck_ir_compiler"] = "src/coding_adventures/brainfuck_ir_compiler/init.lua",
    },
}
