package = "coding-adventures-arc2d"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Elliptical arc in center and SVG endpoint forms",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-trig",
    "coding-adventures-point2d",
    "coding-adventures-bezier2d",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.arc2d"] = "src/coding_adventures/arc2d/init.lua",
    },
}
