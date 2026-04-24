package = "coding-adventures-zip"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ZIP archive format (PKZIP 1989) implemented from scratch in Lua — CMP09",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-lzss",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.zip"] = "src/coding_adventures/zip/init.lua",
    },
}
