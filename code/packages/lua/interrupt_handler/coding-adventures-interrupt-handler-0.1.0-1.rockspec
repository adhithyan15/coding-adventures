package = "coding-adventures-interrupt-handler"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Hardware interrupt controller and handler — IDT, ISR registry, controller, and interrupt frames",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.interrupt_handler"] = "src/coding_adventures/interrupt_handler/init.lua",
    },
}
