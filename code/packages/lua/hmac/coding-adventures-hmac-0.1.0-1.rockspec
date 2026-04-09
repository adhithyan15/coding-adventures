package = "coding-adventures-hmac"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-md5 >= 0.1",
    "coding-adventures-sha1 >= 0.1",
    "coding-adventures-sha256 >= 0.1",
    "coding-adventures-sha512 >= 0.1",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.hmac"] = "src/coding_adventures/hmac/init.lua",
    },
}
