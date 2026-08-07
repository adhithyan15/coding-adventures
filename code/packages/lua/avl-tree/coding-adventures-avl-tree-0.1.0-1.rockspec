package = "coding-adventures-avl-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Persistent AVL tree with order statistics",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.avl_tree"] = "src/coding_adventures/avl_tree/init.lua",
    },
}
