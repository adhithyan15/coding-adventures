load("code/packages/starlark/library-rules/rust_library.star", "rust_library")

_targets = [
    rust_library(
        name = "prolog-parser",
        srcs = ["src/**/*.rs"],
        deps = [
            "//code/packages/rust/grammar-tools:grammar-tools",
            "//code/packages/rust/lexer:lexer",
            "//code/packages/rust/parser:parser",
            "//code/packages/rust/prolog-lexer:prolog-lexer",
            "//code/packages/rust/logic-core:logic-core",
        ],
    ),
]
