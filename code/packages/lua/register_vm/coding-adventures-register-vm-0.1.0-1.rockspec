package = "coding-adventures-register-vm"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Register-based VM with accumulator model and feedback vectors, modeled after V8's Ignition interpreter",
    license = "MIT",
}
dependencies = {
    "lua >= 5.3",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.register_vm"] = "src/coding_adventures/register_vm/init.lua",
    },
}
