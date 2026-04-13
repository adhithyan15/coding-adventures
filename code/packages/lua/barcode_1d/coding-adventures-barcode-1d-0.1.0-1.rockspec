package = "coding-adventures-barcode-1d"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "High-level 1D barcode pipeline for Lua",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-code39 == 0.1.0-1",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.barcode_1d"] =
            "src/coding_adventures/barcode_1d/init.lua",
    },
}
