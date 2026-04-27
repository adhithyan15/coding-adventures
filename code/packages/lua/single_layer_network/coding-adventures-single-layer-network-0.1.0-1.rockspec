package = "coding-adventures-single-layer-network"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Single-layer multi-input multi-output neural network primitives",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.single_layer_network"] = "src/coding_adventures/single_layer_network/init.lua",
    },
}
