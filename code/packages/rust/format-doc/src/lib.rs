//! # `format-doc` — Wadler-style document algebra for pretty-printers.
//!
//! Rust port of [P2D03](../../specs/P2D03-format-doc.md) (the
//! TypeScript `@coding-adventures/format-doc`).  The semantic IR
//! every formatter builds: language-specific formatters compile
//! AST → [`Doc`], this crate realises [`Doc`] → [`DocLayoutTree`].
//!
//! ## Architecture
//!
//! ```text
//! AST + trivia + formatter rules
//!   → Doc                         ← stable semantic IR
//!   → DocLayoutTree               ← first realised layout form  (this crate)
//!   → PaintScene                  ← first concrete rendering scene  (format-doc-to-paint, future)
//!   → paint-vm-ascii              ← terminal string  (downstream)
//! ```
//!
//! Decoupling means the same `Doc` builds drive ASCII output today,
//! canvas / SVG / editor-native paint pipelines tomorrow, all without
//! the formatter author re-emitting strings.
//!
//! ## Doc primitives
//!
//! | Primitive          | Meaning                                                      |
//! |--------------------|--------------------------------------------------------------|
//! | [`text`]           | Emit literal text                                            |
//! | [`concat`]         | Emit child docs in sequence                                  |
//! | [`group`]          | Try to print flat; if it doesn't fit, print broken           |
//! | [`indent`]         | Increase indentation for broken lines inside the wrapped doc |
//! | [`line`]           | Space when flat, newline when broken                         |
//! | [`softline`]       | Empty when flat, newline when broken                         |
//! | [`hardline`]       | Always newline                                               |
//! | [`if_break`]       | Emit `broken` in broken mode, otherwise `flat`               |
//! | [`annotate`]       | Attach metadata to emitted spans without changing layout     |
//! | [`nil`]            | Empty document; neutral element of `concat`                  |
//! | [`join`]           | Convenience: join a list of docs by a separator              |
//!
//! ## Realisation
//!
//! [`layout_doc`] walks the doc with a stack of commands, decides
//! whether each [`group`] fits on the current line via a look-ahead
//! `fits`-style simulation, then emits positioned text spans
//! arranged into [`DocLayoutLine`]s in monospace cell units.
//!
//! When you don't need the full layout tree (e.g. you only want a
//! plain-text dump), [`render_text`] flattens a [`DocLayoutTree`]
//! into a single string with newlines and indent prefixes.
//!
//! ## Example
//!
//! ```rust
//! use format_doc::{
//!     concat, group, indent, layout_doc, line, render_text, softline,
//!     text, LayoutOptions,
//! };
//!
//! let doc = group(concat([
//!     text("foo("),
//!     indent(concat([
//!         softline(),
//!         text("bar,"),
//!         line(),
//!         text("baz"),
//!     ]), 1),
//!     softline(),
//!     text(")"),
//! ]));
//!
//! let narrow = layout_doc(doc.clone(), &LayoutOptions { print_width: 8, ..Default::default() });
//! assert_eq!(render_text(&narrow), "foo(\n  bar,\n  baz\n)");
//!
//! let wide = layout_doc(doc, &LayoutOptions { print_width: 80, ..Default::default() });
//! assert_eq!(render_text(&wide), "foo(bar, baz)");
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------

/// Package version.  Kept in source for cross-package smoke tests.
pub const VERSION: &str = "0.1.0";

/// Default indent width (columns added per indent level).
pub const DEFAULT_INDENT_WIDTH: usize = 2;

/// Default line height (rows added per logical line).
pub const DEFAULT_LINE_HEIGHT: usize = 1;

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

/// Metadata attached to emitted spans by [`annotate`].
///
/// V1 keeps this open-ended at the *cost* of a small, bounded enum
/// (matches the TypeScript `string | number | boolean | null` shape).
/// Future formatter packages can attach node ids, token classes, or
/// source spans without changing the core algebra.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DocAnnotation {
    /// String annotation (token kind, semantic-token name, etc.).
    Str(String),
    /// Integer annotation (node id, source offset, etc.).
    Int(i64),
    /// Boolean annotation (toggle metadata).
    Bool(bool),
    /// Null / absent annotation (used as a deliberate placeholder).
    Null,
}

// ---------------------------------------------------------------------------
// Doc
// ---------------------------------------------------------------------------

