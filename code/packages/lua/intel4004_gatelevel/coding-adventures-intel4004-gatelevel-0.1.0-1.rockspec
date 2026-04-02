package = "coding-adventures-intel4004-gatelevel"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Intel 4004 gate-level simulator — all operations route through logic gates and flip-flops",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-logic-gates >= 0.1.0",
    "coding-adventures-arithmetic >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.intel4004_gatelevel"] = "src/coding_adventures/intel4004_gatelevel/init.lua",
    },
}
