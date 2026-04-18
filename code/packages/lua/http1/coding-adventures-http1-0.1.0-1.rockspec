package = "coding-adventures-http1"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "HTTP/1 request and response head parser with body framing detection",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-http-core >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.http1"] = "src/coding_adventures/http1/init.lua",
    },
}
