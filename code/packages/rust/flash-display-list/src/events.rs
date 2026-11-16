//! Lifecycle and interaction events for Flash display objects and the stage.

/// Interactive button transition events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    /// Mouse pressed inside button bounds.
    Press,
    /// Mouse released inside button bounds.
    Release,
    /// Mouse released outside button bounds after having been pressed inside.
    ReleaseOutside,
    /// Mouse moved over button bounds.
    RollOver,
    /// Mouse moved outside button bounds.
    RollOut,
}

/// Stage-level and display-list lifecycle events.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayListEvent {
    /// Frame tick notification (`onEnterFrame`).
    EnterFrame,
    /// Mouse move event across stage.
    MouseMove {
        /// Horizontal coordinate in pixels.
        x: f64,
        /// Vertical coordinate in pixels.
        y: f64,
    },
    /// Mouse button pressed.
    MouseDown {
        /// Horizontal coordinate in pixels.
        x: f64,
        /// Vertical coordinate in pixels.
        y: f64,
    },
    /// Mouse button released.
    MouseUp {
        /// Horizontal coordinate in pixels.
        x: f64,
        /// Vertical coordinate in pixels.
        y: f64,
    },
    /// Interactive button event dispatched to a button at a given depth.
    ButtonAction {
        /// Depth of the targeted button.
        depth: i32,
        /// Character identifier if known.
        character_id: Option<u16>,
        /// The button event kind.
        event: ButtonEvent,
    },
}
