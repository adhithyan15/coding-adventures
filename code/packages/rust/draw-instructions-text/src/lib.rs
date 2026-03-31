//! # draw-instructions-text
//!
//! ASCII/Unicode text renderer for the draw-instructions scene model.
//!
//! This crate proves the draw-instructions abstraction is truly backend-
//! neutral: the same `DrawScene` that produces SVG or paints a Canvas can
//! also render as box-drawing characters in a terminal.
//!
//! ## How It Works
//!
//! The renderer maps pixel-coordinate scenes to a fixed-width character grid.
//! Each cell in the grid is one character.  The mapping uses a configurable
//! scale factor (default: 8 px per char width, 16 px per char height).
//!
//! ## Intersection Logic
//!
//! When two drawing operations overlap at the same cell, the renderer
//! merges them into the correct junction character using a direction
//! bitmask.  A horizontal line crossing a vertical line becomes `\u{253c}`.
//! A line meeting a box corner becomes the appropriate tee
//! (`\u{252c}` `\u{2534}` `\u{251c}` `\u{2524}`).

use draw_instructions::{
    DrawClipInstruction, DrawGroupInstruction, DrawInstruction, DrawLineInstruction,
    DrawRectInstruction, DrawScene, DrawTextInstruction, Renderer,
};

pub const VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// Direction flags
//
// Each cell in the tag buffer stores a bitmask of directions.  When
// multiple drawing operations overlap, we OR the flags together and
// resolve the combined tag to the correct box-drawing character.
//
//        UP (1)
//         |
// LEFT(8)-+-RIGHT(2)
//         |
//       DOWN(4)
// ---------------------------------------------------------------------------

const UP: u8 = 1;
const RIGHT: u8 = 2;
const DOWN: u8 = 4;
const LEFT: u8 = 8;
const FILL: u8 = 16;
const TEXT: u8 = 32;

// ---------------------------------------------------------------------------
// Clip bounds
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct ClipBounds {
    min_col: i32,
    min_row: i32,
    max_col: i32,
    max_row: i32,
}

// ---------------------------------------------------------------------------
// Box-drawing character resolution
//
// Given a bitmask of directions (UP | DOWN | LEFT | RIGHT), return the
// correct Unicode box-drawing character.  We use a match on the masked
// direction bits.
// ---------------------------------------------------------------------------

fn resolve_box_char(tag: u8) -> char {
    if tag & FILL != 0 {
        return '\u{2588}'; // block
    }
    if tag & TEXT != 0 {
        return ' '; // placeholder -- text chars are stored directly
    }
    match tag & (UP | DOWN | LEFT | RIGHT) {
        v if v == LEFT | RIGHT => '\u{2500}',                    // horizontal
        v if v == UP | DOWN => '\u{2502}',                       // vertical
        v if v == DOWN | RIGHT => '\u{250c}',                    // top-left corner
        v if v == DOWN | LEFT => '\u{2510}',                     // top-right corner
        v if v == UP | RIGHT => '\u{2514}',                      // bottom-left corner
        v if v == UP | LEFT => '\u{2518}',                       // bottom-right corner
        v if v == LEFT | RIGHT | DOWN => '\u{252c}',             // top tee
        v if v == LEFT | RIGHT | UP => '\u{2534}',               // bottom tee
        v if v == UP | DOWN | RIGHT => '\u{251c}',               // left tee
        v if v == UP | DOWN | LEFT => '\u{2524}',                // right tee
        v if v == UP | DOWN | LEFT | RIGHT => '\u{253c}',       // cross
        v if v == RIGHT => '\u{2500}',                            // half-lines -> full
        v if v == LEFT => '\u{2500}',
        v if v == UP => '\u{2502}',
        v if v == DOWN => '\u{2502}',
        _ => '+',
    }
}

// ---------------------------------------------------------------------------
// Character buffer
// ---------------------------------------------------------------------------

