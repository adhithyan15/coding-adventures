//! # window-appkit
//!
//! AppKit-backed desktop window backend for `window-core`.
//!
//! This crate now provides a minimal but real macOS launch path:
//!
//! - validate `window-core` attributes for AppKit
//! - create an `NSApplication`
//! - create and show an `NSWindow`
//! - expose AppKit render-target handles on the returned window
//! - run the AppKit event loop when requested
//!
//! It is intentionally still small. The point is to prove the boundary between
//! `window-core` and a native backend before adding richer event translation.

use std::ffi::{c_int, c_ulong};

#[cfg(target_vendor = "apple")]
use std::ffi::{c_void, CString};

#[cfg(target_vendor = "apple")]
use objc_bridge::{
    class, msg, msg_send_class, nsstring, object_getInstanceVariable, object_setInstanceVariable,
    objc_allocateClassPair, objc_registerClassPair, release, sel, ClassPtr, CGPoint, CGRect, Id,
    Sel, CGSize, NS_BACKING_STORE_BUFFERED, NS_WINDOW_STYLE_MASK_CLOSABLE,
    NS_WINDOW_STYLE_MASK_MINIATURIZABLE, NS_WINDOW_STYLE_MASK_RESIZABLE,
    NS_WINDOW_STYLE_MASK_TITLED, NIL, class_addIvar, class_addMethod,
};
use window_core::{
    AppKitRenderTarget, LogicalSize, MountTarget, PhysicalSize, RenderTarget, SurfacePreference,
    Window, WindowAttributes, WindowBackend, WindowError, WindowEvent, WindowId,
};

/// Crate version, kept explicit for examples and integration tests.
pub const VERSION: &str = "0.1.0";

/// Which AppKit-side surface family the renderer should expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKitSurfaceChoice {
    /// A normal `NSView` or equivalent host view.
    View,
    /// A Metal-friendly host with a `CAMetalLayer`.
    ///
    /// The minimal launch path still exposes the content view immediately. A
    /// concrete `CAMetalLayer` attachment is the next step once the renderer
    /// integration lands.
    MetalLayer,
}

/// A created AppKit window host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppKitWindow {
    id: WindowId,
    logical_size: LogicalSize,
    physical_size: PhysicalSize,
    scale_factor: f64,
    ns_window: usize,
    ns_view: usize,
    metal_layer: Option<usize>,
}

impl AppKitWindow {
    /// Return the AppKit-native render target handles for this window.
    pub fn appkit_target(&self) -> AppKitRenderTarget {
        AppKitRenderTarget {
            ns_window: self.ns_window,
            ns_view: self.ns_view,
            metal_layer: self.metal_layer,
        }
    }
}

impl Window for AppKitWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    fn physical_size(&self) -> PhysicalSize {
        self.physical_size
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn request_redraw(&self) -> Result<(), WindowError> {
        #[cfg(target_vendor = "apple")]
        unsafe {
            let view = self.ns_view as Id;
            msg!(view, "setNeedsDisplay:", true as c_int);
            Ok(())
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            Err(WindowError::UnsupportedPlatform(
                "AppKit windows are only available on Apple platforms",
            ))
        }
    }

    fn set_title(&self, title: &str) -> Result<(), WindowError> {
        #[cfg(target_vendor = "apple")]
        unsafe {
            let title_ns = nsstring(title);
            msg!(self.ns_window as Id, "setTitle:", title_ns);
            objc_bridge::CFRelease(title_ns);
            Ok(())
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = title;
            Err(WindowError::UnsupportedPlatform(
                "AppKit windows are only available on Apple platforms",
            ))
        }
    }

    fn set_visible(&self, visible: bool) -> Result<(), WindowError> {
        #[cfg(target_vendor = "apple")]
        unsafe {
            if visible {
                msg!(self.ns_window as Id, "makeKeyAndOrderFront:", NIL);
            } else {
                msg!(self.ns_window as Id, "orderOut:", NIL);
            }
            Ok(())
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = visible;
            Err(WindowError::UnsupportedPlatform(
                "AppKit windows are only available on Apple platforms",
            ))
        }
    }

    fn render_target(&self) -> RenderTarget {
        RenderTarget::AppKit(self.appkit_target())
    }
}

/// AppKit backend with minimal native window support.
#[derive(Debug, Default)]
pub struct AppKitBackend {
    next_id: u64,
    #[cfg(target_vendor = "apple")]
    app: Option<usize>,
}

impl AppKitBackend {
    /// Construct a new backend.
    pub const fn new() -> Self {
        Self {
            next_id: 0,
            #[cfg(target_vendor = "apple")]
            app: None,
        }
    }

