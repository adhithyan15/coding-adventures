# coding-adventures-python-parser

Parses Python source code using the shared grammar-driven parser and compiled
versioned Python grammars.

## LANG84 release API

`parse_python_at` selects one tagged CPython release by its exact
`code/grammars/python/releases.csv` ID and parses with that release's own
`python<release>.tokens`/`.grammar` pair:

```rust
use coding_adventures_python_parser::{parse_python_at, FrontendBudgets, DEFAULT_RELEASE};

let parsed = parse_python_at("print(-7 // 2)\n", None, &FrontendBudgets::default()).unwrap();
assert_eq!(parsed.release, DEFAULT_RELEASE); // "3.13.16"
assert_eq!(parsed.ast.rule_name, "file");

// Exact IDs only: no fallback to a neighbour or to a minor line.
assert!(parse_python_at("print(1)\n", Some("3.12"), &FrontendBudgets::default()).is_err());
```

Releases with a pair today are 3.0a1, 3.11.17, 3.12.3, 3.13.16 and 3.16.0a1
(LANG84 phase 7a). Their pairs are partial and cover the `py-int-print`
subset. `python-iir-compiler`'s `compile_source_at` is built on this API.
`parse_python(source, "3.12")` and the other minor-line versions are a
separate, unchanged legacy API.

| Path | Input | Releases |
|------|-------|----------|
| `parse_python_at` | exact release ID or `None` | `python<release>.*` pairs |
| `parse_python` | `"2.7"` … `"3.12"` or `""` | legacy minor-line pairs |
