package = "coding-adventures-commonmark-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "CommonMark 0.31.2 compliant Markdown parser producing Document AST",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-document-ast >= 0.1.0",
    "dkjson",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.commonmark_parser"] = "src/coding_adventures/commonmark_parser/init.lua",
        ["coding_adventures.commonmark_parser.scanner"] = "src/coding_adventures/commonmark_parser/scanner.lua",
        ["coding_adventures.commonmark_parser.entities"] = "src/coding_adventures/commonmark_parser/entities.lua",
        ["coding_adventures.commonmark_parser.entity_table"] = "src/coding_adventures/commonmark_parser/entity_table.lua",
    },
}
