package = "coding-adventures-perceptron"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Single-layer perceptron neural network with perceptron learning rule",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-matrix >= 0.1.0",
    "coding-adventures-activation-functions >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.perceptron"] = "src/coding_adventures/perceptron/init.lua",
    },
}
