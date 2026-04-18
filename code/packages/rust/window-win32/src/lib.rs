//! # window-win32
//!
//! Win32 desktop window backend for `window-core`.
//!
//! Like `window-appkit`, this crate is an intentionally honest shell in the
//! first slice. It teaches the mapping from cross-platform window attributes to
//! Win32 expectations without pretending the `HWND` creation and message pump
//! are already implemented.

use window_core::{MountTarget, SurfacePreference, WindowAttributes, WindowError};

/// Crate version, kept explicit for examples and integration tests.
pub const VERSION: &str = "0.1.0";

/// Which Win32 host family the renderer should expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32SurfaceChoice {
    /// A normal `HWND`-backed presentation host.
    Hwnd,
}

/// Backend shell for Win32 validation and future native creation.
#[derive(Debug, Default, Clone, Copy)]
pub struct Win32Backend;

impl Win32Backend {
    /// Construct a new backend shell.
    pub const fn new() -> Self {
        Self
    }

    /// Human-readable backend name.
    pub const fn backend_name(&self) -> &'static str {
        "win32"
    }

    /// Choose the Win32 host surface implied by the renderer preference.
    pub fn choose_surface(
        &self,
        preference: SurfacePreference,
    ) -> Result<Win32SurfaceChoice, WindowError> {
        match preference {
            SurfacePreference::Default
            | SurfacePreference::Direct2D
            | SurfacePreference::Cairo => Ok(Win32SurfaceChoice::Hwnd),
            SurfacePreference::Metal => Err(WindowError::UnsupportedConfiguration(
                "Metal is an Apple renderer and cannot target Win32",
            )),
            SurfacePreference::Canvas2D => Err(WindowError::UnsupportedConfiguration(
                "Canvas2D is a browser renderer and cannot target Win32",
            )),
        }
    }

    /// Validate a `window-core` request against Win32 expectations.
    pub fn validate_attributes(
        &self,
        attributes: &WindowAttributes,
    ) -> Result<Win32SurfaceChoice, WindowError> {
        attributes.validate()?;
        if attributes.mount_target != MountTarget::Native {
            return Err(WindowError::UnsupportedConfiguration(
                "Win32 windows must use MountTarget::Native",
            ));
        }
        self.choose_surface(attributes.preferred_surface)
    }

    /// Reserve the place where real `HWND` creation will land next.
    pub fn create_native_window(
        &mut self,
        attributes: WindowAttributes,
    ) -> Result<Win32SurfaceChoice, WindowError> {
        let surface = self.validate_attributes(&attributes)?;
        Err(WindowError::backend(format!(
            "native Win32 window creation is not wired yet (validated surface: {surface:?})"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use window_core::LogicalSize;

    #[test]
    fn default_direct2d_and_cairo_share_the_hwnd_host() {
        let backend = Win32Backend::new();
        assert_eq!(
            backend.choose_surface(SurfacePreference::Default).unwrap(),
            Win32SurfaceChoice::Hwnd
        );
        assert_eq!(
            backend.choose_surface(SurfacePreference::Direct2D).unwrap(),
            Win32SurfaceChoice::Hwnd
        );
        assert_eq!(
            backend.choose_surface(SurfacePreference::Cairo).unwrap(),
            Win32SurfaceChoice::Hwnd
        );
    }

    #[test]
    fn win32_rejects_browser_mount_targets() {
        let backend = Win32Backend::new();
        let attributes = WindowAttributes {
            mount_target: MountTarget::ElementId("app".to_string()),
            ..WindowAttributes::default()
        };

        let err = backend.validate_attributes(&attributes).unwrap_err();
        assert_eq!(
            err,
            WindowError::UnsupportedConfiguration(
                "Win32 windows must use MountTarget::Native"
            )
        );
    }

    #[test]
    fn win32_rejects_metal_requests() {
        let backend = Win32Backend::new();
        let err = backend.choose_surface(SurfacePreference::Metal).unwrap_err();
        assert_eq!(
            err,
            WindowError::UnsupportedConfiguration(
                "Metal is an Apple renderer and cannot target Win32"
            )
        );
    }

    #[test]
    fn create_native_window_is_honest_about_being_a_shell() {
        let mut backend = Win32Backend::new();
        let err = backend
            .create_native_window(WindowAttributes {
                initial_size: LogicalSize::new(640.0, 480.0),
                preferred_surface: SurfacePreference::Direct2D,
                ..WindowAttributes::default()
            })
            .unwrap_err();

        assert!(err
            .to_string()
            .contains("native Win32 window creation is not wired yet"));
    }
}
