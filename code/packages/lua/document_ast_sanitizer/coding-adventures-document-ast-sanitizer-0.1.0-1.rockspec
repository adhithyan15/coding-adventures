package = "coding-adventures-document-ast-sanitizer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Policy-driven AST sanitizer for Document AST nodes (TE02 spec)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-document-ast >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.document_ast_sanitizer"]           = "src/coding_adventures/document_ast_sanitizer/init.lua",
        ["coding_adventures.document_ast_sanitizer.policy"]    = "src/coding_adventures/document_ast_sanitizer/policy.lua",
        ["coding_adventures.document_ast_sanitizer.url_utils"] = "src/coding_adventures/document_ast_sanitizer/url_utils.lua",
        ["coding_adventures.document_ast_sanitizer.sanitizer"] = "src/coding_adventures/document_ast_sanitizer/sanitizer.lua",
    },
}
