package = "coding-adventures-note-frequency"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Parse note names like A4 and map them to equal-tempered frequencies",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.note_frequency"] = "src/coding_adventures/note_frequency/init.lua",
    },
}
