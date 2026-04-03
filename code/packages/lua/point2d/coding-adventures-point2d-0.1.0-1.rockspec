package = "coding-adventures-point2d"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Immutable 2D point/vector and axis-aligned bounding rectangle",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-trig",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.point2d"] = "src/coding_adventures/point2d/init.lua",
    },
}
