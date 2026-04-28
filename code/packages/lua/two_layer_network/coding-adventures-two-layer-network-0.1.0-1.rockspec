package = "coding-adventures-two-layer-network"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Two-layer neural network primitives for hidden-layer examples",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.two_layer_network"] = "src/coding_adventures/two_layer_network/init.lua",
    },
}
