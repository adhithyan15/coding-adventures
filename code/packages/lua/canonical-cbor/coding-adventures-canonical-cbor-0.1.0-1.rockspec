package = "coding-adventures-canonical-cbor"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "RFC 8949 deterministic canonical CBOR codec (CBR01)",
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.canonical_cbor"] = "src/coding_adventures/canonical_cbor/init.lua",
        ["coding_adventures.canonical_cbor.constants"] = "src/coding_adventures/canonical_cbor/constants.lua",
        ["coding_adventures.canonical_cbor.messages"] = "src/coding_adventures/canonical_cbor/messages.lua",
        ["coding_adventures.canonical_cbor.types"] = "src/coding_adventures/canonical_cbor/types.lua",
        ["coding_adventures.canonical_cbor.codec"] = "src/coding_adventures/canonical_cbor/codec.lua",
    },
}
