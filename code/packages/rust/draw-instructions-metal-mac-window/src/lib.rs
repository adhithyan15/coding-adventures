//! # draw-instructions-metal-window
//!
//! Native macOS window display for draw-instructions scenes.
//!
//! This crate opens a native macOS window and renders a `DrawScene` into it
//! using Metal.  Unlike the headless `draw-instructions-metal` renderer
//! (which reads pixels back to CPU), this path presents the rendered image
//! directly to the screen via the GPU's swap chain — no read-back needed.
//!
//! ## How windowed rendering differs from headless
//!
//! ```text
//! Headless (draw-instructions-metal):
//!   GPU renders → offscreen texture → getBytes() → CPU pixel buffer
//!
//! Windowed (this crate):
//!   GPU renders → window's drawable → presentDrawable → screen
//! ```
//!
//! The windowed path is faster because it skips the GPU→CPU transfer.
//! The rendered image goes straight from GPU memory to the display.
//!
//! ## How it works
//!
//! 1. Create an `NSApplication` (required for any macOS GUI)
//! 2. Create an `NSWindow` at the scene's dimensions
//! 3. Render the scene to a `PixelBuffer` using `draw-instructions-metal`
//! 4. Create an `NSImageView` displaying the rendered image
//! 5. Run the event loop (blocks until window is closed)
//!
//! Note: A full Metal-native approach would use `MTKView` and render
//! directly to the window's drawable.  This simpler approach renders
//! to a pixel buffer first, then displays it as an NSImage.  This is
//! slightly less efficient but dramatically simpler to implement with
//! raw objc-bridge calls, and perfectly adequate for static scenes
//! like barcodes.

pub const VERSION: &str = "0.1.0";

use draw_instructions::DrawScene;
use draw_instructions_metal::render_metal;
use draw_instructions_pixels::PixelBuffer;
use objc_bridge::*;
use std::ffi::{c_int, c_ulong};

/// Open a native macOS window displaying the rendered scene.
///
/// This function blocks until the user closes the window.
///
/// The window title is taken from the scene's metadata "label" field,
/// or defaults to "Draw Instructions" if no label is set.
///
/// ## Example
///
/// ```ignore
/// use draw_instructions::*;
/// use draw_instructions_metal_window::show_in_window;
///
/// let scene = create_scene(400, 200, vec![
///     draw_rect(10, 10, 380, 180, "#3366cc", Metadata::new()),
/// ], "#ffffff", Metadata::new());
///
/// show_in_window(&scene);  // Opens window, blocks until closed
/// ```
pub fn show_in_window(scene: &DrawScene) {
    // Render the scene to pixels using the Metal renderer
    let pixel_buffer = render_metal(scene);

    let title = scene
        .metadata
        .get("label")
        .cloned()
        .unwrap_or_else(|| "Draw Instructions".into());

    unsafe {
        show_window(&pixel_buffer, &title);
    }
}

// ---------------------------------------------------------------------------
// AppKit window creation
// ---------------------------------------------------------------------------

unsafe fn show_window(pixels: &PixelBuffer, title: &str) {
    // Step 1: Create and configure NSApplication
    //
    // Every macOS GUI app needs an NSApplication instance.  The shared
    // application manages the event loop, menus, and window list.
    let app_class = class("NSApplication");
    let app: Id = msg_send_class(app_class, "sharedApplication");

    // Set activation policy to regular (app appears in Dock)
    // NSApplicationActivationPolicyRegular = 0
    msg!(app, "setActivationPolicy:", 0 as c_ulong);

    // Step 2: Create NSWindow
    let window_width = pixels.width.max(200) as f64;
    let window_height = pixels.height.max(100) as f64;
    let frame = CGRect {
        origin: CGPoint { x: 200.0, y: 200.0 },
        size: CGSize {
            width: window_width,
            height: window_height,
        },
    };

    let style_mask = NS_WINDOW_STYLE_MASK_TITLED
        | NS_WINDOW_STYLE_MASK_CLOSABLE
        | NS_WINDOW_STYLE_MASK_MINIATURIZABLE;

    let window_class = class("NSWindow");
    let window: Id = msg!(msg_send_class(window_class, "alloc"), "initWithContentRect:styleMask:backing:defer:", frame, style_mask, NS_BACKING_STORE_BUFFERED, false as c_int);

    // Set window title
    let title_ns = nsstring(title);
    msg!(window, "setTitle:", title_ns);
    CFRelease(title_ns);

    // Step 3: Create NSImage from pixel buffer
    let image = create_nsimage_from_pixels(pixels);

    // Step 4: Create NSImageView to display the image
    let image_rect = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: pixels.width as f64,
            height: pixels.height as f64,
        },
    };

    let image_view_class = class("NSImageView");
    let image_view: Id = msg!(msg_send_class(image_view_class, "alloc"), "initWithFrame:", image_rect);
    msg!(image_view, "setImage:", image);
    // NSImageScaleProportionallyUpOrDown = 3
    msg!(image_view, "setImageScaling:", 3 as c_ulong);

    // Set as window's content view
    msg!(window, "setContentView:", image_view);

    // Step 5: Show window and start event loop
    msg!(window, "makeKeyAndOrderFront:", NIL);

    // Activate the app (bring to front)
    msg!(app, "activateIgnoringOtherApps:", true as c_int);

    // Create a window delegate to handle window close → app terminate
    setup_window_delegate(window, app);

    // Run the event loop (blocks until app terminates)
    msg!(app, "run");

    // No explicit cleanup needed — AppKit takes ownership of the window,
    // its content view, and the image when they are added to the window
    // hierarchy.  Releasing them here would be a double-free since AppKit
    // releases them during application teardown.
}

