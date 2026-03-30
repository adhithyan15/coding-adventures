package = "coding-adventures-loss-functions"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Machine learning loss functions — MSE, MAE, BCE, CCE and their derivatives",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.loss_functions"] = "src/coding_adventures/loss_functions/init.lua",
    },
}
