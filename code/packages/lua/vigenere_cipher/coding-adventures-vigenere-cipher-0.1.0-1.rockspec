package = "coding-adventures-vigenere-cipher"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Vigenere cipher -- polyalphabetic substitution cipher with cryptanalysis",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.vigenere_cipher"] = "src/coding_adventures/vigenere_cipher/init.lua",
    },
}