/// Line-break semantics — see [`line`] / [`softline`] / [`hardline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineMode {
    /// Empty when flat, newline when broken.
    Soft,
    /// Single space when flat, newline when broken.
    Normal,
    /// Always a newline (forces the enclosing group to break).
    Hard,
}

/// A backend-neutral pretty-printing document.
///
/// Cheap-to-clone via the internal [`Arc`] sharing inside
/// [`Doc::Concat`] / [`Doc::Group`] / [`Doc::Indent`] / etc.
/// Compose freely; the realisation pass does the work.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Doc {
    /// The empty document — neutral element of [`concat`].
    Nil,
    /// Literal text.  No newlines allowed inside (use [`hardline`]).
    Text(Arc<str>),
    /// Sequence of child docs, emitted in order.
    Concat(Arc<[Doc]>),
    /// Mark a subtree as a unit that should stay flat if it fits.
    Group(Arc<Doc>),
    /// Increase indentation by `levels` for broken lines inside `content`.
    Indent {
        /// Indent levels added on top of the surrounding context.
        levels: usize,
        /// Wrapped content.
        content: Arc<Doc>,
    },
    /// A line break with mode-dependent flat / broken behaviour —
    /// see [`LineMode`].
    Line(LineMode),
    /// Emit `broken` in broken mode, otherwise `flat`.
    IfBreak {
        /// Doc emitted in broken mode.
        broken: Arc<Doc>,
        /// Doc emitted in flat mode.
        flat: Arc<Doc>,
    },
    /// Attach metadata to the spans emitted by `content`.
    Annotate {
        /// Annotation pinned onto every emitted span inside.
        annotation: DocAnnotation,
        /// Wrapped content.
        content: Arc<Doc>,
    },
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

/// Return the empty document.  Useful as the neutral element of
/// [`concat`].
pub fn nil() -> Doc {
    Doc::Nil
}

/// Wrap literal text.  Empty strings collapse to [`nil`] to keep
/// doc trees tidy.
///
/// Embedded `\n` characters are auto-split into a sequence of
/// [`text`] / [`hardline`] / [`text`] / … so spans always remain
/// single-line — downstream backends that assume monospace cells
/// (`paint-vm-ascii`, canvas, SVG) require this invariant.
/// `\r` / `\r\n` line endings are normalised to `\n`.
pub fn text<S: Into<String>>(value: S) -> Doc {
    let s = value.into();
    if s.is_empty() {
        return Doc::Nil;
    }
    if !s.contains('\n') && !s.contains('\r') {
        return Doc::Text(Arc::from(s.as_str()));
    }
    // Normalise CRLF / CR to LF, then split.  Any embedded line
    // break becomes a hardline; adjacent text fragments stay text.
    let normalised = s.replace("\r\n", "\n").replace('\r', "\n");
    let mut parts: Vec<Doc> = Vec::new();
    let mut first = true;
    for piece in normalised.split('\n') {
        if !first {
            parts.push(hardline());
        }
        first = false;
        if !piece.is_empty() {
            parts.push(Doc::Text(Arc::from(piece)));
        }
    }
    concat(parts)
}

/// Concatenate docs in order.
///
/// Flattens nested concats and drops [`Doc::Nil`] children so
/// printers can compose without producing deeply-nested noise.
pub fn concat<I>(parts: I) -> Doc
where
    I: IntoIterator<Item = Doc>,
{
    let mut flat: Vec<Doc> = Vec::new();
    for part in parts {
        match part {
            Doc::Nil => {}
            Doc::Concat(inner) => {
                for nested in inner.iter() {
                    if !matches!(nested, Doc::Nil) {
                        flat.push(nested.clone());
                    }
                }
            }
            other => flat.push(other),
        }
    }
    match flat.len() {
        0 => Doc::Nil,
        1 => flat.into_iter().next().unwrap(),
        _ => Doc::Concat(Arc::from(flat)),
    }
}

/// Join docs with a separator document.  Returns [`nil`] for
/// empty input.
pub fn join<I>(separator: Doc, parts: I) -> Doc
where
    I: IntoIterator<Item = Doc>,
{
    let parts: Vec<Doc> = parts.into_iter().collect();
    if parts.is_empty() {
        return Doc::Nil;
    }
    let mut out: Vec<Doc> = Vec::with_capacity(parts.len() * 2);
    for (i, p) in parts.into_iter().enumerate() {
        if i > 0 {
            out.push(separator.clone());
        }
        out.push(p);
    }
    concat(out)
}

