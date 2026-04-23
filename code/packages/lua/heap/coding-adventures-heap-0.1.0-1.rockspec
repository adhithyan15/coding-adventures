package = "coding-adventures-heap"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Comparator-based binary min-heaps and max-heaps",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.heap"] = "src/coding_adventures/heap/init.lua",
    },
}
