//! HIR validation rules H1–H20 (subset implemented in v0.1.0).
//!
//! Validation catches obvious structural errors — missing references,
//! duplicate names, transitive self-instantiation — so that downstream
//! passes (synthesis, simulation) can trust the input.
//!
//! Rules not yet implemented (H5 width-mismatch, H7 single-driver, H10
//! combinational-loop) require deeper dataflow analysis and are left for
//! the elaboration / synthesis layers.

use std::collections::HashSet;

use crate::hir::Hir;
use crate::module::Level;

/// Result of a validation run.
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Run all implemented HIR validation rules on `hir`.
pub fn validate(hir: &Hir) -> ValidationReport {
    let mut report = ValidationReport::default();

    // H1 — top module exists.
    if !hir.modules.contains_key(&hir.top) {
        report.errors.push(format!("H1: top module {:?} not in HIR.modules", hir.top));
    }

    // H1b — no module name shadowed across libraries.
    let mut seen: HashSet<&str> = hir.modules.keys().map(String::as_str).collect();
    for lib in hir.libraries.values() {
        for name in lib.modules.keys() {
            if seen.contains(name.as_str()) {
                report.warnings.push(format!(
                    "module {name:?} shadowed across library {:?}", lib.name
                ));
            }
            seen.insert(name.as_str());
        }
    }

    // H3, H4, H6, H12 — per-module checks.
    for (mod_name, module) in &hir.modules {
        // H12: structural module must have no processes.
        if module.level == Level::Structural && !module.processes.is_empty() {
            report.errors.push(format!(
                "H12: module {mod_name:?} is structural but has {} process(es)",
                module.processes.len()
            ));
        }

        let port_names: HashSet<&str> = module.ports.iter().map(|p| p.name.as_str()).collect();
        let net_names: HashSet<&str> = module.nets.iter().map(|n| n.name.as_str()).collect();

        // Duplicate port / net names.
        if port_names.len() != module.ports.len() {
            report.errors.push(format!("module {mod_name:?}: duplicate port names"));
        }
        if net_names.len() != module.nets.len() {
            report.errors.push(format!("module {mod_name:?}: duplicate net names"));
        }

        // Name overlap warning.
        for name in port_names.intersection(&net_names) {
            report.warnings.push(format!(
                "module {mod_name:?}: {name:?} is both a port and a net"
            ));
        }

        // H3, H4 — instances reference known modules / valid ports.
        for inst in &module.instances {
            let target_mod = hir.modules.get(&inst.module).or_else(|| {
                hir.libraries.values().find_map(|lib| lib.modules.get(&inst.module))
            });

            if target_mod.is_none() {
                report.errors.push(format!(
                    "H3: instance {mod_name}.{}: unknown module {:?}",
                    inst.name, inst.module
                ));
                continue;
            }
            let target = target_mod.unwrap();
            let target_ports: HashSet<&str> =
                target.ports.iter().map(|p| p.name.as_str()).collect();

            for conn_pin in inst.connections.keys() {
                if !target_ports.contains(conn_pin.as_str()) {
                    report.errors.push(format!(
                        "H4: instance {mod_name}.{}: connection key {:?} \
                         is not a port of module {:?}",
                        inst.name, conn_pin, inst.module
                    ));
                }
            }
        }
    }

    // H20 — no transitive self-instantiation.
    for mod_name in hir.modules.keys() {
        check_no_self_instantiation(mod_name, hir, &mut report);
    }

    report
}

fn check_no_self_instantiation(start: &str, hir: &Hir, report: &mut ValidationReport) {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = vec![start];
    let mut first = true;

    while let Some(cur) = stack.pop() {
        if !first && cur == start {
            report.errors.push(format!(
                "H20: module {start:?} transitively instantiates itself"
            ));
            return;
        }
        first = false;
        if !seen.insert(cur) {
            continue;
        }

        let module = hir.modules.get(cur).or_else(|| {
            hir.libraries.values().find_map(|lib| lib.modules.get(cur))
        });
        if let Some(m) = module {
            for inst in &m.instances {
                stack.push(inst.module.as_str());
            }
        }
    }
}
