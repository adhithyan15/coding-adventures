package = "coding-adventures-commonmark"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "CommonMark 0.31.2 pipeline: Markdown to HTML via Document AST",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-commonmark-parser >= 0.1.0",
    "coding-adventures-document-ast-to-html >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.commonmark"] = "src/coding_adventures/commonmark/init.lua",
    },
}
