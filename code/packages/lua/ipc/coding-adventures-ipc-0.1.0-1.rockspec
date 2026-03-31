package = "coding-adventures-ipc"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Inter-process communication — pipes, message queues, and shared memory",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.ipc"] = "src/coding_adventures/ipc/init.lua",
    },
}
