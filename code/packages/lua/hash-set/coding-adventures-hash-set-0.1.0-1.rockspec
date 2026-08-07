package = "coding-adventures-hash-set"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Persistent hash set with complete set algebra",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-hash-map >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.hash_set"] = "src/coding_adventures/hash_set/init.lua",
    },
}