/// A 2-D character buffer with a parallel tag buffer for intersection logic.
///
/// The `chars` grid stores the actual character at each cell.  The `tags`
/// grid stores a bitmask of directions passing through each cell.  When
/// writing a box-drawing character we update the tag buffer and resolve the
/// correct character from the combined tag.
struct CharBuffer {
    rows: usize,
    cols: usize,
    chars: Vec<Vec<char>>,
    tags: Vec<Vec<u8>>,
}

impl CharBuffer {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            chars: vec![vec![' '; cols]; rows],
            tags: vec![vec![0u8; cols]; rows],
        }
    }

    /// Write a box-drawing element at (row, col) by adding direction flags.
    /// The actual character is resolved from the combined tag.
    fn write_tag(&mut self, row: i32, col: i32, dir_flags: u8, clip: &ClipBounds) {
        if row < clip.min_row || row >= clip.max_row {
            return;
        }
        if col < clip.min_col || col >= clip.max_col {
            return;
        }
        if row < 0 || col < 0 {
            return;
        }
        let r = row as usize;
        let c = col as usize;
        if r >= self.rows || c >= self.cols {
            return;
        }

        let existing = self.tags[r][c];

        // Don't overwrite text with box-drawing
        if existing & TEXT != 0 {
            return;
        }

        let merged = existing | dir_flags;
        self.tags[r][c] = merged;
        self.chars[r][c] = if dir_flags & FILL != 0 {
            '\u{2588}'
        } else {
            resolve_box_char(merged)
        };
    }

    /// Write a text character directly at (row, col).
    /// Text overwrites any existing content.
    fn write_char(&mut self, row: i32, col: i32, ch: char, clip: &ClipBounds) {
        if row < clip.min_row || row >= clip.max_row {
            return;
        }
        if col < clip.min_col || col >= clip.max_col {
            return;
        }
        if row < 0 || col < 0 {
            return;
        }
        let r = row as usize;
        let c = col as usize;
        if r >= self.rows || c >= self.cols {
            return;
        }

        self.chars[r][c] = ch;
        self.tags[r][c] = TEXT;
    }

    /// Join all rows, trim trailing whitespace, and return the result.
    fn to_string(&self) -> String {
        let lines: Vec<String> = self
            .chars
            .iter()
            .map(|row| {
                let s: String = row.iter().collect();
                s.trim_end().to_string()
            })
            .collect();
        lines.join("\n").trim_end().to_string()
    }
}

// ---------------------------------------------------------------------------
// Coordinate mapping
// ---------------------------------------------------------------------------

fn to_col(x: f64, scale_x: f64) -> i32 {
    (x / scale_x).round() as i32
}

fn to_row(y: f64, scale_y: f64) -> i32 {
    (y / scale_y).round() as i32
}

// ---------------------------------------------------------------------------
// Instruction renderers
// ---------------------------------------------------------------------------

/// Render a rectangle instruction into the character buffer.
///
/// Stroked rectangles produce box-drawing outlines (corners + edges).
/// Filled rectangles produce solid block characters.  A transparent
/// or empty fill with no stroke produces nothing.
fn render_rect(inst: &DrawRectInstruction, buf: &mut CharBuffer, sx: f64, sy: f64, clip: &ClipBounds) {
    let c1 = to_col(inst.x as f64, sx);
    let r1 = to_row(inst.y as f64, sy);
    let c2 = to_col((inst.x + inst.width) as f64, sx);
    let r2 = to_row((inst.y + inst.height) as f64, sy);

    let has_stroke = inst.stroke.is_some() && inst.stroke.as_deref() != Some("");
    let has_fill = !inst.fill.is_empty() && inst.fill != "transparent" && inst.fill != "none";

    if has_stroke {
        // Corners
        buf.write_tag(r1, c1, DOWN | RIGHT, clip);
        buf.write_tag(r1, c2, DOWN | LEFT, clip);
        buf.write_tag(r2, c1, UP | RIGHT, clip);
        buf.write_tag(r2, c2, UP | LEFT, clip);

        // Top and bottom edges
        for c in (c1 + 1)..c2 {
            buf.write_tag(r1, c, LEFT | RIGHT, clip);
            buf.write_tag(r2, c, LEFT | RIGHT, clip);
        }

        // Left and right edges
        for r in (r1 + 1)..r2 {
            buf.write_tag(r, c1, UP | DOWN, clip);
            buf.write_tag(r, c2, UP | DOWN, clip);
        }
    } else if has_fill {
        for r in r1..=r2 {
            for c in c1..=c2 {
                buf.write_tag(r, c, FILL, clip);
            }
        }
    }
}