/// Mark a subtree as a unit that should stay flat if it fits.
pub fn group(content: Doc) -> Doc {
    Doc::Group(Arc::new(content))
}

/// Increase indentation by `levels` for broken lines inside
/// `content`.  Pass `0` to be a no-op.
pub fn indent(content: Doc, levels: usize) -> Doc {
    if levels == 0 {
        content
    } else {
        Doc::Indent { levels, content: Arc::new(content) }
    }
}

/// Emit a single space when flat, a newline + indentation when
/// broken.
pub fn line() -> Doc {
    Doc::Line(LineMode::Normal)
}

/// Emit nothing when flat, a newline + indentation when broken.
pub fn softline() -> Doc {
    Doc::Line(LineMode::Soft)
}

/// Always emit a newline + indentation, forcing the enclosing
/// group into broken mode.
pub fn hardline() -> Doc {
    Doc::Line(LineMode::Hard)
}

/// Emit `broken` in broken mode and `flat` in flat mode.
///
/// The two-arg form covers most uses; pass [`nil`] for `flat`
/// when you want "this only when broken."
pub fn if_break(broken: Doc, flat: Doc) -> Doc {
    Doc::IfBreak { broken: Arc::new(broken), flat: Arc::new(flat) }
}

/// Attach `annotation` to every span emitted inside `content`.
/// Layout decisions are not affected — annotations are pure
/// metadata.
pub fn annotate(annotation: DocAnnotation, content: Doc) -> Doc {
    Doc::Annotate { annotation, content: Arc::new(content) }
}

// ---------------------------------------------------------------------------
// Layout types
// ---------------------------------------------------------------------------

/// One realised span of text on a [`DocLayoutLine`], annotated
/// with the metadata active at emission time.
#[derive(Debug, Clone, PartialEq)]
pub struct DocLayoutSpan {
    /// 0-based monospace-cell column where the span starts.
    pub column: usize,
    /// The text content (no newlines).
    pub text: String,
    /// Annotations from every enclosing [`annotate`], outer-first.
    pub annotations: Vec<DocAnnotation>,
}

/// One realised line in the layout tree.
#[derive(Debug, Clone, PartialEq)]
pub struct DocLayoutLine {
    /// 0-based row index.
    pub row: usize,
    /// Indent in monospace cell columns.  All spans on this line
    /// start at or after `indent_columns`.
    pub indent_columns: usize,
    /// Total width of the line in cell columns (last span's right edge).
    pub width: usize,
    /// Spans in left-to-right order.  Empty for blank lines.
    pub spans: Vec<DocLayoutSpan>,
}

/// The backend-neutral result of realising a [`Doc`].
#[derive(Debug, Clone, PartialEq)]
pub struct DocLayoutTree {
    /// Width budget that was used.
    pub print_width: usize,
    /// Indent step (columns per indent level).
    pub indent_width: usize,
    /// Line height (rows per logical line).
    pub line_height: usize,
    /// Maximum column reached across all lines.
    pub width: usize,
    /// `lines.len() * line_height`.
    pub height: usize,
    /// Lines in order.
    pub lines: Vec<DocLayoutLine>,
}

/// Configuration for the realisation pass — see [`layout_doc`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutOptions {
    /// Width budget; groups try to fit within this column count.
    /// Must be `> 0`.
    pub print_width: usize,
    /// Columns per indent level.  Defaults to [`DEFAULT_INDENT_WIDTH`].
    pub indent_width: usize,
    /// Rows per logical line.  Defaults to [`DEFAULT_LINE_HEIGHT`].
    pub line_height: usize,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            print_width: 80,
            indent_width: DEFAULT_INDENT_WIDTH,
            line_height: DEFAULT_LINE_HEIGHT,
        }
    }
}

// ---------------------------------------------------------------------------
// Realisation interpreter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

#[derive(Debug, Clone)]
struct Command {
    indent_levels: usize,
    mode: Mode,
    annotations: Vec<DocAnnotation>,
    doc: Doc,
}

