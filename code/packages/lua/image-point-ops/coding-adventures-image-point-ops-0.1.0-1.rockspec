package = "coding-adventures-image-point-ops"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IMG03: Per-pixel point operations on PixelContainer (invert, gamma, LUTs, colour matrix, …)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-pixel-container >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.image_point_ops"] = "src/coding_adventures/image_point_ops/init.lua",
    },
}