/// Render a line instruction into the character buffer.
///
/// Horizontal and vertical lines use direction-aware endpoint flags:
/// endpoints only point inward so that junctions with perpendicular
/// elements resolve correctly.
///
/// Diagonal lines are approximated using Bresenham's algorithm.
fn render_line(inst: &DrawLineInstruction, buf: &mut CharBuffer, sx: f64, sy: f64, clip: &ClipBounds) {
    let c1 = to_col(inst.x1, sx);
    let r1 = to_row(inst.y1, sy);
    let c2 = to_col(inst.x2, sx);
    let r2 = to_row(inst.y2, sy);

    if r1 == r2 {
        // Horizontal line
        let min_c = c1.min(c2);
        let max_c = c1.max(c2);
        for c in min_c..=max_c {
            let mut flags: u8 = 0;
            if c > min_c {
                flags |= LEFT;
            }
            if c < max_c {
                flags |= RIGHT;
            }
            if c == min_c && c == max_c {
                flags = LEFT | RIGHT; // single-cell line
            }
            buf.write_tag(r1, c, flags, clip);
        }
    } else if c1 == c2 {
        // Vertical line
        let min_r = r1.min(r2);
        let max_r = r1.max(r2);
        for r in min_r..=max_r {
            let mut flags: u8 = 0;
            if r > min_r {
                flags |= UP;
            }
            if r < max_r {
                flags |= DOWN;
            }
            if r == min_r && r == max_r {
                flags = UP | DOWN; // single-cell line
            }
            buf.write_tag(r, c1, flags, clip);
        }
    } else {
        // Diagonal -- Bresenham approximation
        let dr = (r2 - r1).unsigned_abs() as i32;
        let dc = (c2 - c1).unsigned_abs() as i32;
        let sr: i32 = if r1 < r2 { 1 } else { -1 };
        let sc: i32 = if c1 < c2 { 1 } else { -1 };
        let mut err = dc - dr;
        let mut r = r1;
        let mut c = c1;

        loop {
            let flags = if dc > dr { LEFT | RIGHT } else { UP | DOWN };
            buf.write_tag(r, c, flags, clip);
            if r == r2 && c == c2 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dr {
                err -= dr;
                c += sc;
            }
            if e2 < dc {
                err += dc;
                r += sr;
            }
        }
    }
}

/// Render a text instruction into the character buffer.
///
/// Alignment determines the anchor point:
/// - `"start"`: text begins at the x coordinate
/// - `"middle"`: text is centered on the x coordinate
/// - `"end"`: text ends at the x coordinate
fn render_text_inst(inst: &DrawTextInstruction, buf: &mut CharBuffer, sx: f64, sy: f64, clip: &ClipBounds) {
    let row = to_row(inst.y as f64, sy);
    let text = &inst.value;
    let len = text.chars().count() as i32;

    let start_col = match inst.align.as_str() {
        "middle" => to_col(inst.x as f64, sx) - len / 2,
        "end" => to_col(inst.x as f64, sx) - len,
        _ => to_col(inst.x as f64, sx), // "start"
    };

    for (i, ch) in text.chars().enumerate() {
        buf.write_char(row, start_col + i as i32, ch, clip);
    }
}