    /// Human-readable backend name.
    pub const fn backend_name(&self) -> &'static str {
        "appkit"
    }

    /// Choose the AppKit host surface implied by the renderer preference.
    pub fn choose_surface(
        &self,
        preference: SurfacePreference,
    ) -> Result<AppKitSurfaceChoice, WindowError> {
        match preference {
            SurfacePreference::Default | SurfacePreference::Cairo => Ok(AppKitSurfaceChoice::View),
            SurfacePreference::Metal => Ok(AppKitSurfaceChoice::MetalLayer),
            SurfacePreference::Direct2D => Err(WindowError::UnsupportedConfiguration(
                "Direct2D is a Windows renderer and cannot target AppKit",
            )),
            SurfacePreference::Canvas2D => Err(WindowError::UnsupportedConfiguration(
                "Canvas2D is a browser renderer and cannot target AppKit",
            )),
        }
    }

    /// Validate a `window-core` request against AppKit expectations.
    pub fn validate_attributes(
        &self,
        attributes: &WindowAttributes,
    ) -> Result<AppKitSurfaceChoice, WindowError> {
        attributes.validate()?;
        if attributes.mount_target != MountTarget::Native {
            return Err(WindowError::UnsupportedConfiguration(
                "AppKit windows must use MountTarget::Native",
            ));
        }
        self.choose_surface(attributes.preferred_surface)
    }

    /// Create a real AppKit window on macOS.
    pub fn create_native_window(
        &mut self,
        attributes: WindowAttributes,
    ) -> Result<AppKitWindow, WindowError> {
        self.create_window(attributes)
    }

    /// Run the AppKit message loop.
    pub fn run(&mut self) -> Result<(), WindowError> {
        #[cfg(target_vendor = "apple")]
        unsafe {
            let app = self.shared_application()?;
            msg!(app, "run");
            Ok(())
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            Err(WindowError::UnsupportedPlatform(
                "AppKit windows are only available on Apple platforms",
            ))
        }
    }

    /// Request that the current AppKit app terminates after the given delay.
    pub fn terminate_after(&mut self, seconds: f64) -> Result<(), WindowError> {
        #[cfg(target_vendor = "apple")]
        unsafe {
            let app = self.shared_application()?;
            let terminate_sel = sel("terminate:");
            msg!(
                app,
                "performSelector:withObject:afterDelay:",
                terminate_sel,
                NIL,
                seconds
            );
            Ok(())
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = seconds;
            Err(WindowError::UnsupportedPlatform(
                "AppKit windows are only available on Apple platforms",
            ))
        }
    }

    #[cfg(target_vendor = "apple")]
    unsafe fn shared_application(&mut self) -> Result<Id, WindowError> {
        if let Some(app) = self.app {
            return Ok(app as Id);
        }

        let app_class = class("NSApplication");
        let app: Id = msg_send_class(app_class, "sharedApplication");
        if app.is_null() {
            return Err(WindowError::backend(
                "NSApplication sharedApplication returned nil",
            ));
        }

        // NSApplicationActivationPolicyRegular = 0
        msg!(app, "setActivationPolicy:", 0 as c_ulong);
        msg!(app, "finishLaunching");
        self.app = Some(app as usize);
        Ok(app)
    }

    #[cfg(target_vendor = "apple")]
    unsafe fn create_window_inner(
        &mut self,
        attributes: WindowAttributes,
        surface: AppKitSurfaceChoice,
    ) -> Result<AppKitWindow, WindowError> {
        let app = self.shared_application()?;
        self.next_id += 1;

        let frame = CGRect {
            origin: CGPoint { x: 200.0, y: 200.0 },
            size: CGSize {
                width: attributes.initial_size.width.max(1.0),
                height: attributes.initial_size.height.max(1.0),
            },
        };

        let mut style_mask = if attributes.decorations {
            NS_WINDOW_STYLE_MASK_TITLED
                | NS_WINDOW_STYLE_MASK_CLOSABLE
                | NS_WINDOW_STYLE_MASK_MINIATURIZABLE
        } else {
            0
        };
        if attributes.resizable {
            style_mask |= NS_WINDOW_STYLE_MASK_RESIZABLE;
        }

        let window_class = class("NSWindow");
        let window: Id = msg!(
            msg_send_class(window_class, "alloc"),
            "initWithContentRect:styleMask:backing:defer:",
            frame,
            style_mask,
            NS_BACKING_STORE_BUFFERED,
            false as c_int
        );
        if window.is_null() {
            return Err(WindowError::backend("NSWindow allocation failed"));
        }

        let title_ns = nsstring(&attributes.title);
        msg!(window, "setTitle:", title_ns);
        objc_bridge::CFRelease(title_ns);

        let view: Id = msg!(window, "contentView");
        if view.is_null() {
            release(window);
            return Err(WindowError::backend("NSWindow contentView returned nil"));
        }

        setup_window_delegate(window, app)?;

        if attributes.visible {
            msg!(window, "makeKeyAndOrderFront:", NIL);
            msg!(app, "activateIgnoringOtherApps:", true as c_int);
        }

        let scale_factor = 1.0;
        let physical_size = attributes.initial_size.to_physical(scale_factor)?;

        // Build the Metal-layer host when the caller asked for one.
        // The attachment sequence here matches Apple's recommended
        // setup for hosting a CAMetalLayer inside an NSView:
        //
        //   1. Create a CAMetalLayer instance.
        //   2. Assign the system default MTLDevice.
        //   3. Configure pixelFormat (BGRA8 is the Metal-friendly
        //      default that maps 1:1 with the drawable format our
        //      paint-metal renderer already uses for its offscreen
        //      textures).
        //   4. Size the layer's frame to match the view's bounds.
        //   5. Turn on the view's layer hosting and install our
        //      CAMetalLayer as the view's backing layer.
        let metal_layer = match surface {
            AppKitSurfaceChoice::View => None,
            AppKitSurfaceChoice::MetalLayer => {
                match attach_metal_layer(view) {
                    Ok(layer) => Some(layer as usize),
                    Err(e) => {
                        release(window);
                        return Err(e);
                    }
                }
            }
        };

        Ok(AppKitWindow {
            id: WindowId(self.next_id),
            logical_size: attributes.initial_size,
            physical_size,
            scale_factor,
            ns_window: window as usize,
            ns_view: view as usize,
            metal_layer,
        })
    }
}

