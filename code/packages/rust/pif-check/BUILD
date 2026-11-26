load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "pif-check",
        srcs = ["src/**/*.rs", "tests/**/*.rs"],
        deps = ["lean-kernel", "pif-writer", "pif-prelude", "sha256"],
    ),
]