/// Render a group by recursing into each child instruction.
fn render_group(inst: &DrawGroupInstruction, buf: &mut CharBuffer, sx: f64, sy: f64, clip: &ClipBounds) {
    for child in &inst.children {
        render_instruction(child, buf, sx, sy, clip);
    }
}

/// Render a clip region by intersecting with the parent clip.
fn render_clip(inst: &DrawClipInstruction, buf: &mut CharBuffer, sx: f64, sy: f64, parent_clip: &ClipBounds) {
    let new_clip = ClipBounds {
        min_col: parent_clip.min_col.max(to_col(inst.x, sx)),
        min_row: parent_clip.min_row.max(to_row(inst.y, sy)),
        max_col: parent_clip.max_col.min(to_col(inst.x + inst.width, sx)),
        max_row: parent_clip.max_row.min(to_row(inst.y + inst.height, sy)),
    };
    for child in &inst.children {
        render_instruction(child, buf, sx, sy, &new_clip);
    }
}

/// Dispatch a single draw instruction to the appropriate renderer.
fn render_instruction(inst: &DrawInstruction, buf: &mut CharBuffer, sx: f64, sy: f64, clip: &ClipBounds) {
    match inst {
        DrawInstruction::Rect(r) => render_rect(r, buf, sx, sy, clip),
        DrawInstruction::Line(l) => render_line(l, buf, sx, sy, clip),
        DrawInstruction::Text(t) => render_text_inst(t, buf, sx, sy, clip),
        DrawInstruction::Group(g) => render_group(g, buf, sx, sy, clip),
        DrawInstruction::Clip(c) => render_clip(c, buf, sx, sy, clip),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Text renderer that converts DrawScene objects into box-drawing strings.
///
/// Implements the `Renderer<String>` trait from draw-instructions.
pub struct TextRenderer {
    pub scale_x: f64,
    pub scale_y: f64,
}

impl TextRenderer {
    /// Create a text renderer with default scale (8 px/col, 16 px/row).
    pub fn new() -> Self {
        Self {
            scale_x: 8.0,
            scale_y: 16.0,
        }
    }

    /// Create a text renderer with custom scale factors.
    pub fn with_scale(scale_x: f64, scale_y: f64) -> Self {
        Self { scale_x, scale_y }
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer<String> for TextRenderer {
    fn render(&self, scene: &DrawScene) -> String {
        let cols = (scene.width as f64 / self.scale_x).ceil() as usize;
        let rows = (scene.height as f64 / self.scale_y).ceil() as usize;
        let mut buf = CharBuffer::new(rows, cols);

        let full_clip = ClipBounds {
            min_col: 0,
            min_row: 0,
            max_col: cols as i32,
            max_row: rows as i32,
        };

        for inst in &scene.instructions {
            render_instruction(inst, &mut buf, self.scale_x, self.scale_y, &full_clip);
        }

        buf.to_string()
    }
}

/// Convenience function: render a scene with default scale (8 px/col, 16 px/row).
pub fn render_text(scene: &DrawScene) -> String {
    TextRenderer::new().render(scene)
}

/// Convenience function: render a scene with custom scale.
pub fn render_text_with_scale(scene: &DrawScene, scale_x: f64, scale_y: f64) -> String {
    TextRenderer::with_scale(scale_x, scale_y).render(scene)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use draw_instructions::{
        create_scene, draw_clip, draw_group, draw_line, draw_rect,
        DrawInstruction, DrawRectInstruction, DrawTextInstruction, Metadata,
        render_with,
    };

    /// Helper: render with 1:1 scale for easy coordinate reasoning.
    fn render_1to1(scene: &DrawScene) -> String {
        TextRenderer::with_scale(1.0, 1.0).render(scene)
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // -- Stroked rectangles -----------------------------------------------

    #[test]
    fn stroked_rect_draws_box() {
        let rect = DrawInstruction::Rect(DrawRectInstruction {
            x: 0, y: 0, width: 4, height: 2,
            fill: "transparent".into(),
            stroke: Some("#000".into()),
            stroke_width: Some(1.0),
            metadata: Metadata::new(),
        });
        let scene = create_scene(5, 3, vec![rect], "", Metadata::new());
        let result = render_1to1(&scene);

        assert_eq!(
            result,
            "\u{250c}\u{2500}\u{2500}\u{2500}\u{2510}\n\
             \u{2502}   \u{2502}\n\
             \u{2514}\u{2500}\u{2500}\u{2500}\u{2518}"
        );
    }

    // -- Filled rectangles ------------------------------------------------

    #[test]
    fn filled_rect() {
        let scene = create_scene(3, 2, vec![draw_rect(0, 0, 2, 1, "#000", Metadata::new())], "", Metadata::new());
        let result = render_1to1(&scene);
        assert!(result.contains('\u{2588}'));
    }

    #[test]
    fn transparent_rect_produces_nothing() {
        let scene = create_scene(5, 3, vec![draw_rect(0, 0, 4, 2, "transparent", Metadata::new())], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "");
    }

    #[test]
    fn none_fill_produces_nothing() {
        let scene = create_scene(5, 3, vec![draw_rect(0, 0, 4, 2, "none", Metadata::new())], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "");
    }

    // -- Horizontal lines -------------------------------------------------

    #[test]
    fn horizontal_line() {
        let scene = create_scene(5, 1, vec![draw_line(0.0, 0.0, 4.0, 0.0, "#000", 1.0)], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    }

    #[test]
    fn horizontal_line_reversed() {
        let scene = create_scene(5, 1, vec![draw_line(4.0, 0.0, 0.0, 0.0, "#000", 1.0)], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    }

    // -- Vertical lines ---------------------------------------------------

    #[test]
    fn vertical_line() {
        let scene = create_scene(1, 3, vec![draw_line(0.0, 0.0, 0.0, 2.0, "#000", 1.0)], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "\u{2502}\n\u{2502}\n\u{2502}");
    }

    #[test]
    fn vertical_line_reversed() {
        let scene = create_scene(1, 3, vec![draw_line(0.0, 2.0, 0.0, 0.0, "#000", 1.0)], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "\u{2502}\n\u{2502}\n\u{2502}");
    }

    // -- Intersections ----------------------------------------------------

    #[test]
    fn crossing_lines_produce_cross() {
        let scene = create_scene(5, 3, vec![
            draw_line(0.0, 1.0, 4.0, 1.0, "#000", 1.0),
            draw_line(2.0, 0.0, 2.0, 2.0, "#000", 1.0),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        let lines: Vec<&str> = result.split('\n').collect();

        assert_eq!(lines[0].chars().nth(2), Some('\u{2502}'));
        assert_eq!(lines[1].chars().nth(2), Some('\u{253c}'));
        assert_eq!(lines[2].chars().nth(2), Some('\u{2502}'));
    }

    // -- Table grid -------------------------------------------------------

    #[test]
    fn box_with_horizontal_divider() {
        let rect = DrawInstruction::Rect(DrawRectInstruction {
            x: 0, y: 0, width: 6, height: 2,
            fill: "transparent".into(),
            stroke: Some("#000".into()),
            stroke_width: Some(1.0),
            metadata: Metadata::new(),
        });
        let scene = create_scene(7, 3, vec![
            rect,
            draw_line(0.0, 1.0, 6.0, 1.0, "#000", 1.0),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        let lines: Vec<&str> = result.split('\n').collect();

        assert_eq!(lines[0], "\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}");
        assert_eq!(lines[1].chars().next(), Some('\u{251c}'));
        assert_eq!(lines[1].chars().nth(6), Some('\u{2524}'));
        assert_eq!(lines[2], "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}");
    }

    // -- Text rendering ---------------------------------------------------

    #[test]
    fn text_start_align() {
        // draw_text defaults to "middle" align, so we construct manually
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 0, y: 0, value: "Hello".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(10, 1, vec![text], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn text_middle_align() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 5, y: 0, value: "Hi".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "middle".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(10, 1, vec![text], "", Metadata::new());
        let result = render_1to1(&scene);
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[4], 'H');
        assert_eq!(chars[5], 'i');
    }

    #[test]
    fn text_end_align() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 9, y: 0, value: "End".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "end".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(10, 1, vec![text], "", Metadata::new());
        let result = render_1to1(&scene);
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[6], 'E');
        assert_eq!(chars[7], 'n');
        assert_eq!(chars[8], 'd');
    }

    #[test]
    fn text_overwrites_box_drawing() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 2, y: 0, value: "AB".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(10, 1, vec![
            draw_line(0.0, 0.0, 9.0, 0.0, "#000", 1.0),
            text,
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[2], 'A');
        assert_eq!(chars[3], 'B');
        assert_eq!(chars[0], '\u{2500}');
    }

    #[test]
    fn box_drawing_does_not_overwrite_text() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 2, y: 0, value: "AB".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(10, 1, vec![
            text,
            draw_line(0.0, 0.0, 9.0, 0.0, "#000", 1.0),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[2], 'A');
        assert_eq!(chars[3], 'B');
    }

    // -- Text inside a box ------------------------------------------------

    #[test]
    fn text_inside_stroked_box() {
        let rect = DrawInstruction::Rect(DrawRectInstruction {
            x: 0, y: 0, width: 11, height: 2,
            fill: "transparent".into(),
            stroke: Some("#000".into()),
            stroke_width: Some(1.0),
            metadata: Metadata::new(),
        });
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 1, y: 1, value: "Hello".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(12, 3, vec![rect, text], "", Metadata::new());
        let result = render_1to1(&scene);
        let lines: Vec<&str> = result.split('\n').collect();

        assert_eq!(lines[0], "\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}");
        assert_eq!(lines[1], "\u{2502}Hello     \u{2502}");
        assert_eq!(lines[2], "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}");
    }

    // -- Clips ------------------------------------------------------------

    #[test]
    fn clip_truncates_text() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 0, y: 0, value: "Hello World".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(10, 1, vec![
            draw_clip(0.0, 0.0, 3.0, 1.0, vec![text]),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "Hel");
    }

    #[test]
    fn nested_clips() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 0, y: 0, value: "ABCDEFGHIJ".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(10, 1, vec![
            draw_clip(0.0, 0.0, 5.0, 1.0, vec![
                draw_clip(2.0, 0.0, 5.0, 1.0, vec![text]),
            ]),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result.trim(), "CDE");
    }

    // -- Groups -----------------------------------------------------------

    #[test]
    fn group_recurses() {
        let t1 = DrawInstruction::Text(DrawTextInstruction {
            x: 0, y: 0, value: "AB".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let t2 = DrawInstruction::Text(DrawTextInstruction {
            x: 3, y: 0, value: "CD".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(5, 1, vec![
            draw_group(vec![t1, t2], Metadata::new()),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "AB CD");
    }

    // -- Table demo -------------------------------------------------------

    #[test]
    fn table_demo() {
        let border = DrawInstruction::Rect(DrawRectInstruction {
            x: 0, y: 0, width: 12, height: 5,
            fill: "transparent".into(),
            stroke: Some("#000".into()),
            stroke_width: Some(1.0),
            metadata: Metadata::new(),
        });
        let make_text = |x, y, val: &str| DrawInstruction::Text(DrawTextInstruction {
            x, y, value: val.into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });

        let scene = create_scene(13, 6, vec![
            border,
            draw_line(6.0, 0.0, 6.0, 5.0, "#000", 1.0),
            draw_line(0.0, 2.0, 12.0, 2.0, "#000", 1.0),
            make_text(1, 1, "Name"),
            make_text(7, 1, "Age"),
            make_text(1, 3, "Alice"),
            make_text(7, 3, "30"),
            make_text(1, 4, "Bob"),
            make_text(7, 4, "25"),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        let lines: Vec<&str> = result.split('\n').collect();

        assert_eq!(lines[0], "\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}");
        assert!(lines[1].contains("Name"));
        assert!(lines[1].contains("Age"));
        assert_eq!(lines[2].chars().next(), Some('\u{251c}'));
        assert_eq!(lines[2].chars().nth(6), Some('\u{253c}'));
        assert_eq!(lines[2].chars().nth(12), Some('\u{2524}'));
        assert!(lines[3].contains("Alice"));
        assert!(lines[3].contains("30"));
        assert!(lines[4].contains("Bob"));
        assert!(lines[4].contains("25"));
        assert_eq!(lines[5], "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2534}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}");
    }

    // -- Scale factor -----------------------------------------------------

    #[test]
    fn default_scale() {
        let rect = DrawInstruction::Rect(DrawRectInstruction {
            x: 0, y: 0, width: 80, height: 32,
            fill: "transparent".into(),
            stroke: Some("#000".into()),
            stroke_width: Some(1.0),
            metadata: Metadata::new(),
        });
        let scene = create_scene(88, 48, vec![rect], "", Metadata::new());
        let result = render_text(&scene);
        let lines: Vec<&str> = result.split('\n').collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].chars().next(), Some('\u{250c}'));
        assert_eq!(lines[2].chars().next(), Some('\u{2514}'));
    }

    #[test]
    fn custom_scale() {
        let scene = create_scene(12, 8, vec![
            draw_line(0.0, 0.0, 12.0, 0.0, "#000", 1.0),
        ], "", Metadata::new());
        let result = render_text_with_scale(&scene, 4.0, 4.0);
        assert!(result.contains('\u{2500}'));
    }

    // -- render_with integration ------------------------------------------

    #[test]
    fn render_with_integration() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 0, y: 0, value: "OK".into(), fill: "#000".into(),
            font_family: "monospace".into(), font_size: 16,
            align: "start".into(), font_weight: None, metadata: Metadata::new(),
        });
        let scene = create_scene(5, 1, vec![text], "", Metadata::new());
        let result = render_with(&scene, &TextRenderer::with_scale(1.0, 1.0));
        assert_eq!(result, "OK");
    }

    // -- Empty scene ------------------------------------------------------

    #[test]
    fn empty_scene() {
        let scene = create_scene(0, 0, vec![], "", Metadata::new());
        let result = render_1to1(&scene);
        assert_eq!(result, "");
    }

    // -- Diagonal lines ---------------------------------------------------

    #[test]
    fn diagonal_line() {
        let scene = create_scene(5, 5, vec![
            draw_line(0.0, 0.0, 4.0, 4.0, "#000", 1.0),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        let lines: Vec<&str> = result.split('\n').collect();
        assert!(lines.len() >= 3);
    }

    // -- Single-cell lines ------------------------------------------------

    #[test]
    fn single_cell_horizontal_line() {
        let scene = create_scene(3, 1, vec![
            draw_line(1.0, 0.0, 1.0, 0.0, "#000", 1.0),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        assert!(result.contains('\u{2500}'));
    }

    #[test]
    fn single_cell_vertical_line() {
        // Use distinct y coords to ensure vertical path is taken
        let scene = create_scene(1, 3, vec![
            draw_line(0.0, 0.0, 0.0, 2.0, "#000", 1.0),
        ], "", Metadata::new());
        let result = render_1to1(&scene);
        assert!(result.contains('\u{2502}'));
    }
}
