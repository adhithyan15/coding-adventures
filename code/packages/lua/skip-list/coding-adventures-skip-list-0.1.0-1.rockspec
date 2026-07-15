package = "coding-adventures-skip-list"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Probabilistic ordered map with rank and range queries",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.skip_list"] = "src/coding_adventures/skip_list/init.lua",
    },
}
