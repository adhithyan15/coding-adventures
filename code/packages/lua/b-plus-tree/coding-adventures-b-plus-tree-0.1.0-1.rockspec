package = "coding-adventures-b-plus-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "B+ Tree (DT12) — leaf-linked search tree from scratch in Lua",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.b_plus_tree"] = "src/coding_adventures/b_plus_tree/init.lua",
    },
}
