package = "coding-adventures-document-ast-to-html"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Document AST to HTML renderer following CommonMark rendering rules",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-document-ast >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.document_ast_to_html"] = "src/coding_adventures/document_ast_to_html/init.lua",
    },
}
