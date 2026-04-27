package = "coding-adventures-nib-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Nib parser — grammar-driven Nib parser built on the shared parser engine",
    detailed = [[
        A thin wrapper around the grammar-driven parser infrastructure from the
        coding-adventures-parser package. Loads the shared nib.grammar file,
        tokenizes with coding-adventures-nib-lexer, and produces a generic AST.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-nib-lexer >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.nib_parser"] = "src/coding_adventures/nib_parser/init.lua",
    },
}
