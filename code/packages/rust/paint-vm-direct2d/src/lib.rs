//! # paint-vm-direct2d
//!
//! Direct2D GPU renderer for the paint-instructions scene model (P2D06).
//!
//! This crate takes a [`PaintScene`] (backend-neutral 2D paint instructions)
//! and renders it to a [`PixelContainer`] using Microsoft's Direct2D API —
//! a GPU-accelerated 2D rendering API available on Windows Vista and later.
//!
//! Direct2D is the modern renderer in the paint-* stack on Windows. It uses
//! the GPU for hardware-accelerated rendering with antialiasing. For a
//! simpler CPU-based fallback, see `paint-vm-gdi`.
//!
//! ## Current instruction support
//!
//! | Instruction       | Status                                              |
//! |-------------------|-----------------------------------------------------|
//! | `PaintRect`       | Fully implemented — solid-colour filled rects       |
//! | `PaintLine`       | Fully implemented — DrawLine with stroke width      |
//! | `PaintGroup`      | Fully implemented — recurses into children          |
//! | `PaintClip`       | Fully implemented — PushAxisAlignedClip / Pop       |
//! | `PaintGlyphRun`   | Planned — IDWriteFactory + DrawGlyphRun             |
//! | `PaintEllipse`    | Planned — FillEllipse                               |
//! | `PaintPath`       | Planned — ID2D1PathGeometry                         |
//! | `PaintLayer`      | Planned — PushLayer / PopLayer                      |
//! | `PaintGradient`   | Planned — CreateLinearGradientBrush                 |
//! | `PaintImage`      | Planned — ID2D1Bitmap from PixelContainer           |
//!
//! ## Direct2D pipeline (offscreen, no HWND)
//!
//! ```text
//! PaintScene
//!   │
//!   ├── 1. CoInitializeEx() — COM single-threaded apartment
//!   ├── 2. D2D1CreateFactory() → ID2D1Factory
//!   ├── 3. CoCreateInstance(CLSID_WICImagingFactory) → IWICImagingFactory
//!   ├── 4. CreateBitmap() → IWICBitmap (offscreen RGBA target)
//!   ├── 5. CreateWicBitmapRenderTarget() → ID2D1RenderTarget
//!   ├── 6. BeginDraw → Clear(background) → dispatch instructions → EndDraw
//!   ├── 7. Lock WIC bitmap → read premultiplied BGRA pixels
//!   └── 8. Convert pBGRA → RGBA → PixelContainer
//! ```
//!
//! ## Coordinate system
//!
//! `PaintScene` uses a **top-left origin** with Y increasing downward
//! (same as SVG, HTML Canvas, and CSS).
//!
//! Direct2D also uses a top-left origin with Y increasing downward by
//! default — so no coordinate conversion is needed (unlike Metal which
//! requires NDC conversion).
//!
//! ```text
//!  Scene coordinates:       Direct2D coordinates:
//!  (0,0)──────(w,0)        (0,0)──────(w,0)
//!    │              │           │              │
//!    │              │           │              │
//!    │              │           │              │
//!  (0,h)──────(w,h)        (0,h)──────(w,h)
//! ```
//!
//! ## Premultiplied alpha
//!
//! Direct2D renders to premultiplied BGRA (pBGRA). In premultiplied alpha,
//! each colour channel is pre-scaled by the alpha value:
//!
//! ```text
//! Straight:       (R, G, B, A) = (255, 0, 0, 128) — half-transparent red
//! Premultiplied:  (R, G, B, A) = (128, 0, 0, 128) — R scaled by A/255
//! ```
//!
//! When reading back pixels, we un-premultiply and swap BGRA→RGBA.

pub const VERSION: &str = "0.1.0";

use paint_instructions::{
    PaintClip, PaintInstruction, PaintLine, PaintRect, PaintScene, PixelContainer,
};

// ---------------------------------------------------------------------------
// Platform gate
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
compile_error!(
    "paint-vm-direct2d requires Windows. Use paint-metal on macOS or paint-vm-cairo on Linux."
);

