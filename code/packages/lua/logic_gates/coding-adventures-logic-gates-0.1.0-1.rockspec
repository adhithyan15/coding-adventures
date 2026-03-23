package = "coding-adventures-logic-gates"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "The fundamental building blocks of all digital circuits",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.logic_gates"] = "src/coding_adventures/logic_gates/init.lua",
        ["coding_adventures.logic_gates.gates"] = "src/coding_adventures/logic_gates/gates.lua",
        ["coding_adventures.logic_gates.sequential"] = "src/coding_adventures/logic_gates/sequential.lua",
    },
}
