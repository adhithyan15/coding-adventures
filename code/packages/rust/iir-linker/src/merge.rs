//! Module merging — the second pass of the static linker.
//!
//! # Algorithm
//!
//! After `resolve.rs` verifies that every import is satisfied, `merge_modules`
//! collapses the set of `IIRModule`s into one:
//!
//! 1. **Rename colliding private functions.**  If module A has a private
//!    function `"helper"` and module B also has `"helper"`, both survive in the
//!    merged module under the names `"A::helper"` and `"B::helper"`.  Functions
//!    that are *exported* keep their original name (their callers use that name).
//!
//! 2. **Rewrite call instructions.**  Any `call` instruction whose callee name
//!    matches an import's `local_name()` is rewritten to the merged name of the
//!    exported function.
//!
//! 3. **Preserve entry_point.**  The first module in the slice that has an
//!    `entry_point` set wins.
//!
//! 4. **Clear exports/imports.**  The merged module is self-contained — all
//!    cross-module references have been inlined, so the export/import lists are
//!    emptied.
//!
//! # Name collision strategy
//!
//! A function name is *claimed* by the first module that defines it.  Subsequent
//! modules with the same name prefix their functions with `"<module_name>::"`.
//! Exports always win: if module A exports `"sqrt"`, any private `"sqrt"` in
//! module B becomes `"B::sqrt"`.

use std::collections::HashMap;

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;

use crate::resolve::ResolvedExport;

