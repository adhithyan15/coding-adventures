load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "adjudication-contract-demo",
        srcs = ["src/**/*.rs"],
        deps = [
            "//code/packages/rust/adjudication-audit-trail",
            "//code/packages/rust/adjudication-clarification",
            "//code/packages/rust/adjudication-ir",
            "//code/packages/rust/adjudication-pipeline",
            "//code/packages/rust/llm-cache",
            "//code/packages/rust/llm-gateway",
            "//code/packages/rust/llm-primitives",
            "//code/packages/rust/llm-provider-ollama",
            "//code/packages/rust/logic-core",
        ],
    ),
]
