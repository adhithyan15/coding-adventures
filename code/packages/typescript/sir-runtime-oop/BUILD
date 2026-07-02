load("code/packages/starlark/library-rules/typescript_library.star", "ts_library")

_targets = [
    ts_library(
        name = "sir-runtime-oop",
        srcs = ["src/**/*.ts", "tests/**/*.ts"],
        # Transitive file: deps must ALL be listed (npm install does not
        # recursively install file deps' own file deps): core + its pairs and
        # exceptions deps, plus oop's own direct exceptions dep (T2 — fetch/
        # unknown-method raise typed SirErrors through sir-runtime-exceptions).
        deps = [
            "typescript/sir-runtime-core",
            "typescript/sir-runtime-exceptions",
            "typescript/sir-runtime-pairs",
        ],
        test_runner = "vitest",
    ),
]