/// Merge a set of resolved modules into one `IIRModule`.
///
/// `export_map` is the result of `build_export_map` and is used to look up
/// the merged name of each exported function.
///
/// # Preconditions
///
/// All imports must already be resolved (no `Unresolved` errors).  The caller
/// (`linker.rs`) is responsible for ensuring this.
pub fn merge_modules<'a>(
    modules: &'a [IIRModule],
    export_map: &HashMap<(String, String), ResolvedExport<'a>>,
) -> IIRModule {
    // Step 1: Decide the merged name for every function in every module.
    //
    // We do a two-step process:
    //   a. Collect all *exported* function names first (these must not be
    //      prefixed — external callers know their names).
    //   b. For private functions, prefix with "<module>::" if the bare name
    //      is already taken.

    let mut merged_name_map: HashMap<(*const IIRFunction, usize), String> = HashMap::new();
    let mut global_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Pass 1a: Claim all exported public names.
    for module in modules {
        for export in &module.exports {
            let public_name = export.public_name().to_string();
            global_names.insert(public_name.clone());
            // Associate this export's underlying function with the public name.
            if let Some(fn_) = module.get_function(&export.function_name) {
                let key = (fn_ as *const IIRFunction, fn_ as *const IIRFunction as usize);
                merged_name_map.insert(key, public_name);
            }
        }
    }

    // Pass 1b: Assign names to private functions.
    for module in modules {
        for fn_ in &module.functions {
            let ptr_key = (fn_ as *const IIRFunction, fn_ as *const IIRFunction as usize);
            if merged_name_map.contains_key(&ptr_key) {
                continue; // Already named (it's an exported function).
            }
            let name = if global_names.contains(&fn_.name) {
                // Collision — prefix with module name.
                //
                // Security note: if a *different* module already has a function
                // whose name happens to equal `"<module>::<fn_name>"` (e.g.
                // module "a" exports "b::c" and module "b" has private "c"), the
                // prefixed name would collide a second time.  To guarantee
                // uniqueness we detect this case and append a monotonically
                // increasing counter until we find a free slot.  This prevents
                // the second `merged_name_map.insert` from silently overwriting
                // the first entry and thereby redirecting call-rewrites to the
                // wrong function.
                let base = format!("{}::{}", module.name, fn_.name);
                if !global_names.contains(&base) {
                    base
                } else {
                    // Double-collision: append a counter to guarantee uniqueness.
                    // We start at 0 and increment until we find a free slot.
                    // In practice this loop runs at most once — it fires only
                    // when an adversarial (or pathological) module name was chosen
                    // specifically to trigger the base collision.
                    // Safety cap: the number of distinct names we could ever
                    // need is bounded by the total number of functions in all
                    // modules.  If we exceed that count, every possible name
                    // slot is occupied by adversarially-crafted input — panic
                    // rather than spin indefinitely (DoS guard).
                    let max_iterations = modules.iter().map(|m| m.functions.len()).sum::<usize>() + 1;
                    let mut counter: usize = 0;
                    loop {
                        assert!(
                            counter <= max_iterations,
                            "name-collision counter exceeded safe bound ({max_iterations}): \
                             adversarial module names suspected"
                        );
                        let candidate = format!("{base}${counter}");
                        if !global_names.contains(&candidate) {
                            break candidate;
                        }
                        counter += 1;
                    }
                }
            } else {
                fn_.name.clone()
            };
            global_names.insert(name.clone());
            merged_name_map.insert(ptr_key, name);
        }
    }

    // Step 2: Build an import-resolution map for each module:
    //   local_call_name → merged_function_name
    //
    // This lets us rewrite `call("sqrt", …)` → `call("math::sqrt", …)` etc.
    // when the function was renamed.
    let import_rewrite: HashMap<(String, String), String> = build_import_rewrite(modules, export_map, &merged_name_map);

    // Step 3: Assemble merged functions list.
    let mut merged_functions: Vec<IIRFunction> = Vec::new();
    for module in modules {
        let module_rewrites: HashMap<String, String> = import_rewrite
            .iter()
            .filter(|((mod_name, _), _)| mod_name == &module.name)
            .map(|((_, local), merged)| (local.clone(), merged.clone()))
            .collect();

        for fn_ in &module.functions {
            let ptr_key = (fn_ as *const IIRFunction, fn_ as *const IIRFunction as usize);
            let merged_fn_name = merged_name_map.get(&ptr_key).cloned().unwrap_or_else(|| fn_.name.clone());
            let rewritten = rewrite_function(fn_, &merged_fn_name, &module_rewrites);
            merged_functions.push(rewritten);
        }
    }

    // Step 4: Preserve entry_point from first module that has one.
    let entry_point = modules.iter().find_map(|m| m.entry_point.clone());

    // Step 5: Determine the merged module name and language.
    let name = modules.first().map(|m| m.name.clone()).unwrap_or_else(|| "merged".to_string());
    let language = modules.first().map(|m| m.language.clone()).unwrap_or_else(|| "unknown".to_string());

    IIRModule {
        name,
        functions: merged_functions,
        entry_point,
        language,
        exports: Vec::new(), // Merged module is self-contained.
        imports: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a map `(module_name, local_call_name) → merged_function_name`
/// for every import in every module.
fn build_import_rewrite<'a>(
    modules: &'a [IIRModule],
    export_map: &HashMap<(String, String), ResolvedExport<'a>>,
    merged_name_map: &HashMap<(*const IIRFunction, usize), String>,
) -> HashMap<(String, String), String> {
    let mut map: HashMap<(String, String), String> = HashMap::new();

    for module in modules {
        for import in &module.imports {
            let export_key = (import.module_name.clone(), import.function_name.clone());
            if let Some(resolved) = export_map.get(&export_key) {
                // Look up what the exported function was renamed to.
                let fn_ = resolved.function;
                let ptr_key = (fn_ as *const IIRFunction, fn_ as *const IIRFunction as usize);
                if let Some(merged_name) = merged_name_map.get(&ptr_key) {
                    map.insert(
                        (module.name.clone(), import.local_name().to_string()),
                        merged_name.clone(),
                    );
                }
            }
        }
    }

    map
}

/// Produce a new `IIRFunction` with:
/// - Its name replaced by `new_name`.
/// - All `call` instructions that target a key in `call_rewrites` updated
///   to use the rewritten callee name.
fn rewrite_function(
    fn_: &IIRFunction,
    new_name: &str,
    call_rewrites: &HashMap<String, String>,
) -> IIRFunction {
    let new_instrs: Vec<IIRInstr> = fn_
        .instructions
        .iter()
        .map(|instr| rewrite_instr(instr, call_rewrites))
        .collect();

    IIRFunction {
        name: new_name.to_string(),
        params: fn_.params.clone(),
        return_type: fn_.return_type.clone(),
        instructions: new_instrs,
        register_count: fn_.register_count,
        type_status: fn_.type_status.clone(),
        call_count: 0, // reset
        feedback_slots: std::collections::HashMap::new(),
        source_map: fn_.source_map.clone(),
        param_refinements: fn_.param_refinements.clone(),
        return_refinement: fn_.return_refinement.clone(),
    }
}

/// Rewrite a `call` instruction's callee operand if it appears in `rewrites`.
///
/// The callee is always `srcs[0]` as an `Operand::Var` for IIR `call`
/// instructions.  Non-call instructions are returned unchanged.
fn rewrite_instr(instr: &IIRInstr, rewrites: &HashMap<String, String>) -> IIRInstr {
    if instr.op != "call" || rewrites.is_empty() {
        return instr.clone();
    }
    // srcs[0] is the callee name as Operand::Var.
    if let Some(Operand::Var(callee)) = instr.srcs.first() {
        if let Some(new_callee) = rewrites.get(callee) {
            let mut new_srcs = instr.srcs.clone();
            new_srcs[0] = Operand::Var(new_callee.clone());
            return IIRInstr {
                op: instr.op.clone(),
                dest: instr.dest.clone(),
                srcs: new_srcs,
                type_hint: instr.type_hint.clone(),
                may_alloc: instr.may_alloc,
                observed_type: instr.observed_type.clone(),
                observation_count: instr.observation_count,
                observed_slot: instr.observed_slot.clone(),
                deopt_anchor: instr.deopt_anchor,
                ic_slot: instr.ic_slot,
            };
        }
    }
    instr.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module_exports::IIRExport;

    fn make_fn(name: &str) -> IIRFunction {
        IIRFunction::new(
            name,
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        )
    }

    fn make_fn_calling(name: &str, callee: &str) -> IIRFunction {
        IIRFunction::new(
            name,
            vec![],
            "void",
            vec![
                IIRInstr::new(
                    "call",
                    Some("_ret".into()),
                    vec![Operand::Var(callee.into())],
                    "void",
                ),
                IIRInstr::new("ret_void", None, vec![], "void"),
            ],
        )
    }

    #[test]
    fn merge_single_module_preserves_functions() {
        let mut m = IIRModule::new("app", "twig");
        m.add_or_replace(make_fn("main"));
        let modules = vec![m];
        let (map, _) = crate::resolve::build_export_map(&modules);
        let merged = merge_modules(&modules, &map);
        assert_eq!(merged.functions.len(), 1);
        assert!(merged.get_function("main").is_some());
    }

    #[test]
    fn merge_preserves_entry_point_from_first_module() {
        let mut m1 = IIRModule::new("app", "twig");
        m1.add_or_replace(make_fn("main"));
        m1.entry_point = Some("main".into());

        let mut m2 = IIRModule::new("lib", "twig");
        m2.entry_point = Some("start".into());
        m2.add_or_replace(make_fn("start"));

        let modules = vec![m1, m2];
        let (map, _) = crate::resolve::build_export_map(&modules);
        let merged = merge_modules(&modules, &map);
        assert_eq!(merged.entry_point, Some("main".into()));
    }

    #[test]
    fn merge_renames_private_collision() {
        // Both modules have a "helper" function — the second one gets prefixed.
        let mut m1 = IIRModule::new("a", "twig");
        m1.entry_point = None;
        m1.add_or_replace(make_fn("helper"));

        let mut m2 = IIRModule::new("b", "twig");
        m2.entry_point = None;
        m2.add_or_replace(make_fn("helper"));

        let modules = vec![m1, m2];
        let (map, _) = crate::resolve::build_export_map(&modules);
        let merged = merge_modules(&modules, &map);

        // One of them keeps "helper", the other gets "b::helper" (second module
        // collides with first).
        let names: std::collections::HashSet<&str> =
            merged.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains("helper"));
        assert!(names.contains("b::helper"));
    }

    #[test]
    fn merge_clears_exports_and_imports() {
        let mut m = IIRModule::new("app", "twig");
        m.add_or_replace(make_fn("main"));
        m.exports.push(IIRExport::new("main"));
        let modules = vec![m];
        let (map, _) = crate::resolve::build_export_map(&modules);
        let merged = merge_modules(&modules, &map);
        assert!(merged.exports.is_empty());
        assert!(merged.imports.is_empty());
    }

    #[test]
    fn rewrite_instr_rewrites_callee() {
        let instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("old_name".into()), Operand::Var("arg".into())],
            "void",
        );
        let mut rewrites = HashMap::new();
        rewrites.insert("old_name".to_string(), "new_name".to_string());
        let rewritten = rewrite_instr(&instr, &rewrites);
        assert_eq!(rewritten.srcs[0], Operand::Var("new_name".into()));
        assert_eq!(rewritten.srcs[1], Operand::Var("arg".into()));
    }

    #[test]
    fn rewrite_instr_leaves_non_call_unchanged() {
        let instr = IIRInstr::new(
            "add",
            Some("r".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "i64",
        );
        let mut rewrites = HashMap::new();
        rewrites.insert("a".to_string(), "x".to_string());
        let rewritten = rewrite_instr(&instr, &rewrites);
        // "add" is not a "call" — no rewrite.
        assert_eq!(rewritten.srcs[0], Operand::Var("a".into()));
    }
}
