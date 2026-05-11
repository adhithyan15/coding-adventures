//! # mosaic-emit-paint — Mosaic source → PaintScene → PNG
//!
//! This crate is the **Paint VM backend** for the Mosaic compiler. Instead of
//! producing HTML, JSX, or QML text strings, it produces a [`PaintScene`] — a
//! flat, ordered list of [`PaintInstruction`]s — and can render that scene to
//! PNG bytes using the `barcode-2d` rendering pipeline.
//!
//! ## Why a Paint backend?
//!
//! The HTML/React/WebComponent backends require a browser to render. Paint gives
//! you raster output (PNG) with zero browser or Qt dependency. This is useful
//! for:
//!
//! - Server-side thumbnail generation of UI designs.
//! - Design tools that need a visual preview without a DOM.
//! - Integration tests that compare pixels, not markup strings.
//!
//! ## Architecture
//!
//! ```text
//! Mosaic source text
//!     │
//!     ▼  mosaic_analyzer::analyze()
//! MosaicFile (typed IR)
//!     │
//!     ▼  layout_node()  [this crate]
//! LayoutBox (PaintInstructions + bounding box)
//!     │
//!     ▼  PaintScene::new() + scene.instructions = …
//! PaintScene
//!     │
//!     ▼  barcode_2d::render_scene_png()
//! Vec<u8>  (PNG bytes)
//! ```
//!
//! ## Rendering model — naive box model
//!
//! This crate implements a **naive box model**, not a full CSS flexbox engine
//! (that lives in `layout-flexbox`). Each node gets a rectangular slice of the
//! available space:
//!
//! - **Column** — children stacked top-to-bottom, equal heights.
//! - **Row** — children stacked left-to-right, equal widths.
//! - **Box / Stack** — all children overlaid at the same origin.
//! - **Text** — a single `PaintText` instruction at the baseline.
//! - **Image** — a gray placeholder `PaintRect`.
//! - **Divider** — a 1-pixel-high fill rect spanning full width.
//! - **Icon** — a 24×24 placeholder rect.
//! - **Grid** — header rect + sample row rects.
//! - **Scroll** — same as Box, but with an explicit stroke.
//!
//! Slot references and `when`/`each` blocks produce labeled placeholders so
//! static previews remain meaningful even without runtime data.
//!
//! ## Node → Paint mapping
//!
//! | Mosaic node | Paint output                                     |
//! |-------------|--------------------------------------------------|
//! | Box         | PaintRect fill #f8f8f8 + children overlaid       |
//! | Stack       | PaintRect fill #f8f8f8 + children overlaid       |
//! | Column      | PaintRect + children stacked vertically          |
//! | Row         | PaintRect + children stacked horizontally        |
//! | Text        | PaintText at baseline                            |
//! | Image       | PaintRect fill #cccccc stroke #999999            |
//! | Spacer      | (nothing — absorbs remaining space)              |
//! | Divider     | PaintRect fill #e0e0e0, height 1px               |
//! | Icon        | PaintRect fill #e8e8e8 stroke #999999, 24×24     |
//! | Grid        | Header PaintRect + row PaintRects                |
//! | Scroll      | PaintRect stroke + children overlaid             |
//! | @slot_ref   | PaintText "@{slot}"                             |
//! | when block  | Dashed PaintRect + label PaintText               |
//! | each block  | 3× body with "(1)"/"(2)"/"(3)" labels            |
//! | unknown     | Pale yellow placeholder PaintRect                |

use mosaic_analyzer::{MosaicChild, MosaicNode, MosaicProperty, MosaicValue, analyze};
use paint_instructions::{PaintBase, PaintInstruction, PaintRect, PaintScene, PaintText};

// ============================================================================
// Public version constant
// ============================================================================

/// Crate version — kept in sync with `Cargo.toml`.
pub const VERSION: &str = "0.1.0";

// ============================================================================
// Layout constants
// ============================================================================

/// Default canvas width when no explicit size is supplied.
///
/// 400 px is wide enough to show a single-column mobile layout.
const DEFAULT_WIDTH: f64 = 400.0;

/// Default canvas height when no explicit size is supplied.
const DEFAULT_HEIGHT: f64 = 300.0;

/// Line height for Text nodes in user-space pixels.
///
/// 20 px gives comfortable single-line reading with a 14 px font.
const LINE_HEIGHT_PX: f64 = 20.0;

/// Default font size for Text nodes.
const FONT_SIZE: f64 = 14.0;

/// Padding applied between the canvas edge and the root node.
///
/// This prevents nodes from bleeding right up against the PNG border.
const PADDING_PX: f64 = 8.0;

// ============================================================================
// LayoutBox — internal layout result
// ============================================================================