/// Realise a [`Doc`] into a [`DocLayoutTree`].
///
/// # Panics
///
/// Panics if `options.print_width == 0`.  All other inputs are
/// accepted; a sufficiently-narrow width may produce a tree where
/// individual lines exceed `print_width` (atoms that don't fit
/// can't be broken — by design, no layout is "wrong," just
/// over-budget).
pub fn layout_doc(doc: Doc, options: &LayoutOptions) -> DocLayoutTree {
    assert!(options.print_width > 0, "layout_doc requires print_width > 0");
    let indent_width = options.indent_width;
    let line_height = options.line_height;

    let mut lines: Vec<MutableLine> = vec![MutableLine { row: 0, indent_columns: 0, spans: Vec::new() }];
    let mut current_idx = 0usize;
    let mut column: usize = 0;
    let mut max_column: usize = 0;

    let mut stack: Vec<Command> = vec![Command {
        indent_levels: 0,
        mode: Mode::Break,
        annotations: Vec::new(),
        doc,
    }];

    while let Some(cmd) = stack.pop() {
        match &cmd.doc {
            Doc::Nil => {}
            Doc::Text(value) => {
                push_text(&mut lines, current_idx, &mut column, &mut max_column, value, &cmd.annotations);
            }
            Doc::Concat(parts) => {
                push_docs(&mut stack, &cmd, parts);
            }
            Doc::Group(content) => {
                let child = (**content).clone();
                let pick_flat = cmd.mode == Mode::Flat
                    || fits(
                        options.print_width.saturating_sub(column),
                        &stack,
                        &Command {
                            indent_levels: cmd.indent_levels,
                            mode: Mode::Flat,
                            annotations: cmd.annotations.clone(),
                            doc: child.clone(),
                        },
                    );
                stack.push(Command {
                    indent_levels: cmd.indent_levels,
                    mode: if pick_flat { Mode::Flat } else { Mode::Break },
                    annotations: cmd.annotations.clone(),
                    doc: child,
                });
            }
            Doc::Indent { levels, content } => {
                stack.push(Command {
                    indent_levels: cmd.indent_levels + levels,
                    mode: cmd.mode,
                    annotations: cmd.annotations.clone(),
                    doc: (**content).clone(),
                });
            }
            Doc::Line(line_mode) => {
                match (line_mode, cmd.mode) {
                    (LineMode::Hard, _) => {
                        push_line_break(
                            &mut lines,
                            &mut current_idx,
                            &mut column,
                            &mut max_column,
                            cmd.indent_levels,
                            indent_width,
                        );
                    }
                    (LineMode::Normal, Mode::Flat) => {
                        push_text(
                            &mut lines,
                            current_idx,
                            &mut column,
                            &mut max_column,
                            " ",
                            &cmd.annotations,
                        );
                    }
                    (LineMode::Soft, Mode::Flat) => {
                        // emit nothing
                    }
                    (LineMode::Normal | LineMode::Soft, Mode::Break) => {
                        push_line_break(
                            &mut lines,
                            &mut current_idx,
                            &mut column,
                            &mut max_column,
                            cmd.indent_levels,
                            indent_width,
                        );
                    }
                }
            }
            Doc::IfBreak { broken, flat } => {
                let chosen = match cmd.mode {
                    Mode::Flat => (**flat).clone(),
                    Mode::Break => (**broken).clone(),
                };
                stack.push(Command {
                    indent_levels: cmd.indent_levels,
                    mode: cmd.mode,
                    annotations: cmd.annotations.clone(),
                    doc: chosen,
                });
            }
            Doc::Annotate { annotation, content } => {
                let mut anns = cmd.annotations.clone();
                anns.push(annotation.clone());
                stack.push(Command {
                    indent_levels: cmd.indent_levels,
                    mode: cmd.mode,
                    annotations: anns,
                    doc: (**content).clone(),
                });
            }
        }
    }

    let lines_out: Vec<DocLayoutLine> = lines
        .into_iter()
        .map(|l| {
            let width = if let Some(last) = l.spans.last() {
                last.column + visible_width(&last.text)
            } else {
                l.indent_columns
            };
            DocLayoutLine {
                row: l.row,
                indent_columns: l.indent_columns,
                width,
                spans: l.spans,
            }
        })
        .collect();

    let height = lines_out.len() * line_height;
    DocLayoutTree {
        print_width: options.print_width,
        indent_width,
        line_height,
        width: max_column,
        height,
        lines: lines_out,
    }
}

