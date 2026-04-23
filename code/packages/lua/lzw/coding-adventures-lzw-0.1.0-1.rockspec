package = "coding-adventures-lzw"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "LZW lossless compression algorithm (1984) from scratch — CMP03",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.lzw"] = "src/coding_adventures/lzw/init.lua",
    },
}
