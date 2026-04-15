package = "coding-adventures-url-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "RFC 1738 URL parser with relative resolution and percent-encoding",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.url_parser"] = "src/coding_adventures/url_parser/init.lua",
    },
}
