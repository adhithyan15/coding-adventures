load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "cas-simplify",
        srcs = ["src/**/*.rs", "tests/**/*.rs"],
        deps = ["symbolic-ir", "cas-pattern-matching"],
    ),
]