/// The result of laying out a single [`MosaicNode`] into a rectangular region.
///
/// A `LayoutBox` records where the node was placed (x, y, width, height) and
/// the flat list of [`PaintInstruction`]s generated for it and all its
/// descendants. Callers collect the instructions from the entire tree and hand
/// them to a [`PaintScene`].
///
/// Think of it as one "box" in the CSS box model — it holds its own dimensions
/// and all the drawing commands needed to paint itself and its children.
///
/// `x` and `y` are stored on the struct so that future layout passes (e.g. a
/// flexbox second pass or an accessibility hit-testing traversal) can query the
/// placed origin of any node without re-running layout. They are not used by
/// the current single-pass engine but are part of the public LayoutBox contract.
struct LayoutBox {
    /// Left edge of this box in scene coordinates.
    #[allow(dead_code)]
    x: f64,
    /// Top edge of this box in scene coordinates.
    #[allow(dead_code)]
    y: f64,
    /// Width allocated to this box.
    width: f64,
    /// Height consumed by this box (may be less than `avail_h` for Text).
    height: f64,
    /// All paint instructions for this box and its descendants, back-to-front.
    instructions: Vec<PaintInstruction>,
}

// ============================================================================
// Property helpers
// ============================================================================

/// Find a property by name and return its string value if it is a `Literal`.
///
/// This is the primary way to read plain string properties like `content`,
/// `source`, and `placeholder` from a node's property list.
///
/// Returns `None` if no property with `name` exists, or if the value is not
/// a string literal (e.g. it is a slot reference or a number).
///
/// This helper is not called by the current naive layout engine (which only
/// needs `resolve_text_content`), but it is part of the intended extension
/// point for backends that need to inspect other properties (e.g. `source` for
/// Image, `placeholder` for Text). Suppress dead-code lint rather than remove.
#[allow(dead_code)]
fn resolve_str_prop<'a>(props: &'a [MosaicProperty], name: &str) -> Option<&'a str> {
    props.iter().find_map(|p| {
        if p.name == name {
            if let MosaicValue::Literal(s) = &p.value {
                Some(s.as_str())
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Resolve the text shown by a `Text` node.
///
/// The `content` property may be:
/// - A string literal `"Hello"` → returns `"Hello"`.
/// - A slot reference `@title` → returns `"[title]"` as a labeled placeholder.
/// - Absent → returns an empty string.
///
/// This function produces output that is meaningful in a static preview, even
/// without real runtime data bound to slots.
fn resolve_text_content(props: &[MosaicProperty]) -> String {
    props.iter().find_map(|p| {
        if p.name == "content" {
            match &p.value {
                MosaicValue::Literal(s) => Some(s.clone()),
                MosaicValue::SlotRef(slot) => Some(format!("[{slot}]")),
                _ => None,
            }
        } else {
            None
        }
    }).unwrap_or_default()
}

// ============================================================================
// Core layout engine
// ============================================================================

/// Lay out a single [`MosaicNode`] within the given available space and return
/// the resulting [`LayoutBox`].
///
/// This is the heart of the layout engine. It is called recursively: laying out
/// a Column calls `layout_node` for each of its children, passing them a slice
/// of the column's height. The recursion terminates at leaf nodes (Text, Image,
/// Spacer, Divider, Icon) which have no children.
///
/// # Arguments
///
/// - `node` — the node to lay out.
/// - `x`, `y` — top-left corner in scene coordinates.
/// - `avail_w`, `avail_h` — space offered to this node by its parent.
///
/// # Returns
///
/// A [`LayoutBox`] whose `width` and `height` describe what the node consumed,
/// and whose `instructions` are the complete paint commands for it and all its
/// descendants.
fn layout_node(node: &MosaicNode, x: f64, y: f64, avail_w: f64, avail_h: f64) -> LayoutBox {
    match node.node_type.as_str() {
        // ─── Column ────────────────────────────────────────────────────────
        //
        // A Column stacks its children top-to-bottom. Each child gets the full
        // available width. Heights are divided equally among non-Spacer children;
        // Spacers absorb whatever height is left over.
        //
        // Example: a Column with 3 Text rows and 1 Spacer:
        //   total_h = avail_h
        //   non_spacer_count = 3
        //   child_h = avail_h / 3   (each Text gets this)
        //   spacer_h = avail_h - 3 * child_h   (Spacer gets the remainder)
        "Column" => {
            let mut instrs = Vec::new();
            // Background rect to show the column boundary.
            instrs.push(PaintInstruction::Rect(PaintRect::filled(
                x, y, avail_w, avail_h, "#f8f8f8",
            )));

            let children = &node.children;
            let non_spacer_count = count_non_spacer_children(children);
            let child_h = if non_spacer_count > 0 {
                avail_h / non_spacer_count as f64
            } else {
                avail_h
            };

            let mut cursor_y = y;
            for child in children {
                let lbox = layout_child(child, x, cursor_y, avail_w, child_h);
                cursor_y += lbox.height;
                instrs.extend(lbox.instructions);
            }

            LayoutBox { x, y, width: avail_w, height: avail_h, instructions: instrs }
        }

        // ─── Row ───────────────────────────────────────────────────────────
        //
        // A Row stacks its children left-to-right. Each child gets the full
        // available height. Widths are divided equally among non-Spacer children.
        "Row" => {
            let mut instrs = Vec::new();
            instrs.push(PaintInstruction::Rect(PaintRect::filled(
                x, y, avail_w, avail_h, "#f8f8f8",
            )));

            let children = &node.children;
            let non_spacer_count = count_non_spacer_children(children);
            let child_w = if non_spacer_count > 0 {
                avail_w / non_spacer_count as f64
            } else {
                avail_w
            };

            let mut cursor_x = x;
            for child in children {
                let lbox = layout_child(child, cursor_x, y, child_w, avail_h);
                cursor_x += lbox.width;
                instrs.extend(lbox.instructions);
            }

            LayoutBox { x, y, width: avail_w, height: avail_h, instructions: instrs }
        }

        // ─── Box / Stack ───────────────────────────────────────────────────
        //
        // Both Box and Stack overlay all their children at the same (x, y) origin.
        // This is analogous to `position: relative` in CSS — children can be
        // positioned absolutely within this container, but the naive layout engine
        // simply stacks them all at the top-left corner (pure overlay semantics).
        "Box" | "Stack" => {
            let mut instrs = Vec::new();
            instrs.push(PaintInstruction::Rect(PaintRect::filled(
                x, y, avail_w, avail_h, "#f8f8f8",
            )));
            for child in &node.children {
                let lbox = layout_child(child, x, y, avail_w, avail_h);
                instrs.extend(lbox.instructions);
            }
            LayoutBox { x, y, width: avail_w, height: avail_h, instructions: instrs }
        }

        // ─── Scroll ────────────────────────────────────────────────────────
        //
        // Scroll is semantically a Box with overflow clipping. In a static paint
        // preview we render it exactly like a Box but add a stroke to signal that
        // this region is scrollable.
        "Scroll" => {
            let mut instrs = Vec::new();
            instrs.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x,
                y,
                width: avail_w,
                height: avail_h,
                fill: Some("#f8f8f8".to_string()),
                stroke: Some("#999999".to_string()),
                stroke_width: Some(1.0),
                corner_radius: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            }));
            for child in &node.children {
                let lbox = layout_child(child, x, y, avail_w, avail_h);
                instrs.extend(lbox.instructions);
            }
            LayoutBox { x, y, width: avail_w, height: avail_h, instructions: instrs }
        }

        // ─── Text ──────────────────────────────────────────────────────────
        //
        // A Text node emits one PaintText instruction. The text baseline sits at
        // `y + LINE_HEIGHT_PX * 0.8` — 80% down from the top of the line box,
        // following the CSS `line-height` convention where most fonts have an
        // ascender/descender ratio of roughly 80/20.
        "Text" => {
            let text = resolve_text_content(&node.properties);
            let baseline_y = y + LINE_HEIGHT_PX * 0.8;
            let instr = PaintInstruction::Text(PaintText {
                base: PaintBase::default(),
                x,
                y: baseline_y,
                text,
                font_ref: None,
                font_size: FONT_SIZE,
                fill: Some("#333333".to_string()),
                text_align: None,
            });
            LayoutBox {
                x,
                y,
                width: avail_w,
                height: LINE_HEIGHT_PX,
                instructions: vec![instr],
            }
        }

        // ─── Image ─────────────────────────────────────────────────────────
        //
        // Images are rendered as gray placeholder rectangles. We cap the height
        // at 100 px so that an Image node with no explicit size doesn't swallow
        // the entire available height — this keeps the preview readable.
        "Image" => {
            let h = avail_h.min(100.0);
            let instr = PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x,
                y,
                width: avail_w,
                height: h,
                fill: Some("#cccccc".to_string()),
                stroke: Some("#999999".to_string()),
                stroke_width: Some(1.0),
                corner_radius: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            });
            LayoutBox { x, y, width: avail_w, height: h, instructions: vec![instr] }
        }

        // ─── Spacer ────────────────────────────────────────────────────────
        //
        // A Spacer absorbs space without producing any visual output. It acts
        // like CSS `flex: 1` — the parent is responsible for allocating the
        // appropriate height (or width, in a Row) to the spacer. Here we simply
        // return a zero-height box; the Column/Row handlers use
        // `count_non_spacer_children` to pre-compute sizes that don't account
        // for spacers, which causes spacers to receive `child_h`/`child_w` but
        // emit nothing.
        "Spacer" => {
            LayoutBox { x, y, width: avail_w, height: 0.0, instructions: vec![] }
        }

        // ─── Divider ───────────────────────────────────────────────────────
        //
        // A Divider is a thin horizontal rule — 1 px tall, full width. Think
        // HTML `<hr>`.
        "Divider" => {
            let instr = PaintInstruction::Rect(PaintRect::filled(
                x, y, avail_w, 1.0, "#e0e0e0",
            ));
            LayoutBox { x, y, width: avail_w, height: 1.0, instructions: vec![instr] }
        }

        // ─── Icon ──────────────────────────────────────────────────────────
        //
        // Icons are rendered as 24×24 placeholder squares — the standard "touch
        // target" size for mobile icons (Material Design, HIG). The gray fill
        // and stroke indicate "something graphical goes here."
        "Icon" => {
            let instr = PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x,
                y,
                width: 24.0,
                height: 24.0,
                fill: Some("#e8e8e8".to_string()),
                stroke: Some("#999999".to_string()),
                stroke_width: Some(1.0),
                corner_radius: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            });
            LayoutBox { x, y, width: 24.0, height: 24.0, instructions: vec![instr] }
        }

        // ─── Grid ──────────────────────────────────────────────────────────
        //
        // A Grid renders a header row (light gray, slightly darker) plus two
        // sample data rows. This gives a reasonable static preview of a table-
        // like layout, even without real row data. Row heights are fixed at
        // LINE_HEIGHT_PX + 4 px of padding for comfortable reading.
        "Grid" => {
            let row_h = LINE_HEIGHT_PX + 4.0;
            let mut instrs = Vec::new();

            // Header row
            instrs.push(PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x,
                y,
                width: avail_w,
                height: row_h,
                fill: Some("#f0f0f0".to_string()),
                stroke: Some("#cccccc".to_string()),
                stroke_width: Some(1.0),
                corner_radius: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            }));

            // Sample data rows (2 rows to show the list pattern)
            for i in 1..=2_usize {
                let row_y = y + i as f64 * row_h;
                instrs.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x,
                    y: row_y,
                    width: avail_w,
                    height: row_h,
                    fill: Some("#ffffff".to_string()),
                    stroke: Some("#cccccc".to_string()),
                    stroke_width: Some(1.0),
                    corner_radius: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }

            let total_h = row_h * 3.0;
            LayoutBox { x, y, width: avail_w, height: total_h, instructions: instrs }
        }

        // ─── Unknown / custom component ────────────────────────────────────
        //
        // Any tag that isn't a recognized Mosaic primitive is either an imported
        // component or a typo. We render a pale yellow placeholder with a dashed
        // stroke so it is visually obvious in the preview that "something goes
        // here, but we don't know what."
        _ => {
            let placeholder_h = avail_h.min(60.0);
            let instr = PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x,
                y,
                width: avail_w,
                height: placeholder_h,
                fill: Some("#ffffcc".to_string()),
                stroke: Some("#cccc00".to_string()),
                stroke_width: Some(1.0),
                corner_radius: None,
                stroke_dash: Some(vec![4.0, 4.0]),
                stroke_dash_offset: None,
            });
            LayoutBox {
                x,
                y,
                width: avail_w,
                height: placeholder_h,
                instructions: vec![instr],
            }
        }
    }
}

