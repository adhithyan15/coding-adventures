package = "coding-adventures-huffman-compression"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Huffman lossless compression algorithm (1952) from scratch — CMP04",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-huffman-tree",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.huffman_compression"] = "src/coding_adventures/huffman_compression/init.lua",
    },
}
