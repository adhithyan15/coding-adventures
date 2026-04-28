//! # markdown-reader
//!
//! Native Markdown viewer. End-to-end binary that exercises the
//! whole coding-adventures text stack for the first time:
//!
//! ```text
//!   Markdown string
//!      ↓  commonmark-parser
//!   DocumentNode
//!      ↓  document-ast-to-layout + DocumentTheme
//!   LayoutNode tree
//!      ↓  layout-block + layout-text-measure-native
//!   PositionedNode tree
//!      ↓  layout-to-paint + the same CoreText trio
//!   PaintScene (with pre-shaped PaintGlyphRun instructions)
//!      ↓  platform paint backend
//!   PixelContainer
//!      ↓  NSImageView in an NSWindow
//!   visible text on screen
//! ```
//!
//! Usage:
//!
//! ```text
//!   markdown-reader [path/to/file.md]
//! ```
//!
//! With no argument, renders a built-in sample document. This keeps
//! the v1 demo path self-contained — a user can `cargo run --bin
//! markdown-reader` and immediately see Markdown rendered through
//! the full pipeline.

use std::env;
use std::fs;

use commonmark_parser::parse as parse_markdown;
use document_ast_to_layout::{document_ast_to_layout, document_default_theme};
use layout_block::layout_block;
use layout_ir::constraints_width;
use layout_text_measure_native::NativeMeasurer;
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::PaintScene;
use text_interfaces::{FontMetrics, FontResolver, TextShaper};
use text_native::{NativeMetrics, NativeResolver, NativeShaper};

// Private re-export namespace: the TXT00 trait types come from
// text-interfaces, but text-native re-exports them.
use text_native::text_interfaces;

const WINDOW_WIDTH: f64 = 800.0;
const WINDOW_HEIGHT: f64 = 700.0;

const SAMPLE_MARKDOWN: &str = r#"# Hello, Markdown!

This is a **native** Markdown viewer. Every pixel you see on this window
went through the coding-adventures stack end to end: parse → AST → layout
→ paint → platform pixels.

## Features

- Pure Rust implementation.
- Native OS text shaping.
- Paint VM for device-independent scene model.
- Built from first principles - no HarfBuzz, no Pango, no WebKit.

### Supported today

- Headings (h1-h6)
- Paragraphs with word-wrap
- Unordered and ordered lists
- Blockquotes
- Code blocks

### Coming soon

- Inline `bold`, *italic*, and links (v1 flattens these to plain text).
- Markdown tables.
- Live resize.

> The architecture is the point. Beauty is v2.
"#;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse args: `--png <path>` → render to PNG instead of window.
    // All other positionals are treated as an input Markdown file.
    let mut png_out: Option<String> = None;
    let mut markdown_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--png" => {
                if i + 1 >= args.len() {
                    eprintln!("markdown-reader: --png requires a path argument");
                    std::process::exit(2);
                }
                png_out = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                markdown_path = Some(other.to_string());
                i += 1;
            }
        }
    }

    let markdown = match markdown_path {
        Some(path) => match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("markdown-reader: failed to read {}: {}", path, e);
                SAMPLE_MARKDOWN.to_string()
            }
        },
        None => SAMPLE_MARKDOWN.to_string(),
    };

    // Full pipeline: parse → layout → paint → pixels.
    let scene = render_markdown_to_scene(&markdown);

    #[cfg(target_os = "windows")]
    {
        let pixels = paint_vm_direct2d::render(&scene);
        eprintln!(
            "markdown-reader: rendered {}Ã—{} pixels ({} paint instructions) via Direct2D",
            pixels.width,
            pixels.height,
            scene.instructions.len()
        );
        if let Some(path) = png_out {
            match paint_codec_png::write_png(&pixels, &path) {
                Ok(()) => {
                    eprintln!("markdown-reader: wrote {}", path);
                    return;
                }
                Err(e) => {
                    eprintln!("markdown-reader: failed to write {}: {}", path, e);
                    std::process::exit(1);
                }
            }
        }
        unsafe {
            paint_vm_direct2d::show_scene_in_window(&scene, "Markdown Reader - Direct2D");
        }
        return;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let pixels = paint_metal::render(&scene);
        eprintln!(
            "markdown-reader: rendered {}×{} pixels ({} paint instructions)",
            pixels.width,
            pixels.height,
            scene.instructions.len()
        );

        if let Some(path) = png_out {
            match paint_codec_png::write_png(&pixels, &path) {
                Ok(()) => {
                    eprintln!("markdown-reader: wrote {}", path);
                    return;
                }
                Err(e) => {
                    eprintln!("markdown-reader: failed to write {}: {}", path, e);
                    std::process::exit(1);
                }
            }
        }

        #[cfg(target_vendor = "apple")]
        unsafe {
            show_in_window(&pixels, "Markdown Reader");
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            eprintln!("markdown-reader: use --png <path> on non-Apple targets.");
        }
    }
}