// ============================================================================
// Child dispatch
// ============================================================================

/// Lay out a single [`MosaicChild`] — one of: a nested Node, a SlotRef, a
/// `when` block, or an `each` block — and return the resulting [`LayoutBox`].
///
/// This function is the dispatch point that separates the four child variants.
/// The Column/Row/Box handlers call this for each of their children instead of
/// calling `layout_node` directly, so that non-Node children (slot refs, when,
/// each) are also handled.
fn layout_child(child: &MosaicChild, x: f64, y: f64, avail_w: f64, avail_h: f64) -> LayoutBox {
    match child {
        // A nested node: recurse into the layout engine.
        MosaicChild::Node(n) => layout_node(n, x, y, avail_w, avail_h),

        // A bare slot reference used as a child: `@header;`
        // We render it as gray italic-style text so the reader can see
        // "this slot will provide content at runtime."
        MosaicChild::SlotRef(slot) => {
            let label = format!("@{{{slot}}}");
            let baseline_y = y + LINE_HEIGHT_PX * 0.8;
            let instr = PaintInstruction::Text(PaintText {
                base: PaintBase::default(),
                x,
                y: baseline_y,
                text: label,
                font_ref: None,
                font_size: FONT_SIZE,
                fill: Some("#999999".to_string()),
                text_align: None,
            });
            LayoutBox {
                x,
                y,
                width: avail_w,
                height: LINE_HEIGHT_PX,
                instructions: vec![instr],
            }
        }

        // A conditional block: `when @show { ... }`
        // We render a dashed placeholder rectangle + a label text so the
        // reader knows "this subtree is conditionally visible."
        MosaicChild::When { slot, body: _ } => {
            let h = avail_h.min(40.0);
            let placeholder = PaintInstruction::Rect(PaintRect {
                base: PaintBase::default(),
                x,
                y,
                width: avail_w,
                height: h,
                fill: Some("#f0f8ff".to_string()),
                stroke: Some("#88aacc".to_string()),
                stroke_width: Some(1.0),
                corner_radius: None,
                stroke_dash: Some(vec![5.0, 3.0]),
                stroke_dash_offset: None,
            });
            let label = format!("when @{{{slot}}}");
            let label_instr = PaintInstruction::Text(PaintText {
                base: PaintBase::default(),
                x: x + 4.0,
                y: y + LINE_HEIGHT_PX * 0.8,
                text: label,
                font_ref: None,
                font_size: FONT_SIZE,
                fill: Some("#88aacc".to_string()),
                text_align: None,
            });
            LayoutBox {
                x,
                y,
                width: avail_w,
                height: h,
                instructions: vec![placeholder, label_instr],
            }
        }

        // An iteration block: `each @items as item { ... }`
        // We render 3 repetitions of the body with suffixes "(1)", "(2)", "(3)"
        // to show the "list of items" structure in a static preview.
        // Each repetition is stacked vertically; the body height is estimated as
        // LINE_HEIGHT_PX.
        MosaicChild::Each { slot, item_name: _, body: _ } => {
            let row_h = LINE_HEIGHT_PX;
            let mut instrs = Vec::new();
            for i in 1..=3_usize {
                let row_y = y + (i - 1) as f64 * row_h;
                let label = format!("each @{{{slot}}} ({i})");
                let instr = PaintInstruction::Text(PaintText {
                    base: PaintBase::default(),
                    x,
                    y: row_y + row_h * 0.8,
                    text: label,
                    font_ref: None,
                    font_size: FONT_SIZE,
                    fill: Some("#666666".to_string()),
                    text_align: None,
                });
                instrs.push(instr);
            }
            LayoutBox {
                x,
                y,
                width: avail_w,
                height: row_h * 3.0,
                instructions: instrs,
            }
        }
    }
}

