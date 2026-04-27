package = "coding-adventures-zstd"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ZStd (RFC 8878) lossless compression from scratch — CMP07",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-lzss",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.zstd"] = "src/coding_adventures/zstd/init.lua",
    },
}
