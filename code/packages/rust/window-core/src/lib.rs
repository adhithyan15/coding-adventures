//! # window-core
//!
//! Backend-neutral windowing primitives and renderer-facing host handles.
//!
//! The central idea is simple:
//!
//! - a backend creates a presentation host
//! - the host emits normalized window and input events
//! - renderers receive an explicit render target for that host
//!
//! On desktop platforms the host is a real native window. In the browser the
//! host is a mounted HTML canvas that behaves like a window from the renderer's
//! perspective: it has identity, size, scale factor, redraw cadence, and input.
//! Rust owns the native backend contract; other languages are expected to mirror
//! these semantics even when their concrete runtime types differ.

use std::error::Error;
use std::fmt;

/// Crate version, kept explicit for examples and integration tests.
pub const VERSION: &str = "0.1.0";

/// Stable identifier for one presentation host.
///
/// Desktop backends usually map this to one native top-level window. Browser
/// backends map it to one mounted canvas host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(pub u64);

/// A size in logical units.
///
/// Logical units are:
///
/// - desktop points on high-DPI platforms
/// - CSS pixels in the browser
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

impl LogicalSize {
    /// Construct a logical size.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Validate that the size can safely participate in layout and window
    /// creation.
    pub fn validate(self) -> Result<Self, WindowError> {
        if !self.width.is_finite() || !self.height.is_finite() {
            return Err(WindowError::InvalidAttributes(
                "window sizes must be finite numbers",
            ));
        }
        if self.width < 0.0 || self.height < 0.0 {
            return Err(WindowError::InvalidAttributes(
                "window sizes must be non-negative",
            ));
        }
        Ok(self)
    }

    /// Convert a logical size into physical pixels using the provided scale.
    pub fn to_physical(self, scale_factor: f64) -> Result<PhysicalSize, WindowError> {
        self.validate()?;
        validate_scale_factor(scale_factor)?;

        let width = round_dimension(self.width * scale_factor)?;
        let height = round_dimension(self.height * scale_factor)?;
        Ok(PhysicalSize { width, height })
    }
}

impl Default for LogicalSize {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

/// A size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

impl PhysicalSize {
    /// Construct a physical size.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Convert a physical size back into logical units.
    pub fn to_logical(self, scale_factor: f64) -> Result<LogicalSize, WindowError> {
        validate_scale_factor(scale_factor)?;
        Ok(LogicalSize {
            width: self.width as f64 / scale_factor,
            height: self.height as f64 / scale_factor,
        })
    }
}

/// Which kind of drawing surface the caller hopes to use.
///
/// This is a preference, not a guarantee. Backends may reject impossible
/// combinations such as `Direct2D` on AppKit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfacePreference {
    Default,
    Metal,
    Direct2D,
    Cairo,
    Canvas2D,
}

/// Where a backend should mount or create the presentation host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountTarget {
    /// Create a normal native top-level host.
    Native,
    /// Browser-only: create or attach under `<body>`.
    BrowserBody,
    /// Browser-only: attach to a DOM element with this `id`.
    ElementId(String),
    /// Browser-only: attach using a DOM query selector.
    QuerySelector(String),
}

impl MountTarget {
    /// True when the target refers to browser DOM mounting.
    pub fn is_browser_target(&self) -> bool {
        !matches!(self, Self::Native)
    }

    /// Validate any backend-neutral invariants.
    pub fn validate(&self) -> Result<(), WindowError> {
        match self {
            Self::ElementId(id) if id.trim().is_empty() => Err(WindowError::InvalidAttributes(
                "element ids must not be blank",
            )),
            Self::QuerySelector(selector) if selector.trim().is_empty() => {
                Err(WindowError::InvalidAttributes(
                    "query selectors must not be blank",
                ))
            }
            _ => Ok(()),
        }
    }
}

impl Default for MountTarget {
    fn default() -> Self {
        Self::Native
    }
}

/// Shared creation attributes for a presentation host.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowAttributes {
    pub title: String,
    pub initial_size: LogicalSize,
    pub min_size: Option<LogicalSize>,
    pub max_size: Option<LogicalSize>,
    pub visible: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub preferred_surface: SurfacePreference,
    pub mount_target: MountTarget,
}

