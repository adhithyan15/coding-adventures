## Unreleased — route the RV32I board fixture precisely

Extend the TypeScript architecture simulator's exact repository-input and
fallback-cache row with `os-machine-v1/riscv.json` and its external verifier.
The package remains the sole consumer; nearby fixture files do not select it.
