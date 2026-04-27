package = "coding-adventures-argon2id"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua Argon2id (RFC 9106) -- hybrid memory-hard KDF (recommended)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.3",
    "coding-adventures-blake2b >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.argon2id"] = "src/coding_adventures/argon2id/init.lua",
    },
}
