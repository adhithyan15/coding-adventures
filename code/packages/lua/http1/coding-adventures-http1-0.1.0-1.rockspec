package = "coding-adventures-http1"
version = "0.1.0-1"
source = {
    url = "https://github.com/adhithyan15/coding-adventures.git",
    tag = "b132b780d8400fbc14b3de4cb7d8a26bf3195fd6",
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
