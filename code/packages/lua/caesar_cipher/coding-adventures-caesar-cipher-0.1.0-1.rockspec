package = "coding-adventures-caesar-cipher"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Caesar cipher — the oldest substitution cipher, with brute-force and frequency analysis",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.caesar_cipher"] = "src/coding_adventures/caesar_cipher/init.lua",
    },
}
