package = "coding-adventures-immutable-list"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Persistent immutable linked list with structural sharing — cons, head, tail, map, filter, fold",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.immutable_list"] = "src/coding_adventures/immutable_list/init.lua",
    },
}
