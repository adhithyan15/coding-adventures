package = "coding-adventures-bezier2d"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Quadratic and cubic Bezier curves",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-trig",
    "coding-adventures-point2d",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.bezier2d"] = "src/coding_adventures/bezier2d/init.lua",
    },
}
