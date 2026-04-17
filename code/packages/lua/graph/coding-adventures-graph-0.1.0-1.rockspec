package = "coding-adventures-graph"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Undirected weighted graph with DT00 algorithms in Lua",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.graph"] = "src/coding_adventures/graph/init.lua",
    },
}
