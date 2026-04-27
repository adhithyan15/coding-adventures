package = "coding-adventures-deflate"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "DEFLATE lossless compression algorithm (1996) from scratch — CMP05",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-huffman-tree",
    "coding-adventures-lzss",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.deflate"] = "src/coding_adventures/deflate/init.lua",
    },
}
