package = "coding-adventures-affine2d"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "2D affine transformation matrix (SVG/Canvas convention)",
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
        ["coding_adventures.affine2d"] = "src/coding_adventures/affine2d/init.lua",
    },
}
