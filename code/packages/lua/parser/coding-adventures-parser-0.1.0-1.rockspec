package = "coding-adventures-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Recursive descent parser building Abstract Syntax Trees from token streams",
    detailed = [[
        Provides two parsing modes: a hand-written recursive descent parser
        with operator precedence for a small expression language, and a
        grammar-driven packrat parser that interprets BNF-like grammar rules
        at runtime. Port of the Go parser package.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.parser"] = "src/coding_adventures/parser/init.lua",
    },
}