// ---------------------------------------------------------------------------
// Windows API imports
// ---------------------------------------------------------------------------
//
// Direct2D is a COM-based API. All interactions go through COM interfaces:
//   - ID2D1Factory: creates render targets and geometry objects
//   - ID2D1RenderTarget: the drawing surface (backed by a WIC bitmap here)
//   - ID2D1SolidColorBrush: a brush that paints a single colour
//   - IWICImagingFactory: creates WIC bitmaps for offscreen rendering
//   - IWICBitmap: a CPU-accessible bitmap that Direct2D can render into

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F,
};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1RenderTarget, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, IWICBitmap, IWICImagingFactory, WICBitmapCacheOnLoad,
    WICBitmapLockRead,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
#[cfg(target_os = "windows")]
use windows::core::GUID;

// ---------------------------------------------------------------------------
// Colour parsing
// ---------------------------------------------------------------------------

/// Parse a hex colour string to RGBA floats in the range 0.0–1.0.
///
/// Supported formats:
/// - `"#rrggbb"`   → (r, g, b, 1.0)
/// - `"#rrggbbaa"` → (r, g, b, a)
/// - `"#rgb"`      → expanded to `#rrggbb`
/// - `"transparent"` / anything else → (0.0, 0.0, 0.0, 0.0)
fn parse_hex_color(s: &str) -> (f64, f64, f64, f64) {
    if s == "transparent" {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let hex = s.trim_start_matches('#');
    let hex = if hex.len() == 3 {
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

/// Convert RGBA floats to a Direct2D [`D2D1_COLOR_F`].
///
/// Direct2D uses float4 colours in the range 0.0–1.0, same as our parsed values.
#[cfg(target_os = "windows")]
fn to_d2d_color(r: f64, g: f64, b: f64, a: f64) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32,
        g: g as f32,
        b: b as f32,
        a: a as f32,
    }
}

// ---------------------------------------------------------------------------
// Instruction dispatch — PaintInstruction → Direct2D calls
// ---------------------------------------------------------------------------

/// Render a list of [`PaintInstruction`]s into a Direct2D render target.
///
/// This is the core dispatch loop. It recursively handles Group and Clip
/// nodes, and dispatches Rect and Line to their respective D2D calls.
#[cfg(target_os = "windows")]
unsafe fn render_instructions(rt: &ID2D1RenderTarget, instructions: &[PaintInstruction]) {
    for instr in instructions {
        match instr {
            PaintInstruction::Rect(rect) => render_rect(rt, rect),
            PaintInstruction::Line(line) => render_line(rt, line),
            PaintInstruction::Group(group) => {
                // PaintGroup: render children directly into the same target.
                // Transform support (SetTransform) is deferred — for barcodes,
                // groups are purely logical containers.
                render_instructions(rt, &group.children);
            }
            PaintInstruction::Clip(clip) => render_clip(rt, clip),
            // Planned but not yet implemented:
            PaintInstruction::GlyphRun(_)
            | PaintInstruction::Ellipse(_)
            | PaintInstruction::Path(_)
            | PaintInstruction::Layer(_)
            | PaintInstruction::Gradient(_)
            | PaintInstruction::Image(_) => {
                // No-op for now. Barcodes only need Rect/Line/Group/Clip.
            }
        }
    }
}

/// Render a [`PaintRect`] as a filled rectangle.
///
/// Direct2D's `FillRectangle` takes a `D2D_RECT_F` (left, top, right, bottom)
/// and a brush. We create a temporary `ID2D1SolidColorBrush` for each rect.
///
/// ```text
/// (left, top) ────── (right, top)
///      │                    │
///      │   FillRectangle    │
///      │                    │
/// (left, bottom) ── (right, bottom)
/// ```
#[cfg(target_os = "windows")]
unsafe fn render_rect(rt: &ID2D1RenderTarget, rect: &PaintRect) {
    let fill = rect.fill.as_deref().unwrap_or("transparent");
    let (r, g, b, a) = parse_hex_color(fill);
    if a == 0.0 {
        return;
    }

    let color = to_d2d_color(r, g, b, a);
    let brush = rt
        .CreateSolidColorBrush(&color as *const _, None)
        .expect("Failed to create solid colour brush");

    let d2d_rect = D2D_RECT_F {
        left: rect.x as f32,
        top: rect.y as f32,
        right: (rect.x + rect.width) as f32,
        bottom: (rect.y + rect.height) as f32,
    };

    rt.FillRectangle(&d2d_rect as *const _, &brush);
}

/// Render a [`PaintLine`] using Direct2D's `DrawLine`.
///
/// `DrawLine` takes two `D2D_POINT_2F` endpoints, a brush, and a stroke width.
/// Direct2D handles the perpendicular expansion internally (unlike paint-metal
/// which manually constructs a thin rectangle from triangle vertices).
#[cfg(target_os = "windows")]
unsafe fn render_line(rt: &ID2D1RenderTarget, line: &PaintLine) {
    let (r, g, b, a) = parse_hex_color(&line.stroke);
    if a == 0.0 {
        return;
    }

    let color = to_d2d_color(r, g, b, a);
    let brush = rt
        .CreateSolidColorBrush(&color as *const _, None)
        .expect("Failed to create solid colour brush for line");

    let p0 = D2D_POINT_2F {
        x: line.x1 as f32,
        y: line.y1 as f32,
    };
    let p1 = D2D_POINT_2F {
        x: line.x2 as f32,
        y: line.y2 as f32,
    };
    let stroke_width = line.stroke_width.unwrap_or(1.0) as f32;

    rt.DrawLine(p0, p1, &brush, stroke_width, None);
}

/// Render a [`PaintClip`] using Direct2D's axis-aligned clip.
///
/// Direct2D clip flow:
/// 1. `PushAxisAlignedClip()` — restricts drawing to the clip rectangle
/// 2. Render children
/// 3. `PopAxisAlignedClip()` — restores the previous clip
///
/// Nested clips are intersected automatically by Direct2D.
#[cfg(target_os = "windows")]
unsafe fn render_clip(rt: &ID2D1RenderTarget, clip: &PaintClip) {
    let clip_rect = D2D_RECT_F {
        left: clip.x as f32,
        top: clip.y as f32,
        right: (clip.x + clip.width) as f32,
        bottom: (clip.y + clip.height) as f32,
    };

    rt.PushAxisAlignedClip(&clip_rect as *const _, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    render_instructions(rt, &clip.children);
    rt.PopAxisAlignedClip();
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a [`PaintScene`] to a [`PixelContainer`] using Direct2D.
///
/// This is the main entry point for the `paint-vm-direct2d` crate.
///
/// ## Pipeline
///
/// 1. Initialize COM (CoInitializeEx)
/// 2. Create ID2D1Factory and IWICImagingFactory
/// 3. Create an offscreen WIC bitmap (premultiplied BGRA)
/// 4. Create a WIC bitmap render target
/// 5. BeginDraw → Clear → dispatch instructions → EndDraw
/// 6. Lock the WIC bitmap and read premultiplied BGRA pixels
/// 7. Un-premultiply and convert BGRA→RGBA
/// 8. Return as `PixelContainer`
///
/// ## Requires
///
/// - Windows Vista or later (Direct2D is not available on Windows XP)
/// - A GPU is used if available; falls back to WARP software rasteriser
///
/// ## Chaining with a codec
///
/// ```rust,ignore
/// let scene = barcode_2d::layout(&grid, &config);
/// let pixels = paint_vm_direct2d::render(&scene);
/// let png = paint_codec_png::encode_png(&pixels);
/// std::fs::write("barcode.png", png).unwrap();
/// ```
#[cfg(target_os = "windows")]
pub fn render(scene: &PaintScene) -> PixelContainer {
    let width = scene.width as u32;
    let height = scene.height as u32;

    if width == 0 || height == 0 {
        return PixelContainer::new(width, height);
    }

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

/// The actual rendering logic, wrapped in `unsafe` for COM/D2D FFI calls.
///
/// ## WIC bitmap pixel format
///
/// We use `GUID_WICPixelFormat32bppPBGRA` — premultiplied BGRA. This is
/// the native format Direct2D renders to. After reading back the pixels,
/// we un-premultiply the alpha and swap BGRA→RGBA for PixelContainer.
///
/// ### Premultiplied → straight alpha conversion
///
/// ```text
/// if alpha > 0:
///     R_straight = R_premul * 255 / alpha
///     G_straight = G_premul * 255 / alpha
///     B_straight = B_premul * 255 / alpha
/// else:
///     R = G = B = 0  (fully transparent pixel)
/// ```
#[cfg(target_os = "windows")]
unsafe fn render_unsafe(scene: &PaintScene, width: u32, height: u32) -> PixelContainer {
    // ── Step 1: Initialize COM ───────────────────────────────────────────
    //
    // COM must be initialized on every thread that uses COM objects.
    // COINIT_MULTITHREADED is used because we don't need a message loop.
    // If COM is already initialized, this returns S_FALSE (which is OK).
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

    // ── Step 2: Create factories ─────────────────────────────────────────
    //
    // ID2D1Factory: creates render targets, geometries, etc.
    // IWICImagingFactory: creates WIC bitmaps for offscreen rendering.
    let d2d_factory: ID2D1Factory =
        D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
            .expect("Failed to create D2D1 factory");

    let wic_factory: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
            .expect("Failed to create WIC imaging factory");

    // ── Step 3: Create offscreen WIC bitmap ──────────────────────────────
    //
    // The WIC bitmap is our offscreen render surface. Direct2D renders
    // into it, and we read back pixels from it when done.
    //
    // GUID_WICPixelFormat32bppPBGRA = premultiplied BGRA, 32 bits per pixel
    let pixel_format_guid = GUID::from_u128(0x6fddc324_4e03_4bfe_b185_3d77768dc910);
    let wic_bitmap: IWICBitmap = wic_factory
        .CreateBitmap(width, height, &pixel_format_guid, WICBitmapCacheOnLoad)
        .expect("Failed to create WIC bitmap");

    // ── Step 4: Create render target ─────────────────────────────────────
    //
    // A WIC bitmap render target draws into the WIC bitmap. This is an
    // offscreen rendering path — no window or HWND is involved.
    let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: Default::default(),
    };

    let render_target: ID2D1RenderTarget = d2d_factory
        .CreateWicBitmapRenderTarget(&wic_bitmap, &rt_props as *const _)
        .expect("Failed to create WIC bitmap render target");

    // ── Step 5: Render ───────────────────────────────────────────────────
    //
    // BeginDraw/EndDraw bracket all drawing operations. Clear sets the
    // entire surface to the background colour. Then we dispatch each
    // PaintInstruction to the appropriate D2D call.
    let (bg_r, bg_g, bg_b, bg_a) = parse_hex_color(&scene.background);
    let bg_color = to_d2d_color(bg_r, bg_g, bg_b, bg_a);

    render_target.BeginDraw();
    render_target.Clear(Some(&bg_color as *const _));
    render_instructions(&render_target, &scene.instructions);
    render_target
        .EndDraw(None, None)
        .expect("EndDraw failed");

    // ── Step 6: Read back pixels ─────────────────────────────────────────
    //
    // Lock the WIC bitmap to get a pointer to the pixel data. The pixels
    // are in premultiplied BGRA format. We convert to straight RGBA.
    let lock = wic_bitmap
        .Lock(std::ptr::null(), WICBitmapLockRead.0 as u32)
        .expect("Failed to lock WIC bitmap");

    let stride = lock.GetStride().expect("Failed to get stride");

    let mut data_size: u32 = 0;
    let mut data_ptr: *mut u8 = std::ptr::null_mut();
    lock.GetDataPointer(&mut data_size, &mut data_ptr)
        .expect("Failed to get data pointer");

    let pbgra_slice = std::slice::from_raw_parts(data_ptr, data_size as usize);
    let total_pixels = (width as usize) * (height as usize);
    let mut rgba_data = vec![0u8; total_pixels * 4];

    // ── Step 7: Convert premultiplied BGRA → straight RGBA ──────────────
    //
    // WIC bitmap rows may have padding (stride > width*4). We must read
    // row by row using the stride, not assume contiguous pixel data.
    for row in 0..height as usize {
        let row_start = row * stride as usize;
        for col in 0..width as usize {
            let src_offset = row_start + col * 4;
            let dst_offset = (row * width as usize + col) * 4;

            let pb = pbgra_slice[src_offset]; // premultiplied B
            let pg = pbgra_slice[src_offset + 1]; // premultiplied G
            let pr = pbgra_slice[src_offset + 2]; // premultiplied R
            let a = pbgra_slice[src_offset + 3]; // alpha

            // Un-premultiply: straight_channel = premul_channel * 255 / alpha
            if a == 0 {
                rgba_data[dst_offset] = 0;
                rgba_data[dst_offset + 1] = 0;
                rgba_data[dst_offset + 2] = 0;
                rgba_data[dst_offset + 3] = 0;
            } else if a == 255 {
                // Fully opaque — no division needed, just swap B↔R
                rgba_data[dst_offset] = pr; // R
                rgba_data[dst_offset + 1] = pg; // G
                rgba_data[dst_offset + 2] = pb; // B
                rgba_data[dst_offset + 3] = 255; // A
            } else {
                // General case: un-premultiply
                let a_f = a as f32;
                rgba_data[dst_offset] = ((pr as f32 * 255.0 / a_f).round() as u8).min(255);
                rgba_data[dst_offset + 1] = ((pg as f32 * 255.0 / a_f).round() as u8).min(255);
                rgba_data[dst_offset + 2] = ((pb as f32 * 255.0 / a_f).round() as u8).min(255);
                rgba_data[dst_offset + 3] = a;
            }
        }
    }

    PixelContainer::from_data(width, height, rgba_data)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{PaintBase, PaintGroup, PaintInstruction, PaintRect, PaintScene};

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
    fn parse_transparent() {
        let (r, g, b, a) = parse_hex_color("transparent");
        assert_eq!(a, 0.0);
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    // ─── D2D colour conversion tests ────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn d2d_color_red() {
        let c = to_d2d_color(1.0, 0.0, 0.0, 1.0);
        assert!((c.r - 1.0).abs() < 0.001);
        assert!((c.g - 0.0).abs() < 0.001);
        assert!((c.b - 0.0).abs() < 0.001);
        assert!((c.a - 1.0).abs() < 0.001);
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

    /// Render a solid green background with no instructions.
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

    /// Transparent rects should not draw anything.
    #[cfg(target_os = "windows")]
    #[test]
    fn transparent_rect_is_invisible() {
        let mut scene = PaintScene::new(50.0, 50.0);
        scene.instructions.push(PaintInstruction::Rect(
            PaintRect::filled(0.0, 0.0, 50.0, 50.0, "transparent"),
        ));

        let pixels = render(&scene);
        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255);
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

        let (r, g, b, _a) = pixels.pixel_at(25, 25);
        assert_eq!(r, 255, "left half should be red");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

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
    #[cfg(target_os = "windows")]
    #[test]
    fn render_barcode_pattern() {
        let mut scene = PaintScene::new(200.0, 100.0);
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

        let (r, g, b, _a) = pixels.pixel_at(5, 40);
        assert_eq!(r, 0, "black bar pixel should be black");
        assert_eq!(g, 0);
        assert_eq!(b, 0);

        let (r, g, b, _a) = pixels.pixel_at(15, 40);
        assert_eq!(r, 255, "white gap pixel should be white");
        assert_eq!(g, 255);
        assert_eq!(b, 255);

        let (r, g, b, _a) = pixels.pixel_at(5, 90);
        assert_eq!(r, 255, "below bars should be white background");
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    /// QR-like checkerboard pattern (same as paint-metal's test).
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

        let (r, g, b, _a) = pixels.pixel_at(2, 2);
        assert_eq!(r, 0, "black module should have r=0");
        assert_eq!(g, 0, "black module should have g=0");
        assert_eq!(b, 0, "black module should have b=0");
    }
}
