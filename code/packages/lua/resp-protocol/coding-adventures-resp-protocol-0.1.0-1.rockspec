package = "coding-adventures-resp-protocol"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "RESP2 encoder and incremental streaming decoder",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.resp_protocol"] = "src/coding_adventures/resp_protocol/init.lua",
    },
}
