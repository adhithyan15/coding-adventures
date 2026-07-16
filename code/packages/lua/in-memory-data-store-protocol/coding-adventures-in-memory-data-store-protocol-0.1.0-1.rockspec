package = "coding-adventures-in-memory-data-store-protocol"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Protocol IR for the in-memory data store",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.in_memory_data_store_protocol"] = "src/coding_adventures/in_memory_data_store_protocol/init.lua",
    },
}
