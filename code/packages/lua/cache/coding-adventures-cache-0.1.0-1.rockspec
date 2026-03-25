package = "coding-adventures-cache"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Configurable CPU cache hierarchy simulator",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.cache"] = "src/coding_adventures/cache/init.lua",
        ["coding_adventures.cache.cache_line"] = "src/coding_adventures/cache/cache_line.lua",
        ["coding_adventures.cache.cache_set"] = "src/coding_adventures/cache/cache_set.lua",
        ["coding_adventures.cache.stats"] = "src/coding_adventures/cache/stats.lua",
        ["coding_adventures.cache.cache"] = "src/coding_adventures/cache/cache.lua",
        ["coding_adventures.cache.hierarchy"] = "src/coding_adventures/cache/hierarchy.lua",
    },
}
