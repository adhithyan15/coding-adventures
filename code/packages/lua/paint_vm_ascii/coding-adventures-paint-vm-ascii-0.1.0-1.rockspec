package = "coding-adventures-paint-vm-ascii"
version = "0.1.0-1"

source = {
    url = "https://github.com/adhithyan15/coding-adventures.git",
    tag = "fdb7fc4228d726fe376301d46b4343b0634b3d0b",
}

description = {
    summary = "Terminal backend for backend-neutral paint scenes",
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
        ["coding_adventures.paint_vm_ascii"] =
            "src/coding_adventures/paint_vm_ascii/init.lua",
    },
}
