package = "coding-adventures-repl"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "A pluggable Read-Eval-Print Loop framework with injectable language, prompt, and waiting plug-ins",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.repl"]                 = "src/coding_adventures/repl/init.lua",
        ["coding_adventures.repl.loop"]            = "src/coding_adventures/repl/loop.lua",
        ["coding_adventures.repl.echo_language"]   = "src/coding_adventures/repl/echo_language.lua",
        ["coding_adventures.repl.default_prompt"]  = "src/coding_adventures/repl/default_prompt.lua",
        ["coding_adventures.repl.silent_waiting"]  = "src/coding_adventures/repl/silent_waiting.lua",
    },
}
