package = "coding-adventures-document-html-sanitizer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pattern-based HTML string sanitizer — string in, string out (TE02 spec)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.document_html_sanitizer"]               = "src/coding_adventures/document_html_sanitizer/init.lua",
        ["coding_adventures.document_html_sanitizer.policy"]        = "src/coding_adventures/document_html_sanitizer/policy.lua",
        ["coding_adventures.document_html_sanitizer.url_utils"]     = "src/coding_adventures/document_html_sanitizer/url_utils.lua",
        ["coding_adventures.document_html_sanitizer.html_sanitizer"] = "src/coding_adventures/document_html_sanitizer/html_sanitizer.lua",
    },
}
