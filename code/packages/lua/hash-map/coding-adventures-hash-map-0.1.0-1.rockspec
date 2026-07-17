package = "coding-adventures-hash-map"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Hash map with chaining and open-addressing strategies",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-hash-functions >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.hash_map"] = "src/coding_adventures/hash_map/init.lua",
    },
}
