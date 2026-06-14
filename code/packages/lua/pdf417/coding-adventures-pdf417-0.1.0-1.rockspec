package = "coding-adventures-pdf417"
version = "0.1.0-1"

source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}

description = {
    summary = "PDF417 stacked barcode encoder (ISO/IEC 15438:2015) — byte compaction, GF(929) RS ECC",
    homepage = "https://github.com/adhithyan15/coding-adventures",
    license = "MIT",
}

dependencies = {
    "lua >= 5.4",
}

build = {
    type = "builtin",
    modules = {
        ["coding_adventures.pdf417"] =
            "src/coding_adventures/pdf417/init.lua",
        ["coding_adventures.pdf417.cluster_tables"] =
            "src/coding_adventures/pdf417/cluster_tables.lua",
    },
}
