package = "coding-adventures-barcode-2d"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "Shared 2D barcode abstraction: ModuleGrid and layout() for the PaintVM pipeline",
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
        ["coding_adventures.barcode_2d"] =
            "src/coding_adventures/barcode_2d/init.lua",
    },
}
