package = "coding-adventures-csv-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "RFC 4180 state-machine CSV parser — handles quoted fields, embedded newlines, and escaped quotes",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.csv_parser"] = "src/coding_adventures/csv_parser/init.lua",
    },
}
