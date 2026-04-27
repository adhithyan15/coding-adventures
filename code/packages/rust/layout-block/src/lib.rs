//! # layout-block
//!
//! UI07 — block and inline flow layout in Rust. Takes a [`LayoutNode`]
//! tree, a [`Constraints`] rectangle, and a [`TextMeasurer`], and
//! returns a fully positioned [`PositionedNode`] tree.
//!
//! This is the layout model underlying HTML/CSS normal flow: block
//! elements stack vertically, inline elements flow horizontally and
//! wrap. The engine here implements the subset needed to render
//! structured documents (Markdown, rich text) correctly — not the full
//! CSS spec.
//!
//! ```text
//! LayoutNode tree + Constraints + TextMeasurer
//!     ↓  layout_block()
//! PositionedNode tree            — ready for UI04 layout-to-paint
//! ```
//!
//! ## Algorithm (v1 scope)
//!
//! For a block container, children are stacked vertically. Each child's
//! width resolves from its `width` hint:
//!
//! - `Fill` / `None` → full available width minus horizontal margins
//! - `Fixed(v)`     → v, clamped to `min_width` / `max_width`
//! - `Wrap`         → measured content width, capped at available width
//!
//! Each child's height comes from recursive layout (for containers) or
//! from the measurer (for text leaves). The container's final height is
//! the sum of children heights plus the container's own padding and
//! margins.
//!
//! Text leaves (`TextContent`) are measured with the available width so
//! the measurer performs word-wrap internally. The measurer returns
//! `{width, height, line_count}`; we use `height` directly — it already
//! reflects line wrapping.
//!
//! ### Margin collapsing
//!
//! Adjacent vertical margins between block siblings collapse to the
//! larger of the two values (CSS margin-collapse rule), following UI07.
//! Parent-child margin collapse is NOT implemented in v1.
//!
//! ### Explicit non-goals (v1)
//!
//! - `float` / `clear` / absolute positioning
//! - Mixed block + inline children with anonymous-block promotion. The
//!   primary upstream producer (`document-ast-to-layout`) already emits
//!   strictly block-or-leaf trees, so the missing inline-flow path is
//!   not a blocker for Markdown.
//! - RTL / bidi
//! - CSS columns
//!
//! Each exclusion matches the spec's documented non-goals.

use layout_ir::{
    Constraints, Content, Edges, LayoutNode, MeasureResult, PositionedNode, SizeValue,
    TextContent, TextMeasurer,
};

pub const VERSION: &str = "0.1.0";

// ═══════════════════════════════════════════════════════════════════════════
// Entry point
// ═══════════════════════════════════════════════════════════════════════════

/// Lay out a block container and all of its descendants. Returns a
/// fully positioned [`PositionedNode`] with x, y, width, height
/// resolved for every node.
///
/// `constraints` is the available-space box (min/max width + height).
/// The returned node's `width` will be within the constraints; its
/// `height` is determined by the content and is not clamped to the
/// constraints' max_height — the caller may clip at render time if
/// the height exceeds the viewport.
pub fn layout_block<M: TextMeasurer>(
    container: &LayoutNode,
    constraints: Constraints,
    measurer: &M,
) -> PositionedNode {
    // Position the root at (0, 0) in its own coordinate space.
    // x / y on the returned PositionedNode (and all descendants) are
    // relative to the parent's content-area origin (per UI02 spec).
    // A top-level node's own margin is NOT applied here — the caller
    // decides where to place the root. For typical use (root at 0,0),
    // this matches the expected UI02 semantics.
    lay_out_any(container, constraints, measurer, 0.0, 0.0)
}

// ═══════════════════════════════════════════════════════════════════════════
// Core recursion
// ═══════════════════════════════════════════════════════════════════════════

