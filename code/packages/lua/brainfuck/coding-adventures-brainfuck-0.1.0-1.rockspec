package = "coding-adventures-brainfuck"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Brainfuck interpreter and bytecode compiler with pre-computed jump targets",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-interpreter-ir >= 0.1.0",
    "coding-adventures-vm-core >= 0.1.0",
    "coding-adventures-jit-core >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.brainfuck"] = "src/coding_adventures/brainfuck/init.lua",
        ["coding_adventures.brainfuck.lexer"] = "src/coding_adventures/brainfuck/lexer.lua",
        ["coding_adventures.brainfuck.parser"] = "src/coding_adventures/brainfuck/parser.lua",
        ["coding_adventures.brainfuck.lang_vm"] = "src/coding_adventures/brainfuck/lang_vm.lua",
    },
}
