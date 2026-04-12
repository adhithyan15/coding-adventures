package = "coding-adventures-lzss"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "LZSS lossless compression algorithm (1982) from scratch — CMP02",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.lzss"] = "src/coding_adventures/lzss/init.lua",
    },
}
