package = "coding-adventures-activation-functions"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Neural network activation functions — sigmoid, relu, tanh, elu, softmax",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.activation_functions"] = "src/coding_adventures/activation_functions/init.lua",
    },
}
