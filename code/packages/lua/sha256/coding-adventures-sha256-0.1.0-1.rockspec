package = "coding-adventures-sha256"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua SHA-256 cryptographic hash function implementation",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.sha256"] = "src/coding_adventures/sha256/init.lua",
    },
}
