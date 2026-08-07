//! Thin Qt adapter facade over Venture's backend-neutral Cairo bridge.
//!
//! Browser state, navigation, input handling, layout, and rendering live in
//! `venture-browser-cairo`. The generated Qt host keeps its existing ABI and
//! direct-launch acceptance while sharing the implementation with Flutter and
//! Compose.

pub use venture_browser_cairo::*;