impl WindowAttributes {
    /// Validate the backend-neutral contract.
    pub fn validate(&self) -> Result<(), WindowError> {
        self.initial_size.validate()?;
        self.mount_target.validate()?;

        if let Some(min_size) = self.min_size {
            min_size.validate()?;
            if min_size.width > self.initial_size.width || min_size.height > self.initial_size.height
            {
                return Err(WindowError::InvalidAttributes(
                    "minimum size must not exceed the initial size",
                ));
            }
        }

        if let Some(max_size) = self.max_size {
            max_size.validate()?;
            if max_size.width < self.initial_size.width || max_size.height < self.initial_size.height
            {
                return Err(WindowError::InvalidAttributes(
                    "maximum size must not be smaller than the initial size",
                ));
            }
        }

        if let (Some(min_size), Some(max_size)) = (self.min_size, self.max_size) {
            if min_size.width > max_size.width || min_size.height > max_size.height {
                return Err(WindowError::InvalidAttributes(
                    "minimum size must not exceed maximum size",
                ));
            }
        }

        Ok(())
    }
}

impl Default for WindowAttributes {
    fn default() -> Self {
        Self {
            title: "Coding Adventures Window".to_string(),
            initial_size: LogicalSize::default(),
            min_size: None,
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            preferred_surface: SurfacePreference::Default,
            mount_target: MountTarget::Native,
        }
    }
}

/// Fluent construction helper for [`WindowAttributes`].
#[derive(Debug, Clone, PartialEq)]
pub struct WindowBuilder {
    attributes: WindowAttributes,
}

impl WindowBuilder {
    /// Start a builder with sensible defaults.
    pub fn new() -> Self {
        Self {
            attributes: WindowAttributes::default(),
        }
    }

    /// Replace the window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.attributes.title = title.into();
        self
    }

    /// Set the initial logical size.
    pub fn initial_size(mut self, size: LogicalSize) -> Self {
        self.attributes.initial_size = size;
        self
    }

    /// Set the optional minimum size.
    pub fn min_size(mut self, size: LogicalSize) -> Self {
        self.attributes.min_size = Some(size);
        self
    }

    /// Set the optional maximum size.
    pub fn max_size(mut self, size: LogicalSize) -> Self {
        self.attributes.max_size = Some(size);
        self
    }

    /// Control initial visibility.
    pub fn visible(mut self, visible: bool) -> Self {
        self.attributes.visible = visible;
        self
    }

    /// Control whether the host is resizable.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.attributes.resizable = resizable;
        self
    }

    /// Control whether system decorations are requested.
    pub fn decorations(mut self, decorations: bool) -> Self {
        self.attributes.decorations = decorations;
        self
    }

    /// Request a transparent host when the backend supports it.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.attributes.transparent = transparent;
        self
    }

    /// Express the renderer's preferred surface family.
    pub fn preferred_surface(mut self, preference: SurfacePreference) -> Self {
        self.attributes.preferred_surface = preference;
        self
    }

    /// Select where the host should be mounted or created.
    pub fn mount_target(mut self, mount_target: MountTarget) -> Self {
        self.attributes.mount_target = mount_target;
        self
    }

    /// Borrow the underlying attributes.
    pub fn attributes(&self) -> &WindowAttributes {
        &self.attributes
    }

    /// Finish construction and return the validated attributes.
    pub fn build(self) -> Result<WindowAttributes, WindowError> {
        self.attributes.validate()?;
        Ok(self.attributes)
    }

    /// Validate and hand the request to a backend.
    pub fn build_with<B: WindowBackend>(
        self,
        backend: &mut B,
    ) -> Result<B::Window, WindowError> {
        let attributes = self.build()?;
        backend.create_window(attributes)
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Press or release state for keyboard and pointer events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementState {
    Pressed,
    Released,
}

/// Common pointer buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

/// Named non-text keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedKey {
    Escape,
    Enter,
    Tab,
    Backspace,
    Space,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
}

/// A normalized key identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Named(NamedKey),
    Character(String),
}

