package = "coding-adventures-ed25519"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua Ed25519 (RFC 8032) digital signatures",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-sha512",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.ed25519"] = "src/coding_adventures/ed25519/init.lua",
    },
}
