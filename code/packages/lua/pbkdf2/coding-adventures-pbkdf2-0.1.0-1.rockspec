package = "coding-adventures-pbkdf2"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-hmac >= 0.1",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.pbkdf2"] = "src/coding_adventures/pbkdf2/init.lua",
    },
}
