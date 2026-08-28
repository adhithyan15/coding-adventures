//! Reusable inline formatting for the shared Layout IR.
//!
//! The engine fragments text into line boxes while preserving semantic inline
//! wrappers as per-line positioned nodes. It deliberately knows nothing about
//! HTML links or document annotations: producer metadata remains in `ext` and
//! naturally follows each wrapper fragment.

use std::collections::HashSet;

use layout_ir::{Content, ExtValue, LayoutNode, PositionedNode, SizeValue, TextMeasurer};
use text_flow::{BaseDirection, BreakKind, TextFlow};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct InlineLayout {
    pub children: Vec<PositionedNode>,
    pub width: f64,
    pub height: f64,
    pub line_count: u32,
}

#[derive(Clone)]
struct Wrapper {
    key: usize,
    node: LayoutNode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WhiteSpace {
    Normal,
    NoWrap,
    Pre,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlineOptions {
    white_space: WhiteSpace,
    break_all: bool,
}

impl Default for InlineOptions {
    fn default() -> Self {
        Self {
            white_space: WhiteSpace::Normal,
            break_all: false,
        }
    }
}

impl InlineOptions {
    /// Read inherited inline formatting defaults from a formatting-context
    /// owner. Unknown extension values retain the standards-neutral defaults.
    pub fn from_node(node: &LayoutNode) -> Self {
        Self {
            white_space: match block_property(node, "whiteSpace") {
                Some("pre") => WhiteSpace::Pre,
                Some("nowrap") => WhiteSpace::NoWrap,
                _ => WhiteSpace::Normal,
            },
            break_all: block_property(node, "wordBreak") == Some("break-all"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VerticalAlign {
    Baseline,
    Top,
    Middle,
    Bottom,
}

enum SourceItem {
    Text {
        key: usize,
        node: LayoutNode,
        wrappers: Vec<Wrapper>,
        white_space: WhiteSpace,
        break_all: bool,
    },
    Atomic {
        node: LayoutNode,
        wrappers: Vec<Wrapper>,
        align: VerticalAlign,
    },
    Break {
        node: LayoutNode,
        wrappers: Vec<Wrapper>,
    },
}

struct Atom {
    leaf_key: Option<usize>,
    node: PositionedNode,
    wrappers: Vec<Wrapper>,
    width: f64,
    height: f64,
    baseline: f64,
    align: VerticalAlign,
}

struct PlacedAtom {
    leaf_key: Option<usize>,
    node: PositionedNode,
    wrappers: Vec<Wrapper>,
    line: usize,
}

#[derive(Default)]
struct Line {
    atoms: Vec<Atom>,
    width: f64,
    max_ascent: f64,
    max_descent: f64,
    max_height: f64,
}

impl Line {
    fn push(&mut self, atom: Atom) {
        self.width += atom.width;
        self.max_height = self.max_height.max(atom.height);
        if atom.align == VerticalAlign::Baseline {
            self.max_ascent = self.max_ascent.max(atom.baseline);
            self.max_descent = self.max_descent.max((atom.height - atom.baseline).max(0.0));
        }
        self.atoms.push(atom);
    }

    fn height(&self) -> f64 {
        (self.max_ascent + self.max_descent).max(self.max_height)
    }
}

/// Format a consecutive run of inline-level nodes.
///
/// `layout_atomic` resolves replaced leaves such as images. Text measurement
/// remains owned by this package so fragments and line boxes share one source
/// of truth.
pub fn layout_inline_run<M, F>(
    nodes: &[LayoutNode],
    max_width: f64,
    measurer: &M,
    layout_atomic: F,
) -> InlineLayout
where
    M: TextMeasurer,
    F: FnMut(&LayoutNode, f64) -> PositionedNode,
{
    layout_inline_run_with_options(
        nodes,
        max_width,
        InlineOptions::default(),
        measurer,
        layout_atomic,
    )
}

/// Format an inline run with properties inherited from its formatting-context
/// owner (for example `whiteSpace: pre` on a block container).
pub fn layout_inline_run_with_options<M, F>(
    nodes: &[LayoutNode],
    max_width: f64,
    options: InlineOptions,
    measurer: &M,
    mut layout_atomic: F,
) -> InlineLayout
where
    M: TextMeasurer,
    F: FnMut(&LayoutNode, f64) -> PositionedNode,
{
    let max_width = finite_non_negative(max_width);
    let mut sources = Vec::new();
    let mut next_key = 0;
    for node in nodes {
        flatten(node, &[], options, &mut next_key, &mut sources);
    }

    let mut lines = Vec::new();
    let mut line = Line::default();
    let mut pending_space = false;

    for source in sources {
        match source {
            SourceItem::Text {
                key,
                node,
                wrappers,
                white_space,
                break_all,
            } => {
                if white_space == WhiteSpace::Pre {
                    let segments: Vec<_> = text_value(&node).split('\n').collect();
                    for (index, segment) in segments.iter().enumerate() {
                        if !segment.is_empty() {
                            push_text_atom(
                                &mut line, &mut lines, &node, key, &wrappers, segment, false,
                                false, max_width, measurer,
                            );
                        }
                        if index + 1 < segments.len() {
                            line.push(break_atom(node.clone(), wrappers.clone(), measurer));
                            flush_line(&mut line, &mut lines, false);
                        }
                    }
                    pending_space = false;
                    continue;
                }

                let value = text_value(&node);
                if break_all {
                    let flow = TextFlow::analyze(value, base_direction(&node, &wrappers));
                    for cluster in flow.graphemes {
                        let piece = &value[cluster.bytes];
                        if piece.chars().all(char::is_whitespace) {
                            pending_space = true;
                            continue;
                        }
                        push_text_atom(
                            &mut line,
                            &mut lines,
                            &node,
                            key,
                            &wrappers,
                            piece,
                            pending_space,
                            white_space != WhiteSpace::NoWrap,
                            max_width,
                            measurer,
                        );
                        pending_space = false;
                    }
                    pending_space |= value.chars().last().is_some_and(char::is_whitespace);
                    continue;
                }

                for piece in line_break_pieces(value, base_direction(&node, &wrappers)) {
                    push_text_atom(
                        &mut line,
                        &mut lines,
                        &node,
                        key,
                        &wrappers,
                        piece.value,
                        pending_space || piece.leading_space,
                        white_space != WhiteSpace::NoWrap,
                        max_width,
                        measurer,
                    );
                    pending_space = piece.trailing_space;
                }
                if value.chars().last().is_some_and(char::is_whitespace) {
                    pending_space = true;
                }
            }
            SourceItem::Atomic {
                node,
                wrappers,
                align,
            } => {
                pending_space = false;
                let positioned = layout_atomic(&node, max_width);
                let atom = Atom {
                    leaf_key: None,
                    width: positioned.width,
                    height: positioned.height,
                    baseline: positioned.height,
                    node: positioned,
                    wrappers,
                    align,
                };
                if !line.atoms.is_empty() && line.width + atom.width > max_width {
                    flush_line(&mut line, &mut lines, false);
                }
                line.push(atom);
            }
            SourceItem::Break { node, wrappers } => {
                pending_space = false;
                let atom = break_atom(node, wrappers, measurer);
                line.push(atom);
                flush_line(&mut line, &mut lines, true);
            }
        }
    }
    flush_line(&mut line, &mut lines, false);

    position_lines(lines)
}

struct TextPiece<'a> {
    value: &'a str,
    leading_space: bool,
    trailing_space: bool,
}

fn line_break_pieces(value: &str, direction: BaseDirection) -> Vec<TextPiece<'_>> {
    let flow = TextFlow::analyze(value, direction);
    let mut boundaries: Vec<_> = flow
        .breaks
        .iter()
        .filter(|opportunity| opportunity.kind == BreakKind::Allowed)
        .map(|opportunity| opportunity.byte_index)
        .collect();
    if boundaries.last().copied() != Some(value.len()) {
        boundaries.push(value.len());
    }

    let mut pieces = Vec::new();
    let mut start = 0;
    let mut pending_space = false;
    for end in boundaries {
        let source = &value[start..end];
        start = end;
        let trimmed = source.trim_matches(char::is_whitespace);
        if trimmed.is_empty() {
            pending_space = true;
            continue;
        }
        pieces.push(TextPiece {
            value: trimmed,
            leading_space: pending_space || source.chars().next().is_some_and(char::is_whitespace),
            trailing_space: source.chars().last().is_some_and(char::is_whitespace),
        });
        pending_space = source.chars().last().is_some_and(char::is_whitespace);
    }
    pieces
}

fn base_direction(node: &LayoutNode, wrappers: &[Wrapper]) -> BaseDirection {
    let direction = html_property(node, "dir").or_else(|| {
        wrappers
            .iter()
            .rev()
            .find_map(|wrapper| html_property(&wrapper.node, "dir"))
    });
    match direction {
        Some("rtl") => BaseDirection::Rtl,
        Some("ltr") => BaseDirection::Ltr,
        _ => BaseDirection::Auto,
    }
}

fn html_property<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a str> {
    let ExtValue::Map(html) = node.ext.get("html")? else {
        return None;
    };
    let ExtValue::Str(value) = html.get(name)? else {
        return None;
    };
    Some(value)
}

fn flatten(
    node: &LayoutNode,
    wrappers: &[Wrapper],
    options: InlineOptions,
    next_key: &mut usize,
    out: &mut Vec<SourceItem>,
) {
    if display(node) == Some("line-break") {
        out.push(SourceItem::Break {
            node: node.clone(),
            wrappers: wrappers.to_vec(),
        });
        return;
    }

    if matches!(node.content, Some(Content::Text(_))) {
        let key = *next_key;
        *next_key += 1;
        out.push(SourceItem::Text {
            key,
            node: node.clone(),
            wrappers: wrappers.to_vec(),
            white_space: inherited_white_space(node, wrappers, options.white_space),
            break_all: inherited_property(node, wrappers, "wordBreak")
                .map_or(options.break_all, |value| value == "break-all"),
        });
        return;
    }

    if display(node) == Some("inline-replaced")
        || node.content.is_some()
        || node.children.is_empty()
    {
        out.push(SourceItem::Atomic {
            node: node.clone(),
            wrappers: wrappers.to_vec(),
            align: inherited_vertical_align(node, wrappers),
        });
        return;
    }

    let mut template = node.clone();
    template.children.clear();
    template.content = None;
    template.width = None;
    template.height = None;
    template.margin = None;
    let wrapper = Wrapper {
        key: *next_key,
        node: template,
    };
    *next_key += 1;
    let mut nested = wrappers.to_vec();
    nested.push(wrapper);
    for child in &node.children {
        flatten(child, &nested, options, next_key, out);
    }
}

#[allow(clippy::too_many_arguments)]
fn push_text_atom<M: TextMeasurer>(
    line: &mut Line,
    lines: &mut Vec<Line>,
    source: &LayoutNode,
    source_key: usize,
    wrappers: &[Wrapper],
    word: &str,
    needs_space: bool,
    allow_wrap: bool,
    max_width: f64,
    measurer: &M,
) {
    let mut value = if needs_space && !line.atoms.is_empty() {
        format!(" {word}")
    } else {
        word.to_string()
    };
    let mut measured = measure_text(source, &value, measurer);
    if allow_wrap && !line.atoms.is_empty() && line.width + measured.width > max_width {
        flush_line(line, lines, false);
        value = word.to_string();
        measured = measure_text(source, &value, measurer);
    }

    let mut fragment = source.clone();
    fragment.width = Some(SizeValue::Wrap);
    fragment.height = Some(SizeValue::Wrap);
    if let Some(Content::Text(content)) = &mut fragment.content {
        content.value = value;
        content.wrap = false;
    }
    let node = PositionedNode {
        x: 0.0,
        y: 0.0,
        width: measured.width,
        height: measured.height,
        id: fragment.id.clone(),
        content: fragment.content,
        children: Vec::new(),
        ext: fragment.ext,
    };
    line.push(Atom {
        leaf_key: Some(source_key),
        width: measured.width,
        height: measured.height,
        baseline: measured.baseline.min(measured.height),
        node,
        wrappers: wrappers.to_vec(),
        align: inherited_vertical_align(source, wrappers),
    });
}

fn measure_text<M: TextMeasurer>(
    node: &LayoutNode,
    value: &str,
    measurer: &M,
) -> layout_ir::MeasureResult {
    let Some(Content::Text(content)) = &node.content else {
        unreachable!();
    };
    measurer.measure(value, &content.font, None)
}

fn break_atom<M: TextMeasurer>(node: LayoutNode, wrappers: Vec<Wrapper>, measurer: &M) -> Atom {
    let measured = measure_text(&node, "", measurer);
    let mut content = node.content.clone();
    if let Some(Content::Text(text)) = &mut content {
        text.value.clear();
        text.wrap = false;
    }
    Atom {
        leaf_key: None,
        width: 0.0,
        height: measured.height,
        baseline: measured.baseline.min(measured.height),
        align: VerticalAlign::Baseline,
        wrappers,
        node: PositionedNode {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: measured.height,
            id: node.id,
            content,
            children: Vec::new(),
            ext: node.ext,
        },
    }
}

fn flush_line(line: &mut Line, lines: &mut Vec<Line>, force_empty: bool) {
    if !line.atoms.is_empty() || force_empty {
        lines.push(std::mem::take(line));
    }
}

fn position_lines(lines: Vec<Line>) -> InlineLayout {
    let mut placed = Vec::new();
    let mut y = 0.0;
    let mut width: f64 = 0.0;

    for (line_index, line) in lines.into_iter().enumerate() {
        let line_height = line.height();
        let baseline = if line.max_ascent > 0.0 {
            line.max_ascent
        } else {
            line_height
        };
        let mut x = 0.0;
        width = width.max(line.width);
        for mut atom in line.atoms {
            atom.node.x = x;
            atom.node.y = y + match atom.align {
                VerticalAlign::Baseline => (baseline - atom.baseline).max(0.0),
                VerticalAlign::Top => 0.0,
                VerticalAlign::Middle => (line_height - atom.height) / 2.0,
                VerticalAlign::Bottom => line_height - atom.height,
            };
            x += atom.width;
            if !merge_with_previous(&mut placed, &atom, line_index) {
                placed.push(PlacedAtom {
                    leaf_key: atom.leaf_key,
                    node: atom.node,
                    wrappers: atom.wrappers,
                    line: line_index,
                });
            }
        }
        y += line_height;
    }

    let line_count = placed
        .last()
        .map_or(0, |atom| atom.line.saturating_add(1) as u32);
    let children = rebuild_wrappers(placed);
    InlineLayout {
        children,
        width,
        height: y,
        line_count,
    }
}

fn merge_with_previous(placed: &mut [PlacedAtom], atom: &Atom, line: usize) -> bool {
    let Some(previous) = placed.last_mut() else {
        return false;
    };
    if previous.line != line || previous.leaf_key.is_none() || previous.leaf_key != atom.leaf_key {
        return false;
    }
    let (Some(Content::Text(previous_text)), Some(Content::Text(atom_text))) =
        (&mut previous.node.content, &atom.node.content)
    else {
        return false;
    };
    previous_text.value.push_str(&atom_text.value);
    previous.node.width += atom.width;
    previous.node.height = previous.node.height.max(atom.height);
    true
}

enum Builder {
    Wrapper {
        template: Wrapper,
        children: Vec<Builder>,
    },
    Leaf(PositionedNode),
}

fn rebuild_wrappers(atoms: Vec<PlacedAtom>) -> Vec<PositionedNode> {
    let mut output = Vec::new();
    let mut current_line = None;
    let mut roots = Vec::new();
    let mut emitted_ids = HashSet::new();

    for atom in atoms {
        if current_line.is_some_and(|line| line != atom.line) {
            output.extend(finalize_forest(
                std::mem::take(&mut roots),
                &mut emitted_ids,
            ));
        }
        current_line = Some(atom.line);
        insert_builder(&mut roots, &atom.wrappers, atom.node);
    }
    output.extend(finalize_forest(roots, &mut emitted_ids));
    output
}

fn insert_builder(target: &mut Vec<Builder>, path: &[Wrapper], leaf: PositionedNode) {
    let Some((head, tail)) = path.split_first() else {
        target.push(Builder::Leaf(leaf));
        return;
    };
    if let Some(Builder::Wrapper { template, children }) = target.last_mut() {
        if template.key == head.key {
            insert_builder(children, tail, leaf);
            return;
        }
    }
    let mut children = Vec::new();
    insert_builder(&mut children, tail, leaf);
    target.push(Builder::Wrapper {
        template: head.clone(),
        children,
    });
}

fn finalize_forest(
    builders: Vec<Builder>,
    emitted_ids: &mut HashSet<usize>,
) -> Vec<PositionedNode> {
    builders
        .into_iter()
        .map(|builder| finalize_builder(builder, emitted_ids))
        .collect()
}

fn finalize_builder(builder: Builder, emitted_ids: &mut HashSet<usize>) -> PositionedNode {
    match builder {
        Builder::Leaf(node) => node,
        Builder::Wrapper { template, children } => {
            let mut children = finalize_forest(children, emitted_ids);
            let min_x = children
                .iter()
                .map(|node| node.x)
                .fold(f64::INFINITY, f64::min);
            let min_y = children
                .iter()
                .map(|node| node.y)
                .fold(f64::INFINITY, f64::min);
            let max_x = children
                .iter()
                .map(|node| node.x + node.width)
                .fold(0.0, f64::max);
            let max_y = children
                .iter()
                .map(|node| node.y + node.height)
                .fold(0.0, f64::max);
            for child in &mut children {
                child.x -= min_x;
                child.y -= min_y;
            }
            let first_fragment = emitted_ids.insert(template.key);
            PositionedNode {
                x: min_x,
                y: min_y,
                width: (max_x - min_x).max(0.0),
                height: (max_y - min_y).max(0.0),
                id: first_fragment.then_some(template.node.id).flatten(),
                content: None,
                children,
                ext: template.node.ext,
            }
        }
    }
}

fn text_value(node: &LayoutNode) -> &str {
    match &node.content {
        Some(Content::Text(text)) => &text.value,
        _ => "",
    }
}

fn display(node: &LayoutNode) -> Option<&str> {
    block_property(node, "display")
}

fn block_property<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a str> {
    let ExtValue::Map(block) = node.ext.get("block")? else {
        return None;
    };
    let ExtValue::Str(value) = block.get(name)? else {
        return None;
    };
    Some(value)
}

fn inherited_property<'a>(
    node: &'a LayoutNode,
    wrappers: &'a [Wrapper],
    name: &str,
) -> Option<&'a str> {
    block_property(node, name).or_else(|| {
        wrappers
            .iter()
            .rev()
            .find_map(|wrapper| block_property(&wrapper.node, name))
    })
}

fn inherited_white_space(
    node: &LayoutNode,
    wrappers: &[Wrapper],
    default: WhiteSpace,
) -> WhiteSpace {
    match inherited_property(node, wrappers, "whiteSpace") {
        Some("pre") => WhiteSpace::Pre,
        Some("nowrap") => WhiteSpace::NoWrap,
        Some(_) => WhiteSpace::Normal,
        None => default,
    }
}

fn inherited_vertical_align(node: &LayoutNode, wrappers: &[Wrapper]) -> VerticalAlign {
    match inherited_property(node, wrappers, "verticalAlign") {
        Some("top") => VerticalAlign::Top,
        Some("middle") => VerticalAlign::Middle,
        Some("bottom") => VerticalAlign::Bottom,
        _ => VerticalAlign::Baseline,
    }
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        f64::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layout_ir::{color_black, font_spec, Ext, FontSpec, MeasureResult, TextAlign, TextContent};
    use std::collections::HashMap;

    struct Mono;

    impl TextMeasurer for Mono {
        fn measure(&self, text: &str, font: &FontSpec, _: Option<f64>) -> MeasureResult {
            MeasureResult {
                width: text.chars().count() as f64 * font.size * 0.5,
                height: font.size,
                baseline: font.size * 0.8,
                line_count: 1,
            }
        }
    }

    fn text(value: &str, size: f64) -> LayoutNode {
        LayoutNode::leaf_text(TextContent {
            value: value.into(),
            font: font_spec("Mono", size),
            color: color_black(),
            decoration: None,
            max_lines: None,
            wrap: true,
            text_align: TextAlign::Start,
        })
        .with_ext("block", display_ext("inline-text"))
    }

    fn display_ext(value: &str) -> ExtValue {
        ExtValue::Map(HashMap::from([(
            "display".into(),
            ExtValue::Str(value.into()),
        )]))
    }

    fn link(children: Vec<LayoutNode>) -> LayoutNode {
        let mut html: Ext = HashMap::new();
        html.insert("role".into(), ExtValue::Str("link".into()));
        LayoutNode::container(children)
            .with_ext("block", display_ext("inline"))
            .with_ext("html", ExtValue::Map(html))
    }

    #[test]
    fn words_fragment_across_lines_without_internal_wrapping() {
        let layout = layout_inline_run(
            &[text("one two three", 10.0)],
            42.0,
            &Mono,
            |_, _| unreachable!(),
        );
        assert_eq!(layout.line_count, 2);
        assert_eq!(layout.children.len(), 2);
        assert_eq!(
            layout.children[0].content.as_ref().and_then(text_content),
            Some("one two")
        );
        assert_eq!(
            layout.children[1].content.as_ref().and_then(text_content),
            Some("three")
        );
        assert_eq!(layout.children[1].x, 0.0);
        assert_eq!(layout.children[1].y, 10.0);
    }

    #[test]
    fn mixed_font_sizes_share_a_baseline() {
        let layout = layout_inline_run(
            &[text("small", 10.0), text("BIG", 20.0)],
            200.0,
            &Mono,
            |_, _| unreachable!(),
        );
        let small = &layout.children[0];
        let big = &layout.children[1];
        assert_eq!(small.y + 8.0, big.y + 16.0);
        assert_eq!(layout.height, 20.0);
    }

    #[test]
    fn semantic_wrappers_are_split_once_per_line() {
        let layout = layout_inline_run(
            &[link(vec![text("one two three four", 10.0)])],
            42.0,
            &Mono,
            |_, _| unreachable!(),
        );
        assert_eq!(layout.line_count, 3);
        assert_eq!(layout.children.len(), 3);
        assert!(layout.children.iter().all(|node| {
            matches!(node.ext.get("html"), Some(ExtValue::Map(values)) if values.get("role") == Some(&ExtValue::Str("link".into())))
        }));
        assert!(layout.children.iter().all(|node| node.width <= 42.0));
    }

    #[test]
    fn cjk_text_wraps_at_unicode_opportunities_without_spaces() {
        let layout = layout_inline_run(
            &[text("日本語文", 10.0)],
            10.0,
            &Mono,
            |_, _| unreachable!(),
        );
        assert_eq!(layout.line_count, 2);
        assert_eq!(
            layout.children[0].content.as_ref().and_then(text_content),
            Some("日本")
        );
        assert_eq!(
            layout.children[1].content.as_ref().and_then(text_content),
            Some("語文")
        );
    }

    #[test]
    fn break_all_keeps_extended_graphemes_together() {
        let mut node = text("e\u{301}x", 10.0);
        node.ext.insert(
            "block".into(),
            ExtValue::Map(HashMap::from([
                ("display".into(), ExtValue::Str("inline".into())),
                ("wordBreak".into(), ExtValue::Str("break-all".into())),
            ])),
        );
        let layout = layout_inline_run(&[node], 10.0, &Mono, |_, _| unreachable!());
        let values: Vec<_> = layout
            .children
            .iter()
            .filter_map(|node| node.content.as_ref().and_then(text_content))
            .collect();
        assert_eq!(values, vec!["e\u{301}", "x"]);
    }

    #[test]
    fn leading_unicode_whitespace_collapses_across_text_nodes() {
        let layout = layout_inline_run(
            &[text("one", 10.0), text("  two", 10.0)],
            100.0,
            &Mono,
            |_, _| unreachable!(),
        );
        assert_eq!(
            layout.children[1].content.as_ref().and_then(text_content),
            Some(" two")
        );
    }

    fn text_content(content: &Content) -> Option<&str> {
        match content {
            Content::Text(text) => Some(&text.value),
            _ => None,
        }
    }
}
