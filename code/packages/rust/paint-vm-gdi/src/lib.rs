//! # paint-vm-gdi
//!
//! GDI renderer for the paint-instructions scene model (P2D07).
//!
//! This crate takes a [`PaintScene`] (backend-neutral 2D paint instructions)
//! and renders it to a [`PixelContainer`] using Windows GDI (Graphics Device
//! Interface) — the original Windows drawing API dating back to Windows 1.0.
//!
//! GDI is the CPU-based fallback renderer in the paint-* stack. It requires
//! no COM initialization, no GPU, and works on every version of Windows.
//! The trade-off is no hardware acceleration and no antialiasing.
//!
//! ## Current instruction support
//!
//! | Instruction       | Status                                          |
//! |-------------------|-------------------------------------------------|
//! | `PaintRect`       | Fully implemented — solid-colour filled rects   |
//! | `PaintLine`       | Fully implemented — rendered via pen + LineTo   |
//! | `PaintGroup`      | Fully implemented — recurses into children      |
//! | `PaintClip`       | Fully implemented — IntersectClipRect + restore |
//! | `PaintGlyphRun`   | Implemented — ExtTextOutW + ETO_GLYPH_INDEX     |
//! | `PaintEllipse`    | Planned — Ellipse()                             |
//! | `PaintPath`       | Planned — BeginPath/PolyBezierTo                |
//! | `PaintLayer`      | Planned — offscreen DC + BitBlt                 |
//! | `PaintGradient`   | Planned — GradientFill (limited)                |
//! | `PaintImage`      | Planned — StretchDIBits / AlphaBlend            |
//!
//! ## GDI pipeline
//!
//! ```text
//! PaintScene
//!   │
//!   ├── 1. CreateCompatibleDC(NULL) — memory device context
//!   ├── 2. CreateDIBSection() — HBITMAP backed by accessible pixel memory
//!   ├── 3. SelectObject() — bind bitmap to DC
//!   ├── 4. FillRect() — clear to scene.background colour
//!   ├── 5. Dispatch PaintInstructions → GDI calls (FillRect, LineTo, etc.)
//!   ├── 6. Read DIBSection pixel data (BGRA → RGBA swap)
//!   └── 7. DeleteObject + DeleteDC — cleanup
//! ```
//!
//! ## Coordinate system
//!
//! `PaintScene` uses a **top-left origin** with Y increasing downward
//! (same as SVG, HTML Canvas, and CSS).
//!
//! GDI also uses a top-left origin with Y increasing downward in the
//! default `MM_TEXT` mapping mode — so no coordinate conversion is needed.
//!
//! ```text
//!  Scene coordinates:       GDI coordinates (MM_TEXT):
//!  (0,0)──────(w,0)        (0,0)──────(w,0)
//!    │              │           │              │
//!    │              │           │              │
//!    │              │           │              │
//!  (0,h)──────(w,h)        (0,h)──────(w,h)
//! ```
//!
//! Note: `CreateDIBSection` with a *negative* `biHeight` gives us a top-down
//! DIB (row 0 at the top). A positive `biHeight` gives bottom-up (row 0 at
//! the bottom, which would require a Y-flip). We use **negative** `biHeight`
//! to match the scene coordinate system directly.

pub const VERSION: &str = "0.1.0";

use paint_instructions::{
    FillRule, PaintClip, PaintEllipse, PaintGlyphRun, PaintInstruction, PaintLine, PaintPath,
    PaintRect, PaintScene, PathCommand, PixelContainer,
};

// ---------------------------------------------------------------------------
// Platform gate — this crate only compiles on Windows
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
compile_error!("paint-vm-gdi requires Windows. Use paint-metal on macOS or paint-vm-cairo on Linux.");

// ---------------------------------------------------------------------------
// Windows API imports
// ---------------------------------------------------------------------------
//
// GDI is the oldest Windows graphics API. It operates on a "device context"
// (HDC) — an abstract drawing surface backed by a bitmap, a printer, or a
// screen window. We create an in-memory DC backed by a DIBSection (a bitmap
// whose pixel memory we can read directly).

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{POINT, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    BeginPath, CloseFigure, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, Ellipse, EndPath, ExtCreatePen, ExtTextOutW,
    FillPath, FillRect, GetStockObject, IntersectClipRect, LineTo, MoveToEx, PolyBezierTo,
    Rectangle, RestoreDC, RoundRect, SaveDC, SelectObject, SetBkMode, SetPolyFillMode,
    SetTextAlign, SetTextColor, StrokeAndFillPath, StrokePath, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, BS_SOLID, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_CHARSET, DEFAULT_PITCH,
    DIB_RGB_COLORS, ETO_GLYPH_INDEX, LOGBRUSH, NULL_BRUSH, NULL_PEN, OUT_DEFAULT_PRECIS,
    PS_ENDCAP_FLAT, PS_GEOMETRIC, PS_JOIN_MITER, PS_SOLID, PS_USERSTYLE, TA_BASELINE, TA_LEFT,
    TRANSPARENT, ALTERNATE, WINDING,
};

// ---------------------------------------------------------------------------
// Colour parsing
// ---------------------------------------------------------------------------
//
// Parse a CSS-style hex colour to RGBA floats. This is the same function used
// by paint-metal — duplicated here so paint-vm-gdi has no dependency on
// paint-metal.

