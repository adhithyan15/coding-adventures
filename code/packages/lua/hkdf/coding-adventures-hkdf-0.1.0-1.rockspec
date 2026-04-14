package = "coding-adventures-hkdf"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "HKDF (HMAC-based Extract-and-Expand Key Derivation Function) — RFC 5869 — implemented from scratch",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-hmac >= 0.1",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.hkdf"] = "src/coding_adventures/hkdf/init.lua",
    },
}