/// Attach a freshly-created `CAMetalLayer` to the given `NSView`.
///
/// Returns the layer's `Id` pointer on success. The layer is retained
/// internally by the view; the window's `Drop` implementation releases
/// the view (and, transitively, the layer).
#[cfg(target_vendor = "apple")]
unsafe fn attach_metal_layer(view: Id) -> Result<Id, WindowError> {
    use objc_bridge::{
        msg, MTLCreateSystemDefaultDevice, MTL_PIXEL_FORMAT_BGRA8_UNORM,
    };

    let layer_class = class("CAMetalLayer");
    if layer_class.is_null() {
        return Err(WindowError::backend(
            "CAMetalLayer class not available — QuartzCore framework not linked?",
        ));
    }

    // [[CAMetalLayer alloc] init]
    let layer: Id = msg!(msg_send_class(layer_class, "alloc"), "init");
    if layer.is_null() {
        return Err(WindowError::backend("CAMetalLayer allocation failed"));
    }

    let device = MTLCreateSystemDefaultDevice();
    if device.is_null() {
        release(layer);
        return Err(WindowError::backend(
            "MTLCreateSystemDefaultDevice returned nil",
        ));
    }
    let _: Id = msg!(layer, "setDevice:", device);
    let _: Id = msg!(layer, "setPixelFormat:", MTL_PIXEL_FORMAT_BGRA8_UNORM);
    // framebufferOnly = NO so paint-metal can read the drawable's
    // texture back when diagnostics or snapshot tests want to.
    let _: Id = msg!(layer, "setFramebufferOnly:", false as c_int);

    // Mirror the view's current bounds into the layer's frame.
    let bounds: objc_bridge::CGRect = {
        // We can't use msg! for a struct-returning selector generically —
        // use a typed function pointer via msg_send_bounds.
        msg_send_bounds(view)
    };
    let _: Id = msg!(layer, "setFrame:", bounds);

    // [view setWantsLayer:YES]; [view setLayer:layer];
    let _: Id = msg!(view, "setWantsLayer:", true as c_int);
    let _: Id = msg!(view, "setLayer:", layer);

    Ok(layer)
}

/// Call `[view bounds]` — a CGRect-returning selector. Rust's msg!
/// macro is Id-returning only; for struct returns on arm64 we use
/// `objc_msgSend_stret` pattern via a typed function cast.
#[cfg(target_vendor = "apple")]
unsafe fn msg_send_bounds(view: Id) -> objc_bridge::CGRect {
    use objc_bridge::{objc_msgSend, sel};
    let fn_ptr: unsafe extern "C" fn(Id, objc_bridge::Sel) -> objc_bridge::CGRect =
        std::mem::transmute(objc_msgSend as *const ());
    fn_ptr(view, sel("bounds"))
}


impl WindowBackend for AppKitBackend {
    type Window = AppKitWindow;

    fn backend_name(&self) -> &'static str {
        self.backend_name()
    }

    fn create_window(
        &mut self,
        attributes: WindowAttributes,
    ) -> Result<Self::Window, WindowError> {
        let surface = self.validate_attributes(&attributes)?;

        #[cfg(target_vendor = "apple")]
        unsafe {
            self.create_window_inner(attributes, surface)
        }

        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = surface;
            let _ = attributes;
            Err(WindowError::UnsupportedPlatform(
                "AppKit windows are only available on Apple platforms",
            ))
        }
    }

    fn pump_events(&mut self) -> Result<Vec<WindowEvent>, WindowError> {
        Ok(Vec::new())
    }
}

