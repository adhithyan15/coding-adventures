package = "coding-adventures-grammar-tools"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Grammar definition and manipulation — declarative specifications for lexers and parsers",
    detailed = [[
        Parses and validates .tokens files (lexical token definitions) and
        .grammar files (EBNF-like parser rules). Supports cross-validation
        to ensure token and parser grammars are consistent. Ported from Go.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.grammar_tools"] = "src/coding_adventures/grammar_tools/init.lua",
        ["coding_adventures.grammar_tools.compiler"] = "src/coding_adventures/grammar_tools/compiler.lua",
    },
}
