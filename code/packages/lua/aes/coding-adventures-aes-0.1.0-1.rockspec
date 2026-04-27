package = "coding-adventures-aes"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "AES block cipher (FIPS 197) — AES-128, AES-192, and AES-256",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.aes"] = "src/coding_adventures/aes/init.lua",
    },
}