/// Keyboard modifier state captured at the time of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModifiersState {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Window and input events normalized across backends.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    Created {
        window_id: WindowId,
    },
    Resized {
        window_id: WindowId,
        logical_size: LogicalSize,
        physical_size: PhysicalSize,
        scale_factor: f64,
    },
    RedrawRequested {
        window_id: WindowId,
    },
    CloseRequested {
        window_id: WindowId,
    },
    Destroyed {
        window_id: WindowId,
    },
    FocusChanged {
        window_id: WindowId,
        focused: bool,
    },
    VisibilityChanged {
        window_id: WindowId,
        visible: bool,
    },
    PointerMoved {
        window_id: WindowId,
        x: f64,
        y: f64,
    },
    PointerButton {
        window_id: WindowId,
        button: PointerButton,
        state: ElementState,
    },
    Scroll {
        window_id: WindowId,
        delta_x: f64,
        delta_y: f64,
    },
    Key {
        window_id: WindowId,
        key: Key,
        state: ElementState,
        modifiers: ModifiersState,
        text: Option<String>,
    },
    TextInput {
        window_id: WindowId,
        text: String,
    },
}

/// AppKit-specific render target information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppKitRenderTarget {
    pub ns_window: usize,
    pub ns_view: usize,
    pub metal_layer: Option<usize>,
}

/// Win32-specific render target information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32RenderTarget {
    pub hwnd: usize,
}

/// Browser canvas render target information.
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserCanvasRenderTarget {
    pub mount_target: MountTarget,
    pub logical_size: LogicalSize,
    pub physical_size: PhysicalSize,
    pub device_pixel_ratio: f64,
}

/// Future Wayland render target information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaylandRenderTarget {
    pub display: usize,
    pub surface: usize,
}

/// Future X11 render target information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X11RenderTarget {
    pub display: usize,
    pub window: u64,
}

/// Renderer-facing native target handle.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderTarget {
    AppKit(AppKitRenderTarget),
    Win32(Win32RenderTarget),
    BrowserCanvas(BrowserCanvasRenderTarget),
    Wayland(WaylandRenderTarget),
    X11(X11RenderTarget),
}

impl RenderTarget {
    /// Human-readable kind tag for diagnostics and tests.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::AppKit(_) => "appkit",
            Self::Win32(_) => "win32",
            Self::BrowserCanvas(_) => "browser-canvas",
            Self::Wayland(_) => "wayland",
            Self::X11(_) => "x11",
        }
    }
}

/// Errors produced by builder validation or backend work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    InvalidAttributes(&'static str),
    UnsupportedConfiguration(&'static str),
    UnsupportedPlatform(&'static str),
    Backend(String),
}

impl WindowError {
    /// Convenience helper for backend-specific string messages.
    pub fn backend(message: impl Into<String>) -> Self {
        Self::Backend(message.into())
    }
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAttributes(message) => write!(f, "invalid window attributes: {message}"),
            Self::UnsupportedConfiguration(message) => {
                write!(f, "unsupported window configuration: {message}")
            }
            Self::UnsupportedPlatform(message) => write!(f, "unsupported platform: {message}"),
            Self::Backend(message) => write!(f, "backend error: {message}"),
        }
    }
}

impl Error for WindowError {}

/// Common behaviour exposed by all created hosts.
pub trait Window {
    fn id(&self) -> WindowId;
    fn logical_size(&self) -> LogicalSize;
    fn physical_size(&self) -> PhysicalSize;
    fn scale_factor(&self) -> f64;
    fn request_redraw(&self) -> Result<(), WindowError>;
    fn set_title(&self, title: &str) -> Result<(), WindowError>;
    fn set_visible(&self, visible: bool) -> Result<(), WindowError>;
    fn render_target(&self) -> RenderTarget;
}

/// Backend contract for creating hosts and pumping events.
pub trait WindowBackend {
    type Window: Window;

    /// Human-readable backend name for diagnostics.
    fn backend_name(&self) -> &'static str;

    /// Create one presentation host.
    fn create_window(
        &mut self,
        attributes: WindowAttributes,
    ) -> Result<Self::Window, WindowError>;

    /// Return all currently available window events without blocking forever.
    fn pump_events(&mut self) -> Result<Vec<WindowEvent>, WindowError>;
}