fn render_markdown_to_scene(markdown: &str) -> PaintScene {
    // Step 1. Parse into a DocumentNode tree.
    let doc = parse_markdown(markdown);

    // Step 2. Convert into a LayoutNode tree with a default theme.
    let theme = document_default_theme();
    let layout_root = document_ast_to_layout(&doc, &theme);

    // Step 3. Lay out using layout-block with a native text measurer.
    let measurer = NativeMeasurer::new();
    let constraints = constraints_width(WINDOW_WIDTH);
    let positioned = layout_block(&layout_root, constraints, &measurer);

    // Step 4. Convert to a PaintScene. The shaper/metrics/resolver
    //        trio MUST match a single font binding — by using the
    //        NativeResolver/NativeMetrics/NativeShaper triple they
    //        all share one backend-specific `Handle`.
    let resolver = NativeResolver::new();
    let metrics = NativeMetrics::new();
    let shaper = NativeShaper::new();

    // Help the type checker concretely specialize the call below.
    let _: &dyn FontResolver<Handle = _> = &resolver;
    let _: &dyn FontMetrics<Handle = _> = &metrics;
    let _: &dyn TextShaper<Handle = _> = &shaper;

    let options = LayoutToPaintOptions {
        width: WINDOW_WIDTH,
        height: WINDOW_HEIGHT,
        background: layout_ir::color_white(),
        device_pixel_ratio: 1.0,
        shaper: &shaper,
        metrics: &metrics,
        resolver: &resolver,
    };
    layout_to_paint(&positioned, &options)
}

// ---------------------------------------------------------------------------
// Windows Direct2D window display
// ---------------------------------------------------------------------------
//
// This is the first Windows-native Markdown slice. It intentionally renders
// Markdown text straight through Direct2D/DirectWrite so the Direct2D window
// path is usable before the full TXT03b DirectWrite shaper and PaintGlyphRun
// registry are in place.

#[cfg(any())]
static DIRECT2D_MARKDOWN: std::sync::OnceLock<String> = std::sync::OnceLock::new();

#[cfg(any())]
unsafe fn show_direct2d_markdown_window(markdown: &str, title: &str) {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH};
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW,
        RegisterClassW, ShowWindow, TranslateMessage, UpdateWindow, CS_HREDRAW, CS_VREDRAW,
        CW_USEDEFAULT, HMENU, IDC_ARROW, MSG, SW_SHOW, WINDOW_EX_STYLE, WM_DESTROY, WM_PAINT,
        WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    let _ = DIRECT2D_MARKDOWN.set(markdown.to_string());
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

    let instance = GetModuleHandleW(None).expect("GetModuleHandleW failed");
    let class_name = w!("CodingAdventuresMarkdownReaderDirect2D");

    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(direct2d_window_proc),
        hInstance: instance.into(),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
        lpszClassName: class_name,
        ..Default::default()
    };
    RegisterClassW(&wc);

    let title_wide = wide_null(title);
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        class_name,
        PCWSTR(title_wide.as_ptr()),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        WINDOW_WIDTH as i32,
        WINDOW_HEIGHT as i32,
        HWND::default(),
        HMENU::default(),
        instance,
        None,
    );
    assert!(!hwnd.is_invalid(), "CreateWindowExW failed");

    ShowWindow(hwnd, SW_SHOW);
    let _ = UpdateWindow(hwnd);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    unsafe extern "system" fn direct2d_window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                paint_direct2d_window(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(any())]
