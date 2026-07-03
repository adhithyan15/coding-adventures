//! # Hardware VM
//!
//! Event-driven simulator for HIR (Hardware Intermediate Representation).
//!
//! v0.1.0 scope: **combinational** circuits only.
//! - Continuous assignments (`ContAssign`) are evaluated reactively.
//! - Inputs are driven via [`HardwareVm::set_input`]; outputs are read via [`HardwareVm::read`].
//! - Every signal value-change fires registered [`HardwareVm::subscribe`] callbacks.
//!
//! Behavioral processes (`always`, `initial`, `wait`/`@`/`#`) are v0.2.0 work.
//!
//! ## How combinational simulation works
//!
//! ```text
//! assign sum = a + b   ←  "whenever a or b changes, re-evaluate sum"
//! ```
//!
//! 1. At init, build a **sensitivity map**: which signals appear in each ContAssign RHS?
//! 2. Evaluate every ContAssign once at t=0 (bootstrap).
//! 3. On `set_input(sig, val)`: update `sig`, then cascade re-evaluate all
//!    ContAssigns sensitive to `sig` until quiescence.
//!
//! ## Example — adder with JSON HIR
//!
//! ```rust
//! use hardware_vm::HardwareVm;
//!
//! let hir_json = r#"{
//!   "format": "HIR", "version": "0.1.0", "top": "adder",
//!   "modules": {
//!     "adder": {
//!       "name": "adder",
//!       "ports": [
//!         {"name": "a",   "type": {"kind": "vec", "width": 4}, "direction": "in"},
//!         {"name": "b",   "type": {"kind": "vec", "width": 4}, "direction": "in"},
//!         {"name": "sum", "type": {"kind": "vec", "width": 5}, "direction": "out"}
//!       ],
//!       "cont_assigns": [{
//!         "target": {"kind": "port_ref", "name": "sum"},
//!         "rhs":    {"kind": "binary", "op": "+",
//!                    "lhs": {"kind": "port_ref", "name": "a"},
//!                    "rhs": {"kind": "port_ref", "name": "b"}}
//!       }]
//!     }
//!   }
//! }"#;
//! let hir = hdl_ir::Hir::from_json(hir_json).unwrap();
//! let mut vm = HardwareVm::new(hir).unwrap();
//! vm.set_input("a", 3).unwrap();
//! vm.set_input("b", 5).unwrap();
//! assert_eq!(vm.read("sum"), 8);
//! ```

pub mod eval;

use std::collections::HashMap;

use hdl_ir::{ContAssign, Direction, Expr, Hir, Ty};

use crate::eval::{evaluate, referenced_signals};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A signal value-change event — the payload passed to every subscriber.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub time: u64,
    pub signal: String,
    pub new_value: i64,
    pub old_value: i64,
}

/// Statistics from a simulation run.
#[derive(Debug, Default)]
pub struct RunResult {
    pub final_time: u64,
    pub event_count: u64,
    pub cont_assign_runs: u64,
}

/// Errors returned by `HardwareVm`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    TopModuleNotFound(String),
    SignalNotFound(String),
    NotAnInput(String),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::TopModuleNotFound(n) => write!(f, "top module {n:?} not found in HIR"),
            VmError::SignalNotFound(n)    => write!(f, "signal {n:?} not found"),
            VmError::NotAnInput(n)        => write!(f, "signal {n:?} is not an input port"),
        }
    }
}

// ---------------------------------------------------------------------------
// HardwareVm
// ---------------------------------------------------------------------------

type Subscriber = Box<dyn Fn(&Event) + Send + 'static>;

/// Event-driven simulator for an HIR document.
pub struct HardwareVm {
    hir: Hir,
    time: u64,
    values: HashMap<String, i64>,
    widths: HashMap<String, u32>,
    cont_assigns: Vec<(ContAssign, Vec<String>)>,
    signal_to_cas: HashMap<String, Vec<usize>>,
    subscribers: Vec<Subscriber>,
    forced: HashMap<String, i64>,
    cont_assign_runs: u64,
    event_count: u64,
}

impl HardwareVm {
    /// Construct and initialise the simulator for `hir`.
    pub fn new(hir: Hir) -> Result<Self, VmError> {
        let mut vm = HardwareVm {
            hir,
            time: 0,
            values: HashMap::new(),
            widths: HashMap::new(),
            cont_assigns: Vec::new(),
            signal_to_cas: HashMap::new(),
            subscribers: Vec::new(),
            forced: HashMap::new(),
            cont_assign_runs: 0,
            event_count: 0,
        };
        vm.initialize()?;
        Ok(vm)
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Drive a top-level input (or inout) port.
    pub fn set_input(&mut self, signal: &str, value: i64) -> Result<(), VmError> {
        let top_name = self.hir.top.clone();
        let module = self.hir.modules.get(&top_name)
            .ok_or_else(|| VmError::TopModuleNotFound(top_name.clone()))?;
        let port = module.ports.iter().find(|p| p.name == signal)
            .ok_or_else(|| VmError::SignalNotFound(signal.to_string()))?;
        if port.direction != Direction::In && port.direction != Direction::Inout {
            return Err(VmError::NotAnInput(signal.to_string()));
        }
        self.update_signal(signal, value);
        Ok(())
    }

    /// Read the current value of any signal (port or net).
    pub fn read(&self, signal: &str) -> i64 {
        self.lookup(signal)
    }

    /// Force a signal to a value, overriding any normal driver.
    pub fn force(&mut self, signal: &str, value: i64) {
        let old = self.values.get(signal).copied().unwrap_or(0);
        self.forced.insert(signal.to_string(), value);
        if old != value {
            self.values.insert(signal.to_string(), value);
            self.event_count += 1;
            let ev = Event { time: self.time, signal: signal.to_string(), new_value: value, old_value: old };
            for sub in &self.subscribers { sub(&ev); }
        }
    }

    /// Release a forced signal so normal drivers take over.
    pub fn release(&mut self, signal: &str) {
        self.forced.remove(signal);
        let cas: Vec<usize> = (0..self.cont_assigns.len())
            .filter(|&i| Self::ca_drives(&self.cont_assigns[i].0.target, signal))
            .collect();
        for idx in cas { self.run_cont_assign(idx); }
    }

    /// Register a callback called on every value-change event.
    pub fn subscribe<F: Fn(&Event) + Send + 'static>(&mut self, cb: F) {
        self.subscribers.push(Box::new(cb));
    }

