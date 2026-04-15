#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelState {
    Constructed,
    Booted,
    Running,
}

pub struct Kernel {
    state: KernelState,
}

impl Kernel {
    pub const fn new() -> Self {
        Self {
            state: KernelState::Constructed,
        }
    }

    pub const fn state(&self) -> KernelState {
        self.state
    }

    pub fn boot(&mut self) {
        if self.state == KernelState::Constructed {
            self.state = KernelState::Booted;
        }
    }

    pub fn enter_running_state(&mut self) {
        if self.state == KernelState::Constructed {
            self.boot();
        }
        self.state = KernelState::Running;
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let kernel = Kernel::new();
        assert_eq!(kernel.state(), KernelState::Constructed);
    }

    #[test]
    fn test_boot_state() {
        let mut kernel = Kernel::new();
        kernel.boot();
        assert_eq!(kernel.state(), KernelState::Booted);
    }

    #[test]
    fn test_running_state() {
        let mut kernel = Kernel::new();
        kernel.enter_running_state();
        assert_eq!(kernel.state(), KernelState::Running);
    }
}
