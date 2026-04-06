package = "coding-adventures-image-codec-qoi"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "IC03: QOI (Quite OK Image) encoder and decoder — all 6 ops, RGBA, big-endian header",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-pixel-container >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.image_codec_qoi"] = "src/coding_adventures/image_codec_qoi/init.lua",
    },
}
