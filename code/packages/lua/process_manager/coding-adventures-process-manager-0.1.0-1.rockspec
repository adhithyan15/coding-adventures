package = "coding-adventures-process-manager"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Process lifecycle management — fork, exec, wait, signals, and priority scheduling",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.process_manager"] = "src/coding_adventures/process_manager/init.lua",
    },
}