/// Lay out a single node (leaf or container) at the given `(x, y)` in
/// its parent's content-area coordinate space.
fn lay_out_any<M: TextMeasurer>(
    node: &LayoutNode,
    constraints: Constraints,
    measurer: &M,
    x: f64,
    y: f64,
) -> PositionedNode {
    // `x` and `y` are the already-margin-adjusted coordinates of this
    // node's top-left corner in its parent's content-area space. We
    // do NOT add this node's margin again here — the caller (either
    // the layout_block entry point or lay_out_container) has already
    // accounted for it.
    let margin = node.margin.unwrap_or_default();
    let padding = node.padding.unwrap_or_default();

    // Available width for this node's content *inside* its margins.
    let outer_max_width = (constraints.max_width - margin.left - margin.right).max(0.0);

    // Resolve the node's outer width (the box width including its own
    // padding). Handled below via resolve_width.
    let is_leaf_text = matches!(node.content, Some(Content::Text(_)));
    let is_leaf_image = matches!(node.content, Some(Content::Image(_)));

    // Decide the content-area max width by subtracting the node's own
    // padding. Negative results clamp to zero.
    let padding_horizontal = padding.left + padding.right;

    if is_leaf_text {
        if let Some(Content::Text(tc)) = &node.content {
            return lay_out_text_leaf(
                node, tc, constraints, measurer, x, y, margin, padding, outer_max_width,
                padding_horizontal,
            );
        }
        unreachable!();
    }

    if is_leaf_image {
        return lay_out_image_leaf(node, constraints, x, y, margin, padding, outer_max_width);
    }

    // Container path.
    lay_out_container(
        node,
        constraints,
        measurer,
        x,
        y,
        margin,
        padding,
        outer_max_width,
        padding_horizontal,
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Container layout — stack block children vertically
// ═══════════════════════════════════════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
fn lay_out_container<M: TextMeasurer>(
    node: &LayoutNode,
    constraints: Constraints,
    measurer: &M,
    x: f64,
    y: f64,
    margin: Edges,
    padding: Edges,
    outer_max_width: f64,
    padding_horizontal: f64,
) -> PositionedNode {
    // Resolve the container's outer width.
    let outer_width = resolve_container_width(node, outer_max_width);
    let inner_max_width = (outer_width - padding_horizontal).max(0.0);

    // Lay out children in source order, stacking vertically.
    let mut children_positioned: Vec<PositionedNode> = Vec::with_capacity(node.children.len());
    let mut cursor_y = padding.top;
    let mut prev_margin_bottom: f64 = 0.0;
    let mut have_placed_a_child = false;

    for child in &node.children {
        let child_margin = child.margin.unwrap_or_default();

        // Margin collapse between adjacent block siblings.
        let collapsed_top = if have_placed_a_child {
            collapse_margin(prev_margin_bottom, child_margin.top)
        } else {
            child_margin.top
        };

        if have_placed_a_child {
            // Subtract the previously-added margin.bottom because we
            // are replacing it with the collapsed value.
            cursor_y -= prev_margin_bottom;
            cursor_y += collapsed_top;
        } else {
            cursor_y += child_margin.top;
        }

        // Child constraints: width-limited by the container's inner
        // content area; height is unconstrained here — the parent
        // accumulates it from children.
        let child_constraints = Constraints {
            min_width: 0.0,
            max_width: inner_max_width,
            min_height: 0.0,
            max_height: f64::MAX,
        };

        // Pass parent-relative x/y to the child. Coordinates are
        // relative to the parent's content-area origin (per UI02
        // spec). `cursor_y` already has the (collapsed) top-margin
        // spacing added — the leaf primitives do NOT re-add margin.
        let child_x = padding.left + child_margin.left;
        let child_y = cursor_y;

        let positioned = lay_out_any(child, child_constraints, measurer, child_x, child_y);
        cursor_y += positioned.height + child_margin.bottom;

        prev_margin_bottom = child_margin.bottom;
        have_placed_a_child = true;
        children_positioned.push(positioned);
    }

    let content_height = cursor_y + padding.bottom;

    // Resolve outer height: explicit hint overrides content height.
    let outer_height = resolve_container_height(node, content_height);

    PositionedNode {
        x,
        y,
        width: outer_width,
        height: outer_height,
        id: node.id.clone(),
        content: None,
        children: children_positioned,
        ext: node.ext.clone(),
    }
}

fn resolve_container_width(node: &LayoutNode, outer_max_width: f64) -> f64 {
    let raw = match node.width {
        Some(SizeValue::Fixed(v)) => v,
        Some(SizeValue::Fill) | None => outer_max_width,
        Some(SizeValue::Wrap) => outer_max_width,
    };
    clamp_with_min_max(raw, node.min_width, node.max_width).min(outer_max_width)
}

fn resolve_container_height(node: &LayoutNode, content_height: f64) -> f64 {
    let raw = match node.height {
        Some(SizeValue::Fixed(v)) => v,
        Some(SizeValue::Fill) | Some(SizeValue::Wrap) | None => content_height,
    };
    clamp_with_min_max(raw, node.min_height, node.max_height)
}

fn clamp_with_min_max(v: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let v = v.max(min.unwrap_or(0.0));
    match max {
        Some(m) => v.min(m),
        None => v,
    }
}

fn collapse_margin(a: f64, b: f64) -> f64 {
    a.max(b)
}

// ═══════════════════════════════════════════════════════════════════════════
// Leaf layout — text
// ═══════════════════════════════════════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
fn lay_out_text_leaf<M: TextMeasurer>(
    node: &LayoutNode,
    tc: &TextContent,
    constraints: Constraints,
    measurer: &M,
    x: f64,
    y: f64,
    margin: Edges,
    padding: Edges,
    outer_max_width: f64,
    padding_horizontal: f64,
) -> PositionedNode {
    // Available width for the actual text glyphs (inside padding).
    let inner_max_width = (outer_max_width - padding_horizontal).max(0.0);

    // Ask the measurer to wrap within the inner max width.
    let max_wrap_width = if node.width == Some(SizeValue::Wrap) {
        None
    } else {
        Some(inner_max_width)
    };

    let measured: MeasureResult = measurer.measure(&tc.value, &tc.font, max_wrap_width);

    // The node's outer width: for Fill / None, use the full outer_max_width.
    // For Wrap or Fixed, base on the measurement (or the hint).
    let outer_width = match node.width {
        Some(SizeValue::Fixed(v)) => v,
        Some(SizeValue::Wrap) => (measured.width + padding_horizontal).min(outer_max_width),
        Some(SizeValue::Fill) | None => outer_max_width,
    };
    let outer_width = clamp_with_min_max(outer_width, node.min_width, node.max_width)
        .min(outer_max_width);

    let content_height = measured.height;
    let outer_height_raw = content_height + padding.top + padding.bottom;
    let outer_height = match node.height {
        Some(SizeValue::Fixed(v)) => v,
        _ => outer_height_raw,
    };
    let outer_height = clamp_with_min_max(outer_height, node.min_height, node.max_height);

    // The content is carried through unchanged — paint-layer consumer
    // will re-measure or re-shape as needed. The caller has already
    // added margin to x/y; we place the box at exactly (x, y).
    let _ = margin;
    PositionedNode {
        x,
        y,
        width: outer_width,
        height: outer_height,
        id: node.id.clone(),
        content: node.content.clone(),
        children: Vec::new(),
        ext: node.ext.clone(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Leaf layout — image
// ═══════════════════════════════════════════════════════════════════════════

fn lay_out_image_leaf(
    node: &LayoutNode,
    _constraints: Constraints,
    x: f64,
    y: f64,
    _margin: Edges,
    _padding: Edges,
    outer_max_width: f64,
) -> PositionedNode {
    // Image layout v1: size purely from node's width/height hints.
    // No intrinsic-size resolution (the renderer handles that).
    let outer_width = match node.width {
        Some(SizeValue::Fixed(v)) => v,
        _ => outer_max_width,
    };
    let outer_width = clamp_with_min_max(outer_width, node.min_width, node.max_width)
        .min(outer_max_width);

    let outer_height = match node.height {
        Some(SizeValue::Fixed(v)) => v,
        _ => 0.0, // placeholder; renderer supplies intrinsic height if known
    };
    let outer_height = clamp_with_min_max(outer_height, node.min_height, node.max_height);

    // Caller has already accounted for margin in the passed-in x, y.
    PositionedNode {
        x,
        y,
        width: outer_width,
        height: outer_height,
        id: node.id.clone(),
        content: node.content.clone(),
        children: Vec::new(),
        ext: node.ext.clone(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use layout_ir::{
        constraints_fixed, edges_all, edges_xy, font_spec, rgb, size_fill, size_fixed, size_wrap,
        Color, Content, FontSpec, LayoutNode, TextAlign, TextContent,
    };

    /// Trivial measurer: every character is (size * 0.5) wide, single
    /// line, height = size * line_height. Wraps at max_width.
    struct MonoMeasurer {
        char_width_factor: f64,
    }

    impl MonoMeasurer {
        fn new() -> Self {
            Self { char_width_factor: 0.5 }
        }
    }

    impl TextMeasurer for MonoMeasurer {
        fn measure(
            &self,
            text: &str,
            font: &FontSpec,
            max_width: Option<f64>,
        ) -> MeasureResult {
            let chars = text.chars().count() as f64;
            let full_width = chars * self.char_width_factor * font.size;
            let line_h = font.size * font.line_height;

            match max_width {
                None => MeasureResult {
                    width: full_width,
                    height: line_h,
                    line_count: 1,
                },
                Some(max) if full_width <= max || max <= 0.0 => MeasureResult {
                    width: full_width,
                    height: line_h,
                    line_count: 1,
                },
                Some(max) => {
                    let lines = (full_width / max).ceil().max(1.0) as u32;
                    MeasureResult {
                        width: max,
                        height: lines as f64 * line_h,
                        line_count: lines,
                    }
                }
            }
        }
    }

    fn text(value: &str, size: f64) -> TextContent {
        TextContent {
            value: value.into(),
            font: font_spec("Test", size),
            color: Color { r: 0, g: 0, b: 0, a: 255 },
            max_lines: None,
            text_align: TextAlign::Start,
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Basic leaf
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn single_text_leaf_sized_by_measurer() {
        let node = LayoutNode::leaf_text(text("Hello", 10.0));
        let p = layout_block(&node, constraints_fixed(500.0, 500.0), &MonoMeasurer::new());
        // "Hello" = 5 chars × 0.5 × 10 = 25 wide. line_height default 1.2 → 12 px
        // width is fill because no explicit hint → 500 (constraints max)
        assert_eq!(p.width, 500.0);
        assert!((p.height - 12.0).abs() < 1e-6);
        assert!(p.children.is_empty());
        assert!(matches!(p.content, Some(Content::Text(_))));
    }

    #[test]
    fn wrap_width_text_shrinks_to_content() {
        let node = LayoutNode::leaf_text(text("Hi", 10.0)).with_width(size_wrap());
        let p = layout_block(&node, constraints_fixed(500.0, 500.0), &MonoMeasurer::new());
        // "Hi" = 2 × 0.5 × 10 = 10 wide
        assert_eq!(p.width, 10.0);
    }

    #[test]
    fn fixed_width_text() {
        let node = LayoutNode::leaf_text(text("Hello", 10.0)).with_width(size_fixed(200.0));
        let p = layout_block(&node, constraints_fixed(500.0, 500.0), &MonoMeasurer::new());
        assert_eq!(p.width, 200.0);
    }

    #[test]
    fn text_wraps_within_constraint_width() {
        // "0123456789" = 10 chars × 0.5 × 10 = 50 wide. Force wrap at 25.
        let node = LayoutNode::leaf_text(text("0123456789", 10.0));
        let p = layout_block(&node, constraints_fixed(25.0, 500.0), &MonoMeasurer::new());
        assert_eq!(p.width, 25.0);
        // 2 lines: height = 2 × 12 = 24
        assert!((p.height - 24.0).abs() < 1e-6);
    }

    // ─────────────────────────────────────────────────────────────
    // Block container stacking
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn two_blocks_stack_vertically() {
        let node = LayoutNode::container(vec![
            LayoutNode::leaf_text(text("a", 10.0)),
            LayoutNode::leaf_text(text("b", 10.0)),
        ]);
        let p = layout_block(&node, constraints_fixed(300.0, 300.0), &MonoMeasurer::new());
        assert_eq!(p.children.len(), 2);
        assert_eq!(p.children[0].y, 0.0);
        // Child 0 height = 12, so child 1 y = 12
        assert!((p.children[1].y - 12.0).abs() < 1e-6);
        // Container height = sum of children heights
        assert!((p.height - 24.0).abs() < 1e-6);
    }

    #[test]
    fn children_receive_parent_padding_offset() {
        let node = LayoutNode::container(vec![LayoutNode::leaf_text(text("a", 10.0))])
            .with_padding(edges_all(5.0));
        let p = layout_block(&node, constraints_fixed(300.0, 300.0), &MonoMeasurer::new());
        assert_eq!(p.children.len(), 1);
        assert_eq!(p.children[0].x, 5.0);
        assert_eq!(p.children[0].y, 5.0);
        // Container height = 5 (padding top) + 12 (child) + 5 (padding bottom)
        assert!((p.height - 22.0).abs() < 1e-6);
    }

    #[test]
    fn margin_collapsing_between_siblings() {
        // child0 margin bottom = 10; child1 margin top = 6.
        // Collapsed gap = max(10, 6) = 10.
        let node = LayoutNode::container(vec![
            LayoutNode::leaf_text(text("a", 10.0))
                .with_margin(edges_xy(0.0, 5.0))  // top:5 bottom:5... but edges_xy sets y on top & bottom
                .with_margin(Edges { top: 0.0, right: 0.0, bottom: 10.0, left: 0.0 }),
            LayoutNode::leaf_text(text("b", 10.0))
                .with_margin(Edges { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        ]);
        let p = layout_block(&node, constraints_fixed(300.0, 300.0), &MonoMeasurer::new());
        // child0 y = 0, height = 12. With child0.bottom=10 and child1.top=6 collapsed to 10,
        // child1 y = 12 + 10 = 22.
        assert!((p.children[1].y - 22.0).abs() < 1e-6);
    }

    // ─────────────────────────────────────────────────────────────
    // Nested containers
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn nested_container_respects_outer_padding() {
        let inner = LayoutNode::container(vec![LayoutNode::leaf_text(text("hi", 10.0))])
            .with_padding(edges_all(3.0));
        let outer = LayoutNode::container(vec![inner]).with_padding(edges_all(5.0));
        let p = layout_block(&outer, constraints_fixed(300.0, 300.0), &MonoMeasurer::new());
        assert_eq!(p.children.len(), 1);
        let inner_p = &p.children[0];
        assert_eq!(inner_p.x, 5.0);
        assert_eq!(inner_p.y, 5.0);
        let leaf_p = &inner_p.children[0];
        assert_eq!(leaf_p.x, 3.0);
        assert_eq!(leaf_p.y, 3.0);
    }

    // ─────────────────────────────────────────────────────────────
    // Size hints
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn fixed_height_overrides_content_height() {
        let node = LayoutNode::leaf_text(text("Hello", 10.0)).with_height(size_fixed(100.0));
        let p = layout_block(&node, constraints_fixed(500.0, 500.0), &MonoMeasurer::new());
        assert_eq!(p.height, 100.0);
    }

    #[test]
    fn fill_width_uses_constraint_max() {
        let node = LayoutNode::leaf_text(text("a", 10.0)).with_width(size_fill());
        let p = layout_block(&node, constraints_fixed(400.0, 400.0), &MonoMeasurer::new());
        assert_eq!(p.width, 400.0);
    }

    #[test]
    fn min_max_width_clamping() {
        let node = LayoutNode::leaf_text(text("Hello", 10.0))
            .with_width(size_fixed(200.0));
        let mut constrained = node.clone();
        constrained.max_width = Some(100.0);
        let p = layout_block(&constrained, constraints_fixed(500.0, 500.0), &MonoMeasurer::new());
        assert!(p.width <= 100.0);
    }

    // ─────────────────────────────────────────────────────────────
    // Content passthrough
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn text_content_is_passed_through_unchanged() {
        let tc = text("Rendered", 14.0);
        let node = LayoutNode::leaf_text(tc.clone()).with_id("greeting");
        let p = layout_block(&node, constraints_fixed(500.0, 500.0), &MonoMeasurer::new());
        assert_eq!(p.id.as_deref(), Some("greeting"));
        match p.content.as_ref().unwrap() {
            Content::Text(got) => assert_eq!(got.value, tc.value),
            _ => panic!("expected text"),
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Empty / degenerate cases
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn empty_container_has_zero_height() {
        let p = layout_block(
            &LayoutNode::empty(),
            constraints_fixed(300.0, 300.0),
            &MonoMeasurer::new(),
        );
        assert_eq!(p.children.len(), 0);
        assert_eq!(p.height, 0.0);
    }

    #[test]
    fn empty_container_with_padding_has_padding_height() {
        let p = layout_block(
            &LayoutNode::empty().with_padding(edges_all(8.0)),
            constraints_fixed(300.0, 300.0),
            &MonoMeasurer::new(),
        );
        assert_eq!(p.height, 16.0); // 8 top + 8 bottom
    }

    // ─────────────────────────────────────────────────────────────
    // A small realistic doc shape
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn realistic_document_lays_out_sensibly() {
        // h1 "Hello" + paragraph "World lorem ipsum"
        let _ = rgb(0, 0, 0); // silence unused-import on builds that don't need it
        let h1 = LayoutNode::leaf_text(text("Hello", 24.0))
            .with_margin(Edges { top: 0.0, right: 0.0, bottom: 12.0, left: 0.0 });
        let p1 = LayoutNode::leaf_text(text("World lorem ipsum", 16.0))
            .with_margin(Edges { top: 8.0, right: 0.0, bottom: 8.0, left: 0.0 });
        let doc = LayoutNode::container(vec![h1, p1]).with_padding(edges_all(24.0));
        let out = layout_block(&doc, constraints_fixed(800.0, 1000.0), &MonoMeasurer::new());
        // Document starts at 0, 0. Inner children offset by padding 24 on both axes.
        assert_eq!(out.x, 0.0);
        assert_eq!(out.y, 0.0);
        assert_eq!(out.children.len(), 2);
        assert_eq!(out.children[0].x, 24.0);
        assert_eq!(out.children[0].y, 24.0);
        // h1 height = 24 × 1.2 = 28.8. Collapsed margin between children = max(12, 8) = 12
        // p1 y = 24 + 28.8 + 12 = 64.8
        assert!((out.children[1].y - 64.8).abs() < 1e-6);
    }
}