// ============================================================================
// Helper: count non-Spacer children
// ============================================================================

/// Count how many children are *not* a Spacer node.
///
/// Column and Row use this to divide available space. Spacers are excluded so
/// that the non-spacer children each get a fair share of the space; spacers
/// then absorb what remains. This mirrors the CSS `flex: 1` pattern.
///
/// # Example
///
/// A Column with 2 Text nodes and 1 Spacer:
/// - `count_non_spacer_children` → 2
/// - each Text gets `avail_h / 2`
/// - the Spacer gets `avail_h / 2` allocated (by the loop) but emits nothing
fn count_non_spacer_children(children: &[MosaicChild]) -> usize {
    children.iter().filter(|c| !is_spacer_child(c)).count()
}

/// Returns `true` if the child is a `Spacer` node.
fn is_spacer_child(child: &MosaicChild) -> bool {
    matches!(child, MosaicChild::Node(n) if n.node_type == "Spacer")
}

// ============================================================================
// Public API
// ============================================================================

/// Compile a Mosaic source string into a [`PaintScene`] with the given canvas
/// dimensions.
///
/// The source is analyzed with `mosaic_analyzer::analyze`, then the root node
/// is laid out with [`layout_node`], and the resulting instructions are placed
/// into a fresh `PaintScene` with a white background.
///
/// # Errors
///
/// Returns a `String` error message if the source fails to lex, parse, or
/// analyze. The error message is human-readable and suitable for display.
///
/// # Example
///
/// ```rust
/// use mosaic_emit_paint::render_scene;
///
/// let scene = render_scene("component Card { Box { } }", 400.0, 300.0).unwrap();
/// assert_eq!(scene.width, 400.0);
/// assert_eq!(scene.background, "#ffffff");
/// assert!(!scene.instructions.is_empty());
/// ```
pub fn render_scene(source: &str, width: f64, height: f64) -> Result<PaintScene, String> {
    // `mosaic_analyzer::analyze` calls `mosaic_parser::parse` internally, which
    // panics on syntactically invalid input (the parser was not designed to return
    // errors — it assumes lexer-level validation has already filtered bad tokens).
    // We use `catch_unwind` to convert those panics into well-formed `Err` strings
    // so that `render_scene` always returns (never propagates a panic to callers).
    let owned = source.to_string();
    let file = std::panic::catch_unwind(|| analyze(&owned))
        .map_err(|_| "Mosaic parse error (panic in lexer/parser)".to_string())?
        .map_err(|e| e.to_string())?;
    let root = &file.component.root;
    let lbox = layout_node(
        root,
        PADDING_PX,
        PADDING_PX,
        width - 2.0 * PADDING_PX,
        height - 2.0 * PADDING_PX,
    );
    let mut scene = PaintScene::new(width, height);
    scene.instructions = lbox.instructions;
    Ok(scene)
}

