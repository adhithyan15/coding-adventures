package = "coding-adventures-blake2b"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua BLAKE2b (RFC 7693) cryptographic hash function",
    license = "MIT",
}
dependencies = {
    "lua >= 5.3",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.blake2b"] = "src/coding_adventures/blake2b/init.lua",
    },
}
