//! ISR (Interrupt Service Routine) Registry.
//!
//! Maps interrupt numbers to Rust handler closures. This is the "software
//! side" of interrupt handling: the IDT maps interrupt numbers to memory
//! addresses (hardware simulation), while the ISR Registry maps them to
//! actual Rust functions (emulation).

use std::collections::HashMap;
use crate::frame::InterruptFrame;

/// Type alias for ISR handler functions.
///
/// The frame contains saved CPU state; the second parameter is an opaque
/// kernel handle (we use `&mut dyn std::any::Any` to avoid circular deps).
pub type ISRHandler = Box<dyn FnMut(&mut InterruptFrame)>;

/// Maps interrupt numbers to Rust handler closures.
///
/// # Example
///
/// ```
/// use interrupt_handler::{ISRRegistry, InterruptFrame};
///
/// let mut registry = ISRRegistry::new();
/// registry.register(32, Box::new(|frame| {
///     // Handle timer interrupt
/// }));
/// ```
pub struct ISRRegistry {
    handlers: HashMap<usize, ISRHandler>,
}

impl ISRRegistry {
    /// Create an empty ISR registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Install a handler for the given interrupt number.
    /// Overwrites any previously registered handler.
    pub fn register(&mut self, interrupt_number: usize, handler: ISRHandler) {
        self.handlers.insert(interrupt_number, handler);
    }

    /// Call the registered handler for the given interrupt number.
    ///
    /// # Panics
    ///
    /// Panics if no handler is registered (double fault condition).
    pub fn dispatch(&mut self, interrupt_number: usize, frame: &mut InterruptFrame) {
        let handler = self
            .handlers
            .get_mut(&interrupt_number)
            .unwrap_or_else(|| panic!("No ISR handler registered for interrupt {interrupt_number}"));
        handler(frame);
    }

    /// Return `true` if a handler is registered for this interrupt number.
    pub fn has_handler(&self, interrupt_number: usize) -> bool {
        self.handlers.contains_key(&interrupt_number)
    }
}

impl Default for ISRRegistry {
    fn default() -> Self {
        Self::new()
    }
}