/// Compile a Mosaic source string and render it to PNG bytes.
///
/// This is the end-to-end function for raster output. It calls [`render_scene`]
/// to produce a [`PaintScene`], then hands that scene to
/// `barcode_2d::render_scene_png` which dispatches to the best available
/// rendering backend (Metal on macOS, Direct2D on Windows, Cairo/Skia on
/// Linux).
///
/// # Errors
///
/// Returns a `String` error if analysis, layout, or PNG encoding fails.
///
/// # Example
///
/// ```rust
/// use mosaic_emit_paint::render_png;
///
/// let png = render_png("component Card { Box { } }", 400.0, 300.0).unwrap();
/// assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
/// ```
pub fn render_png(source: &str, width: f64, height: f64) -> Result<Vec<u8>, String> {
    let scene = render_scene(source, width, height)?;
    barcode_2d::render_scene_png(&scene)
}

/// Compile a Mosaic source string into a [`PaintScene`] using the default
/// canvas size of 400×300 pixels.
///
/// This is a convenience wrapper around [`render_scene`] for callers that do
/// not need to specify dimensions.
pub fn render_scene_with_defaults(source: &str) -> Result<PaintScene, String> {
    render_scene(source, DEFAULT_WIDTH, DEFAULT_HEIGHT)
}

