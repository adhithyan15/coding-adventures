package = "coding-adventures-brotli"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Brotli lossless compression algorithm (2013) from scratch — CMP06",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-huffman-tree",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.brotli"] = "src/coding_adventures/brotli/init.lua",
    },
}
