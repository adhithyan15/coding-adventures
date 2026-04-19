package = "coding-adventures-note-frequency"
version = "0.1.0-1"
source = {
    url = "https://github.com/adhithyan15/coding-adventures.git",
    tag = "131cb3cacda192b556bd913281e5a37cbf4f99ff",
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
