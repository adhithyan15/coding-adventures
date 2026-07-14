package = "coding-adventures-binary-search-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Persistent binary search tree with order statistics",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.binary_search_tree"] = "src/coding_adventures/binary_search_tree/init.lua",
    },
}
