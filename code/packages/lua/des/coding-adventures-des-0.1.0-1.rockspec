package = "coding-adventures-des"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "DES and Triple DES (TDEA) block cipher — FIPS 46-3 / SP 800-67",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.des"] = "src/coding_adventures/des/init.lua",
    },
}
