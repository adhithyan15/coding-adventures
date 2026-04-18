package = "coding-adventures-http-core"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Shared HTTP message types and helpers for request/response heads and body framing",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.http_core"] = "src/coding_adventures/http_core/init.lua",
    },
}
