package = "coding-adventures-radix-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Path-compressed radix tree for UTF-8 string keys",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.radix_tree"] = "src/coding_adventures/radix_tree/init.lua",
    },
}
