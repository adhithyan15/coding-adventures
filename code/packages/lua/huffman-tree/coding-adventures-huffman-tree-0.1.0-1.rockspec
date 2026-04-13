package = "coding-adventures-huffman-tree"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Huffman Tree optimal prefix-free entropy coding — DT27",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-heap >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.huffman_tree"] = "src/coding_adventures/huffman_tree/init.lua",
    },
}
