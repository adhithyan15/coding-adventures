//! `md2docx` — the conversion core behind the CLI.
//!
//! A hair-thin wrapper over [`markdown_docx`]: pick the CommonMark or GFM parser,
//! and hand the Markdown to the pipeline. Kept as a library (separate from
//! `main.rs`) so the conversion is unit-testable without spawning a process.

#![forbid(unsafe_code)]

/// Which Markdown dialect to parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    /// Strict CommonMark.
    CommonMark,
    /// GitHub-Flavored Markdown (adds pipe tables, task lists, strikethrough).
    Gfm,
}

/// Convert Markdown `source` to `.docx` bytes using the chosen [`Dialect`].
///
/// Total and panic-free (the pipeline's depth guard bounds adversarial nesting);
/// an empty document yields a valid, empty `.docx`.
pub fn convert(source: &str, dialect: Dialect) -> Vec<u8> {
    match dialect {
        Dialect::CommonMark => markdown_docx::markdown_to_docx(source),
        Dialect::Gfm => markdown_docx::gfm_to_docx(source),
    }
}

/// A built-in sample document for `md2docx --demo`, exercising headings, inline
/// formatting, a list, a code block, a GFM table, and a task list.
pub const SAMPLE_MARKDOWN: &str = "\
# md2docx

A **native** Markdown → `.docx` converter — no pandoc, no external anything.

## Features

- Headings, **bold**, *italic*, and `code`
- Lists, blockquotes, and fenced code blocks
- GFM tables and task lists

> Everything routes through the shared Document AST.

```rust
fn main() { println!(\"hello, docx\"); }
```

| Feature | Supported |
| ------- | --------- |
| Tables  | yes       |
| Tasks   | yes       |

- [x] parse Markdown
- [ ] rule the world
";

#[cfg(test)]
mod tests {
    use super::*;

    /// Both dialects produce a valid `.docx` (ZIP/OPC magic).
    #[test]
    fn convert_produces_a_docx() {
        assert_eq!(&convert("# Hi\n", Dialect::CommonMark)[..2], b"PK");
        assert_eq!(&convert("# Hi\n", Dialect::Gfm)[..2], b"PK");
    }

    /// The GFM dialect recognizes a pipe table that CommonMark would leave as
    /// plain text — a visible behavioural difference between the two paths.
    #[test]
    fn gfm_recognizes_tables() {
        use coding_adventures_wordprocessingml::open_docx;
        let md = "| Name | Qty |\n| ---- | --- |\n| Apple | 3 |\n";
        let gfm = open_docx(&convert(md, Dialect::Gfm)).unwrap();
        assert_eq!(gfm.tables().count(), 1, "GFM parses the pipe table");
        let cm = open_docx(&convert(md, Dialect::CommonMark)).unwrap();
        assert_eq!(cm.tables().count(), 0, "CommonMark leaves it as text");
    }

    /// The bundled sample converts cleanly.
    #[test]
    fn sample_converts() {
        assert_eq!(&convert(SAMPLE_MARKDOWN, Dialect::Gfm)[..2], b"PK");
    }
}
