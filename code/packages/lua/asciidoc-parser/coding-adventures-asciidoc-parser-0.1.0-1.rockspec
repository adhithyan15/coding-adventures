package = "coding-adventures-asciidoc-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "AsciiDoc parser producing Document AST nodes",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-document-ast >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.asciidoc_parser"] = "src/coding_adventures/asciidoc_parser/init.lua",
        ["coding_adventures.asciidoc_parser.block_parser"] = "src/coding_adventures/asciidoc_parser/block_parser.lua",
        ["coding_adventures.asciidoc_parser.inline_parser"] = "src/coding_adventures/asciidoc_parser/inline_parser.lua",
    },
}
