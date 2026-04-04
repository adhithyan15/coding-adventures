package = "coding-adventures-scytale-cipher"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Scytale cipher — ancient Spartan transposition cipher",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.scytale_cipher"] = "src/coding_adventures/scytale_cipher/init.lua",
    },
}
