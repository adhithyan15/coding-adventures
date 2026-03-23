package = "coding-adventures-arithmetic"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Integer arithmetic circuits built from logic gates — half adder, full adder, ripple-carry adder, ALU",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-logic-gates >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.arithmetic"] = "src/coding_adventures/arithmetic/init.lua",
        ["coding_adventures.arithmetic.adder"] = "src/coding_adventures/arithmetic/adder.lua",
        ["coding_adventures.arithmetic.alu"] = "src/coding_adventures/arithmetic/alu.lua",
    },
}
