package = "coding-adventures-in-memory-data-store"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Pure Lua RESP facade for the in-memory data store",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-in-memory-data-store-engine >= 0.1.0",
    "coding-adventures-in-memory-data-store-protocol >= 0.1.0",
    "coding-adventures-resp-protocol >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.in_memory_data_store"] = "src/coding_adventures/in_memory_data_store/init.lua",
    },
}
