load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "reed-solomon",
        srcs = ["src/**/*.rs", "tests/**/*.rs"],
        deps = ["gf256"],
    ),
]
