package = "coding-adventures-progress-bar"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Terminal progress bar for tracking build execution",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.progress_bar"] = "src/coding_adventures/progress_bar/init.lua",
    },
}