#[derive(Debug, Clone)]
struct MutableLine {
    row: usize,
    indent_columns: usize,
    spans: Vec<DocLayoutSpan>,
}

fn push_text(
    lines: &mut [MutableLine],
    current: usize,
    column: &mut usize,
    max_column: &mut usize,
    value: &str,
    annotations: &[DocAnnotation],
) {
    if value.is_empty() {
        return;
    }
    let line = &mut lines[current];
    if let Some(last) = line.spans.last_mut() {
        if same_annotations(&last.annotations, annotations)
            && last.column + visible_width(&last.text) == *column
        {
            last.text.push_str(value);
            *column += visible_width(value);
            if *column > *max_column {
                *max_column = *column;
            }
            return;
        }
    }
    line.spans.push(DocLayoutSpan {
        column: *column,
        text: value.to_owned(),
        annotations: annotations.to_vec(),
    });
    *column += visible_width(value);
    if *column > *max_column {
        *max_column = *column;
    }
}

fn push_line_break(
    lines: &mut Vec<MutableLine>,
    current: &mut usize,
    column: &mut usize,
    max_column: &mut usize,
    indent_levels: usize,
    indent_width: usize,
) {
    let indent_columns = indent_levels * indent_width;
    let new_row = lines.len();
    lines.push(MutableLine { row: new_row, indent_columns, spans: Vec::new() });
    *current = new_row;
    *column = indent_columns;
    if *column > *max_column {
        *max_column = *column;
    }
}

fn push_docs(stack: &mut Vec<Command>, base: &Command, docs: &Arc<[Doc]>) {
    for doc in docs.iter().rev() {
        stack.push(Command {
            indent_levels: base.indent_levels,
            mode: base.mode,
            annotations: base.annotations.clone(),
            doc: doc.clone(),
        });
    }
}

fn same_annotations(left: &[DocAnnotation], right: &[DocAnnotation]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right.iter()).all(|(a, b)| a == b)
}

/// Look ahead to see whether the pending document stack can stay
/// on the current line if we continue in flat mode.
///
/// **Memory:** the parent `stack` is borrowed, never cloned —
/// only the small `pending` Vec of children we descend into
/// during this call grows.  This keeps look-ahead at `O(work
/// done)` per call rather than `O(stack depth)`, which matters
/// for adversarial inputs with deeply-nested groups.
fn fits(remaining: usize, stack: &[Command], next: &Command) -> bool {
    let mut budget: i64 = remaining as i64;
    let mut pending: Vec<Command> = Vec::new();
    pending.push(next.clone());
    let mut stack_idx = stack.len();

    while budget >= 0 {
        let cmd = match pending.pop() {
            Some(c) => c,
            None => {
                // No more local descent — walk one step backwards
                // through the parent stack instead of cloning it.
                if stack_idx == 0 {
                    return true;
                }
                stack_idx -= 1;
                stack[stack_idx].clone()
            }
        };

        match &cmd.doc {
            Doc::Nil => {}
            Doc::Text(value) => {
                budget -= visible_width(value) as i64;
            }
            Doc::Concat(parts) => {
                push_docs(&mut pending, &cmd, parts);
            }
            Doc::Group(content) => {
                pending.push(Command {
                    indent_levels: cmd.indent_levels,
                    mode: Mode::Flat,
                    annotations: cmd.annotations.clone(),
                    doc: (**content).clone(),
                });
            }
            Doc::Indent { levels, content } => {
                pending.push(Command {
                    indent_levels: cmd.indent_levels + levels,
                    mode: cmd.mode,
                    annotations: cmd.annotations.clone(),
                    doc: (**content).clone(),
                });
            }
            Doc::Line(line_mode) => match (line_mode, cmd.mode) {
                (LineMode::Hard, _) => return false,
                (LineMode::Normal, Mode::Flat) => budget -= 1,
                (LineMode::Soft, Mode::Flat) => {}
                (_, Mode::Break) => return true,
            },
            Doc::IfBreak { broken, flat } => {
                let chosen = match cmd.mode {
                    Mode::Flat => (**flat).clone(),
                    Mode::Break => (**broken).clone(),
                };
                pending.push(Command {
                    indent_levels: cmd.indent_levels,
                    mode: cmd.mode,
                    annotations: cmd.annotations.clone(),
                    doc: chosen,
                });
            }
            Doc::Annotate { content, .. } => {
                pending.push(Command {
                    indent_levels: cmd.indent_levels,
                    mode: cmd.mode,
                    annotations: cmd.annotations.clone(),
                    doc: (**content).clone(),
                });
            }
        }
    }
    false
}