/// Parse a CSS colour string to RGBA floats in the range 0.0–1.0.
///
/// Supported formats:
/// - `"#rrggbb"`   → (r, g, b, 1.0)
/// - `"#rrggbbaa"` → (r, g, b, a)
/// - `"#rgb"`      → expanded to `#rrggbb`
/// - `"rgb(r,g,b)"` / `"rgba(r,g,b,a)"`
/// - `"transparent"` / anything else → (0.0, 0.0, 0.0, 0.0)
///
/// Returns `(0.0, 0.0, 0.0, 1.0)` for unrecognised non-transparent input.
fn parse_css_color(s: &str) -> (f64, f64, f64, f64) {
    let s = s.trim();
    if s == "transparent" || s == "none" {
        return (0.0, 0.0, 0.0, 0.0);
    }
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 4 {
            return (
                parse_css_channel(parts[0]),
                parse_css_channel(parts[1]),
                parse_css_channel(parts[2]),
                parts[3].parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0),
            );
        }
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            return (
                parse_css_channel(parts[0]),
                parse_css_channel(parts[1]),
                parse_css_channel(parts[2]),
                1.0,
            );
        }
    }
    let hex = s.trim_start_matches('#');
    let hex = if hex.len() == 3 {
        // Expand shorthand: #f00 → #ff0000
        let mut expanded = String::with_capacity(6);
        for c in hex.chars() {
            expanded.push(c);
            expanded.push(c);
        }
        expanded
    } else {
        hex.to_string()
    };
    if hex.len() < 6 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;
    let a = if hex.len() >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f64 / 255.0
    } else {
        1.0
    };
    (r, g, b, a)
}

fn parse_css_channel(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0).clamp(0.0, 255.0) / 255.0
}

fn parse_hex_color(s: &str) -> (f64, f64, f64, f64) {
    parse_css_color(s)
}

#[cfg(target_os = "windows")]
#[derive(Clone, Debug)]
struct GdiFontSpec {
    family: String,
    weight: i32,
    italic: bool,
}

#[cfg(target_os = "windows")]
fn font_spec_for_ref(font_ref: &str) -> GdiFontSpec {
    if let Some(spec) = parse_directwrite_font_ref(font_ref) {
        return spec;
    }
    if let Some(spec) = parse_canvas_font_ref(font_ref) {
        return spec;
    }
    GdiFontSpec {
        family: "Segoe UI".to_string(),
        weight: 400,
        italic: false,
    }
}

#[cfg(target_os = "windows")]
fn parse_directwrite_font_ref(font_ref: &str) -> Option<GdiFontSpec> {
    let body = font_ref.strip_prefix("directwrite:")?;
    let (family_part, rest) = body.split_once('@')?;
    let mut weight = 400i32;
    let mut italic = false;
    for part in rest.split(';').skip(1) {
        if let Some(value) = part.strip_prefix("w=") {
            weight = value.parse().unwrap_or(weight);
        } else if let Some(value) = part.strip_prefix("style=") {
            italic = matches!(value, "italic" | "oblique");
        }
    }
    Some(GdiFontSpec {
        family: map_font_family(&unescape_ref_component(family_part)),
        weight,
        italic,
    })
}

#[cfg(target_os = "windows")]
fn parse_canvas_font_ref(font_ref: &str) -> Option<GdiFontSpec> {
    let body = font_ref.strip_prefix("canvas:")?;
    let (family_part, rest) = body.split_once('@')?;
    let mut weight = 400i32;
    let mut italic = false;
    let parts: Vec<&str> = rest.split(':').collect();
    if let Some(value) = parts.get(1) {
        weight = value.parse().unwrap_or(weight);
    }
    if let Some(value) = parts.get(2) {
        italic = matches!((*value).trim(), "italic" | "oblique");
    }
    Some(GdiFontSpec {
        family: map_font_family(family_part),
        weight,
        italic,
    })
}

