package = "coding-adventures-image-codec-ppm"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IC02: PPM (P6) image encoder and decoder — RGB, alpha dropped on encode / set to 255 on decode",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-pixel-container >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.image_codec_ppm"] = "src/coding_adventures/image_codec_ppm/init.lua",
    },
}
