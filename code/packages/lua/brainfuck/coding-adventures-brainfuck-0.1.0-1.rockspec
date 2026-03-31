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
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.brainfuck"] = "src/coding_adventures/brainfuck/init.lua",
    },
}
