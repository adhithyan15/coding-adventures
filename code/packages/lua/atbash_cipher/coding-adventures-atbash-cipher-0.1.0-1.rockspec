package = "coding-adventures-atbash-cipher"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Atbash cipher — fixed reverse-alphabet substitution, self-inverse",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.atbash_cipher"] = "src/coding_adventures/atbash_cipher/init.lua",
    },
}
