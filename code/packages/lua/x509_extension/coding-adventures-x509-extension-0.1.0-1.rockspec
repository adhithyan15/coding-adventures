package = "coding-adventures-x509-extension"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Bounded generic X.509 Extension decoding",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-der-asn1 >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.x509_extension"] = "src/coding_adventures/x509_extension/init.lua",
    },
}
