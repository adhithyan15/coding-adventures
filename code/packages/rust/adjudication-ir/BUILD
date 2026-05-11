load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "adjudication-ir",
        srcs = ["src/**/*.rs", "tests/**/*.rs"],
        deps = ["//code/packages/rust/logic-core:logic-core"],
    ),
]
