package = "coding-adventures-type-checker-protocol"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Shared type-check diagnostics and generic hook dispatch for Lua compiler frontends",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.type_checker_protocol"] = "src/coding_adventures/type_checker_protocol/init.lua",
    },
}