unsafe fn paint_direct2d_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Direct2D::Common::{
        D2D1_ALPHA_MODE_UNKNOWN, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
    };
    use windows::Win32::Graphics::Direct2D::{
        D2D1CreateFactory, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
        D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE,
        D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
        D2D1_RENDER_TARGET_USAGE_NONE,
    };
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
        DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
        DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL,
    };
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;
    use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, GetClientRect, PAINTSTRUCT};

    let mut ps = PAINTSTRUCT::default();
    BeginPaint(hwnd, &mut ps);

    let mut rc = RECT::default();
    if GetClientRect(hwnd, &mut rc).is_ok() {
        let width = (rc.right - rc.left).max(1) as u32;
        let height = (rc.bottom - rc.top).max(1) as u32;

        let d2d_factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).expect("D2D factory");
        let dwrite_factory: IDWriteFactory =
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");

        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_UNKNOWN,
                alphaMode: D2D1_ALPHA_MODE_UNKNOWN,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: Default::default(),
        };
        let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd,
            pixelSize: D2D_SIZE_U { width, height },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        let target = d2d_factory
            .CreateHwndRenderTarget(&rt_props, &hwnd_props)
            .expect("D2D HWND render target");

        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }));

        let text_brush = target
            .CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.08,
                    g: 0.09,
                    b: 0.11,
                    a: 1.0,
                },
                None,
            )
            .expect("D2D text brush");

        let accent_brush = target
            .CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.86,
                    g: 0.90,
                    b: 0.96,
                    a: 1.0,
                },
                None,
            )
            .expect("D2D accent brush");

        let mut y = 24.0_f32;
        let content_width = (width as f32 - 48.0).max(120.0);
        let markdown = DIRECT2D_MARKDOWN.get().map(String::as_str).unwrap_or("");
        for line in markdown_blocks(markdown) {
            let size = line.size;
            let weight = if line.bold {
                DWRITE_FONT_WEIGHT_BOLD
            } else {
                DWRITE_FONT_WEIGHT_NORMAL
            };
            let format = create_text_format(&dwrite_factory, size, weight);
            let wide = wide_null(&line.text);
            let left = 24.0 + line.indent;
            let rect = D2D_RECT_F {
                left,
                top: y + line.margin_top,
                right: left + content_width - line.indent,
                bottom: y + line.margin_top + line.box_height,
            };
            if line.accent {
                target.FillRectangle(
                    &D2D_RECT_F {
                        left: 18.0,
                        top: rect.top - 4.0,
                        right: rect.right + 6.0,
                        bottom: rect.bottom,
                    },
                    &accent_brush,
                );
            }
            target.DrawText(
                &wide[..wide.len().saturating_sub(1)],
                &format,
                &rect,
                &text_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
            y = rect.bottom + line.margin_bottom;
            if y > height as f32 {
                break;
            }
        }

        target.EndDraw(None, None).expect("D2D EndDraw");
    }

    EndPaint(hwnd, &ps);

    unsafe fn create_text_format(
        factory: &IDWriteFactory,
        size: f32,
        weight: DWRITE_FONT_WEIGHT,
    ) -> windows::Win32::Graphics::DirectWrite::IDWriteTextFormat {
        let family = wide_null("Segoe UI");
        let locale = wide_null("en-us");
        factory
            .CreateTextFormat(
                windows::core::PCWSTR(family.as_ptr()),
                None,
                weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                windows::core::PCWSTR(locale.as_ptr()),
            )
            .expect("DWrite text format")
    }
}

#[cfg(any())]
#[derive(Debug)]
struct MarkdownDrawLine {
    text: String,
    size: f32,
    bold: bool,
    indent: f32,
    margin_top: f32,
    margin_bottom: f32,
    box_height: f32,
    accent: bool,
}

#[cfg(any())]
fn markdown_blocks(markdown: &str) -> Vec<MarkdownDrawLine> {
    let mut out = Vec::new();
    let mut in_code = false;
    for raw in markdown.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if trimmed.is_empty() {
            out.push(MarkdownDrawLine {
                text: String::new(),
                size: 12.0,
                bold: false,
                indent: 0.0,
                margin_top: 0.0,
                margin_bottom: 8.0,
                box_height: 4.0,
                accent: false,
            });
            continue;
        }

        let (text, size, bold, indent, margin_top, margin_bottom, accent) = if in_code {
            (raw.to_string(), 14.0, false, 18.0, 4.0, 6.0, true)
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            (
                strip_inline_markup(rest),
                32.0,
                true,
                0.0,
                10.0,
                12.0,
                false,
            )
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            (strip_inline_markup(rest), 24.0, true, 0.0, 8.0, 10.0, false)
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            (strip_inline_markup(rest), 19.0, true, 0.0, 6.0, 8.0, false)
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            (strip_inline_markup(rest), 16.0, false, 18.0, 4.0, 8.0, true)
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            (
                format!("• {}", strip_inline_markup(rest)),
                16.0,
                false,
                20.0,
                2.0,
                6.0,
                false,
            )
        } else if let Some(rest) = ordered_list_body(trimmed) {
            (
                format!("• {}", strip_inline_markup(rest)),
                16.0,
                false,
                20.0,
                2.0,
                6.0,
                false,
            )
        } else {
            (
                strip_inline_markup(trimmed),
                16.0,
                false,
                0.0,
                2.0,
                8.0,
                false,
            )
        };

        out.push(MarkdownDrawLine {
            text,
            size,
            bold,
            indent,
            margin_top,
            margin_bottom,
            box_height: (size * 1.55).max(24.0),
            accent,
        });
    }
    out
}

