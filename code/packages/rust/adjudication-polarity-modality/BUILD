load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "adjudication-polarity-modality",
        srcs = ["src/**/*.rs"],
        deps = [
            "//code/packages/rust/adjudication-ir:adjudication-ir",
        ],
    ),
]
