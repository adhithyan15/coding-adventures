load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "adjudication-connector",
        srcs = ["src/**/*.rs", "tests/**/*.rs"],
        deps = [
            "//code/packages/rust/adjudication-ir:adjudication-ir",
            "//code/packages/rust/logic-core:logic-core",
            "//code/packages/rust/logic-engine:logic-engine",
        ],
    ),
]