/// Create an NSImage from a PixelBuffer.
///
/// The pixel buffer is RGBA8, row-major, top-left origin.
/// NSBitmapImageRep expects the same format, so we can use the data directly.
///
/// # Safety
///
/// NSBitmapImageRep does NOT copy the pixel data — it references the
/// buffer directly.  The caller MUST ensure the PixelBuffer outlives
/// the returned NSImage.  In `show_window`, this is guaranteed because
/// `msg!(app, "run")` blocks until the window closes, and the PixelBuffer
/// borrow is held for the entire duration.
unsafe fn create_nsimage_from_pixels(pixels: &PixelBuffer) -> Id {
    let rep_class = class("NSBitmapImageRep");

    // Create NSBitmapImageRep with our pixel data
    //
    // initWithBitmapDataPlanes:pixelsWide:pixelsHigh:bitsPerSample:
    //   samplesPerPixel:hasAlpha:isPlanar:colorSpaceName:bytesPerRow:bitsPerPixel:
    let color_space_name = nsstring("NSDeviceRGBColorSpace");
    let bytes_per_row = (pixels.width as usize) * 4;

    // We need to pass a pointer to the data planes array.
    // For non-planar data, it's a single pointer.
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
        1usize,   // has alpha (BOOL = YES)
        0usize,   // not planar (BOOL = NO)
        color_space_name,
        bytes_per_row,
        32usize
    );
    CFRelease(color_space_name);

    // Create NSImage and add the rep
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

/// Set up a window delegate that terminates the app when the window closes.
///
/// We create a new Objective-C class at runtime using the ObjC runtime API.
/// This class implements `windowWillClose:` to call `[NSApp terminate:]`.
unsafe fn setup_window_delegate(window: Id, app: Id) {
    // Create a new ObjC class for the delegate
    let superclass = class("NSObject");
    let delegate_class_name = std::ffi::CString::new("DrawInstructionsWindowDelegate").unwrap();

    let delegate_class = objc_allocateClassPair(
        superclass,
        delegate_class_name.as_ptr(),
        0,
    );

    if delegate_class.is_null() {
        // Class already exists (e.g. from a previous call) — look it up
        let delegate_class = class("DrawInstructionsWindowDelegate");
        let delegate: Id = msg!(delegate_class as Id, "alloc");
        let delegate = msg!(delegate, "init");

        // Store app reference in an ivar so windowWillClose can use it
        let app_ivar_name = std::ffi::CString::new("_app").unwrap();
        object_setInstanceVariable(delegate, app_ivar_name.as_ptr(), app as *mut _);

        msg!(window, "setDelegate:", delegate);
        return;
    }

    // Add an instance variable to store the app reference
    let ivar_name = std::ffi::CString::new("_app").unwrap();
    let ivar_type = std::ffi::CString::new("@").unwrap();
    class_addIvar(
        delegate_class,
        ivar_name.as_ptr(),
        std::mem::size_of::<Id>(),
        std::mem::align_of::<Id>() as u8,
        ivar_type.as_ptr(),
    );

    // Add the windowWillClose: method
    let method_types = std::ffi::CString::new("v@:@").unwrap();
    class_addMethod(
        delegate_class,
        sel("windowWillClose:"),
        window_will_close as *const _,
        method_types.as_ptr(),
    );

    objc_registerClassPair(delegate_class);

    // Create an instance of the delegate
    let delegate: Id = msg!(delegate_class as Id, "alloc");
    let delegate = msg!(delegate, "init");

    // Store app reference
    object_setInstanceVariable(delegate, ivar_name.as_ptr(), app as *mut _);

    msg!(window, "setDelegate:", delegate);
}

/// Called when the window is about to close.
/// Terminates the NSApplication event loop so `show_in_window` returns.
extern "C" fn window_will_close(this: Id, _sel: Sel, _notification: Id) {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // Note: We can't run automated tests for show_in_window because it
    // opens a real window and blocks.  The function is tested manually.
    //
    // To test manually:
    //   1. Create a binary that calls show_in_window with a Code39 scene
    //   2. Run it: cargo run --example barcode_window
    //   3. Verify the window opens and displays the barcode
    //   4. Close the window — the program should exit
}
