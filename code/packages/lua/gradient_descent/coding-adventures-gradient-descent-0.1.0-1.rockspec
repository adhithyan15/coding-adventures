package = "coding-adventures-gradient-descent"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Gradient descent optimiser — SGD training loop with numerical gradient support",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-matrix >= 0.1.0",
    "coding-adventures-loss-functions >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.gradient_descent"] = "src/coding_adventures/gradient_descent/init.lua",
    },
}