fn visible_width(s: &str) -> usize {
    s.chars().count()
}

// ---------------------------------------------------------------------------
// Render helpers
// ---------------------------------------------------------------------------

/// Flatten a [`DocLayoutTree`] into a plain text dump — useful for
/// formatters that just want a `String`.  Each line is
/// `<indent spaces><spans concatenated>`; lines are joined by `\n`.
/// No trailing newline.
pub fn render_text(layout: &DocLayoutTree) -> String {
    let mut out = String::new();
    for (i, line) in layout.lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for _ in 0..line.indent_columns {
            out.push(' ');
        }
        let mut col = line.indent_columns;
        for span in &line.spans {
            while col < span.column {
                out.push(' ');
                col += 1;
            }
            out.push_str(&span.text);
            col += visible_width(&span.text);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(width: usize) -> LayoutOptions {
        LayoutOptions { print_width: width, ..Default::default() }
    }

    fn render(doc: Doc, width: usize) -> String {
        render_text(&layout_doc(doc, &opts(width)))
    }

    #[test]
    fn nil_renders_empty() {
        assert_eq!(render(nil(), 80), "");
    }

    #[test]
    fn text_renders_literal() {
        assert_eq!(render(text("hello"), 80), "hello");
    }

    #[test]
    fn text_empty_collapses_to_nil() {
        assert!(matches!(text(""), Doc::Nil));
    }

    #[test]
    fn concat_flattens_nested() {
        let d = concat([text("a"), concat([text("b"), text("c")]), text("d")]);
        if let Doc::Concat(parts) = &d {
            assert_eq!(parts.len(), 4);
        } else {
            panic!("expected Concat, got {d:?}");
        }
    }

    #[test]
    fn concat_drops_nil() {
        let d = concat([text("a"), nil(), text("b"), nil()]);
        if let Doc::Concat(parts) = &d {
            assert_eq!(parts.len(), 2);
        } else {
            panic!("expected Concat");
        }
    }

    #[test]
    fn concat_singleton_unwraps() {
        let d = concat([text("only")]);
        assert!(matches!(d, Doc::Text(_)));
    }

    #[test]
    fn concat_empty_is_nil() {
        let d = concat(std::iter::empty());
        assert!(matches!(d, Doc::Nil));
    }

    #[test]
    fn join_basic() {
        let s = render(join(text(", "), [text("a"), text("b"), text("c")]), 80);
        assert_eq!(s, "a, b, c");
    }

    #[test]
    fn join_empty_is_nil() {
        let d = join(text(", "), std::iter::empty());
        assert!(matches!(d, Doc::Nil));
    }

    #[test]
    fn join_singleton_no_separator() {
        let s = render(join(text(", "), [text("only")]), 80);
        assert_eq!(s, "only");
    }

    #[test]
    fn indent_zero_levels_is_noop() {
        let d = text("x");
        let i = indent(d.clone(), 0);
        assert!(matches!(i, Doc::Text(ref s) if s.as_ref() == "x"));
    }

    #[test]
    fn line_in_flat_mode_is_space() {
        let doc = group(concat([text("a"), line(), text("b")]));
        assert_eq!(render(doc, 80), "a b");
    }

    #[test]
    fn line_in_broken_mode_is_newline_with_indent() {
        let doc = group(concat([text("aaaaa"), line(), text("bbbbb")]));
        assert_eq!(render(doc, 5), "aaaaa\nbbbbb");
    }

    #[test]
    fn softline_in_flat_mode_is_empty() {
        let doc = group(concat([text("a"), softline(), text("b")]));
        assert_eq!(render(doc, 80), "ab");
    }

    #[test]
    fn softline_in_broken_mode_is_newline() {
        let doc = group(concat([text("aaaaa"), softline(), text("bbbbb")]));
        assert_eq!(render(doc, 5), "aaaaa\nbbbbb");
    }

    #[test]
    fn hardline_always_breaks() {
        let doc = group(concat([text("a"), hardline(), text("b")]));
        assert_eq!(render(doc, 80), "a\nb");
    }

    #[test]
    fn group_flat_when_fits() {
        let doc = group(concat([text("("), softline(), text("x"), softline(), text(")")]));
        assert_eq!(render(doc, 80), "(x)");
    }

    #[test]
    fn group_broken_when_does_not_fit() {
        let doc = group(concat([
            text("foo("),
            indent(concat([softline(), text("bar,"), line(), text("baz")]), 1),
            softline(),
            text(")"),
        ]));
        assert_eq!(render(doc, 8), "foo(\n  bar,\n  baz\n)");
    }

    #[test]
    fn group_broken_uses_indent_width_setting() {
        let doc = group(concat([text("a"), indent(concat([hardline(), text("b")]), 2)]));
        let layout = layout_doc(doc, &LayoutOptions { print_width: 80, indent_width: 4, ..Default::default() });
        assert_eq!(render_text(&layout), "a\n        b");
    }

    #[test]
    fn if_break_picks_flat_when_flat() {
        let doc = group(concat([text("a"), if_break(text("BROKEN"), text("FLAT")), text("b")]));
        assert_eq!(render(doc, 80), "aFLATb");
    }

    #[test]
    fn if_break_picks_broken_when_broken() {
        let doc = group(concat([
            text("aaaaa"),
            line(),
            if_break(text("BROKEN"), text("FLAT")),
        ]));
        assert_eq!(render(doc, 5), "aaaaa\nBROKEN");
    }

    #[test]
    fn annotations_attach_to_emitted_spans() {
        let ann = DocAnnotation::Str("kw".into());
        let doc = annotate(ann.clone(), text("if"));
        let layout = layout_doc(doc, &opts(80));
        assert_eq!(layout.lines[0].spans[0].annotations, vec![ann]);
    }

    #[test]
    fn nested_annotations_accumulate_outer_first() {
        let outer = DocAnnotation::Str("statement".into());
        let inner = DocAnnotation::Str("keyword".into());
        let doc = annotate(outer.clone(), annotate(inner.clone(), text("if")));
        let layout = layout_doc(doc, &opts(80));
        assert_eq!(layout.lines[0].spans[0].annotations, vec![outer, inner]);
    }

    #[test]
    fn annotations_do_not_change_layout() {
        let plain = render(text("hi there"), 80);
        let annotated = render(annotate(DocAnnotation::Bool(true), text("hi there")), 80);
        assert_eq!(plain, annotated);
    }

    #[test]
    fn span_coalescing_merges_only_when_annotations_match() {
        let same = concat([
            annotate(DocAnnotation::Int(1), text("foo")),
            annotate(DocAnnotation::Int(1), text("bar")),
        ]);
        let layout = layout_doc(same, &opts(80));
        assert_eq!(layout.lines[0].spans.len(), 1);
        assert_eq!(layout.lines[0].spans[0].text, "foobar");

        let diff = concat([
            annotate(DocAnnotation::Int(1), text("foo")),
            annotate(DocAnnotation::Int(2), text("bar")),
        ]);
        let layout = layout_doc(diff, &opts(80));
        assert_eq!(layout.lines[0].spans.len(), 2);
    }

    #[test]
    fn layout_tree_records_print_width_and_dimensions() {
        let layout = layout_doc(text("abc"), &opts(80));
        assert_eq!(layout.print_width, 80);
        assert_eq!(layout.indent_width, DEFAULT_INDENT_WIDTH);
        assert_eq!(layout.line_height, DEFAULT_LINE_HEIGHT);
        assert_eq!(layout.width, 3);
        assert_eq!(layout.height, 1);
    }

    #[test]
    fn line_height_multiplies_height() {
        let layout = layout_doc(
            concat([text("a"), hardline(), text("b")]),
            &LayoutOptions { print_width: 80, indent_width: 2, line_height: 3 },
        );
        assert_eq!(layout.height, 6);
    }

    #[test]
    fn lines_have_correct_widths() {
        let layout = layout_doc(
            concat([text("hello"), hardline(), text("world!")]),
            &opts(80),
        );
        assert_eq!(layout.lines[0].width, 5);
        assert_eq!(layout.lines[1].width, 6);
        assert_eq!(layout.width, 6);
    }

    #[test]
    fn render_text_handles_indented_lines() {
        let layout = layout_doc(
            concat([text("a"), indent(concat([hardline(), text("b")]), 1)]),
            &opts(80),
        );
        assert_eq!(render_text(&layout), "a\n  b");
    }

    #[test]
    fn render_text_handles_blank_lines() {
        let layout = layout_doc(concat([hardline(), hardline()]), &opts(80));
        assert_eq!(render_text(&layout), "\n\n");
    }

    #[test]
    fn fits_succeeds_when_content_fits() {
        let doc = group(concat([text("a"), line(), text("b")]));
        assert_eq!(render(doc, 80), "a b");
    }

    #[test]
    fn fits_fails_on_hardline_inside_group() {
        let doc = group(concat([text("a"), hardline(), text("b")]));
        assert_eq!(render(doc, 1000), "a\nb");
    }

    #[test]
    fn spec_example_narrow_breaks() {
        let doc = group(concat([
            text("foo("),
            indent(
                concat([softline(), text("bar,"), line(), text("baz")]),
                1,
            ),
            softline(),
            text(")"),
        ]));
        let narrow = render_text(&layout_doc(doc.clone(), &opts(8)));
        assert_eq!(narrow, "foo(\n  bar,\n  baz\n)");
        let wide = render_text(&layout_doc(doc, &opts(80)));
        assert_eq!(wide, "foo(bar, baz)");
    }

    #[test]
    #[should_panic(expected = "print_width > 0")]
    fn zero_width_panics() {
        let _ = layout_doc(text("x"), &LayoutOptions { print_width: 0, ..Default::default() });
    }

    // ---------- Hardening (security review) ----------

    #[test]
    fn text_with_lf_auto_splits_on_hardline() {
        // text("a\nb") becomes concat([Text("a"), hardline, Text("b")])
        // — spans must remain single-line.
        let layout = layout_doc(text("a\nb"), &opts(80));
        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.lines[0].spans[0].text, "a");
        assert!(!layout.lines[0].spans[0].text.contains('\n'));
        assert_eq!(layout.lines[1].spans[0].text, "b");
    }

    #[test]
    fn text_normalises_crlf_and_cr() {
        let layout = layout_doc(text("a\r\nb\rc"), &opts(80));
        assert_eq!(layout.lines.len(), 3);
        assert_eq!(layout.lines[0].spans[0].text, "a");
        assert_eq!(layout.lines[1].spans[0].text, "b");
        assert_eq!(layout.lines[2].spans[0].text, "c");
    }

    #[test]
    fn text_with_only_newlines_produces_blank_lines() {
        let layout = layout_doc(text("\n\n"), &opts(80));
        assert_eq!(layout.lines.len(), 3);
        for l in &layout.lines {
            assert!(l.spans.is_empty(), "expected blank line, got {l:?}");
        }
    }

    #[test]
    fn deeply_nested_groups_do_not_blow_stack_or_memory() {
        // 1000 nested groups — would have been O(N^2) clones with the
        // old fits() implementation; with the borrowed-stack version
        // it's O(N) total.  Should complete quickly.
        let mut doc = text("x");
        for _ in 0..1000 {
            doc = group(doc);
        }
        let layout = layout_doc(doc, &opts(80));
        assert_eq!(render_text(&layout), "x");
    }

    #[test]
    fn deeply_nested_groups_with_concat_siblings_dont_blow_up() {
        // Worst case for the old O(N^2): each nested group's content
        // includes both children and surrounding text, so every
        // descent triggered a fits() call that walked the entire
        // (large) parent stack.  Now linear.
        let mut doc = concat([text("end")]);
        for i in 0..500 {
            doc = group(concat([text(format!("L{i}(")), doc, text(")")]));
        }
        let _layout = layout_doc(doc, &opts(80));
    }

    #[test]
    fn same_doc_same_options_same_tree() {
        let doc = group(concat([
            text("foo("),
            indent(concat([softline(), text("bar,"), line(), text("baz")]), 1),
            softline(),
            text(")"),
        ]));
        let a = layout_doc(doc.clone(), &opts(8));
        let b = layout_doc(doc, &opts(8));
        assert_eq!(a, b);
    }
}
