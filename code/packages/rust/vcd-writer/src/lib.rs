//! # VCD Writer — Value Change Dump
//!
//! Streaming text writer for the VCD format defined in IEEE 1364-2005 §18.
//! VCD files are read by GTKWave, Surfer, ModelSim, and every other waveform
//! viewer.
//!
//! The writer is intentionally **decoupled from any specific simulator**: you
//! feed it time-stamped value changes via `value_change()`. An [`attach`]
//! helper integrates it with the `hardware-vm` event callback interface.
//!
//! ## VCD file structure
//!
//! ```text
//! $date   2026-06-13 12:00:00 UTC   $end
//! $version Silicon-Stack VCD Writer 0.1.0 $end
//! $timescale 1ps $end
//! $scope module adder $end
//!   $var wire 1 ! clk $end
//!   $var wire 4 " a [3:0] $end
//! $upscope $end
//! $enddefinitions $end
//! #0
//! $dumpvars
//! b0000 "
//! $end
//! #5000
//! b0011 "
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use vcd_writer::VcdWriter;
//!
//! let mut vcd = VcdWriter::new("1ps");
//! vcd.open_scope("adder");
//! let a_id  = vcd.declare("a",   4, "wire");
//! let sum_id = vcd.declare("sum", 5, "wire");
//! vcd.close_scope();
//! vcd.end_definitions();
//!
//! vcd.time(0);
//! vcd.value_change(&a_id,   0);
//! vcd.value_change(&sum_id, 0);
//!
//! vcd.time(10);
//! vcd.value_change(&a_id,   3);
//! vcd.value_change(&sum_id, 8); // 3 + 5 = 8
//!
//! let text = vcd.finish();
//! assert!(text.contains("#10"));
//! assert!(text.contains("b1000")); // 8 in binary
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// VarDef
// ---------------------------------------------------------------------------

/// One variable declared in the VCD header.
#[derive(Debug, Clone, PartialEq)]
pub struct VarDef {
    pub name: String,
    pub width: u32,
    pub var_id: String,
    pub kind: String,
}

// ---------------------------------------------------------------------------
// IdAllocator — compact printable-ASCII variable identifiers
// ---------------------------------------------------------------------------

/// Generates compact printable-ASCII identifiers ('!' → '~', then two-char, …).
///
/// VCD allows any printable ASCII in variable identifiers. We use the range
/// `!` (0x21) through `~` (0x7E), i.e. 94 characters. The first 94 variables
/// get single-character IDs; the next 94² get two-character IDs, etc.
struct IdAllocator {
    next: usize,
}

impl IdAllocator {
    fn new() -> Self { IdAllocator { next: 0 } }

