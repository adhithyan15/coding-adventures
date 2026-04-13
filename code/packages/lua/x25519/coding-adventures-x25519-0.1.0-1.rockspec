package = "coding-adventures-x25519"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua X25519 (RFC 7748) elliptic curve Diffie-Hellman",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.x25519"] = "src/coding_adventures/x25519/init.lua",
    },
}
