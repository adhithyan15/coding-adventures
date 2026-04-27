package = "coding-adventures-image-geometric-transforms"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IMG04: Geometric transforms on PixelContainer",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-pixel-container >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.image_geometric_transforms"] =
            "src/coding_adventures/image_geometric_transforms/init.lua",
    },
}
