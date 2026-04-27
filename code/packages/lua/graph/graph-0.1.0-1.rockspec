package = "graph"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Undirected graph data structure from scratch",
    detailed = [[
        An undirected graph library with basic graph operations,
        neighbor queries, and foundational algorithms.
    ]],
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