#[cfg(target_vendor = "apple")]
unsafe fn setup_window_delegate(window: Id, app: Id) -> Result<(), WindowError> {
    let delegate_class = ensure_delegate_class()?;
    let delegate: Id = msg!(delegate_class as Id, "alloc");
    let delegate = msg!(delegate, "init");
    let app_ivar_name = CString::new("_app").expect("static ivar name");
    object_setInstanceVariable(delegate, app_ivar_name.as_ptr(), app as *mut _);
    msg!(window, "setDelegate:", delegate);
    Ok(())
}

#[cfg(target_vendor = "apple")]
unsafe fn ensure_delegate_class() -> Result<ClassPtr, WindowError> {
    let class_name = CString::new("WindowAppKitDelegate").expect("static class name");
    let existing = objc_bridge::objc_getClass(class_name.as_ptr());
    if !existing.is_null() {
        return Ok(existing);
    }

    let superclass = class("NSObject");
    let delegate_class = objc_allocateClassPair(superclass, class_name.as_ptr(), 0);
    if delegate_class.is_null() {
        return Err(WindowError::backend(
            "objc_allocateClassPair failed for WindowAppKitDelegate",
        ));
    }

    let ivar_name = CString::new("_app").expect("static ivar name");
    let ivar_type = CString::new("@").expect("static ivar type");
    class_addIvar(
        delegate_class,
        ivar_name.as_ptr(),
        std::mem::size_of::<Id>(),
        std::mem::align_of::<Id>() as u8,
        ivar_type.as_ptr(),
    );

    let method_types = CString::new("v@:@").expect("static method type");
    class_addMethod(
        delegate_class,
        sel("windowWillClose:"),
        window_will_close as *const _,
        method_types.as_ptr(),
    );

    objc_registerClassPair(delegate_class);
    Ok(delegate_class)
}

#[cfg(target_vendor = "apple")]
extern "C" fn window_will_close(this: Id, _sel: Sel, _notification: Id) {
    unsafe {
        let ivar_name = CString::new("_app").expect("static ivar name");
        let mut app_ptr: *mut c_void = std::ptr::null_mut();
        object_getInstanceVariable(this, ivar_name.as_ptr(), &mut app_ptr);
        let app = app_ptr as Id;
        if !app.is_null() {
            msg!(app, "terminate:", NIL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_cairo_prefer_a_normal_view() {
        let backend = AppKitBackend::new();
        assert_eq!(
            backend.choose_surface(SurfacePreference::Default).unwrap(),
            AppKitSurfaceChoice::View
        );
        assert_eq!(
            backend.choose_surface(SurfacePreference::Cairo).unwrap(),
            AppKitSurfaceChoice::View
        );
    }

    #[test]
    fn metal_prefers_a_metal_layer() {
        let backend = AppKitBackend::new();
        assert_eq!(
            backend.choose_surface(SurfacePreference::Metal).unwrap(),
            AppKitSurfaceChoice::MetalLayer
        );
    }

    #[test]
    fn appkit_rejects_non_native_mount_targets() {
        let backend = AppKitBackend::new();
        let attributes = WindowAttributes {
            mount_target: MountTarget::BrowserBody,
            ..WindowAttributes::default()
        };

        let err = backend.validate_attributes(&attributes).unwrap_err();
        assert_eq!(
            err,
            WindowError::UnsupportedConfiguration(
                "AppKit windows must use MountTarget::Native"
            )
        );
    }

    #[test]
    fn appkit_rejects_direct2d_requests() {
        let backend = AppKitBackend::new();
        let err = backend
            .choose_surface(SurfacePreference::Direct2D)
            .unwrap_err();
        assert_eq!(
            err,
            WindowError::UnsupportedConfiguration(
                "Direct2D is a Windows renderer and cannot target AppKit"
            )
        );
    }

    #[test]
    fn appkit_window_reports_an_appkit_render_target() {
        let window = AppKitWindow {
            id: WindowId(7),
            logical_size: LogicalSize::new(320.0, 200.0),
            physical_size: PhysicalSize::new(320, 200),
            scale_factor: 1.0,
            ns_window: 10,
            ns_view: 20,
            metal_layer: None,
        };

        assert_eq!(window.render_target().kind(), "appkit");
        assert_eq!(
            window.appkit_target(),
            AppKitRenderTarget {
                ns_window: 10,
                ns_view: 20,
                metal_layer: None
            }
        );
    }
}
