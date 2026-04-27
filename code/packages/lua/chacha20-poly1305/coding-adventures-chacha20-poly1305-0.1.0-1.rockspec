package = "coding-adventures-chacha20-poly1305"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ChaCha20-Poly1305 authenticated encryption (RFC 8439)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.chacha20_poly1305"] = "src/coding_adventures/chacha20_poly1305/init.lua",
    },
}
