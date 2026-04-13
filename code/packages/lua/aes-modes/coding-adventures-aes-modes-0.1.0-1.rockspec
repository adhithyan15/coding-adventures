package = "coding-adventures-aes-modes"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "AES modes of operation (ECB, CBC, CTR, GCM)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-aes >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.aes_modes"] = "src/coding_adventures/aes_modes/init.lua",
    },
}
