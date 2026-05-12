load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "adjudication-pipeline",
        srcs = ["src/**/*.rs"],
        deps = [
            "//code/packages/rust/adjudication-audit-trail",
            "//code/packages/rust/adjudication-connector",
            "//code/packages/rust/adjudication-coverage",
            "//code/packages/rust/adjudication-ir",
            "//code/packages/rust/adjudication-polarity-modality",
            "//code/packages/rust/logic-engine",
        ],
    ),
]