#[cfg(any())]
fn ordered_list_body(line: &str) -> Option<&str> {
    let dot = line.find('.')?;
    if dot == 0 || !line[..dot].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(line[dot + 1..].trim_start())
}

#[cfg(any())]
fn strip_inline_markup(input: &str) -> String {
    input
        .replace("**", "")
        .replace('*', "")
        .replace('`', "")
        .replace('[', "")
        .replace("](", " (")
        .replace(')', "")
}

#[cfg(any())]
fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------------------------------------------------------------------------
// macOS window display
// ---------------------------------------------------------------------------
//
// Working pattern: create an NSApplication, build an NSWindow at the
// scene's dimensions, convert the PixelContainer into an NSImage backed
// by NSBitmapImageRep, display via NSImageView as the window's content
// view, run the event loop until the user closes the window.
//
// v1 is static: one render, one display. No resize-triggered relayout.
// Handled as a v2 concern.

#[cfg(target_vendor = "apple")]
unsafe fn show_in_window(pixels: &paint_instructions::PixelContainer, title: &str) {
    use objc_bridge::{
        class, msg, msg_send_class, nsstring, release, CFRelease, CGPoint, CGRect, CGSize, Id, NIL,
        NS_BACKING_STORE_BUFFERED, NS_WINDOW_STYLE_MASK_CLOSABLE,
        NS_WINDOW_STYLE_MASK_MINIATURIZABLE, NS_WINDOW_STYLE_MASK_RESIZABLE,
        NS_WINDOW_STYLE_MASK_TITLED,
    };
    use std::ffi::{c_int, c_ulong};

    let app_class = class("NSApplication");
    let app: Id = msg_send_class(app_class, "sharedApplication");
    // NSApplicationActivationPolicyRegular = 0 (app appears in Dock)
    msg!(app, "setActivationPolicy:", 0 as c_ulong);

    let w = pixels.width.max(200) as f64;
    let h = pixels.height.max(100) as f64;
    let frame = CGRect {
        origin: CGPoint { x: 200.0, y: 200.0 },
        size: CGSize {
            width: w,
            height: h,
        },
    };

    let style_mask = NS_WINDOW_STYLE_MASK_TITLED
        | NS_WINDOW_STYLE_MASK_CLOSABLE
        | NS_WINDOW_STYLE_MASK_MINIATURIZABLE
        | NS_WINDOW_STYLE_MASK_RESIZABLE;

    let window_class = class("NSWindow");
    let window: Id = msg!(
        msg_send_class(window_class, "alloc"),
        "initWithContentRect:styleMask:backing:defer:",
        frame,
        style_mask,
        NS_BACKING_STORE_BUFFERED,
        false as c_int
    );
    assert!(!window.is_null(), "NSWindow allocation failed");

    let title_ns = nsstring(title);
    msg!(window, "setTitle:", title_ns);
    CFRelease(title_ns);

    // Build an NSImage wrapping the pixel buffer.
    let image = create_nsimage_from_pixels(pixels);

    let image_view_class = class("NSImageView");
    let image_view: Id = msg!(
        msg_send_class(image_view_class, "alloc"),
        "initWithFrame:",
        CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: w,
                height: h,
            },
        }
    );
    msg!(image_view, "setImage:", image);
    // NSImageScaleProportionallyUpOrDown = 3 — fills the window on resize.
    msg!(image_view, "setImageScaling:", 3 as c_ulong);

    msg!(window, "setContentView:", image_view);
    msg!(window, "makeKeyAndOrderFront:", NIL);
    msg!(app, "activateIgnoringOtherApps:", true as c_int);

    setup_window_delegate(window, app);

    // Block until the user closes the window (delegate calls
    // [NSApp terminate:] from windowWillClose:).
    msg!(app, "run");

    // AppKit owns the window / view / image once added to the
    // hierarchy. Skipping explicit release avoids a double-free.
    let _ = release; // silence unused-import warning when this path is taken
}

