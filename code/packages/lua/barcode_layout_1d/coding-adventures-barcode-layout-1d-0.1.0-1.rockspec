package = "coding-adventures-barcode-layout-1d"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "Pure 1D barcode layout that converts barcode runs into paint scenes",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-paint-instructions == 0.1.0-1",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.barcode_layout_1d"] =
            "src/coding_adventures/barcode_layout_1d/init.lua",
    },
}
