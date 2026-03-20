# ca-capability-analyzer

Static capability analyzer for Rust source code. Detects OS-level capability usage (filesystem, network, process, environment, FFI) by walking the AST with `syn`, and compares detected capabilities against a declared manifest.

## How It Works

The analyzer parses Rust source files into an AST using [`syn`](https://crates.io/crates/syn) and walks the tree with the `Visit` trait. It detects three kinds of patterns:

1. **Use statements** — `use std::fs::File` implies filesystem access
2. **Function calls** — `File::open("data.txt")` reads a specific file
3. **Banned constructs** — `unsafe` blocks, `extern "C"`, `mem::transmute`

Each detection maps to a **capability triple**: `category:action:target` (e.g., `fs:read:data.txt`).

## Usage

```bash
# Detect all capabilities in a file or directory
ca-capability-analyzer detect src/

# Compare against a manifest (CI gate)
ca-capability-analyzer check src/ --manifest required_capabilities.json

# Find banned constructs only
ca-capability-analyzer banned src/
```

## Capability Categories

| Category | Description | Example |
|----------|-------------|---------|
| `fs` | Filesystem access | `File::open`, `fs::write` |
| `net` | Network access | `TcpStream::connect` |
| `proc` | Process execution | `Command::new` |
| `env` | Environment variables | `env::var`, `env::set_var` |
| `ffi` | Foreign function interface | `unsafe`, `extern "C"` |

## How It Fits in the Stack

This is the Rust implementation of the capability analyzer, complementing the Python version at `code/packages/python/ca-capability-analyzer/`. Both implementations detect the same capability categories but target their respective language's AST patterns. The manifest comparison logic (fnmatch-style glob matching, default deny) is identical across implementations.

## Dependencies

- `syn` (v2) — Rust AST parsing with the `Visit` trait
- `serde` + `serde_json` — JSON serialization for manifests and output
