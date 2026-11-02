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
//! 2. **Rewrite every function reference.**  Each module gets its own
//!    *rename map* `original name → merged name`, built from two sources:
//!
//!    * the module's **own functions** — so that when module B's private
//!      `helper` became `"B::helper"`, B's references to `helper` follow it;
//!    * the module's **imports** — `local_name()` → the merged name of the
//!      exported function it resolves to.
//!
//!    The map is then applied to every operand that *names a function*
//!    (see `function_name_slot`), not just to `call` callees.  The
//!    module's `entry_point` is mapped through the same table.
//!
//! 3. **Preserve entry_point.**  The first module in the slice that has an
//!    `entry_point` set wins; the name is mapped through that module's
//!    rename map in case its entry function was renamed.
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
//!
//! If even `"B::sqrt"` is taken (another module crafted an export alias
//! spelled that way), a `$0`, `$1`, … suffix is added until the name is
//! free — see `first_free_suffix`.
//!
//! Exports are never renamed, so this strategy only works if no two modules
//! export the same public name.  `link` guarantees that before calling
//! `merge_modules` (`LinkError::ConflictingExport`, issue #18429), together
//! with unique module names (`DuplicateModuleName`) and an entry point that
//! resolves inside its own module (`UnresolvedEntryPoint`).  See the
//! *Preconditions* on [`merge_modules`].
//!
//! # Why renaming must reach *every* function-name operand (issue #18421)
//!
//! Renaming a function is only half the job: every place in the program that
//! *refers* to the old name must be updated too, or the reference silently
//! re-binds to whichever function now owns the bare name.  Twig makes this
//! easy to hit, because every module is compiled by a fresh compiler whose
//! anonymous-function counter starts at zero:
//!
//! ```text
//!   module adder:  __lambda_0 = (lambda (x) (+ x n))     alloc_closure("__lambda_0", n)
//!   module subber: __lambda_0 = (lambda (x) (- x n))     alloc_closure("__lambda_0", n)
//!
//!   after merge:   __lambda_0           (adder's — claimed first)
//!                  subber::__lambda_0   (subber's — renamed)
//! ```
//!
//! The original linker only rewrote `call` callees, and only for imported
//! names.  Subber's `alloc_closure("__lambda_0", n)` was left alone, so its
//! closures ran *adder's* body: `(sub5 1000)` returned 1005 instead of 995.
//! When the two lambdas had different capture counts the VM instead raised
//! an arity mismatch, because adder's parameter list was walked with
//! subber's capture chain.  The same blind spot affected ordinary private
//! helpers: a module's own `call helper` was never redirected to
//! `"<module>::helper"`.

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
/// The caller (`linker.rs`) must have run every check in `resolve.rs` and
/// found no errors.  In particular:
///
/// | Precondition | Checked by | What goes wrong without it |
/// |--------------|------------|----------------------------|
/// | every import resolves | `resolve_imports` | the import's calls keep a dangling name |
/// | module names are unique | `check_module_names` | two modules share one set of import rewrites |
/// | each public name has one owner | `build_export_map` | two functions keep the same bare name; references bind to the first |
/// | the entry point resolves in its module | `check_entry_point` | the entry binds to another module's function |
/// | every function reference is defined or imported | `check_function_references` | the bare name binds to another module's (possibly private) function (#18432) |
///
/// The one-owner rule is re-asserted in debug builds below, because breaking
/// it does not crash — it silently runs the wrong function.
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
    //
    // Debug-only guard for the one-owner precondition: remember which
    // module claimed each public name and complain if another one does.
    #[cfg(debug_assertions)]
    let mut public_owner: HashMap<String, &str> = HashMap::new();
    for module in modules {
        for export in &module.exports {
            let public_name = export.public_name().to_string();
            #[cfg(debug_assertions)]
            {
                let owner = *public_owner.entry(public_name.clone()).or_insert(&module.name);
                debug_assert!(
                    owner == module.name,
                    "merge_modules precondition violated: {owner:?} and {:?} both export \
                     {public_name:?} (link() reports this as LinkError::ConflictingExport)",
                    module.name,
                );
            }
            global_names.insert(public_name.clone());
            // Associate this export's underlying function with the public name.
            if let Some(fn_) = module.get_function(&export.function_name) {
                let key = (fn_ as *const IIRFunction, fn_ as *const IIRFunction as usize);
                merged_name_map.insert(key, public_name);
            }
        }
    }

    // Pass 1b: Assign names to private functions.
    //
    // `next_suffix` remembers, per prefixed base name, where the `$N`
    // search should resume (see `first_free_suffix`).
    let mut next_suffix: HashMap<String, usize> = HashMap::new();
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
                // uniqueness we detect this case and append a `$N` counter
                // until we find a free slot (`first_free_suffix` — it always
                // finds one and never panics, #18432).  This prevents
                // the second `merged_name_map.insert` from silently overwriting
                // the first entry and thereby redirecting call-rewrites to the
                // wrong function.
                let base = format!("{}::{}", module.name, fn_.name);
                if !global_names.contains(&base) {
                    base
                } else {
                    // Double-collision: append a counter to guarantee uniqueness.
                    first_free_suffix(&base, &global_names, &mut next_suffix)
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

    // Step 3: Build each module's complete rename map and assemble the
    // merged functions list.
    let mut merged_functions: Vec<IIRFunction> = Vec::new();
    // The entry point is resolved through the rename map of the module it
    // came from (Step 4), so remember each module's map as we go.
    let mut module_rename_maps: Vec<HashMap<String, String>> = Vec::with_capacity(modules.len());
    for module in modules {
        let module_rewrites = build_module_rename_map(module, &import_rewrite, &merged_name_map);

        for fn_ in &module.functions {
            let ptr_key = (fn_ as *const IIRFunction, fn_ as *const IIRFunction as usize);
            let merged_fn_name = merged_name_map.get(&ptr_key).cloned().unwrap_or_else(|| fn_.name.clone());
            let rewritten = rewrite_function(fn_, &merged_fn_name, &module_rewrites);
            merged_functions.push(rewritten);
        }
        module_rename_maps.push(module_rewrites);
    }

    // Step 4: Preserve entry_point from first module that has one.
    //
    // The entry point is a function *name*, so it is a function reference
    // like any other: if the entry module's `main` was renamed (because an
    // earlier module already claimed `main`), the merged module must start
    // at the renamed function, not at the other module's `main`.
    //
    // The `unwrap_or_else` keeps a name the rename map does not mention.
    // That is only correct because `check_entry_point` has already proven
    // the name is one of the module's own functions (kept its bare name) or
    // one of its imports (kept its bare public name) — never a name that
    // could fall through to some other module's private function (#18429).
    let entry_point = modules.iter().zip(&module_rename_maps).find_map(|(m, renames)| {
        m.entry_point
            .as_ref()
            .map(|ep| renames.get(ep).cloned().unwrap_or_else(|| ep.clone()))
    });

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
        release: None,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// The first name `"{base}${n}"` (n = 0, 1, 2, …) that is not in `taken`.
///
/// Used when a private function's prefixed name `"<module>::<fn>"` is itself
/// already claimed — which only happens when some *other* name was spelled
/// to look like it, e.g. module `a` exports `f` as `b::c` while module `b`
/// has a private `c`:
///
/// ```text
///   claimed:  c, b::c, b::c$0, b::c$1      (export aliases of a's f)
///   b's c  →  c? taken  →  b::c? taken  →  b::c$0? $1? taken  →  b::c$2
/// ```
///
/// # Why this cannot panic or loop forever (issue #18432)
///
/// `taken` is a finite set.  Of the `taken.len() + 1` candidates
/// `$0 … ${taken.len()}`, at most `taken.len()` can be taken, so one is
/// free — the pigeonhole principle.  The search therefore always ends, and
/// needs no safety bound.
///
/// The previous version did have one: an `assert!` that gave up after
/// *total functions + 1* tries.  But export aliases claim names without
/// adding functions, so one exported function with a handful of crafted
/// aliases (`b::c$0` … `b::c$3`) exhausted the bound and `link()` panicked —
/// in release builds too, since `assert!` is not compiled out.  A linker
/// must report bad input, never crash on it.
///
/// # Why `next`
///
/// Without memory, `k` private functions that all collide on the same base
/// (one module holding several functions of the same name) would rescan
/// `$0 … $(k-1)` each time — quadratic work an input could choose.  `taken`
/// only grows, so a suffix found taken stays taken; `next[base]` records
/// where the previous search for `base` stopped and the next one resumes
/// there.  Every candidate is then tried at most once per base.
fn first_free_suffix(
    base: &str,
    taken: &std::collections::HashSet<String>,
    next: &mut HashMap<String, usize>,
) -> String {
    let counter = next.entry(base.to_string()).or_insert(0);
    loop {
        let candidate = format!("{base}${counter}");
        *counter += 1;
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
}

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

/// Build the complete `original name → merged name` map for one module.
///
/// Two kinds of name can appear in a function-name operand of `module`:
///
/// | Source | Example | Maps to |
/// |--------|---------|---------|
/// | a function the module defines | `__lambda_0` in module `subber` | its merged name, e.g. `subber::__lambda_0` |
/// | an import's `local_name()` | `sqrt` imported from `math` | the exported function's merged name |
///
/// **Precedence.**  Imports are inserted last, so if a module both defines
/// and imports the same bare name the import wins.  That is exactly what
/// the linker did before #18421 (it only knew about imports), so the fix
/// does not change behaviour for that already-ambiguous case.
///
/// Import rewrites are looked up by `module.name`.  That is sound only
/// because `link` rejects two input modules with the same name
/// (`DuplicateModuleName`, #18429); otherwise each would pick up the
/// other's imports, and an import could override the other module's own
/// function of the same name.
///
/// Identity entries (`helper → helper`) are kept out of the map: they are
/// harmless, but omitting them keeps `rewrite_instr`'s fast path ("nothing
/// to rename in this module") effective for the common single-module link.
fn build_module_rename_map(
    module: &IIRModule,
    import_rewrite: &HashMap<(String, String), String>,
    merged_name_map: &HashMap<(*const IIRFunction, usize), String>,
) -> HashMap<String, String> {
    let mut renames: HashMap<String, String> = HashMap::new();

    // (a) The module's own functions.  This is the half that was missing:
    //     without it, a renamed private function (lambda or helper) is
    //     still referenced by its *old* bare name, which now belongs to a
    //     different module's function.
    //
    //     A module may hold *two* functions with the same name — Twig, for
    //     instance, emits the user's `(define (main) …)` and its own
    //     synthesized top-level `main` side by side.  Inside the module a
    //     reference to that name means the *first* one (that is what
    //     `IIRModule::get_function` and the VM's lookup return), so only the
    //     first occurrence of each name decides where references go; later
    //     duplicates are still renamed apart, but nothing points at them.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for fn_ in &module.functions {
        if !seen.insert(fn_.name.as_str()) {
            continue; // a later duplicate — references bind to the first.
        }
        let ptr_key = (fn_ as *const IIRFunction, fn_ as *const IIRFunction as usize);
        if let Some(merged) = merged_name_map.get(&ptr_key) {
            if merged != &fn_.name {
                renames.insert(fn_.name.clone(), merged.clone());
            }
        }
    }

    // (b) Imports — inserted second so they take precedence (see above).
    for ((mod_name, local), merged) in import_rewrite {
        if mod_name == &module.name {
            if merged == local {
                // An import whose target kept the same bare name must still
                // override a same-named *own* entry from (a).
                renames.remove(local);
            } else {
                renames.insert(local.clone(), merged.clone());
            }
        }
    }

    renames
}

/// Produce a new `IIRFunction` with:
/// - Its name replaced by `new_name`.
/// - Every function-name operand (see [`function_name_slot`]) that names a
///   key in `renames` updated to the merged name.
fn rewrite_function(
    fn_: &IIRFunction,
    new_name: &str,
    renames: &HashMap<String, String>,
) -> IIRFunction {
    let new_instrs: Vec<IIRInstr> = fn_
        .instructions
        .iter()
        .map(|instr| rewrite_instr(instr, renames))
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

/// Which source operand of `op` holds a **function name**, if any.
///
/// This table is the single place that knows how IIR spells a reference to
/// a function.  Anything not listed here is *data* and must never be
/// renamed — in particular `const(Str("…"))` is a user string literal, and
/// `global_load` / `global_store` / `catch` carry global or exception-kind
/// names, which live in different namespaces from functions.
///
/// | Op | Slot | Shape (from `interpreter_ir::opcodes`) |
/// |----|------|----------------------------------------|
/// | `call` | `srcs[0]` | `call(Var(fn_name), args…)` |
/// | `alloc_closure` | `srcs[0]` | `alloc_closure(Str(fn_name), captures…)` (LANG34) |
/// | `task_spawn` | `srcs[0]` | `task_spawn(fn_name, args…)` (LANG28) |
/// | `group_spawn` | `srcs[1]` | `group_spawn(group, fn_name, args…)` (LANG28) |
///
/// **Contract (LANG28, pinned by issue #18429).**  The `fn` operand of
/// `task_spawn` and `group_spawn` is the *name* of a function in the module
/// — the same kind of operand as `call`'s callee — and never a register
/// holding a closure.  The rewrite below matches on the operand's text
/// alone, so it relies on this: a closure *register* that happened to be
/// spelled like a renamed function would be "renamed" too, and would then
/// name a register that does not exist.  A producer that wants to spawn a
/// closure must spawn a named trampoline function and pass the closure as
/// an ordinary argument.  No IIR producer emits either op today; the
/// contract is fixed now so the first one is written against it.
///
/// Ops that take a closure *handle* (`call_closure`) need no entry: the
/// handle was produced by an `alloc_closure` that is rewritten here.
///
/// Not covered: the pre-LANG34 `call_builtin "make_closure"` form, which
/// carries the body name in a *register* loaded by a separate `const`.  No
/// linker input producer emits it any more (twig-ir-compiler switched to
/// `alloc_closure`), and renaming it would need dataflow to tell the name
/// apart from an ordinary string constant.
///
/// The same table drives `resolve::check_function_references`, which
/// rejects a name in one of these slots that the module neither defines nor
/// imports (`LinkError::UndeclaredCall`, #18432) — so the check and the
/// rewrite can never disagree about what counts as a function reference.
pub(crate) fn function_name_slot(op: &str) -> Option<usize> {
    match op {
        "call" | "alloc_closure" | "task_spawn" => Some(0),
        "group_spawn" => Some(1),
        _ => None,
    }
}

/// The function name `instr` refers to, if it has a function-name slot
/// (see [`function_name_slot`]) holding a name.
///
/// Both spellings count — `Operand::Var` (`call`, the spawns) and
/// `Operand::Str` (`alloc_closure`).  A slot holding anything else (an
/// integer, a missing operand) is malformed IIR that the VM rejects on its
/// own; it names no function, so the linker neither renames nor checks it.
pub(crate) fn function_name_operand(instr: &IIRInstr) -> Option<&str> {
    match instr.srcs.get(function_name_slot(&instr.op)?)? {
        Operand::Var(name) | Operand::Str(name) => Some(name),
        _ => None,
    }
}

/// Rewrite the function-name operand of `instr` if it appears in `renames`.
///
/// Both operand spellings are accepted — `call` uses `Operand::Var` and
/// `alloc_closure` uses `Operand::Str` — and the original spelling is
/// preserved, so downstream passes see exactly the shape they expect.
/// Instructions without a function-name slot are returned unchanged.
fn rewrite_instr(instr: &IIRInstr, renames: &HashMap<String, String>) -> IIRInstr {
    if renames.is_empty() {
        return instr.clone();
    }
    let Some(slot) = function_name_slot(&instr.op) else {
        return instr.clone();
    };
    let renamed = match instr.srcs.get(slot) {
        Some(Operand::Var(name)) => renames.get(name).map(|n| Operand::Var(n.clone())),
        Some(Operand::Str(name)) => renames.get(name).map(|n| Operand::Str(n.clone())),
        _ => None,
    };
    match renamed {
        Some(new_operand) => {
            // Struct-update syntax keeps every profiling / deopt field of the
            // original instruction intact; only the one operand changes.
            let mut new_srcs = instr.srcs.clone();
            new_srcs[slot] = new_operand;
            IIRInstr { srcs: new_srcs, ..instr.clone() }
        }
        None => instr.clone(),
    }
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

    /// A function whose only work is `call(callee)` — used to check that
    /// callee names are rewritten.
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
    // ── Issue #18421: every function reference follows a rename ────────────

    /// A module shaped like Twig's output for `(define (mk n) (lambda (x) …))`:
    /// a lambda body `__lambda_0` (1 capture + 1 param) and a function `mk`
    /// that wraps it in a closure with `alloc_closure(Str("__lambda_0"), n)`.
    fn module_with_lambda(module: &str, maker: &str) -> IIRModule {
        let mut m = IIRModule::new(module, "twig");
        m.entry_point = None;
        m.add_or_replace(IIRFunction::new(
            "__lambda_0",
            vec![("n".into(), "any".into()), ("x".into(), "any".into())],
            "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "any")],
        ));
        m.add_or_replace(IIRFunction::new(
            maker,
            vec![("n".into(), "any".into())],
            "any",
            vec![
                IIRInstr::new(
                    "alloc_closure",
                    Some("c".into()),
                    vec![Operand::Str("__lambda_0".into()), Operand::Var("n".into())],
                    "closure",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "any"),
            ],
        ));
        m.exports.push(IIRExport::new(maker));
        m
    }

    fn link_two(modules: Vec<IIRModule>) -> IIRModule {
        let (map, errs) = crate::resolve::build_export_map(&modules);
        assert!(errs.is_empty(), "unexpected export errors: {errs:?}");
        merge_modules(&modules, &map)
    }

    #[test]
    fn alloc_closure_body_name_follows_rename() {
        // Both modules define `__lambda_0`.  `adder` claims the bare name;
        // `subber`'s copy becomes `subber::__lambda_0`, and subber's
        // `alloc_closure` must be rewritten to point at it.  On the old
        // linker it still said "__lambda_0" — adder's body.
        let merged = link_two(vec![
            module_with_lambda("adder", "make_adder"),
            module_with_lambda("subber", "make_subber"),
        ]);
        assert!(merged.get_function("subber::__lambda_0").is_some());

        let closure_target = |maker: &str| {
            let f = merged.get_function(maker).unwrap();
            let alloc = f.instructions.iter().find(|i| i.op == "alloc_closure").unwrap();
            alloc.srcs[0].clone()
        };
        assert_eq!(closure_target("make_adder"), Operand::Str("__lambda_0".into()));
        assert_eq!(
            closure_target("make_subber"),
            Operand::Str("subber::__lambda_0".into()),
            "subber's closure must name subber's own lambda body",
        );
        // The capture operand is untouched.
        let f = merged.get_function("make_subber").unwrap();
        assert_eq!(f.instructions[0].srcs[1], Operand::Var("n".into()));
    }

    #[test]
    fn private_call_follows_rename_within_its_module() {
        // `b` calls its own private `helper`; `a` already claimed `helper`.
        // b's call must go to `b::helper`, not to a's function.
        let mut a = IIRModule::new("a", "twig");
        a.entry_point = None;
        a.add_or_replace(make_fn("helper"));

        let mut b = IIRModule::new("b", "twig");
        b.entry_point = None;
        b.add_or_replace(make_fn("helper"));
        b.add_or_replace(make_fn_calling("run_b", "helper"));

        let merged = link_two(vec![a, b]);
        let run_b = merged.get_function("run_b").unwrap();
        assert_eq!(run_b.instructions[0].srcs[0], Operand::Var("b::helper".into()));
    }

    #[test]
    fn string_constants_are_not_mistaken_for_function_names() {
        // A user string literal that happens to spell a renamed function's
        // name is data, not a reference — it must survive unchanged.
        let mut a = IIRModule::new("a", "twig");
        a.entry_point = None;
        a.add_or_replace(make_fn("helper"));

        let mut b = IIRModule::new("b", "twig");
        b.entry_point = None;
        b.add_or_replace(make_fn("helper"));
        b.add_or_replace(IIRFunction::new(
            "greet",
            vec![],
            "any",
            vec![
                IIRInstr::new("const", Some("s".into()), vec![Operand::Str("helper".into())], "str"),
                IIRInstr::new(
                    "global_store",
                    None,
                    vec![Operand::Str("helper".into()), Operand::Var("s".into())],
                    "void",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "any"),
            ],
        ));

        let merged = link_two(vec![a, b]);
        let greet = merged.get_function("greet").unwrap();
        assert_eq!(greet.instructions[0].srcs[0], Operand::Str("helper".into()));
        assert_eq!(greet.instructions[1].srcs[0], Operand::Str("helper".into()));
    }

    #[test]
    fn import_still_wins_over_same_named_private_function() {
        // `app` both imports `helper` from `lib` and defines a private
        // `helper`.  Pre-#18421 behaviour was "the import wins"; the fix
        // must not change that.
        use interpreter_ir::module_exports::IIRImport;
        let mut lib = IIRModule::new("lib", "twig");
        lib.entry_point = None;
        lib.add_or_replace(make_fn("helper"));
        lib.exports.push(IIRExport::new("helper"));

        let mut app = IIRModule::new("app", "twig");
        app.entry_point = None;
        app.add_or_replace(make_fn("helper"));
        app.add_or_replace(make_fn_calling("run", "helper"));
        app.imports.push(IIRImport::new("lib", "helper", "any"));

        let merged = link_two(vec![lib, app]);
        assert!(merged.get_function("app::helper").is_some());
        let run = merged.get_function("run").unwrap();
        assert_eq!(run.instructions[0].srcs[0], Operand::Var("helper".into()));
    }

    #[test]
    fn entry_point_follows_rename() {
        // `lib` comes first and has a function called `main` but no entry
        // point; `app` is the program and its `main` gets renamed.  The
        // merged entry point must be app's renamed `main`.
        let mut lib = IIRModule::new("lib", "twig");
        lib.entry_point = None;
        lib.add_or_replace(make_fn("main"));

        let mut app = IIRModule::new("app", "twig");
        app.add_or_replace(make_fn("main"));
        app.entry_point = Some("main".into());

        let merged = link_two(vec![lib, app]);
        assert_eq!(merged.entry_point.as_deref(), Some("app::main"));
    }

    #[test]
    fn duplicate_names_in_one_module_bind_to_the_first() {
        // Twig emits the user's `(define (main) …)` *and* its synthesized
        // top-level `main` into the same module.  The second copy is renamed
        // apart by the merge, but references to `main` — including the
        // entry point — must keep meaning the first one.
        let mut app = IIRModule::new("app", "twig");
        app.entry_point = Some("main".into());
        let mut user_main = make_fn("main");
        user_main.return_type = "user".into(); // marker to tell them apart
        app.functions.push(user_main);
        app.functions.push(make_fn("main")); // synthesized duplicate
        app.functions.push(make_fn_calling("caller", "main"));

        let merged = link_two(vec![app]);
        assert_eq!(merged.entry_point.as_deref(), Some("main"));
        assert_eq!(merged.get_function("main").unwrap().return_type, "user");
        let caller = merged.get_function("caller").unwrap();
        assert_eq!(caller.instructions[0].srcs[0], Operand::Var("main".into()));
    }

    /// Calling `merge_modules` directly on input that `link` would reject
    /// with `ConflictingExport` trips the debug-build precondition guard
    /// instead of silently producing two functions with one name (#18429).
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "precondition violated")]
    fn merge_guards_the_one_owner_precondition_in_debug_builds() {
        let mk = |name: &str| {
            let mut m = IIRModule::new(name, "twig");
            m.entry_point = None;
            m.add_or_replace(make_fn("foo"));
            m.exports.push(IIRExport::new("foo"));
            m
        };
        let modules = vec![mk("left"), mk("right")];
        let (map, _) = crate::resolve::build_export_map(&modules);
        let _ = merge_modules(&modules, &map);
    }

    #[test]
    fn first_free_suffix_skips_taken_names_and_resumes() {
        let taken: std::collections::HashSet<String> =
            ["b::c$0", "b::c$1", "b::c$3"].iter().map(|s| s.to_string()).collect();
        let mut next = HashMap::new();
        assert_eq!(first_free_suffix("b::c", &taken, &mut next), "b::c$2");
        // The caller inserts what it got; the next search resumes after it
        // instead of rescanning from $0, and still skips taken names.
        assert_eq!(first_free_suffix("b::c", &taken, &mut next), "b::c$4");
        // Each base keeps its own position.
        assert_eq!(first_free_suffix("x::y", &taken, &mut next), "x::y$0");
    }

    #[test]
    fn many_same_named_functions_in_one_module_get_distinct_names() {
        // `a` claims `f` and `b::f`; `b` holds five functions called `f`.
        // Every copy needs the `$N` fallback, and every name must differ.
        let mut a = IIRModule::new("a", "twig");
        a.entry_point = None;
        a.add_or_replace(make_fn("f"));
        a.exports.push(IIRExport::new("f"));
        a.exports.push(IIRExport::new("f").with_alias("b::f"));
        let mut b = IIRModule::new("b", "twig");
        b.entry_point = None;
        for _ in 0..5 {
            b.functions.push(make_fn("f"));
        }
        let merged = link_two(vec![a, b]);
        let mut names: Vec<&str> = merged.functions.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 6, "got {names:?}");
    }

    #[test]
    fn function_name_operand_reads_both_spellings() {
        let call = IIRInstr::new("call", None, vec![Operand::Var("f".into())], "any");
        let alloc = IIRInstr::new("alloc_closure", None, vec![Operand::Str("g".into())], "any");
        let bad = IIRInstr::new("call", None, vec![Operand::Int(1)], "any");
        let data = IIRInstr::new("const", None, vec![Operand::Str("h".into())], "str");
        assert_eq!(function_name_operand(&call), Some("f"));
        assert_eq!(function_name_operand(&alloc), Some("g"));
        assert_eq!(function_name_operand(&bad), None);
        assert_eq!(function_name_operand(&data), None);
    }

    #[test]
    fn function_name_slot_table() {
        assert_eq!(function_name_slot("call"), Some(0));
        assert_eq!(function_name_slot("alloc_closure"), Some(0));
        assert_eq!(function_name_slot("task_spawn"), Some(0));
        assert_eq!(function_name_slot("group_spawn"), Some(1));
        for data_op in ["const", "global_load", "global_store", "catch", "call_closure", "call_builtin"] {
            assert_eq!(function_name_slot(data_op), None, "{data_op} has no function-name slot");
        }
    }

    #[test]
    fn rewrite_instr_preserves_operand_spelling_and_slot() {
        let mut renames = HashMap::new();
        renames.insert("f".to_string(), "m::f".to_string());

        let alloc = IIRInstr::new(
            "alloc_closure",
            Some("c".into()),
            vec![Operand::Str("f".into()), Operand::Var("f".into())],
            "closure",
        );
        let out = rewrite_instr(&alloc, &renames);
        assert_eq!(out.srcs[0], Operand::Str("m::f".into()));
        // A capture register that happens to be called `f` is not a name.
        assert_eq!(out.srcs[1], Operand::Var("f".into()));

        let spawn = IIRInstr::new(
            "group_spawn",
            Some("t".into()),
            vec![Operand::Var("f".into()), Operand::Var("f".into())],
            "any",
        );
        let out = rewrite_instr(&spawn, &renames);
        assert_eq!(out.srcs[0], Operand::Var("f".into()), "slot 0 is the group");
        assert_eq!(out.srcs[1], Operand::Var("m::f".into()));
    }
}
