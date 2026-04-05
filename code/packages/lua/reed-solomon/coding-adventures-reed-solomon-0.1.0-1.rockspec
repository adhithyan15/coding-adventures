package = "coding-adventures-reed-solomon"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Reed-Solomon error-correcting codes over GF(256) — encode, decode, syndromes",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-gf256",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.reed_solomon"] = "src/coding_adventures/reed_solomon/init.lua",
    },
}
