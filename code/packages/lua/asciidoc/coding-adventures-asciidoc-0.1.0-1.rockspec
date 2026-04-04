package = "coding-adventures-asciidoc"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "AsciiDoc to HTML pipeline: parse AsciiDoc and render to HTML via Document AST",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-asciidoc-parser >= 0.1.0",
    "coding-adventures-document-ast-to-html >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.asciidoc"] = "src/coding_adventures/asciidoc/init.lua",
    },
}
