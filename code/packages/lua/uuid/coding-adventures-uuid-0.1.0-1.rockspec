package = "coding-adventures-uuid"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "UUID v1/v3/v4/v5/v7 generation and parsing",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-md5 >= 0.1.0",
    "coding-adventures-sha1 >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.uuid"] = "src/coding_adventures/uuid/init.lua",
    },
}
