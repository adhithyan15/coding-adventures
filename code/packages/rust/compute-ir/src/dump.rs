//! Pretty-printer for [`ComputeGraph`].
//!
//! Produces the human-readable text shown in spec MX02 §"Inspection".
//! Used as a teaching artifact (it shows every transfer and its
//! estimated cost) and as a debugging tool (golden tests assert
//! byte-for-byte stability of the output).

use crate::graph::ComputeGraph;
use crate::placement::PlacedOp;
use core::fmt::Write;

impl ComputeGraph {
    /// Pretty-print the graph as text, one op per line, with executor
    /// assignments and estimated costs.
    ///
    /// Output format (stable across versions for golden testing):
    ///
    /// ```text
    /// ComputeGraph (format vN, K inputs, M outputs, P ops)
    ///   inputs:
    ///     tID: dtype shape   @ executor X buf Y
    ///     ...
    ///   ops:
    ///     [00]  alloc            buf B @ exec X   (S bytes)
    ///     [01]  transfer tID     exec A buf B  ->  exec C buf D   (≈ N µs)
    ///     [02]  compute  OPNAME  exec X tA tB -> tOUT             (≈ N µs)
    ///     [03]  free             buf B @ exec X
    ///   outputs:
    ///     tID: dtype shape   @ executor X buf Y
    /// ```
    ///
    /// Format is intentionally human-friendly (whitespace alignment,
    /// SI-like units for nanoseconds) rather than machine-friendly.
    /// Use [`to_bytes`](Self::to_bytes) for round-trippable output.
    pub fn dump(&self) -> String {
        let mut out = String::with_capacity(256);
        let _ = writeln!(
            out,
            "ComputeGraph (format v{}, {} inputs, {} outputs, {} ops)",
            self.format_version,
            self.inputs.len(),
            self.outputs.len(),
            self.ops.len()
        );

        if !self.inputs.is_empty() {
            out.push_str("  inputs:\n");
            for t in &self.inputs {
                let _ = writeln!(
                    out,
                    "    t{}: {} {}   @ {}",
                    t.id.0, t.dtype, t.shape, t.residency
                );
            }
        }

        if !self.constants.is_empty() {
            out.push_str("  constants:\n");
            for (ci, c) in self.constants.iter().enumerate() {
                let _ = writeln!(
                    out,
                    "    c{} → t{}   @ {}   ({} bytes)",
                    ci,
                    c.tensor.0,
                    c.residency,
                    c.bytes.len()
                );
            }
        }

        out.push_str("  ops:\n");
        for (i, op) in self.ops.iter().enumerate() {
            match op {
                PlacedOp::Alloc { residency, bytes } => {
                    let _ = writeln!(
                        out,
                        "    [{:02}]  alloc            buf {} @ exec {}   ({})",
                        i,
                        residency.buffer.0,
                        residency.executor.0,
                        format_bytes(*bytes)
                    );
                }
                PlacedOp::Free { residency } => {
                    let _ = writeln!(
                        out,
                        "    [{:02}]  free             buf {} @ exec {}",
                        i, residency.buffer.0, residency.executor.0
                    );
                }
                PlacedOp::Transfer {
                    tensor,
                    src,
                    dst,
                    bytes,
                    timing,
                } => {
                    let _ = writeln!(
                        out,
                        "    [{:02}]  transfer t{:<4}  {}  ->  {}   ({}, {})",
                        i,
                        tensor.0,
                        src,
                        dst,
                        format_bytes(*bytes),
                        format_ns(timing.estimated_ns)
                    );
                }
                PlacedOp::Compute {
                    op: under,
                    executor,
                    timing,
                } => {
                    let inputs_s = under
                        .inputs()
                        .iter()
                        .map(|t| format!("t{}", t.0))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = writeln!(
                        out,
                        "    [{:02}]  compute  {:<8} exec {} {} -> t{}                 ({})",
                        i,
                        op_name(under),
                        executor.0,
                        inputs_s,
                        under.output().0,
                        format_ns(timing.estimated_ns)
                    );
                }
            }
        }

        if !self.outputs.is_empty() {
            out.push_str("  outputs:\n");
            for t in &self.outputs {
                let _ = writeln!(
                    out,
                    "    t{}: {} {}   @ {}",
                    t.id.0, t.dtype, t.shape, t.residency
                );
            }
        }

        out
    }
}

/// Format a byte count using IEC units (KiB, MiB, …) with a single
/// decimal of precision when the rounded value would be small.
fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    if bytes < KIB {
        format!("{} B", bytes)
    } else if bytes < MIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else if bytes < GIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    }
}

/// Format a nanosecond count using SI-like units (ns, µs, ms, s).
fn format_ns(ns: u64) -> String {
    if ns < 1_000 {
        format!("≈ {} ns", ns)
    } else if ns < 1_000_000 {
        format!("≈ {:.1} µs", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("≈ {:.1} ms", ns as f64 / 1_000_000.0)
    } else {
        format!("≈ {:.2} s", ns as f64 / 1_000_000_000.0)
    }
}

fn op_name(op: &matrix_ir::Op) -> &'static str {
    use matrix_ir::Op;
    match op {
        Op::Neg { .. } => "neg",
        Op::Abs { .. } => "abs",
        Op::Sqrt { .. } => "sqrt",
        Op::Exp { .. } => "exp",
        Op::Log { .. } => "log",
        Op::Tanh { .. } => "tanh",
        Op::Recip { .. } => "recip",
        Op::Add { .. } => "add",
        Op::Sub { .. } => "sub",
        Op::Mul { .. } => "mul",
        Op::Div { .. } => "div",
        Op::Max { .. } => "max",
        Op::Min { .. } => "min",
        Op::Pow { .. } => "pow",
        Op::ReduceSum { .. } => "reduce_sum",
        Op::ReduceMax { .. } => "reduce_max",
        Op::ReduceMean { .. } => "reduce_mean",
        Op::Reshape { .. } => "reshape",
        Op::Transpose { .. } => "transpose",
        Op::Broadcast { .. } => "broadcast",
        Op::MatMul { .. } => "matmul",
        Op::Equal { .. } => "equal",
        Op::Less { .. } => "less",
        Op::Greater { .. } => "greater",
        Op::Where { .. } => "where",
        Op::Cast { .. } => "cast",
        Op::Const { .. } => "const",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_uses_iec_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(format_bytes(1024u64 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn format_ns_uses_si_units() {
        assert_eq!(format_ns(0), "≈ 0 ns");
        assert_eq!(format_ns(500), "≈ 500 ns");
        assert_eq!(format_ns(1500), "≈ 1.5 µs");
        assert_eq!(format_ns(1_500_000), "≈ 1.5 ms");
        assert_eq!(format_ns(1_500_000_000), "≈ 1.50 s");
    }
}