fn validate_scale_factor(scale_factor: f64) -> Result<(), WindowError> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(WindowError::InvalidAttributes(
            "scale factors must be finite positive numbers",
        ));
    }
    Ok(())
}

fn round_dimension(value: f64) -> Result<u32, WindowError> {
    if value < 0.0 || value > u32::MAX as f64 {
        return Err(WindowError::InvalidAttributes(
            "scaled dimensions must fit into u32",
        ));
    }
    Ok(value.round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, Clone)]
    struct MockWindow {
        id: WindowId,
        logical_size: LogicalSize,
        physical_size: PhysicalSize,
        scale_factor: f64,
        render_target: RenderTarget,
        redraw_requests: Cell<u32>,
        visibility: Cell<bool>,
        title: RefCell<String>,
    }

    impl Window for MockWindow {
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
            self.redraw_requests.set(self.redraw_requests.get() + 1);
            Ok(())
        }

        fn set_title(&self, title: &str) -> Result<(), WindowError> {
            *self.title.borrow_mut() = title.to_string();
            Ok(())
        }

        fn set_visible(&self, visible: bool) -> Result<(), WindowError> {
            self.visibility.set(visible);
            Ok(())
        }

        fn render_target(&self) -> RenderTarget {
            self.render_target.clone()
        }
    }

    #[derive(Default)]
    struct MockBackend {
        last_attributes: Option<WindowAttributes>,
        next_id: u64,
    }

    impl WindowBackend for MockBackend {
        type Window = MockWindow;

        fn backend_name(&self) -> &'static str {
            "mock"
        }

        fn create_window(
            &mut self,
            attributes: WindowAttributes,
        ) -> Result<Self::Window, WindowError> {
            self.last_attributes = Some(attributes.clone());
            self.next_id += 1;

            Ok(MockWindow {
                id: WindowId(self.next_id),
                logical_size: attributes.initial_size,
                physical_size: attributes.initial_size.to_physical(2.0)?,
                scale_factor: 2.0,
                render_target: RenderTarget::BrowserCanvas(BrowserCanvasRenderTarget {
                    mount_target: attributes.mount_target.clone(),
                    logical_size: attributes.initial_size,
                    physical_size: attributes.initial_size.to_physical(2.0)?,
                    device_pixel_ratio: 2.0,
                }),
                redraw_requests: Cell::new(0),
                visibility: Cell::new(attributes.visible),
                title: RefCell::new(attributes.title),
            })
        }

        fn pump_events(&mut self) -> Result<Vec<WindowEvent>, WindowError> {
            Ok(vec![WindowEvent::Created {
                window_id: WindowId(self.next_id),
            }])
        }
    }

    #[test]
    fn logical_size_rejects_non_finite_values() {
        let err = LogicalSize::new(f64::NAN, 10.0).validate().unwrap_err();
        assert_eq!(
            err,
            WindowError::InvalidAttributes("window sizes must be finite numbers")
        );
    }

    #[test]
    fn logical_size_rejects_negative_values() {
        let err = LogicalSize::new(-1.0, 10.0).validate().unwrap_err();
        assert_eq!(
            err,
            WindowError::InvalidAttributes("window sizes must be non-negative")
        );
    }

    #[test]
    fn logical_size_converts_to_physical_pixels() {
        let physical = LogicalSize::new(320.0, 200.0)
            .to_physical(2.0)
            .unwrap();
        assert_eq!(physical, PhysicalSize::new(640, 400));
    }

    #[test]
    fn physical_size_converts_back_to_logical_units() {
        let logical = PhysicalSize::new(960, 540).to_logical(1.5).unwrap();
        assert_eq!(logical, LogicalSize::new(640.0, 360.0));
    }

    #[test]
    fn scale_factor_must_be_positive_and_finite() {
        let err = LogicalSize::new(100.0, 100.0).to_physical(0.0).unwrap_err();
        assert_eq!(
            err,
            WindowError::InvalidAttributes("scale factors must be finite positive numbers")
        );
    }

    #[test]
    fn mount_targets_validate_non_blank_browser_strings() {
        let err = MountTarget::ElementId("   ".to_string()).validate().unwrap_err();
        assert_eq!(
            err,
            WindowError::InvalidAttributes("element ids must not be blank")
        );
    }

    #[test]
    fn browser_mount_detection_matches_variant() {
        assert!(MountTarget::BrowserBody.is_browser_target());
        assert!(MountTarget::QuerySelector("#app canvas".to_string()).is_browser_target());
        assert!(!MountTarget::Native.is_browser_target());
    }

    #[test]
    fn attributes_reject_min_size_above_initial_size() {
        let attributes = WindowAttributes {
            min_size: Some(LogicalSize::new(900.0, 700.0)),
            ..WindowAttributes::default()
        };

        let err = attributes.validate().unwrap_err();
        assert_eq!(
            err,
            WindowError::InvalidAttributes("minimum size must not exceed the initial size")
        );
    }

    #[test]
    fn attributes_reject_max_size_below_initial_size() {
        let attributes = WindowAttributes {
            max_size: Some(LogicalSize::new(400.0, 300.0)),
            ..WindowAttributes::default()
        };

        let err = attributes.validate().unwrap_err();
        assert_eq!(
            err,
            WindowError::InvalidAttributes("maximum size must not be smaller than the initial size")
        );
    }

    #[test]
    fn builder_defaults_match_default_attributes() {
        let builder = WindowBuilder::new();
        assert_eq!(builder.attributes(), &WindowAttributes::default());
    }

    #[test]
    fn builder_produces_valid_custom_attributes() {
        let attributes = WindowBuilder::new()
            .title("Window Lab")
            .initial_size(LogicalSize::new(1024.0, 768.0))
            .min_size(LogicalSize::new(400.0, 300.0))
            .max_size(LogicalSize::new(1200.0, 900.0))
            .visible(false)
            .transparent(true)
            .preferred_surface(SurfacePreference::Metal)
            .build()
            .unwrap();

        assert_eq!(attributes.title, "Window Lab");
        assert_eq!(attributes.initial_size, LogicalSize::new(1024.0, 768.0));
        assert_eq!(attributes.min_size, Some(LogicalSize::new(400.0, 300.0)));
        assert_eq!(attributes.max_size, Some(LogicalSize::new(1200.0, 900.0)));
        assert!(!attributes.visible);
        assert!(attributes.transparent);
        assert_eq!(attributes.preferred_surface, SurfacePreference::Metal);
    }

    #[test]
    fn builder_can_create_windows_through_a_backend() {
        let mut backend = MockBackend::default();
        let window = WindowBuilder::new()
            .title("Canvas Host")
            .mount_target(MountTarget::BrowserBody)
            .preferred_surface(SurfacePreference::Canvas2D)
            .build_with(&mut backend)
            .unwrap();

        assert_eq!(window.id(), WindowId(1));
        assert_eq!(backend.last_attributes.unwrap().title, "Canvas Host");
    }

    #[test]
    fn windows_expose_render_targets_and_state_mutators() {
        let mut backend = MockBackend::default();
        let window = WindowBuilder::new()
            .mount_target(MountTarget::BrowserBody)
            .build_with(&mut backend)
            .unwrap();

        assert_eq!(window.render_target().kind(), "browser-canvas");
        assert_eq!(window.scale_factor(), 2.0);

        window.request_redraw().unwrap();
        window.request_redraw().unwrap();
        window.set_title("Retitled").unwrap();
        window.set_visible(false).unwrap();

        assert_eq!(window.redraw_requests.get(), 2);
        assert_eq!(&*window.title.borrow(), "Retitled");
        assert!(!window.visibility.get());
    }

    #[test]
    fn backend_can_pump_normalized_events() {
        let mut backend = MockBackend::default();
        let window = WindowBuilder::new().build_with(&mut backend).unwrap();
        let events = backend.pump_events().unwrap();

        assert_eq!(
            events,
            vec![WindowEvent::Created {
                window_id: window.id()
            }]
        );
    }

    #[test]
    fn render_target_kind_reports_the_variant_name() {
        let target = RenderTarget::Win32(Win32RenderTarget { hwnd: 42 });
        assert_eq!(target.kind(), "win32");
    }

    #[test]
    fn window_error_backend_helper_wraps_strings() {
        let err = WindowError::backend("message pump failed");
        assert_eq!(err.to_string(), "backend error: message pump failed");
    }
}
