package = "coding-adventures-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Tree data structure built on directed graphs — parent-child relationships, traversals",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-directed-graph >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.tree"] = "src/coding_adventures/tree/init.lua",
    },
}
