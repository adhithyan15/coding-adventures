package = "coding-adventures-paint-instructions"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "Backend-neutral paint scene primitives",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.paint_instructions"] =
            "src/coding_adventures/paint_instructions/init.lua",
    },
}
