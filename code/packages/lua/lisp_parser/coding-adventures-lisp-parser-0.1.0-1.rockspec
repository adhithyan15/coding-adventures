package = "coding-adventures-lisp-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Lisp parser — builds ASTs from S-expressions",
    detailed = [[
        A grammar-driven parser for Lisp/Scheme source text.  Tokenizes
        source using coding-adventures-lisp-lexer, then uses the shared
        lisp.grammar file and GrammarParser to produce an Abstract Syntax
        Tree.  Handles atoms, lists, quoted forms, dotted pairs, and
        multi-expression programs.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-lisp-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.lisp_parser"] = "src/coding_adventures/lisp_parser/init.lua",
    },
}
