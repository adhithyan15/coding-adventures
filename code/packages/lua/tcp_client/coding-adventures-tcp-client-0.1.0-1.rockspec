package = "coding-adventures-tcp-client"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "TCP client with buffered I/O and configurable timeouts",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "luasocket",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.tcp_client"] = "src/coding_adventures/tcp_client/init.lua",
    },
}
