package = "coding-adventures-fenwick-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Binary Indexed Tree with prefix sums and point updates",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.fenwick_tree"] = "src/coding_adventures/fenwick_tree/init.lua",
    },
}
