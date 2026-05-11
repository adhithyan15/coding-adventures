load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "cas-algebraic",
        srcs = ["src/**/*.rs", "tests/**/*.rs"],
        deps = ["cas-factor", "symbolic-ir"],
    ),
]
