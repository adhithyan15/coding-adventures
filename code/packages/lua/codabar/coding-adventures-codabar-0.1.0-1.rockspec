-- Rockspec for coding-adventures-codabar
-- ======================================

package = "coding-adventures-codabar"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary  = "Codabar barcode encoder — normalize, encode, expand runs, emit paint scenes",
    detailed = [[
        A dependency-free Codabar barcode package. Encodes values with
        start/stop guards into narrow/wide bar patterns, emits alternating
        bar/space run sequences, and lays them out into backend-neutral
        paint scenes.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license  = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-barcode-layout-1d == 0.1.0-1",
}

build = {
    type    = "builtin",
    modules = {
        ["coding_adventures.codabar"] =
            "src/coding_adventures/codabar/init.lua",
    },
}