    fn alloc(&mut self) -> String {
        let mut n = self.next;
        self.next += 1;
        let mut chars = Vec::new();
        loop {
            chars.push((b'!' + (n % 94) as u8) as char);
            n /= 94;
            if n == 0 { break; }
            n -= 1;
        }
        chars.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// VcdWriter
// ---------------------------------------------------------------------------

/// Streaming VCD writer. Produces a complete VCD document in a `String`.
///
/// Two phases:
/// 1. **Header** — `open_scope`, `declare`, `close_scope`, `end_definitions`.
/// 2. **Body** — `time(t)` then `value_change(id, value)` pairs.
///
/// Finalise with [`finish`](Self::finish) to retrieve the accumulated text.
pub struct VcdWriter {
    timescale: String,
    buf: String,
    id_alloc: IdAllocator,
    defs_ended: bool,
    cur_time: Option<u64>,
    last_values: HashMap<String, i64>,
    var_defs: HashMap<String, VarDef>,
    scope_depth: usize,
}

impl VcdWriter {
    /// Create a new writer with the given timescale (e.g. `"1ps"`, `"1ns"`).
    pub fn new(timescale: &str) -> Self {
        let mut w = VcdWriter {
            timescale: timescale.to_string(),
            buf: String::new(),
            id_alloc: IdAllocator::new(),
            defs_ended: false,
            cur_time: None,
            last_values: HashMap::new(),
            var_defs: HashMap::new(),
            scope_depth: 0,
        };
        w.write_header_preamble();
        w
    }

    // ------------------------------------------------------------------
    // Header
    // ------------------------------------------------------------------

    fn write_header_preamble(&mut self) {
        self.emit("$date 2026-06-13 00:00:00 UTC $end\n");
        self.emit("$version Silicon-Stack VCD Writer 0.1.0 $end\n");
        self.emit(&format!("$timescale {} $end\n", self.timescale));
    }

    /// Open a hierarchical scope (writes `$scope module <name> $end`).
    pub fn open_scope(&mut self, name: &str) {
        self.open_scope_kind(name, "module");
    }

    /// Open a scope with an explicit kind (e.g. `"module"`, `"task"`, `"begin"`).
    pub fn open_scope_kind(&mut self, name: &str, kind: &str) {
        self.emit(&format!("$scope {kind} {name} $end\n"));
        self.scope_depth += 1;
    }

    /// Close the current scope (writes `$upscope $end`).
    pub fn close_scope(&mut self) {
        self.emit("$upscope $end\n");
        if self.scope_depth > 0 { self.scope_depth -= 1; }
    }

    /// Declare a variable. Returns the compact VCD identifier for this variable.
    pub fn declare(&mut self, name: &str, width: u32, kind: &str) -> String {
        let var_id = self.id_alloc.alloc();
        let def = VarDef { name: name.to_string(), width, var_id: var_id.clone(), kind: kind.to_string() };
        if width > 1 {
            self.emit(&format!("$var {kind} {width} {var_id} {name} [{}:0] $end\n", width - 1));
        } else {
            self.emit(&format!("$var {kind} {width} {var_id} {name} $end\n"));
        }
        self.var_defs.insert(var_id.clone(), def);
        var_id
    }

    /// End the definitions section (writes `$enddefinitions $end`).
    /// Called automatically before the first `time()` call if not done manually.
    pub fn end_definitions(&mut self) {
        while self.scope_depth > 0 { self.close_scope(); }
        self.emit("$enddefinitions $end\n");
        self.defs_ended = true;
    }

    // ------------------------------------------------------------------
    // Body
    // ------------------------------------------------------------------

    /// Advance to simulation time `t`. Must be non-decreasing.
    pub fn time(&mut self, t: u64) {
        if !self.defs_ended { self.end_definitions(); }
        if self.cur_time != Some(t) {
            self.emit(&format!("#{t}\n"));
            self.cur_time = Some(t);
        }
    }

    /// Emit a `$dumpvars` block with initial values for all declared variables.
    pub fn dump_initial(&mut self, values: &HashMap<String, i64>) {
        if self.cur_time.is_none() { self.time(0); }
        let ids: Vec<String> = self.var_defs.keys().cloned().collect();
        self.emit("$dumpvars\n");
        for id in &ids {
            let v = values.get(id).copied().unwrap_or(0);
            let line = self.format_value_change(id, v);
            self.emit(&line);
            self.last_values.insert(id.clone(), v);
        }
        self.emit("$end\n");
    }

    /// Emit one value change for the variable with the given `var_id`.
    /// Skips silently if the value has not changed since last emit.
    pub fn value_change(&mut self, var_id: &str, value: i64) {
        if self.last_values.get(var_id) == Some(&value) { return; }
        self.last_values.insert(var_id.to_string(), value);
        let line = self.format_value_change(var_id, value);
        self.emit(&line);
    }

    /// Convenience: advance time then emit a value change.
    pub fn value_change_at(&mut self, t: u64, var_id: &str, value: i64) {
        self.time(t);
        self.value_change(var_id, value);
    }

    /// Return the accumulated VCD text and consume the writer.
    pub fn finish(self) -> String {
        self.buf
    }

    /// Borrow the accumulated VCD text so far (without consuming).
    pub fn text(&self) -> &str {
        &self.buf
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn emit(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    fn format_value_change(&self, var_id: &str, value: i64) -> String {
        let def = match self.var_defs.get(var_id) {
            Some(d) => d,
            None => return String::new(),
        };
        if def.kind == "real" {
            return format!("r{value} {var_id}\n");
        }
        if def.width == 1 {
            format!("{}{var_id}\n", value & 1)
        } else {
            let masked = value & ((1i64 << def.width) - 1);
            let bits = format!("{masked:b}");
            format!("b{bits} {var_id}\n")
        }
    }
}

// ---------------------------------------------------------------------------
// Attach helper: integrate with hardware-vm event callbacks
// ---------------------------------------------------------------------------

/// A value-change event with a time, signal name, and new value.
/// Matches the interface of `hardware_vm::Event` (duck-typed via closure).
pub struct SignalEvent {
    pub time: u64,
    pub signal: String,
    pub new_value: i64,
}

/// Build the state needed to route hardware-vm events to a `VcdWriter`.
///
/// Returns a closure that forwards events to the writer through a shared
/// channel. Callers collect events and replay them via `replay_events`.
pub fn attach(name_to_var_id: HashMap<String, String>) -> impl Fn(SignalEvent) -> Option<(u64, String, i64)> {
    move |ev: SignalEvent| {
        name_to_var_id.get(&ev.signal).map(|id| (ev.time, id.clone(), ev.new_value))
    }
}
