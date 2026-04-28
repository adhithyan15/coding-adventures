//! Terminal backend for `paint-instructions`.

use paint_instructions::{PaintInstruction, PaintRect, PaintScene};

pub const VERSION: &str = "0.1.0";

/// How scene coordinates map to character cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsciiOptions {
    pub scale_x: u32,
    pub scale_y: u32,
}

impl Default for AsciiOptions {
    fn default() -> Self {
        Self {
            scale_x: 8,
            scale_y: 16,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PaintVmAsciiError {
    UnsupportedInstruction(&'static str),
}

struct CharBuffer {
    rows: usize,
    cols: usize,
    chars: Vec<Vec<char>>,
}

impl CharBuffer {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            chars: vec![vec![' '; cols]; rows],
        }
    }

    fn write_char(&mut self, row: i32, col: i32, ch: char) {
        if row < 0 || col < 0 {
            return;
        }
        let row = row as usize;
        let col = col as usize;
        if row >= self.rows || col >= self.cols {
            return;
        }
        self.chars[row][col] = ch;
    }

    fn to_text(&self) -> String {
        self.chars
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string()
    }
}

fn to_col(x: f64, scale_x: u32) -> i32 {
    (x / scale_x as f64).round() as i32
}

fn to_row(y: f64, scale_y: u32) -> i32 {
    (y / scale_y as f64).round() as i32
}

fn render_rect(rect: &PaintRect, buf: &mut CharBuffer, options: AsciiOptions) {
    let fill = rect.fill.as_deref().unwrap_or("#000000");
    if fill.is_empty() || fill == "transparent" || fill == "none" {
        return;
    }

    let c1 = to_col(rect.x, options.scale_x);
    let r1 = to_row(rect.y, options.scale_y);
    let c2 = to_col(rect.x + rect.width, options.scale_x);
    let r2 = to_row(rect.y + rect.height, options.scale_y);

    for row in r1..=r2 {
        for col in c1..=c2 {
            buf.write_char(row, col, '\u{2588}');
        }
    }
}

/// Render a paint scene to a terminal-friendly string.
pub fn render(scene: &PaintScene, options: AsciiOptions) -> Result<String, PaintVmAsciiError> {
    let cols = (scene.width / options.scale_x as f64).ceil() as usize;
    let rows = (scene.height / options.scale_y as f64).ceil() as usize;
    let mut buf = CharBuffer::new(rows, cols);

    for instruction in &scene.instructions {
        match instruction {
            PaintInstruction::Rect(rect) => render_rect(rect, &mut buf, options),
            PaintInstruction::Ellipse(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("ellipse"))
            }
            PaintInstruction::Path(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("path"))
            }
            PaintInstruction::GlyphRun(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("glyph_run"))
            }
            PaintInstruction::Group(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("group"))
            }
            PaintInstruction::Layer(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("layer"))
            }
            PaintInstruction::Line(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("line"))
            }
            PaintInstruction::Clip(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("clip"))
            }
            PaintInstruction::Gradient(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("gradient"))
            }
            PaintInstruction::Image(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("image"))
            }
            PaintInstruction::Text(_) => {
                return Err(PaintVmAsciiError::UnsupportedInstruction("text"))
            }
        }
    }

    Ok(buf.to_text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{PaintInstruction, PaintLine};

    #[test]
    fn version_matches() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn renders_filled_rect() {
        let scene = PaintScene {
            width: 3.0,
            height: 2.0,
            background: "#ffffff".to_string(),
            instructions: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 2.0, 1.0, "#000000",
            ))],
            id: None,
            metadata: None,
        };

        let output = render(
            &scene,
            AsciiOptions {
                scale_x: 1,
                scale_y: 1,
            },
        )
        .expect("rect scene should render");

        assert!(output.contains('\u{2588}'));
    }

    #[test]
    fn rejects_unsupported_instruction_kinds() {
        let scene = PaintScene {
            width: 3.0,
            height: 2.0,
            background: "#ffffff".to_string(),
            instructions: vec![PaintInstruction::Line(PaintLine {
                base: Default::default(),
                x1: 0.0,
                y1: 0.0,
                x2: 2.0,
                y2: 0.0,
                stroke: "#000000".to_string(),
                stroke_width: None,
                stroke_cap: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            })],
            id: None,
            metadata: None,
        };

        let err = render(&scene, AsciiOptions::default()).unwrap_err();
        assert_eq!(err, PaintVmAsciiError::UnsupportedInstruction("line"));
    }
}