#[cfg(target_vendor = "apple")]
unsafe fn create_nsimage_from_pixels(
    pixels: &paint_instructions::PixelContainer,
) -> objc_bridge::Id {
    use objc_bridge::{class, msg, msg_send_class, nsstring, release, CFRelease, CGSize, Id};

    let rep_class = class("NSBitmapImageRep");

    // NSDeviceRGBColorSpace describes the byte layout as R,G,B,A,
    // matching paint-metal's MTLPixelFormatRGBA8Unorm output. If the
    // display appears with swapped channels, we'll swap this to
    // NSCalibratedRGBColorSpace or rebuild the PixelContainer in BGRA.
    let color_space_name = nsstring("NSDeviceRGBColorSpace");
    let bytes_per_row = (pixels.width as usize) * 4;
    let mut data_ptr = pixels.data.as_ptr() as *mut u8;
    let planes_ptr = &mut data_ptr as *mut *mut u8;

    let rep: Id = msg!(
        msg_send_class(rep_class, "alloc"),
        "initWithBitmapDataPlanes:pixelsWide:pixelsHigh:bitsPerSample:samplesPerPixel:hasAlpha:isPlanar:colorSpaceName:bytesPerRow:bitsPerPixel:",
        planes_ptr,
        pixels.width as usize,
        pixels.height as usize,
        8usize,
        4usize,
        1usize,
        0usize,
        color_space_name,
        bytes_per_row,
        32usize
    );
    CFRelease(color_space_name);

    let size = CGSize {
        width: pixels.width as f64,
        height: pixels.height as f64,
    };

    let image_class = class("NSImage");
    let image: Id = msg!(msg_send_class(image_class, "alloc"), "initWithSize:", size);
    msg!(image, "addRepresentation:", rep);
    release(rep);

    image
}

#[cfg(target_vendor = "apple")]
unsafe fn setup_window_delegate(window: objc_bridge::Id, app: objc_bridge::Id) {
    use objc_bridge::{
        class, class_addIvar, class_addMethod, msg, objc_allocateClassPair, objc_registerClassPair,
        object_setInstanceVariable, sel, Id,
    };

    let superclass = class("NSObject");
    let delegate_class_name = std::ffi::CString::new("MarkdownReaderWindowDelegate").unwrap();

    let delegate_class = objc_allocateClassPair(superclass, delegate_class_name.as_ptr(), 0);

    if delegate_class.is_null() {
        // Class already registered (e.g. across successive runs of
        // the same process in tests). Re-use it.
        let delegate_class = class("MarkdownReaderWindowDelegate");
        let delegate: Id = msg!(delegate_class as Id, "alloc");
        let delegate = msg!(delegate, "init");
        let app_ivar_name = std::ffi::CString::new("_app").unwrap();
        object_setInstanceVariable(delegate, app_ivar_name.as_ptr(), app as *mut _);
        msg!(window, "setDelegate:", delegate);
        return;
    }

    let ivar_name = std::ffi::CString::new("_app").unwrap();
    let ivar_type = std::ffi::CString::new("@").unwrap();
    class_addIvar(
        delegate_class,
        ivar_name.as_ptr(),
        std::mem::size_of::<Id>(),
        std::mem::align_of::<Id>() as u8,
        ivar_type.as_ptr(),
    );

    let method_types = std::ffi::CString::new("v@:@").unwrap();
    class_addMethod(
        delegate_class,
        sel("windowWillClose:"),
        window_will_close as *const _,
        method_types.as_ptr(),
    );

    objc_registerClassPair(delegate_class);

    let delegate: Id = msg!(delegate_class as Id, "alloc");
    let delegate = msg!(delegate, "init");
    object_setInstanceVariable(delegate, ivar_name.as_ptr(), app as *mut _);

    msg!(window, "setDelegate:", delegate);
}

#[cfg(target_vendor = "apple")]
extern "C" fn window_will_close(
    this: objc_bridge::Id,
    _sel: objc_bridge::Sel,
    _notification: objc_bridge::Id,
) {
    use objc_bridge::{msg, object_getInstanceVariable, Id, NIL};
    unsafe {
        let ivar_name = std::ffi::CString::new("_app").unwrap();
        let mut app_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        object_getInstanceVariable(this, ivar_name.as_ptr(), &mut app_ptr);
        let app = app_ptr as Id;
        if !app.is_null() {
            msg!(app, "terminate:", NIL);
        }
    }
}
