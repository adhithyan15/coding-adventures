//! Core data types for jit-profiling-insights.
//!
//! Three layers of abstraction:
//!
//! 1. [`DispatchCost`] — a four-level enum ranking the cost of each dynamic
//!    dispatch strategy the JIT may choose.  The ordering
//!    `NONE < GUARD < GENERIC_CALL < DEOPT` is intentional and drives the
//!    impact formula.
//!
//! 2. [`TypeSite`] — one instruction in one function that the insight pass
//!    identified as a candidate for improvement.  It bundles the raw profiler
//!    data (`call_count`, `observed_type`) with a human-readable diagnosis
//!    (`savings_description`).
//!
//! 3. [`ProfilingReport`] — the top-level result of calling [`crate::analyze`].
//!    Contains the program name, total instruction count, and a ranked list of
//!    `TypeSite` entries.  Consumers can call [`ProfilingReport::top_n`],
//!    [`ProfilingReport::format_text`], or [`ProfilingReport::format_json`].

// ---------------------------------------------------------------------------
// DispatchCost
// ---------------------------------------------------------------------------

/// How expensive is the dynamic dispatch path the JIT chose?
///
/// The numeric weight of each variant is used in the impact formula
/// `call_count × cost_weight`.
///
/// Ordered from cheapest to most expensive:
///
/// | Variant | Weight | Meaning |
/// |---|---|---|
/// | `None` | 0 | Statically typed — direct typed op, zero overhead |
/// | `Guard` | 1 | One `type_assert` branch per call |
/// | `GenericCall` | 10 | Full runtime dispatch table (~10× slower) |
/// | `Deopt` | 100 | Interpreter fallback on guard failure (~100× slower) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DispatchCost {
    /// No overhead — the instruction is statically typed.
    None,
    /// One `type_assert` guard per call.
    Guard,
    /// Full generic runtime dispatch (no concrete type inferred).
    GenericCall,
    /// Guard emitted but failed — interpreter fallback (deoptimisation).
    Deopt,
}

impl DispatchCost {
    /// Return the cost multiplier for the impact formula.
    ///
    /// These weights reflect the approximate relative cost of each dispatch
    /// strategy on a modern out-of-order CPU:
    ///
    /// - `None`        → 0   (no overhead; statically resolved)
    /// - `Guard`       → 1   (one conditional branch per call)
    /// - `GenericCall` → 10  (virtual dispatch + tag check ≈ 10× slower)
    /// - `Deopt`       → 100 (interpreter fallback ≈ 100× slower)
    pub fn weight(self) -> u64 {
        match self {
            DispatchCost::None        => 0,
            DispatchCost::Guard       => 1,
            DispatchCost::GenericCall => 10,
            DispatchCost::Deopt       => 100,
        }
    }

    /// Return the JSON/text serialisation name.
    pub fn as_str(self) -> &'static str {
        match self {
            DispatchCost::None        => "none",
            DispatchCost::Guard       => "guard",
            DispatchCost::GenericCall => "generic",
            DispatchCost::Deopt       => "deopt",
        }
    }
}

// ---------------------------------------------------------------------------
// TypeSite
// ---------------------------------------------------------------------------

/// One instruction-level hotspot identified by the insight pass.
///
/// A `TypeSite` answers three questions:
///
/// - *What happened?* — `instruction_op`, `call_count`, `observed_type`
/// - *Why is it expensive?* — `dispatch_cost`, `deopt_count`
/// - *What should you do?* — `savings_description` (human-readable advice)
#[derive(Debug, Clone)]
pub struct TypeSite {
    /// Name of the `IIRFunction` containing this instruction.
    pub function: String,

    /// The mnemonic of the hot instruction (e.g. `"add"`, `"cmp_lt"`).
    pub instruction_op: String,

    /// The SSA register whose `type_hint == "any"` is causing the overhead.
    /// The insight pass traces the data-flow chain back to find the root
    /// register, which is often a function parameter.
    pub source_register: String,

    /// The actual runtime type the profiler saw on this register,
    /// e.g. `"u8"`. `"polymorphic"` if multiple types were seen.
    pub observed_type: String,

    /// The declared type from the source program.  Almost always `"any"` for
    /// instructions the insight pass flags.
    pub type_hint: String,

