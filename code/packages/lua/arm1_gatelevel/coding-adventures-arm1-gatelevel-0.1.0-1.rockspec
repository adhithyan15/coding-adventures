package = "coding-adventures-arm1-gatelevel"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "ARM1 gate-level simulator — every ALU and barrel-shift operation routes through logic gate calls",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-logic-gates",
    "coding-adventures-arithmetic",
    "coding-adventures-arm1-simulator",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.arm1_gatelevel"] = "src/coding_adventures/arm1_gatelevel/init.lua",
    },
}
