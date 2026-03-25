package = "coding-adventures-document-ast"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Format-agnostic Document AST node constructors and type predicates",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.document_ast"] = "src/coding_adventures/document_ast/init.lua",
    },
}
