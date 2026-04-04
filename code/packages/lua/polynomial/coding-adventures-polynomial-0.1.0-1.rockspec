package = "coding-adventures-polynomial"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Coefficient-array polynomial arithmetic over real numbers — add, subtract, multiply, divide, evaluate, GCD",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.polynomial"] = "src/coding_adventures/polynomial/init.lua",
    },
}
