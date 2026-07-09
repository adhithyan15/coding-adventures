# md2docx

CLI: convert a **Markdown** (or GitHub-Flavored Markdown) file to a real Word
**`.docx`**, natively — the runnable end-goal of the Markdown → Document AST →
OOXML pipeline ([`MD02`](../../../specs/MD02-markdown-to-docx.md)).

The whole path is zero-dependency Rust:

```text
  Markdown → commonmark-parser / gfm-parser → document_ast::DocumentNode
           → document-ast-to-docx → docx-writer → .docx bytes
```

## Usage

```bash
md2docx <in.md> [out.docx]     # convert (default output: <in>.docx)
md2docx --gfm <in.md> [out]    # parse as GitHub-Flavored Markdown (tables, task lists)
md2docx --demo [out.docx]      # convert the built-in sample (default: md2docx-demo.docx)
md2docx --help
```

Example:

```bash
cargo run -p md2docx -- --demo report.docx   # writes a sample .docx you can open in Word
cargo run -p md2docx -- --gfm notes.md        # writes notes.docx
```

`md2docx` is a thin CLI over the [`markdown-docx`](../../../packages/rust/markdown-docx)
library; the conversion core (`convert`, `Dialect`) is a testable library
(`src/lib.rs`) so the CLI wiring is exercised without spawning a process.

## Build & test

```bash
bash BUILD   # cargo test -p md2docx
```