    /// The classified dispatch strategy chosen by the JIT.
    pub dispatch_cost: DispatchCost,

    /// How many times this instruction executed (from `observation_count`).
    pub call_count: u64,

    /// How many times a guard on this register failed and triggered an
    /// interpreter fallback.  Zero unless `dispatch_cost == Deopt`.
    pub deopt_count: u64,

    /// A one-sentence human-readable explanation of what adding a type
    /// annotation would eliminate.
    pub savings_description: String,
}

impl TypeSite {
    /// Impact score = `call_count × cost_weight`.
    ///
    /// Higher is worse.  Used to sort sites from most to least urgent.
    pub fn impact(&self) -> u64 {
        self.call_count.saturating_mul(self.dispatch_cost.weight())
    }
}

// ---------------------------------------------------------------------------
// ProfilingReport
// ---------------------------------------------------------------------------

/// Top-level output of the insight pass.
///
/// Produced by [`crate::analyze`] and consumed by the CLI, LSP, REPL, or CI
/// tooling.  All fields are populated at construction time; `sites` is
/// expected to be pre-sorted by impact (highest first).
#[derive(Debug, Clone)]
pub struct ProfilingReport {
    /// Friendly label for the output — typically the IIRModule name or the
    /// source file name.
    pub program_name: String,

    /// Sum of `observation_count` across all instructions in all functions.
    /// Used to compute the percentage of overhead that each site represents.
    pub total_instructions_executed: u64,

    /// Ranked list of `TypeSite` entries.  Sorted by impact (highest first)
    /// by `analyze()` before being stored here.
    pub sites: Vec<TypeSite>,
}

impl ProfilingReport {
    /// Return the top *n* sites by impact score.
    ///
    /// The list is already sorted; this is just a slice.
    pub fn top_n(&self, n: usize) -> &[TypeSite] {
        let end = n.min(self.sites.len());
        &self.sites[..end]
    }

    /// Return a deduplicated, order-preserving list of function names that
    /// have at least one non-NONE dispatch site.
    pub fn functions_with_issues(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for site in &self.sites {
            if site.dispatch_cost != DispatchCost::None
                && !seen.contains(site.function.as_str())
            {
                seen.insert(site.function.as_str());
                result.push(site.function.as_str());
            }
        }
        result
    }

    /// Return `true` if any site is classified as `Deopt`.
    pub fn has_deopts(&self) -> bool {
        self.sites.iter().any(|s| s.dispatch_cost == DispatchCost::Deopt)
    }

    /// Render the report as a human-readable text string.
    ///
    /// The format uses emoji markers so it renders in terminals, CI logs,
    /// and Markdown-capable viewers.
    ///
    /// # Example output
    ///
    /// ```text
    /// JIT Profiling Report — fibonacci (8,388,608 total instructions)
    /// ═══════════════════════════════════════════════════════════════
    ///
    /// 🔴 HIGH IMPACT  fibonacci::add
    ///   Source: %r0 (type_hint="any")
    ///   Observed: u8 on 1,048,576 calls (12% of total)
    ///   Cost: GUARD — would eliminate 1 type_assert per call
    ///   Estimated speedup: ~12%
    ///
    /// ✅ No deoptimisations occurred.
    ///
    /// Summary: 1 annotation site would eliminate ~12% of total overhead.
    /// ```
    pub fn format_text(&self) -> String {
        let mut lines = Vec::new();
        let title = format!(
            "JIT Profiling Report — {} ({} total instructions)",
            self.program_name,
            format_number(self.total_instructions_executed),
        );
        lines.push(title.clone());
        lines.push("═".repeat(title.chars().count()));

        let active: Vec<&TypeSite> = self.sites
            .iter()
            .filter(|s| s.dispatch_cost != DispatchCost::None)
            .collect();

        if active.is_empty() {
            lines.push(String::new());
            lines.push("✅ No dispatch overhead detected — all hot paths are typed.".into());
            return lines.join("\n");
        }

        for site in &active {
            lines.push(String::new());
            let (label, icon) = tier_label(site);
            lines.push(format!("{icon} {label}  {}::{}", site.function, site.instruction_op));
            lines.push(format!(
                "  Source: {} (type_hint={:?})",
                site.source_register, site.type_hint,
            ));

            let pct = if self.total_instructions_executed > 0 {
                let p = 100.0 * site.call_count as f64
                    / self.total_instructions_executed as f64;
                format!(" ({:.0}% of total)", p)
            } else {
                String::new()
            };
            lines.push(format!(
                "  Observed: {} on {} calls{}",
                site.observed_type,
                format_number(site.call_count),
                pct,
            ));

            let cost_name = site.dispatch_cost.as_str().to_uppercase();
            lines.push(format!("  Cost: {cost_name} — {}", site.savings_description));

            if site.deopt_count > 0 {
                lines.push(format!(
                    "  Deoptimisations: {} guard failures",
                    format_number(site.deopt_count),
                ));
            }

            if let Some(speedup) = estimate_speedup(site, self.total_instructions_executed) {
                lines.push(format!("  Estimated speedup: ~{speedup}%"));
            }
        }

        lines.push(String::new());
        if self.has_deopts() {
            let cnt = self.sites.iter().filter(|s| s.dispatch_cost == DispatchCost::Deopt).count();
            lines.push(format!(
                "⚠️  {cnt} deoptimisation(s) detected — highest priority fixes.",
            ));
        } else {
            lines.push("✅ No deoptimisations occurred.".into());
        }

        let n = active.len();
        let noun = if n == 1 { "site" } else { "sites" };
        let total_savings: u64 = active.iter()
            .filter_map(|s| estimate_speedup(s, self.total_instructions_executed))
            .sum();
        lines.push(format!(
            "\nSummary: {n} annotation {noun} would eliminate ~{total_savings}% of total overhead.",
        ));

        lines.join("\n")
    }

