package = "coding-adventures-binary-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Binary tree with traversal, shape, and array-conversion helpers",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.binary_tree"] = "src/coding_adventures/binary_tree/init.lua",
    },
}
