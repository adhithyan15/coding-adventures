package = "coding-adventures-argon2d"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua Argon2d (RFC 9106) -- data-dependent memory-hard KDF",
    license = "MIT",
}
dependencies = {
    "lua >= 5.3",
    "coding-adventures-blake2b >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.argon2d"] = "src/coding_adventures/argon2d/init.lua",
    },
}
