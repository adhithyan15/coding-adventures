package = "coding-adventures-lz77"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "LZ77 lossless compression algorithm (1977) from scratch — CMP00",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.lz77"] = "src/coding_adventures/lz77/init.lua",
    },
}