    /// Render the report as a pretty-printed JSON string.
    ///
    /// # Example output
    ///
    /// ```text
    /// {
    ///   "program_name": "fibonacci",
    ///   "total_instructions_executed": 8388608,
    ///   "sites": [ ... ]
    /// }
    /// ```
    pub fn format_json(&self) -> String {
        let sites_json: Vec<String> = self.sites.iter().map(site_to_json).collect();
        format!(
            "{{\n  \"program_name\": {:?},\n  \"total_instructions_executed\": {},\n  \"sites\": [\n{}\n  ]\n}}",
            self.program_name,
            self.total_instructions_executed,
            sites_json.join(",\n"),
        )
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn format_number(n: u64) -> String {
    // Simple thousands-separator formatter (no external dep).
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn tier_label(site: &TypeSite) -> (&'static str, &'static str) {
    match site.dispatch_cost {
        DispatchCost::Deopt => ("CRITICAL", "🚨"),
        DispatchCost::GenericCall | DispatchCost::Guard => {
            let impact = site.impact();
            if impact >= 100_000 {
                ("HIGH IMPACT", "🔴")
            } else if impact >= 1_000 {
                ("MEDIUM IMPACT", "🟡")
            } else {
                ("LOW IMPACT", "🟢")
            }
        }
        DispatchCost::None => ("NO IMPACT", "✅"),
    }
}

fn estimate_speedup(site: &TypeSite, total: u64) -> Option<u64> {
    if total == 0 || site.dispatch_cost == DispatchCost::None {
        return None;
    }
    let savings = site.call_count.saturating_mul(site.dispatch_cost.weight());
    let pct = (100 * savings) / total;
    if pct > 0 { Some(pct) } else { None }
}

fn site_to_json(site: &TypeSite) -> String {
    format!(
        concat!(
            "    {{\n",
            "      \"function\": {:?},\n",
            "      \"instruction_op\": {:?},\n",
            "      \"source_register\": {:?},\n",
            "      \"observed_type\": {:?},\n",
            "      \"type_hint\": {:?},\n",
            "      \"dispatch_cost\": {:?},\n",
            "      \"call_count\": {},\n",
            "      \"deopt_count\": {},\n",
            "      \"savings_description\": {:?},\n",
            "      \"impact\": {}\n",
            "    }}",
        ),
        site.function,
        site.instruction_op,
        site.source_register,
        site.observed_type,
        site.type_hint,
        site.dispatch_cost.as_str(),
        site.call_count,
        site.deopt_count,
        site.savings_description,
        site.impact(),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_site(cost: DispatchCost, calls: u64) -> TypeSite {
        TypeSite {
            function: "fib".into(),
            instruction_op: "add".into(),
            source_register: "%r0".into(),
            observed_type: "u8".into(),
            type_hint: "any".into(),
            dispatch_cost: cost,
            call_count: calls,
            deopt_count: 0,
            savings_description: "test".into(),
        }
    }

    #[test]
    fn dispatch_cost_weight() {
        assert_eq!(DispatchCost::None.weight(), 0);
        assert_eq!(DispatchCost::Guard.weight(), 1);
        assert_eq!(DispatchCost::GenericCall.weight(), 10);
        assert_eq!(DispatchCost::Deopt.weight(), 100);
    }

    #[test]
    fn dispatch_cost_as_str() {
        assert_eq!(DispatchCost::None.as_str(), "none");
        assert_eq!(DispatchCost::Guard.as_str(), "guard");
        assert_eq!(DispatchCost::GenericCall.as_str(), "generic");
        assert_eq!(DispatchCost::Deopt.as_str(), "deopt");
    }

    #[test]
    fn type_site_impact_formula() {
        let guard_site = make_site(DispatchCost::Guard, 1_000_000);
        assert_eq!(guard_site.impact(), 1_000_000);

        let generic_site = make_site(DispatchCost::GenericCall, 50_000);
        assert_eq!(generic_site.impact(), 500_000);

        let deopt_site = make_site(DispatchCost::Deopt, 10);
        assert_eq!(deopt_site.impact(), 1_000);

        let none_site = make_site(DispatchCost::None, 1_000_000);
        assert_eq!(none_site.impact(), 0);
    }

    #[test]
    fn profiling_report_top_n() {
        let report = ProfilingReport {
            program_name: "test".into(),
            total_instructions_executed: 100,
            sites: vec![
                make_site(DispatchCost::Guard, 50),
                make_site(DispatchCost::GenericCall, 10),
            ],
        };
        assert_eq!(report.top_n(1).len(), 1);
        assert_eq!(report.top_n(10).len(), 2); // capped at len
    }

    #[test]
    fn profiling_report_has_deopts_false() {
        let report = ProfilingReport {
            program_name: "test".into(),
            total_instructions_executed: 100,
            sites: vec![make_site(DispatchCost::Guard, 50)],
        };
        assert!(!report.has_deopts());
    }

    #[test]
    fn profiling_report_has_deopts_true() {
        let report = ProfilingReport {
            program_name: "test".into(),
            total_instructions_executed: 100,
            sites: vec![make_site(DispatchCost::Deopt, 50)],
        };
        assert!(report.has_deopts());
    }

    #[test]
    fn profiling_report_empty_format_text() {
        let report = ProfilingReport {
            program_name: "empty".into(),
            total_instructions_executed: 0,
            sites: vec![],
        };
        let text = report.format_text();
        assert!(text.contains("No dispatch overhead detected"));
    }

    #[test]
    fn profiling_report_format_text_with_site() {
        let report = ProfilingReport {
            program_name: "fib".into(),
            total_instructions_executed: 1_000_000,
            sites: vec![make_site(DispatchCost::Guard, 500_000)],
        };
        let text = report.format_text();
        assert!(text.contains("fib::add"));
        assert!(text.contains("GUARD"));
    }

    #[test]
    fn profiling_report_format_json_contains_fields() {
        let report = ProfilingReport {
            program_name: "test".into(),
            total_instructions_executed: 42,
            sites: vec![],
        };
        let json = report.format_json();
        assert!(json.contains("\"program_name\""));
        assert!(json.contains("\"total_instructions_executed\""));
        assert!(json.contains("42"));
    }

    #[test]
    fn functions_with_issues_dedup() {
        let mut s1 = make_site(DispatchCost::Guard, 10);
        s1.function = "f".into();
        let mut s2 = make_site(DispatchCost::Guard, 5);
        s2.function = "f".into();
        let report = ProfilingReport {
            program_name: "t".into(),
            total_instructions_executed: 100,
            sites: vec![s1, s2],
        };
        let fns = report.functions_with_issues();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0], "f");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1_048_576), "1,048,576");
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1_000), "1,000");
    }
}