/// Compile a Mosaic source string and render it to PNG bytes using the default
/// canvas size of 400×300 pixels.
///
/// This is a convenience wrapper around [`render_png`] for callers that do not
/// need to specify dimensions.
pub fn render_png_with_defaults(source: &str) -> Result<Vec<u8>, String> {
    render_png(source, DEFAULT_WIDTH, DEFAULT_HEIGHT)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::PaintInstruction;

    // ─── helpers ─────────────────────────────────────────────────────────────

    /// Count PaintText instructions anywhere in the scene.
    fn count_texts(scene: &PaintScene) -> usize {
        scene.instructions.iter().filter(|i| matches!(i, PaintInstruction::Text(_))).count()
    }

    /// Count PaintRect instructions anywhere in the scene.
    fn count_rects(scene: &PaintScene) -> usize {
        scene.instructions.iter().filter(|i| matches!(i, PaintInstruction::Rect(_))).count()
    }

    /// Collect all PaintText text values from the scene.
    fn all_texts(scene: &PaintScene) -> Vec<&str> {
        scene.instructions.iter().filter_map(|i| {
            if let PaintInstruction::Text(t) = i { Some(t.text.as_str()) } else { None }
        }).collect()
    }

    // ─── version ─────────────────────────────────────────────────────────────

    /// The VERSION constant must match the Cargo.toml version so tooling and
    /// runtime introspection see a consistent identifier.
    #[test]
    fn version_is_0_1_0() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ─── basic scene construction ─────────────────────────────────────────────

    /// An empty Box should still produce at least one instruction (its background
    /// rect), so the scene is not empty.
    #[test]
    fn render_scene_empty_box() {
        let scene = render_scene("component X { Box { } }", 400.0, 300.0).unwrap();
        assert!(!scene.instructions.is_empty());
    }

    /// The scene dimensions should exactly match the arguments we pass in.
    #[test]
    fn render_scene_returns_correct_dimensions() {
        let scene = render_scene("component X { Box { } }", 800.0, 600.0).unwrap();
        assert_eq!(scene.width, 800.0);
        assert_eq!(scene.height, 600.0);
    }

    /// The background should always be #ffffff (white) — the paint VM paints
    /// this before all instructions.
    #[test]
    fn render_scene_background_is_white() {
        let scene = render_scene("component X { Box { } }", 400.0, 300.0).unwrap();
        assert_eq!(scene.background, "#ffffff");
    }

    /// `render_scene_with_defaults` must produce a 400×300 scene.
    #[test]
    fn render_scene_with_defaults_matches_400x300() {
        let scene = render_scene_with_defaults("component X { Box { } }").unwrap();
        assert_eq!(scene.width, DEFAULT_WIDTH);
        assert_eq!(scene.height, DEFAULT_HEIGHT);
    }

    // ─── layout: Column ──────────────────────────────────────────────────────

    /// A Column containing a Text node must produce at least one PaintText.
    #[test]
    fn render_scene_column_with_text() {
        let src = r#"component X { Column { Text { content: "Hello"; } } }"#;
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(count_texts(&scene) >= 1);
    }

    // ─── layout: Row ─────────────────────────────────────────────────────────

    /// A Row with two Box children must produce at least two PaintRect
    /// instructions (the background rect for each child Box).
    #[test]
    fn render_scene_row_with_two_boxes() {
        let src = "component X { Row { Box { } Box { } } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        // Row bg + child1 bg + child2 bg = at least 3 rects
        assert!(count_rects(&scene) >= 3);
    }

    // ─── layout: Text content ────────────────────────────────────────────────

    /// A Text node with a literal `content` property should produce a PaintText
    /// whose `.text` matches the property value.
    #[test]
    fn render_scene_text_content() {
        let src = r#"component X { Text { content: "Welcome"; } }"#;
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        let texts = all_texts(&scene);
        assert!(texts.iter().any(|t| *t == "Welcome"), "texts: {texts:?}");
    }

    // ─── layout: SlotRef in Text content ─────────────────────────────────────

    /// When a Text node's `content` is a slot reference (`@title`), the
    /// emitted PaintText should contain `[title]` as a labeled placeholder.
    #[test]
    fn render_scene_slot_ref() {
        let src = "component X { slot title: text; Text { content: @title; } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        let texts = all_texts(&scene);
        assert!(
            texts.iter().any(|t| t.contains("[title]")),
            "expected [title] placeholder, got: {texts:?}"
        );
    }

    // ─── layout: Image ───────────────────────────────────────────────────────

    /// An Image node must produce a PaintRect (the gray placeholder).
    #[test]
    fn render_scene_image() {
        let src = "component X { Image { } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(count_rects(&scene) >= 1);
        // The image placeholder should have a gray fill
        let has_gray = scene.instructions.iter().any(|i| {
            if let PaintInstruction::Rect(r) = i {
                r.fill == Some("#cccccc".to_string())
            } else {
                false
            }
        });
        assert!(has_gray, "expected gray image placeholder rect");
    }

    // ─── layout: Spacer ──────────────────────────────────────────────────────

    /// A Spacer inside a Column should produce no PaintInstruction of its own.
    /// (The Column does produce its background rect, but the Spacer contributes
    /// nothing.)
    #[test]
    fn render_scene_spacer_no_instructions() {
        // Column with only a Spacer — the Spacer itself should produce 0 instructions.
        let src = "component X { Column { Spacer { } } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        // The only rect should be the Column background
        assert_eq!(count_rects(&scene), 1, "Spacer must not emit a rect");
        assert_eq!(count_texts(&scene), 0, "Spacer must not emit text");
    }

    // ─── layout: Divider ─────────────────────────────────────────────────────

    /// A Divider must produce at least one PaintRect (the 1px fill rect).
    #[test]
    fn render_scene_divider_produces_rect() {
        let src = "component X { Divider { } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(count_rects(&scene) >= 1);
        // The divider rect should be 1px tall
        let has_1px = scene.instructions.iter().any(|i| {
            if let PaintInstruction::Rect(r) = i {
                (r.height - 1.0).abs() < 0.001
            } else {
                false
            }
        });
        assert!(has_1px, "expected a 1px-tall divider rect");
    }

    // ─── layout: Stack ───────────────────────────────────────────────────────

    /// A Stack must produce its background PaintRect.
    #[test]
    fn render_scene_stack_produces_rect() {
        let src = "component X { Stack { } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(count_rects(&scene) >= 1);
    }

    // ─── layout: Icon ────────────────────────────────────────────────────────

    /// An Icon must produce a 24×24 PaintRect.
    #[test]
    fn render_scene_icon() {
        let src = "component X { Icon { } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        let has_24x24 = scene.instructions.iter().any(|i| {
            if let PaintInstruction::Rect(r) = i {
                (r.width - 24.0).abs() < 0.001 && (r.height - 24.0).abs() < 0.001
            } else {
                false
            }
        });
        assert!(has_24x24, "expected a 24×24 Icon rect");
    }

    // ─── layout: when block ──────────────────────────────────────────────────

    /// A `when @show { }` block must produce at least one instruction (the
    /// dashed placeholder rect or the label text).
    #[test]
    fn render_scene_when_block() {
        let src = "component X { slot show: bool; Column { when @show { Box { } } } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(!scene.instructions.is_empty());
        // Should have at least a label text containing "when"
        let texts = all_texts(&scene);
        assert!(
            texts.iter().any(|t| t.contains("when")),
            "expected a 'when' label, got: {texts:?}"
        );
    }

    // ─── layout: each block ──────────────────────────────────────────────────

    /// An `each @items as item { }` block must produce 3 repetition labels.
    #[test]
    fn render_scene_each_block() {
        let src = "component X { slot items: list<text>; Column { each @items as item { Text { content: @item; } } } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        let texts = all_texts(&scene);
        // We expect 3 "(1)", "(2)", "(3)" labels
        assert!(
            texts.iter().any(|t| t.contains("(1)")),
            "expected (1) label, got: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("(2)")),
            "expected (2) label, got: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("(3)")),
            "expected (3) label, got: {texts:?}"
        );
    }

    // ─── PNG output ──────────────────────────────────────────────────────────

    /// PNG files always start with the 8-byte magic sequence:
    /// `\x89 P N G \r \n \x1a \n`
    /// If the output starts with these bytes, we know we got a valid PNG header.
    #[test]
    fn render_png_produces_valid_png() {
        let src = "component X { Box { } }";
        let bytes = render_png(src, 400.0, 300.0).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "output is not a PNG file");
    }

    /// Same PNG validity check for the `_with_defaults` convenience function.
    #[test]
    fn render_png_with_defaults_valid_png() {
        let src = "component X { Box { } }";
        let bytes = render_png_with_defaults(src).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "output is not a PNG file");
    }

    // ─── error handling ──────────────────────────────────────────────────────

    /// Invalid Mosaic source should return an `Err`, not panic. The lexer and
    /// parser in `mosaic-analyzer` panic on truly invalid tokens; `render_scene`
    /// wraps those panics via `catch_unwind` and converts them to `Err(String)`.
    #[test]
    fn render_png_on_invalid_source_returns_err() {
        // Contains `!!!` which the Mosaic lexer cannot tokenize — triggers a panic
        // that `render_png` must catch and convert to Err.
        let result = render_png("this is not valid mosaic !!!", 400.0, 300.0);
        assert!(result.is_err());
    }

    /// Same error-path check for `render_scene`.
    #[test]
    fn render_scene_on_invalid_source_returns_err() {
        // Same invalid source — `render_scene` must return Err, not propagate a panic.
        let result = render_scene("this is not valid mosaic !!!", 400.0, 300.0);
        assert!(result.is_err());
    }

    // ─── nesting ─────────────────────────────────────────────────────────────

    /// Column → Row → Text nesting: ensures layout_node recurses correctly
    /// through multiple levels of nesting.
    #[test]
    fn render_scene_nested_column_row() {
        let src = r#"component X { Column { Row { Text { content: "nested"; } } } }"#;
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        let texts = all_texts(&scene);
        assert!(
            texts.iter().any(|t| *t == "nested"),
            "expected 'nested' text, got: {texts:?}"
        );
    }

    // ─── Scroll ──────────────────────────────────────────────────────────────

    /// A Scroll node must produce at least one instruction.
    #[test]
    fn render_scene_scroll_node() {
        let src = "component X { Scroll { } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(!scene.instructions.is_empty());
    }

    // ─── Grid ────────────────────────────────────────────────────────────────

    /// A Grid must produce multiple PaintRects (header + rows).
    #[test]
    fn render_scene_grid_node() {
        let src = "component X { Grid { } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(count_rects(&scene) >= 2, "Grid should produce header + at least one row rect");
    }

    // ─── unknown tag ─────────────────────────────────────────────────────────

    /// An unrecognized node type (custom component or typo) should produce a
    /// pale yellow placeholder PaintRect rather than panicking.
    #[test]
    fn render_scene_unknown_tag() {
        // "MyCustomWidget" is not a Mosaic primitive
        let src = "component X { Box { MyCustomWidget { } } }";
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        let has_yellow = scene.instructions.iter().any(|i| {
            if let PaintInstruction::Rect(r) = i {
                r.fill == Some("#ffffcc".to_string())
            } else {
                false
            }
        });
        assert!(has_yellow, "expected pale yellow placeholder rect for unknown tag");
    }

    // ─── multiple Text nodes ─────────────────────────────────────────────────

    /// Multiple Text nodes in a Column should each produce their own PaintText.
    #[test]
    fn render_scene_multiple_text_nodes() {
        let src = r#"component X { Column {
            Text { content: "First"; }
            Text { content: "Second"; }
            Text { content: "Third"; }
        } }"#;
        let scene = render_scene(src, 400.0, 300.0).unwrap();
        assert!(count_texts(&scene) >= 3, "expected at least 3 PaintText instructions");
        let texts = all_texts(&scene);
        assert!(texts.contains(&"First"));
        assert!(texts.contains(&"Second"));
        assert!(texts.contains(&"Third"));
    }
}