    /// Current simulation statistics.
    pub fn stats(&self) -> RunResult {
        RunResult {
            final_time: self.time,
            event_count: self.event_count,
            cont_assign_runs: self.cont_assign_runs,
        }
    }

    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    fn initialize(&mut self) -> Result<(), VmError> {
        let top_name = self.hir.top.clone();
        let module = self.hir.modules.get(&top_name)
            .ok_or_else(|| VmError::TopModuleNotFound(top_name.clone()))?
            .clone();

        for p in &module.ports {
            self.values.insert(p.name.clone(), 0);
            self.widths.insert(p.name.clone(), ty_width(&p.ty));
        }
        for n in &module.nets {
            self.values.insert(n.name.clone(), 0);
            self.widths.insert(n.name.clone(), ty_width(&n.ty));
        }
        for ca in module.cont_assigns {
            let sens = referenced_signals(&ca.rhs);
            let idx = self.cont_assigns.len();
            for sig in &sens {
                self.signal_to_cas.entry(sig.clone()).or_default().push(idx);
            }
            self.cont_assigns.push((ca, sens));
        }
        // Bootstrap: evaluate all at t=0.
        for idx in 0..self.cont_assigns.len() {
            self.run_cont_assign(idx);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn lookup(&self, name: &str) -> i64 {
        if let Some(&v) = self.forced.get(name) { return v; }
        self.values.get(name).copied().unwrap_or(0)
    }

    fn update_signal(&mut self, name: &str, new_value: i64) {
        if self.forced.contains_key(name) { return; }
        let old = self.values.get(name).copied().unwrap_or(0);
        if old == new_value { return; }
        self.values.insert(name.to_string(), new_value);
        self.event_count += 1;
        let ev = Event { time: self.time, signal: name.to_string(), new_value, old_value: old };
        for sub in &self.subscribers { sub(&ev); }
        let dep_indices: Vec<usize> = self.signal_to_cas.get(name).cloned().unwrap_or_default();
        for idx in dep_indices { self.run_cont_assign(idx); }
    }

    fn run_cont_assign(&mut self, idx: usize) {
        self.cont_assign_runs += 1;
        let ca = self.cont_assigns[idx].0.clone();
        let rhs_value = {
            let values = &self.values;
            let forced = &self.forced;
            evaluate(&ca.rhs, &|n: &str| {
                if let Some(&v) = forced.get(n) { return v; }
                values.get(n).copied().unwrap_or(0)
            })
        };
        self.apply_lhs(&ca.target.clone(), rhs_value);
    }

    fn apply_lhs(&mut self, target: &Expr, value: i64) {
        match target {
            Expr::NetRef { name, .. } | Expr::PortRef { name, .. } => {
                self.update_signal(name, value);
            }
            Expr::Slice { base, msb, lsb, .. } => {
                if let Some(base_name) = extract_base_name(base) {
                    let (msb, lsb) = if msb >= lsb { (*msb, *lsb) } else { (*lsb, *msb) };
                    let width = msb - lsb + 1;
                    let mask = (1i64 << width) - 1;
                    let old = self.values.get(&base_name).copied().unwrap_or(0);
                    let new_bits = (value & mask) << lsb;
                    let clear_mask = !(mask << lsb);
                    self.update_signal(&base_name, (old & clear_mask) | new_bits);
                }
            }
            Expr::Concat { parts, .. } => {
                let widths: Vec<u32> = parts.iter()
                    .map(|p| self.widths_of(p))
                    .collect();
                let total: u32 = widths.iter().sum();
                let mut offset = total;
                for (part, w) in parts.iter().zip(widths.iter()) {
                    offset -= w;
                    let part_val = (value >> offset) & ((1i64 << w) - 1);
                    self.apply_lhs(part, part_val);
                }
            }
            _ => {}
        }
    }

    fn widths_of(&self, expr: &Expr) -> u32 {
        match expr {
            Expr::NetRef { name, .. } | Expr::PortRef { name, .. } => {
                self.widths.get(name.as_str()).copied().unwrap_or(1)
            }
            Expr::Slice { msb, lsb, .. } => (*msb as i64 - *lsb as i64).unsigned_abs() as u32 + 1,
            Expr::Concat { parts, .. } => parts.iter().map(|p| self.widths_of(p)).sum(),
            _ => 1,
        }
    }

    fn ca_drives(target: &Expr, signal: &str) -> bool {
        match target {
            Expr::NetRef { name, .. } | Expr::PortRef { name, .. } => name == signal,
            Expr::Slice { base, .. } => Self::ca_drives(base, signal),
            Expr::Concat { parts, .. } => parts.iter().any(|p| Self::ca_drives(p, signal)),
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ty_width(ty: &Ty) -> u32 {
    ty.width().unwrap_or(1)
}

fn extract_base_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::NetRef { name, .. } | Expr::PortRef { name, .. } => Some(name.clone()),
        Expr::Slice { base, .. } => extract_base_name(base),
        _ => None,
    }
}
