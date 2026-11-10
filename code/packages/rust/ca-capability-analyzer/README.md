# ca-capability-analyzer-rs

A zero-third-party-dependency static capability analyzer and deny-by-default name allowlist for Rust packages in the monorepo ([VER08a](../../../specs/VER08a-rust-capability-analyzer.md), Spec 13 Layers 3 & 4).

## What It Does

Every Rust package in this repository declares its operating-system level capabilities in a `required_capabilities.json` manifest. Pure computation packages (such as `rust/ct-compare`, math libraries, codecs, and parsers) declare an empty capability list (`capabilities: []`), establishing a deny-by-default sandbox.

`ca-capability-analyzer-rs` verifies that:
1. Every ambient OS access in `.rs` files matches a declared capability (`CAP001` check);
2. All banned constructs (`unsafe` blocks/functions, inline assembly, uninspected foreign linkage) have matching hardware-key exceptions (`CAP002` check);
3. Manifests conform to `required_capabilities.schema.json` version 1;
4. Traversal adheres to strict resource quotas and refuses symlinked files;
5. Sealed capability modules (`src/capabilities.rs`) enforce "you cannot name what you did not declare" (VER23);
6. Canonical effect proof certificates (`effect_certificate.json`, schema `ver24-effect-cert-v1`) verify proof-carrying capability bounds via an independent small checker (VER24).

```text
  Package Rust Source Files
             │
             ▼
   ca-capability-analyzer-rs
             │
             ├── Traverses package source recursively (max 10,000 files, refuses symlinks)
             ├── Detects raw stdlib calls → std::fs, std::net, std::process, std::env, ...
             ├── Detects banned constructs → unsafe blocks, asm!, #[no_mangle], ...
             │
             └── Compares against required_capabilities.json
                    │
                    ├── Detected ⊆ Declared (and all banned constructs exempt) → EXIT 0 (pass)
                    └── Detected ⊄ Declared (or unexempt banned construct)   → EXIT 1 (violations)
```

## CLI Usage

```bash
# Analyze a specific package directory
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare

# Verbose mode: display all detected capabilities and inspected files
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --verbose

# Generate sealed capabilities module (VER23)
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --generate-module

# Verify capabilities module is up-to-date with manifest
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --check-module

# Emit canonical effect proof certificate (VER24)
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --emit-cert code/packages/rust/ct-compare/effect_certificate.json

# Verify effect certificate using the small independent checker
cargo run -p coding_adventures_ca_capability_analyzer -- --dir code/packages/rust/ct-compare --verify-cert code/packages/rust/ct-compare/effect_certificate.json
```

## Exit Codes

| Code | Meaning |
|---|---|
| `0` | All detected capabilities are declared; no unexempt banned constructs |
| `1` | One or more violations found (`CAP001` or `CAP002`) |
| `2` | Input error, malformed manifest JSON, resource ceiling exceeded, or parse error |
