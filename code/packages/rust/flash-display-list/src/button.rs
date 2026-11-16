//! 4-state interactive button display object (`SimpleButton`).

use crate::display_object::DisplayObject;
use crate::events::ButtonEvent;

/// The active visual state of an interactive button control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Idle state when the mouse cursor is outside button hit bounds.
    Up,
    /// Hover state when the mouse cursor is over button hit bounds.
    Over,
    /// Pressed state when the mouse button is pressed over the button.
    Down,
    /// Invisible hit test geometry defining the interactive area.
    HitTest,
}

/// An interactive control responding to mouse movements, roll-overs, and clicks.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleButton {
    /// Up state display object.
    pub up_record: Option<Box<DisplayObject>>,
    /// Over state display object.
    pub over_record: Option<Box<DisplayObject>>,
    /// Down state display object.
    pub down_record: Option<Box<DisplayObject>>,
    /// Hit test geometry defining the mouse target bounds.
    pub hit_record: Option<Box<DisplayObject>>,
    /// Current button state.
    pub current_state: ButtonState,
    /// Whether the mouse cursor is currently over the hit target.
    pub is_mouse_over: bool,
    /// Whether the mouse button is currently held down.
    pub is_mouse_down: bool,
    /// Whether the button is interactively enabled.
    pub enabled: bool,
    /// Whether menu tracking behavior is enabled.
    pub track_as_menu: bool,
}

impl Default for SimpleButton {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleButton {
    /// Creates a new default simple button.
    pub fn new() -> Self {
        Self {
            up_record: None,
            over_record: None,
            down_record: None,
            hit_record: None,
            current_state: ButtonState::Up,
            is_mouse_over: false,
            is_mouse_down: false,
            enabled: true,
            track_as_menu: false,
        }
    }

    /// Returns the current state of the button.
    #[inline]
    pub fn button_current_state(&self) -> ButtonState {
        self.current_state
    }

    /// Sets the display object for the Up state.
    #[inline]
    pub fn button_set_up_record(&mut self, obj: DisplayObject) {
        self.up_record = Some(Box::new(obj));
    }

    /// Sets the display object for the Over state.
    #[inline]
    pub fn button_set_over_record(&mut self, obj: DisplayObject) {
        self.over_record = Some(Box::new(obj));
    }

    /// Sets the display object for the Down state.
    #[inline]
    pub fn button_set_down_record(&mut self, obj: DisplayObject) {
        self.down_record = Some(Box::new(obj));
    }

    /// Sets the display object for the HitTest geometry.
    #[inline]
    pub fn button_set_hit_record(&mut self, obj: DisplayObject) {
        self.hit_record = Some(Box::new(obj));
    }

    /// Returns the display object for the Up state, if any.
    #[inline]
    pub fn button_up_record(&self) -> Option<&DisplayObject> {
        self.up_record.as_deref()
    }

    /// Returns the display object for the Over state, if any.
    #[inline]
    pub fn button_over_record(&self) -> Option<&DisplayObject> {
        self.over_record.as_deref()
    }

    /// Returns the display object for the Down state, if any.
    #[inline]
    pub fn button_down_record(&self) -> Option<&DisplayObject> {
        self.down_record.as_deref()
    }

    /// Returns the display object for the HitTest geometry, if any.
    #[inline]
    pub fn button_hit_record(&self) -> Option<&DisplayObject> {
        self.hit_record.as_deref()
    }

    /// Returns whether the button is interactive.
    #[inline]
    pub fn button_is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enables or disables button interactivity.
    #[inline]
    pub fn button_set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_state = ButtonState::Up;
            self.is_mouse_over = false;
            self.is_mouse_down = false;
        }
    }

    /// Returns whether menu tracking is active.
    #[inline]
    pub fn button_track_as_menu(&self) -> bool {
        self.track_as_menu
    }

    /// Configures menu tracking.
    #[inline]
    pub fn button_set_track_as_menu(&mut self, track: bool) {
        self.track_as_menu = track;
    }

    /// Returns the active display record corresponding to the current state.
    pub fn button_active_record(&self) -> Option<&DisplayObject> {
        match self.current_state {
            ButtonState::Down => self
                .down_record
                .as_deref()
                .or(self.over_record.as_deref())
                .or(self.up_record.as_deref()),
            ButtonState::Over => self.over_record.as_deref().or(self.up_record.as_deref()),
            ButtonState::Up => self.up_record.as_deref(),
            ButtonState::HitTest => self.hit_record.as_deref().or(self.up_record.as_deref()),
        }
    }

    /// Processes mouse move over or away from button target.
    pub fn button_process_mouse_move(&mut self, hit: bool) -> Option<ButtonEvent> {
        if !self.enabled {
            return None;
        }

        if hit {
            if !self.is_mouse_over {
                self.is_mouse_over = true;
                if self.is_mouse_down {
                    self.current_state = ButtonState::Down;
                } else {
                    self.current_state = ButtonState::Over;
                }
                Some(ButtonEvent::RollOver)
            } else {
                None
            }
        } else if self.is_mouse_over {
            self.is_mouse_over = false;
            self.current_state = ButtonState::Up;
            Some(ButtonEvent::RollOut)
        } else {
            None
        }
    }

    /// Processes mouse button down event.
    pub fn button_process_mouse_down(&mut self, hit: bool) -> Option<ButtonEvent> {
        if !self.enabled {
            return None;
        }

        if hit {
            self.is_mouse_over = true;
            self.is_mouse_down = true;
            self.current_state = ButtonState::Down;
            Some(ButtonEvent::Press)
        } else {
            None
        }
    }

    /// Processes mouse button up event.
    pub fn button_process_mouse_up(&mut self, hit: bool) -> Option<ButtonEvent> {
        if !self.enabled {
            return None;
        }

        if self.is_mouse_down {
            self.is_mouse_down = false;
            if hit {
                self.is_mouse_over = true;
                self.current_state = ButtonState::Over;
                Some(ButtonEvent::Release)
            } else {
                self.is_mouse_over = false;
                self.current_state = ButtonState::Up;
                Some(ButtonEvent::ReleaseOutside)
            }
        } else {
            None
        }
    }
}
