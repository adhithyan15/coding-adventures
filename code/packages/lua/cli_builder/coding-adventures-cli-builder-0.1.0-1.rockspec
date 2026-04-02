-- Rockspec for coding-adventures-cli-builder
-- ============================================

package = "coding-adventures-cli-builder"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Declarative CLI argument parser driven by JSON specs",
    detailed = [[
        A runtime library for declarative CLI argument parsing, driven by a
        JSON specification file. Separates what a CLI accepts from what it
        does. Features: flags (boolean, string, integer, float, enum, count),
        subcommands, positional arguments, help generation, error messaging,
        and flag constraint validation (required, conflicts_with, requires).
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.cli_builder"] =
            "src/coding_adventures/cli_builder/init.lua",
    },
}
