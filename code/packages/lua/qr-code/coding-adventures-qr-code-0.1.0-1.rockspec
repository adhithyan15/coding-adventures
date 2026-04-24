package = "coding-adventures-qr-code"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "QR Code encoder (ISO/IEC 18004:2015) — versions 1–40, all ECC levels",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
    "coding-adventures-gf256 == 0.1.0-1",
    "coding-adventures-barcode-2d == 0.1.0-1",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.qr_code"] =
            "src/coding_adventures/qr_code/init.lua",
    },
}