#[cfg(target_os = "windows")]
fn map_font_family(family: &str) -> String {
    match family.trim().to_ascii_lowercase().as_str() {
        "system-ui" | "ui-sans-serif" => "Segoe UI".to_string(),
        other => {
            if other.is_empty() {
                "Segoe UI".to_string()
            } else {
                family.trim().to_string()
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn unescape_ref_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(target_os = "windows")]
fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Convert RGBA floats (0.0–1.0) to a Win32 COLORREF (0x00BBGGRR).
///
/// GDI's COLORREF stores colours as a 32-bit integer with the format:
/// ```text
/// 0x00BBGGRR
///       ││││││
///       ││││└┘── Red   (low byte)
///       ││└┘──── Green (middle byte)
///       └┘────── Blue  (high byte)
/// ```
///
/// Note the reversed byte order compared to hex CSS colours (#RRGGBB).
/// GDI does not support per-pixel alpha in basic drawing operations, so
/// the alpha channel is used only to skip fully transparent instructions.
#[cfg(target_os = "windows")]
fn color_to_colorref(r: f64, g: f64, b: f64) -> u32 {
    let ri = (r * 255.0).round() as u32;
    let gi = (g * 255.0).round() as u32;
    let bi = (b * 255.0).round() as u32;
    ri | (gi << 8) | (bi << 16)
}

#[cfg(target_os = "windows")]
fn parse_colorref(s: &str) -> Option<windows::Win32::Foundation::COLORREF> {
    let (r, g, b, a) = parse_hex_color(s);
    if a == 0.0 {
        return None;
    }
    Some(windows::Win32::Foundation::COLORREF(color_to_colorref(r, g, b)))
}

#[cfg(target_os = "windows")]
unsafe fn create_pen_for_stroke(
    colorref: windows::Win32::Foundation::COLORREF,
    width: Option<f64>,
    dash: Option<&[f64]>,
) -> windows::Win32::Graphics::Gdi::HPEN {
    let width_px = width.unwrap_or(1.0).max(1.0).round() as u32;
    if let Some(pattern) = dash.filter(|pattern| !pattern.is_empty()) {
        let styles: Vec<u32> = pattern
            .iter()
            .map(|value| value.max(1.0).round() as u32)
            .collect();
        let brush = LOGBRUSH {
            lbStyle: BS_SOLID,
            lbColor: colorref,
            lbHatch: 0,
        };
        ExtCreatePen(
            windows::Win32::Graphics::Gdi::PEN_STYLE(
                PS_GEOMETRIC.0 | PS_USERSTYLE.0 | PS_ENDCAP_FLAT.0 | PS_JOIN_MITER.0,
            ),
            width_px,
            &brush,
            Some(&styles),
        )
    } else {
        CreatePen(
            PS_SOLID,
            width_px as i32,
            colorref,
        )
    }
}

// ---------------------------------------------------------------------------
// Instruction dispatch — PaintInstruction → GDI calls
// ---------------------------------------------------------------------------
//
// Unlike paint-metal which collects all instructions into a vertex buffer
// then draws in one batch, GDI is an immediate-mode API — we issue draw
// calls one at a time. Each instruction translates to one or more GDI calls.

/// Render a list of [`PaintInstruction`]s into a GDI device context.
///
/// This is the core dispatch loop. It recursively handles Group and Clip
/// nodes, and dispatches Rect and Line to their respective GDI calls.
/// Unimplemented instruction types are silently skipped (same as paint-metal).
#[cfg(target_os = "windows")]
unsafe fn render_instructions(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    instructions: &[PaintInstruction],
) {
    for instr in instructions {
        match instr {
            PaintInstruction::Rect(rect) => render_rect(hdc, rect),
            PaintInstruction::Line(line) => render_line(hdc, line),
            PaintInstruction::Group(group) => {
                // PaintGroup: render children directly into the same DC.
                // Transform support (SetWorldTransform) is deferred — for
                // barcodes, groups are used purely for logical containment.
                render_instructions(hdc, &group.children);
            }
            PaintInstruction::Clip(clip) => render_clip(hdc, clip),
            PaintInstruction::GlyphRun(run) => render_glyph_run(hdc, run),
            PaintInstruction::Ellipse(ellipse) => render_ellipse(hdc, ellipse),
            PaintInstruction::Path(path) => render_path(hdc, path),
            // Planned but not yet implemented — same skip list as paint-metal:
            PaintInstruction::Text(_)
            | PaintInstruction::Layer(_)
            | PaintInstruction::Gradient(_)
            | PaintInstruction::Image(_) => {
                // No-op for now. Barcodes only need Rect/Line/Group/Clip.
            }
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn render_glyph_run(hdc: windows::Win32::Graphics::Gdi::HDC, run: &PaintGlyphRun) {
    if run.glyphs.is_empty() {
        return;
    }

    let fill = run.fill.as_deref().unwrap_or("#000000");
    let (r, g, b, a) = parse_hex_color(fill);
    if a == 0.0 {
        return;
    }

    let spec = font_spec_for_ref(&run.font_ref);
    let height = -(run.font_size.max(1.0).round() as i32);
    let family_w = wide_null(&spec.family);
    let hfont = CreateFontW(
        height,
        0,
        0,
        0,
        spec.weight,
        spec.italic as u32,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        PCWSTR(family_w.as_ptr()),
    );
    if hfont.is_invalid() {
        return;
    }

    let old_font = SelectObject(hdc, hfont);
    let _ = SetTextAlign(
        hdc,
        windows::Win32::Graphics::Gdi::TEXT_ALIGN_OPTIONS(TA_LEFT.0 | TA_BASELINE.0),
    );
    let _ = SetBkMode(hdc, TRANSPARENT);
    let colorref = color_to_colorref(r, g, b);
    let _ = SetTextColor(hdc, windows::Win32::Foundation::COLORREF(colorref));

    for glyph in &run.glyphs {
        let Ok(glyph_index) = u16::try_from(glyph.glyph_id) else {
            continue;
        };
        let glyphs = [glyph_index];
        let _ = ExtTextOutW(
            hdc,
            glyph.x.round() as i32,
            glyph.y.round() as i32,
            ETO_GLYPH_INDEX,
            None,
            PCWSTR(glyphs.as_ptr()),
            glyphs.len() as u32,
            None,
        );
    }

    let _ = SelectObject(hdc, old_font);
    let _ = DeleteObject(hfont);
}

/// Render a [`PaintRect`] as a filled rectangle.
///
/// GDI's `Rectangle` / `RoundRect` can paint fill and stroke in one pass using
/// the currently selected brush and pen.
#[cfg(target_os = "windows")]
unsafe fn render_rect(hdc: windows::Win32::Graphics::Gdi::HDC, rect: &PaintRect) {
    let fill_color = rect.fill.as_deref().and_then(parse_colorref);
    let stroke_color = rect.stroke.as_deref().and_then(parse_colorref);
    if fill_color.is_none() && stroke_color.is_none() {
        return;
    }

    let mut owned_brush = None;
    let brush_obj = if let Some(colorref) = fill_color {
        let brush = CreateSolidBrush(colorref);
        owned_brush = Some(brush);
        brush.into()
    } else {
        GetStockObject(NULL_BRUSH)
    };

    let mut owned_pen = None;
    let pen_obj = if let Some(colorref) = stroke_color {
        let pen = create_pen_for_stroke(
            colorref,
            rect.stroke_width,
            rect.stroke_dash.as_deref(),
        );
        owned_pen = Some(pen);
        pen.into()
    } else {
        GetStockObject(NULL_PEN)
    };

    let old_pen = SelectObject(hdc, pen_obj);
    let old_brush = SelectObject(hdc, brush_obj);
    let left = rect.x.round() as i32;
    let top = rect.y.round() as i32;
    let right = (rect.x + rect.width).round() as i32;
    let bottom = (rect.y + rect.height).round() as i32;
    let radius = rect.corner_radius.unwrap_or(0.0).max(0.0).round() as i32;
    if radius > 0 {
        let diameter = (radius * 2).max(1);
        let _ = RoundRect(hdc, left, top, right, bottom, diameter, diameter);
    } else {
        let _ = Rectangle(hdc, left, top, right, bottom);
    }

    let _ = SelectObject(hdc, old_pen);
    let _ = SelectObject(hdc, old_brush);
    if let Some(pen) = owned_pen {
        let _ = DeleteObject(pen);
    }
    if let Some(brush) = owned_brush {
        let _ = DeleteObject(brush);
    }
}

/// Render a [`PaintLine`] using GDI's pen + MoveToEx/LineTo.
///
/// GDI draws lines by:
/// 1. Creating a pen with the desired colour and width
/// 2. Selecting it into the DC
/// 3. Moving to the start point
/// 4. Drawing a line to the end point
///
/// The NULL_BRUSH is selected to prevent GDI from filling any implicit shape.
///
/// ```text
/// (x1,y1) ────────────── (x2,y2)
///         ← pen width →
/// ```
#[cfg(target_os = "windows")]
unsafe fn render_line(hdc: windows::Win32::Graphics::Gdi::HDC, line: &PaintLine) {
    let Some(colorref) = parse_colorref(&line.stroke) else {
        return;
    };
    let pen = create_pen_for_stroke(colorref, line.stroke_width, line.stroke_dash.as_deref());

    let old_pen = SelectObject(hdc, pen);
    let null_brush = GetStockObject(NULL_BRUSH);
    let old_brush = SelectObject(hdc, null_brush);

    let _ = MoveToEx(hdc, line.x1.round() as i32, line.y1.round() as i32, None);
    let _ = LineTo(hdc, line.x2.round() as i32, line.y2.round() as i32);

    let _ = SelectObject(hdc, old_pen);
    let _ = SelectObject(hdc, old_brush);
    let _ = DeleteObject(pen);
}

#[cfg(target_os = "windows")]
unsafe fn render_ellipse(hdc: windows::Win32::Graphics::Gdi::HDC, ellipse: &PaintEllipse) {
    let fill_color = ellipse.fill.as_deref().and_then(parse_colorref);
    let stroke_color = ellipse.stroke.as_deref().and_then(parse_colorref);
    if fill_color.is_none() && stroke_color.is_none() {
        return;
    }

    let mut owned_brush = None;
    let brush_obj = if let Some(colorref) = fill_color {
        let brush = CreateSolidBrush(colorref);
        owned_brush = Some(brush);
        brush.into()
    } else {
        GetStockObject(NULL_BRUSH)
    };

    let mut owned_pen = None;
    let pen_obj = if let Some(colorref) = stroke_color {
        let pen = create_pen_for_stroke(
            colorref,
            ellipse.stroke_width,
            ellipse.stroke_dash.as_deref(),
        );
        owned_pen = Some(pen);
        pen.into()
    } else {
        GetStockObject(NULL_PEN)
    };

    let old_pen = SelectObject(hdc, pen_obj);
    let old_brush = SelectObject(hdc, brush_obj);
    let left = (ellipse.cx - ellipse.rx).round() as i32;
    let top = (ellipse.cy - ellipse.ry).round() as i32;
    let right = (ellipse.cx + ellipse.rx).round() as i32;
    let bottom = (ellipse.cy + ellipse.ry).round() as i32;
    let _ = Ellipse(hdc, left, top, right, bottom);

    let _ = SelectObject(hdc, old_pen);
    let _ = SelectObject(hdc, old_brush);
    if let Some(pen) = owned_pen {
        let _ = DeleteObject(pen);
    }
    if let Some(brush) = owned_brush {
        let _ = DeleteObject(brush);
    }
}

#[cfg(target_os = "windows")]
unsafe fn render_path(hdc: windows::Win32::Graphics::Gdi::HDC, path: &PaintPath) {
    let fill_color = path.fill.as_deref().and_then(parse_colorref);
    let stroke_color = path.stroke.as_deref().and_then(parse_colorref);
    if fill_color.is_none() && stroke_color.is_none() {
        return;
    }

    let saved = SaveDC(hdc);
    let old_fill_mode = SetPolyFillMode(
        hdc,
        match path.fill_rule.as_ref().unwrap_or(&FillRule::NonZero) {
            FillRule::EvenOdd => ALTERNATE,
            FillRule::NonZero => WINDING,
        },
    );

    let mut owned_brush = None;
    let brush_obj = if let Some(colorref) = fill_color {
        let brush = CreateSolidBrush(colorref);
        owned_brush = Some(brush);
        brush.into()
    } else {
        GetStockObject(NULL_BRUSH)
    };

    let mut owned_pen = None;
    let pen_obj = if let Some(colorref) = stroke_color {
        let pen = create_pen_for_stroke(
            colorref,
            path.stroke_width,
            path.stroke_dash.as_deref(),
        );
        owned_pen = Some(pen);
        pen.into()
    } else {
        GetStockObject(NULL_PEN)
    };

    let _ = SelectObject(hdc, pen_obj);
    let _ = SelectObject(hdc, brush_obj);
    let _ = BeginPath(hdc);

    let mut current: Option<(f64, f64)> = None;
    let mut subpath_start: Option<(f64, f64)> = None;
    for command in &path.commands {
        match *command {
            PathCommand::MoveTo { x, y } => {
                let _ = MoveToEx(hdc, x.round() as i32, y.round() as i32, None);
                current = Some((x, y));
                subpath_start = Some((x, y));
            }
            PathCommand::LineTo { x, y } => {
                if current.is_none() {
                    let _ = MoveToEx(hdc, x.round() as i32, y.round() as i32, None);
                    subpath_start = Some((x, y));
                } else {
                    let _ = LineTo(hdc, x.round() as i32, y.round() as i32);
                }
                current = Some((x, y));
            }
            PathCommand::QuadTo { cx, cy, x, y } => {
                let (sx, sy) = current.unwrap_or((x, y));
                let cubic = [
                    POINT {
                        x: (sx + (2.0 / 3.0) * (cx - sx)).round() as i32,
                        y: (sy + (2.0 / 3.0) * (cy - sy)).round() as i32,
                    },
                    POINT {
                        x: (x + (2.0 / 3.0) * (cx - x)).round() as i32,
                        y: (y + (2.0 / 3.0) * (cy - y)).round() as i32,
                    },
                    POINT {
                        x: x.round() as i32,
                        y: y.round() as i32,
                    },
                ];
                if current.is_none() {
                    let _ = MoveToEx(hdc, sx.round() as i32, sy.round() as i32, None);
                    subpath_start = Some((sx, sy));
                }
                let _ = PolyBezierTo(hdc, &cubic);
                current = Some((x, y));
            }
            PathCommand::CubicTo {
                cx1,
                cy1,
                cx2,
                cy2,
                x,
                y,
            } => {
                if current.is_none() {
                    let _ = MoveToEx(hdc, x.round() as i32, y.round() as i32, None);
                    subpath_start = Some((x, y));
                } else {
                    let points = [
                        POINT {
                            x: cx1.round() as i32,
                            y: cy1.round() as i32,
                        },
                        POINT {
                            x: cx2.round() as i32,
                            y: cy2.round() as i32,
                        },
                        POINT {
                            x: x.round() as i32,
                            y: y.round() as i32,
                        },
                    ];
                    let _ = PolyBezierTo(hdc, &points);
                }
                current = Some((x, y));
            }
            PathCommand::ArcTo { x, y, .. } => {
                if current.is_none() {
                    let _ = MoveToEx(hdc, x.round() as i32, y.round() as i32, None);
                    subpath_start = Some((x, y));
                } else {
                    let _ = LineTo(hdc, x.round() as i32, y.round() as i32);
                }
                current = Some((x, y));
            }
            PathCommand::Close => {
                let _ = CloseFigure(hdc);
                current = subpath_start;
            }
        }
    }

    let _ = EndPath(hdc);
    match (fill_color.is_some(), stroke_color.is_some()) {
        (true, true) => {
            let _ = StrokeAndFillPath(hdc);
        }
        (true, false) => {
            let _ = FillPath(hdc);
        }
        (false, true) => {
            let _ = StrokePath(hdc);
        }
        (false, false) => {}
    }

    let _ = SetPolyFillMode(hdc, windows::Win32::Graphics::Gdi::CREATE_POLYGON_RGN_MODE(old_fill_mode));
    let _ = RestoreDC(hdc, saved);
    if let Some(pen) = owned_pen {
        let _ = DeleteObject(pen);
    }
    if let Some(brush) = owned_brush {
        let _ = DeleteObject(brush);
    }
}

/// Render a [`PaintClip`] using GDI's clip region save/restore.
///
/// GDI clip flow:
/// 1. `SaveDC()` — saves entire DC state including clip region
/// 2. `IntersectClipRect()` — narrows the clip to the clip rectangle
/// 3. Render children (only pixels inside the clip are visible)
/// 4. `RestoreDC(-1)` — restores the previous clip region
///
/// This correctly nests: inner clips intersect with outer clips.
#[cfg(target_os = "windows")]
unsafe fn render_clip(hdc: windows::Win32::Graphics::Gdi::HDC, clip: &PaintClip) {
    let _ = SaveDC(hdc);

    let _ = IntersectClipRect(
        hdc,
        clip.x.round() as i32,
        clip.y.round() as i32,
        (clip.x + clip.width).round() as i32,
        (clip.y + clip.height).round() as i32,
    );

    render_instructions(hdc, &clip.children);

    let _ = RestoreDC(hdc, -1);
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a [`PaintScene`] to a [`PixelContainer`] using Windows GDI.
///
/// This is the main entry point for the `paint-vm-gdi` crate.
///
/// ## Pipeline
///
/// 1. Create a memory device context (CreateCompatibleDC)
/// 2. Create a top-down DIBSection bitmap (BGRA, 32 bpp)
/// 3. Clear the bitmap to `scene.background`
/// 4. Dispatch each PaintInstruction to GDI drawing calls
/// 5. Read back the BGRA pixel data, convert to RGBA
/// 6. Return as `PixelContainer`
///
/// ## Requires
///
/// - Windows (any version — GDI has been available since Windows 1.0)
///
/// ## Chaining with a codec
///
/// ```rust,ignore
/// let scene = barcode_2d::layout(&grid, &config);
/// let pixels = paint_vm_gdi::render(&scene);
/// let png = paint_codec_png::encode_png(&pixels);
/// std::fs::write("barcode.png", png).unwrap();
/// ```
#[cfg(target_os = "windows")]
pub fn render(scene: &PaintScene) -> PixelContainer {
    // Validate that dimensions are finite, non-negative, and within u32 range
    // before casting. NaN, negative, or infinite values would produce nonsensical
    // results from `as u32` (saturating cast in release mode).
    if scene.width < 0.0
        || scene.height < 0.0
        || !scene.width.is_finite()
        || !scene.height.is_finite()
    {
        return PixelContainer::new(0, 0);
    }

    let width = scene.width as u32;
    let height = scene.height as u32;

    if width == 0 || height == 0 {
        return PixelContainer::new(width, height);
    }

    // Guard against accidental huge allocations. A 16384x16384 BGRA image
    // is ~1 GB — beyond this most systems would OOM.
    const MAX_DIMENSION: u32 = 16384;
    assert!(
        width <= MAX_DIMENSION && height <= MAX_DIMENSION,
        "Scene dimensions {}x{} exceed maximum {}x{}",
        width,
        height,
        MAX_DIMENSION,
        MAX_DIMENSION
    );

    unsafe { render_unsafe(scene, width, height) }
}

/// The actual rendering logic, wrapped in `unsafe` for GDI FFI calls.
///
/// ## DIBSection memory layout
///
/// A DIBSection with negative `biHeight` gives us a top-down bitmap where
/// row 0 is at the top of the image (matching our coordinate system).
/// Each pixel is 4 bytes in BGRA order:
///
/// ```text
/// byte offset:  [0]  [1]  [2]  [3]  [4]  [5]  [6]  [7] ...
/// channel:       B    G    R    A    B    G    R    A   ...
/// pixel:       ←── pixel 0 ──→ ←── pixel 1 ──→
/// ```
///
/// We convert to RGBA by swapping B↔R for each pixel.
#[cfg(target_os = "windows")]
unsafe fn render_unsafe(scene: &PaintScene, width: u32, height: u32) -> PixelContainer {
    // ── Step 1: Create memory device context ─────────────────────────────
    //
    // A memory DC is an invisible drawing surface. CreateCompatibleDC(None)
    // creates one compatible with the screen but not connected to any window.
    let hdc = CreateCompatibleDC(None);
    assert!(!hdc.is_invalid(), "Failed to create memory DC");

    // ── Step 2: Create DIBSection ────────────────────────────────────────
    //
    // A DIBSection is a Windows bitmap whose pixel memory we can access
    // directly (unlike a DDB which lives in driver memory). We use:
    //   - 32 bits per pixel (BGRA)
    //   - Negative biHeight for top-down row order
    //   - BI_RGB (uncompressed)
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Negative = top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default()],
    };

    let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let hbitmap = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0);
    assert!(
        !hbitmap.is_err(),
        "Failed to create DIBSection"
    );
    let hbitmap = hbitmap.unwrap();
    assert!(!bits_ptr.is_null(), "DIBSection pixel pointer is null");

    // ── Step 3: Select bitmap into DC ────────────────────────────────────
    let old_bitmap = SelectObject(hdc, hbitmap);

    // ── Step 4: Clear to background colour ───────────────────────────────
    let (bg_r, bg_g, bg_b, _bg_a) = parse_hex_color(&scene.background);
    let bg_colorref = color_to_colorref(bg_r, bg_g, bg_b);
    let bg_brush = CreateSolidBrush(windows::Win32::Foundation::COLORREF(bg_colorref));
    let full_rect = RECT {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    };
    let _ = FillRect(hdc, &full_rect, bg_brush);
    let _ = DeleteObject(bg_brush);

    // ── Step 5: Dispatch PaintInstructions ────────────────────────────────
    render_instructions(hdc, &scene.instructions);

    // ── Step 6: Read back pixels (BGRA → RGBA) ──────────────────────────
    //
    // The DIBSection's pixel memory is at `bits_ptr`. Each pixel is 4 bytes
    // in BGRA order (Windows native). We need RGBA for PixelContainer.
    let total_bytes = (width as usize) * (height as usize) * 4;
    let bgra_slice = std::slice::from_raw_parts(bits_ptr as *const u8, total_bytes);
    let mut rgba_data = vec![0u8; total_bytes];

    for i in (0..total_bytes).step_by(4) {
        rgba_data[i] = bgra_slice[i + 2]; // R ← B position
        rgba_data[i + 1] = bgra_slice[i + 1]; // G stays
        rgba_data[i + 2] = bgra_slice[i]; // B ← R position
        rgba_data[i + 3] = 255; // GDI doesn't write alpha; force opaque
    }

    // ── Step 7: Cleanup GDI objects ──────────────────────────────────────
    let _ = SelectObject(hdc, old_bitmap);
    let _ = DeleteObject(hbitmap);
    let _ = DeleteDC(hdc);

    PixelContainer::from_data(width, height, rgba_data)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        PaintBase, PaintEllipse, PaintGroup, PaintInstruction, PaintPath, PaintRect, PaintScene,
        PathCommand,
    };

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ─── Colour parser tests ────────────────────────────────────────────────

    #[test]
    fn parse_hex_color_6_digit() {
        let (r, g, b, a) = parse_hex_color("#ff0000");
        assert!((r - 1.0).abs() < 0.01, "r should be 1.0");
        assert!((g - 0.0).abs() < 0.01, "g should be 0.0");
        assert!((b - 0.0).abs() < 0.01, "b should be 0.0");
        assert!((a - 1.0).abs() < 0.01, "a should be 1.0");
    }

    #[test]
    fn parse_hex_color_8_digit_with_alpha() {
        let (_r, _g, _b, a) = parse_hex_color("#00ff0080");
        // 0x80 = 128 → 128/255 ≈ 0.502
        assert!((a - 0.502).abs() < 0.01, "alpha should be ~0.502");
    }

    #[test]
    fn parse_hex_color_3_digit() {
        let (r, g, b, a) = parse_hex_color("#f00");
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
        assert!((a - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_css_rgb() {
        let (r, g, b, a) = parse_hex_color("rgb(255, 128, 0)");
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - (128.0 / 255.0)).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
        assert!((a - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_css_rgba() {
        let (r, g, b, a) = parse_hex_color("rgba(0, 255, 0, 0.5)");
        assert!((r - 0.0).abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);
        assert!((a - 0.5).abs() < 0.01);
    }

    #[test]
    fn parse_transparent() {
        let (r, g, b, a) = parse_hex_color("transparent");
        assert_eq!(a, 0.0);
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn parse_none_as_transparent() {
        let (r, g, b, a) = parse_hex_color("none");
        assert_eq!(a, 0.0);
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    // ─── COLORREF conversion tests ──────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn colorref_red() {
        // Pure red: R=255, G=0, B=0 → COLORREF 0x000000FF
        let cr = color_to_colorref(1.0, 0.0, 0.0);
        assert_eq!(cr, 0x000000FF);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn colorref_blue() {
        // Pure blue: R=0, G=0, B=255 → COLORREF 0x00FF0000
        let cr = color_to_colorref(0.0, 0.0, 1.0);
        assert_eq!(cr, 0x00FF0000);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn colorref_white() {
        let cr = color_to_colorref(1.0, 1.0, 1.0);
        assert_eq!(cr, 0x00FFFFFF);
    }

    // ─── Rendering tests (Windows only) ─────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn empty_scene_returns_empty_pixel_container() {
        let scene = PaintScene::new(0.0, 0.0);
        let pixels = render(&scene);
        assert_eq!(pixels.width, 0);
        assert_eq!(pixels.height, 0);
        assert!(pixels.data.is_empty());
    }

    /// Render a scene with a red rectangle on a white background.
    /// The pixel at the rectangle's centre should be red; the corner should be white.
    #[cfg(target_os = "windows")]
    #[test]
    fn render_red_rect_on_white() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene.instructions.push(PaintInstruction::Rect(
            PaintRect::filled(10.0, 10.0, 80.0, 80.0, "#ff0000"),
        ));

        let pixels = render(&scene);
        assert_eq!(pixels.width, 100);
        assert_eq!(pixels.height, 100);

        // Centre of the red rectangle should be red
        let (r, g, b, a) = pixels.pixel_at(50, 50);
        assert_eq!(r, 255, "red channel at centre");
        assert_eq!(g, 0, "green channel at centre");
        assert_eq!(b, 0, "blue channel at centre");
        assert_eq!(a, 255, "alpha at centre");

        // Top-left corner is outside the rect → white background
        let (r, g, b, a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 255, "red channel at corner (background)");
        assert_eq!(g, 255, "green channel at corner (background)");
        assert_eq!(b, 255, "blue channel at corner (background)");
        assert_eq!(a, 255, "alpha at corner (background)");
    }

    /// Render a scene with only a background colour (green) and no instructions.
    /// Every pixel should be green.
    #[cfg(target_os = "windows")]
    #[test]
    fn render_solid_green_background() {
        let mut scene = PaintScene::new(50.0, 50.0);
        scene.background = "#00ff00".to_string();

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 0, "green bg: r should be 0");
        assert_eq!(g, 255, "green bg: g should be 255");
        assert_eq!(b, 0, "green bg: b should be 0");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn render_css_rgb_background() {
        let mut scene = PaintScene::new(50.0, 50.0);
        scene.background = "rgb(255, 255, 255)".to_string();

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255, "rgb bg: r should be 255");
        assert_eq!(g, 255, "rgb bg: g should be 255");
        assert_eq!(b, 255, "rgb bg: b should be 255");
    }

    /// Transparent rects should not draw anything — background shows through.
    #[cfg(target_os = "windows")]
    #[test]
    fn transparent_rect_is_invisible() {
        let mut scene = PaintScene::new(50.0, 50.0);
        scene.instructions.push(PaintInstruction::Rect(
            PaintRect::filled(0.0, 0.0, 50.0, 50.0, "transparent"),
        ));

        let pixels = render(&scene);
        // Should be white background everywhere
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn rect_stroke_and_fill_render() {
        let mut scene = PaintScene::new(80.0, 80.0);
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 10.0,
            y: 10.0,
            width: 60.0,
            height: 60.0,
            fill: Some("#00ff00".to_string()),
            stroke: Some("#ff0000".to_string()),
            stroke_width: Some(2.0),
            corner_radius: Some(8.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(40, 40);
        assert_eq!(r, 0, "center should preserve fill color");
        assert_eq!(g, 255);
        assert_eq!(b, 0);

        let (r, g, b, _a) = pixels.pixel_at(10, 40);
        assert_eq!(r, 255, "left edge should draw stroke");
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn ellipse_fill_and_stroke_render() {
        let mut scene = PaintScene::new(80.0, 80.0);
        scene.instructions.push(PaintInstruction::Ellipse(PaintEllipse {
            base: PaintBase::default(),
            cx: 40.0,
            cy: 40.0,
            rx: 20.0,
            ry: 15.0,
            fill: Some("#0000ff".to_string()),
            stroke: Some("#ff0000".to_string()),
            stroke_width: Some(2.0),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(40, 40);
        assert_eq!(r, 0, "ellipse center should keep fill color");
        assert_eq!(g, 0);
        assert_eq!(b, 255);

        let (r, g, b, _a) = pixels.pixel_at(40, 25);
        assert_eq!(r, 255, "ellipse top edge should draw stroke");
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn stroked_path_with_none_fill_keeps_interior_clear() {
        let mut scene = PaintScene::new(50.0, 50.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 25.0, y: 5.0 },
                PathCommand::LineTo { x: 45.0, y: 45.0 },
                PathCommand::LineTo { x: 5.0, y: 45.0 },
                PathCommand::Close,
            ],
            fill: Some("none".to_string()),
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(1.0),
            stroke_cap: None,
            stroke_join: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255, "triangle interior should remain background");
        assert_eq!(g, 255);
        assert_eq!(b, 255);

        let (r, g, b, _a) = pixels.pixel_at(25, 5);
        assert_eq!(r, 0, "triangle edge should draw stroke");
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dashed_path_renders_gaps() {
        let mut scene = PaintScene::new(100.0, 40.0);
        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands: vec![
                PathCommand::MoveTo { x: 5.0, y: 20.0 },
                PathCommand::LineTo { x: 95.0, y: 20.0 },
            ],
            fill: Some("none".to_string()),
            fill_rule: None,
            stroke: Some("#000000".to_string()),
            stroke_width: Some(1.0),
            stroke_cap: None,
            stroke_join: None,
            stroke_dash: Some(vec![4.0, 4.0]),
            stroke_dash_offset: None,
        }));

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(7, 20);
        assert_eq!(r, 0, "first dash should draw");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b, _a) = pixels.pixel_at(11, 20);
        assert_eq!(r, 255, "gap between dashes should stay background");
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// Group should recurse into children and render both rects.
    #[cfg(target_os = "windows")]
    #[test]
    fn group_recurses_into_children() {
        let mut scene = PaintScene::new(100.0, 50.0);
        let group = PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![
                PaintInstruction::Rect(PaintRect::filled(0.0, 0.0, 50.0, 50.0, "#ff0000")),
                PaintInstruction::Rect(PaintRect::filled(50.0, 0.0, 50.0, 50.0, "#0000ff")),
            ],
            transform: None,
            opacity: None,
        });
        scene.instructions.push(group);

        let pixels = render(&scene);

        // Left half should be red
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255, "left half should be red");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        // Right half should be blue
        let (r, g, b, _a) = pixels.pixel_at(75, 25);
        assert_eq!(r, 0, "right half should be blue");
        assert_eq!(g, 0);
        assert_eq!(b, 255);
    }

    /// Clip should restrict drawing to the clip rectangle.
    #[cfg(target_os = "windows")]
    #[test]
    fn clip_restricts_drawing() {
        let mut scene = PaintScene::new(100.0, 100.0);
        // A red rect that fills the whole scene, clipped to the centre 50×50
        let clip = PaintInstruction::Clip(PaintClip {
            base: PaintBase::default(),
            x: 25.0,
            y: 25.0,
            width: 50.0,
            height: 50.0,
            children: vec![PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 100.0, 100.0, "#ff0000",
            ))],
        });
        scene.instructions.push(clip);

        let pixels = render(&scene);

        // Centre should be red (inside clip)
        let (r, g, b, _a) = pixels.pixel_at(50, 50);
        assert_eq!(r, 255, "centre should be red (inside clip)");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        // Corner should be white (outside clip)
        let (r, g, b, _a) = pixels.pixel_at(5, 5);
        assert_eq!(r, 255, "corner should be white background");
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// Barcode-style pattern: alternating black/white vertical bars.
    /// This is the primary e2e test for verifying barcode rendering works.
    #[cfg(target_os = "windows")]
    #[test]
    fn render_barcode_pattern() {
        let mut scene = PaintScene::new(200.0, 100.0);
        // 10 black bars (even indices), each 10px wide, 80px tall
        for i in 0..20u32 {
            if i % 2 == 0 {
                scene.instructions.push(PaintInstruction::Rect(
                    PaintRect::filled(i as f64 * 10.0, 0.0, 10.0, 80.0, "#000000"),
                ));
            }
        }

        let pixels = render(&scene);
        assert_eq!(pixels.width, 200);
        assert_eq!(pixels.height, 100);

        // Black bar at x=5 (centre of first bar), y=40 (middle height)
        let (r, g, b, _a) = pixels.pixel_at(5, 40);
        assert_eq!(r, 0, "black bar pixel should be black");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        // White gap at x=15 (centre of first gap), y=40
        let (r, g, b, _a) = pixels.pixel_at(15, 40);
        assert_eq!(r, 255, "white gap pixel should be white");
        assert_eq!(g, 255);
        assert_eq!(b, 255);

        // Below the bars at y=90 — should be white background
        let (r, g, b, _a) = pixels.pixel_at(5, 90);
        assert_eq!(r, 255, "below bars should be white background");
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// Render a QR-like checkerboard pattern (same as paint-metal's test).
    #[cfg(target_os = "windows")]
    #[test]
    fn render_black_modules_on_white() {
        let module_size = 4.0_f64;
        let mut scene = PaintScene::new(40.0, 40.0);

        for row in 0..4u32 {
            for col in 0..4u32 {
                if (row + col) % 2 == 0 {
                    scene.instructions.push(PaintInstruction::Rect(PaintRect::filled(
                        col as f64 * module_size,
                        row as f64 * module_size,
                        module_size,
                        module_size,
                        "#000000",
                    )));
                }
            }
        }

        let pixels = render(&scene);
        assert_eq!(pixels.width, 40);
        assert_eq!(pixels.height, 40);

        // Top-left module (0,0) is black
        let (r, g, b, _a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 0, "black module should have r=0");
        assert_eq!(g, 0, "black module should have g=0");
        assert_eq!(b, 0, "black module should have b=0");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn render_directwrite_glyph_run_draws_text_pixels() {
        use paint_instructions::{GlyphPosition, PaintGlyphRun};
        use text_interfaces::{FontQuery, FontResolver, ShapeOptions, TextShaper};
        use text_native::{NativeResolver, NativeShaper};

        let resolver = NativeResolver::new();
        let shaper = NativeShaper::new();
        let handle = resolver.resolve(&FontQuery::named("Segoe UI")).unwrap();
        let shaped = shaper
            .shape("Hi", &handle, 32.0, &ShapeOptions::default())
            .unwrap();
        let run = &shaped.runs[0];

        let mut x = 12.0;
        let glyphs: Vec<GlyphPosition> = run
            .glyphs
            .iter()
            .map(|g| {
                let positioned = GlyphPosition {
                    glyph_id: g.glyph_id,
                    x,
                    y: 46.0,
                };
                x += g.x_advance as f64;
                positioned
            })
            .collect();

        let mut scene = PaintScene::new(100.0, 64.0);
        scene
            .instructions
            .push(PaintInstruction::GlyphRun(PaintGlyphRun {
                base: PaintBase::default(),
                glyphs,
                font_ref: run.font_ref.clone(),
                font_size: 32.0,
                fill: Some("#000000".to_string()),
            }));

        let pixels = render(&scene);
        let dark_pixels = pixels
            .data
            .chunks_exact(4)
            .filter(|px| px[0] < 128 && px[1] < 128 && px[2] < 128 && px[3] > 0)
            .count();
        assert!(dark_pixels > 20, "expected visible glyph pixels");
    }
}
