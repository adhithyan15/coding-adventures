load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "llm-cache",
        srcs = ["src/**/*.rs"],
        deps = [
            "//code/packages/rust/llm-gateway",
            "//code/packages/rust/llm-primitives",
        ],
    ),
]
