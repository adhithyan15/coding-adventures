package = "coding-adventures-gf256"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Galois Field GF(2^8) arithmetic — add, subtract, multiply, divide, power, inverse — with log/antilog tables",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.gf256"] = "src/coding_adventures/gf256/init.lua",
    },
}
