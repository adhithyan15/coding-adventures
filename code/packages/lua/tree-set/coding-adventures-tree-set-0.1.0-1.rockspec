package = "coding-adventures-tree-set"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "AVL-backed ordered set with rank, range, and set algebra",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-avl-tree >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.tree_set"] = "src/coding_adventures/tree_set/init.lua",
    },
}
