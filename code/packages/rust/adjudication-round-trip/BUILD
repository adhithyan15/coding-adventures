load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "adjudication-round-trip",
        srcs = ["src/**/*.rs"],
        deps = [
            "//code/packages/rust/adjudication-ir",
            "//code/packages/rust/llm-primitives",
            "//code/packages/rust/logic-core",
        ],
    ),
]
