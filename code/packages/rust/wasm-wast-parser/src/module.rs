//! # Module-form parsing — `(module ...)` text into `wasm_types::WasmModule`.
//!
//! This is the crate's core: turning a nested S-expression tree into
//! exactly the same [`WasmModule`] shape `wasm-module-parser` would have
//! produced from a binary `.wasm` file — including **encoding** every
//! function body straight to raw WASM bytecode (`FunctionBody.code:
//! Vec<u8>`), not a parallel text-specific instruction AST.
//!
//! ## Two passes, because WAT allows forward references
//!
//! A function can reference a global declared later in the same module; a
//! `call` can name a function that hasn't been parsed yet. So this module
//! makes two passes over the module's top-level forms:
//!
//! 1. **Collect** every symbolic name (`$name`) in every index space
//!    (types, funcs, tables, memories, globals) and assign it its real
//!    numeric index — imports first (regardless of where they're written
//!    textually relative to non-import definitions, per the WAT spec),
//!    then module-defined entities in textual order.
//! 2. **Build**: walk the forms again, this time actually encoding function
//!    bodies, globals' init expressions, and element/data segments, with
//!    every `$name` resolvable because pass 1 already assigned it an index.
//!
//! Function bodies additionally need their own two-level scope during pass
//! 2: **locals** (params + declared locals, function-scoped) and
//! **labels** (one per enclosing `block`/`loop`/`if`, pushed/popped as the
//! instruction encoder walks nested folded or flat instruction lists).

use crate::numeric::{parse_f32_bits, parse_f64_bits, parse_i32, parse_i64};
use crate::sexpr::{expect_get, parse_source, SExpr};
use crate::WastParseError;
use std::collections::HashMap;
use wasm_types::{
    DataSegment, Element, Export, ExternalKind, FuncType, FunctionBody, Global, GlobalType,
    Import, ImportTypeInfo, Limits, MemoryType, TableType, ValueType, WasmModule, FUNCREF,
};

/// Parse a single `(module ...)` text form (the plain-`.wat` entry point).
pub fn parse_module(src: &str) -> Result<WasmModule, WastParseError> {
    let exprs = parse_source(src)?;
    let module_expr = exprs
        .first()
        .ok_or(WastParseError::UnexpectedEof)?;
    parse_module_expr(module_expr)
}

/// As [`parse_module`], but starting from an already-parsed
/// `(module ...)` [`SExpr`] — the entry point `script.rs` uses, since a
/// `.wast` script's `module` directives are parsed alongside the other
/// script directives in one S-expression pass.
pub fn parse_module_expr(module_expr: &SExpr) -> Result<WasmModule, WastParseError> {
    let items = module_expr
        .as_list()
        .ok_or(WastParseError::UnexpectedToken {
            pos: module_expr.pos(),
            found: "atom".to_string(),
            expected: "a (module ...) list",
        })?;
    if items.first().and_then(|i| i.as_atom()) != Some("module") {
        return Err(WastParseError::UnexpectedToken {
            pos: module_expr.pos(),
            found: items.first().and_then(|i| i.as_atom()).unwrap_or("").to_string(),
            expected: "'module'",
        });
    }
    // Skip the leading `module` atom and an optional module-name identifier
    // (`(module $name ...)`), neither of which affects encoding.
    let fields: Vec<&SExpr> = items
        .iter()
        .skip(1)
        .skip_while(|e| matches!(e, SExpr::Atom(s, _) if s.starts_with('$')))
        .collect();

    let mut ctx = ModuleCtx::default();
    collect_symbols(&fields, &mut ctx)?;
    build(&fields, &mut ctx)?;
    Ok(ctx.module)
}

#[derive(Default)]
struct ModuleCtx {
    module: WasmModule,
    type_names: HashMap<String, u32>,
    func_names: HashMap<String, u32>,
    table_names: HashMap<String, u32>,
    memory_names: HashMap<String, u32>,
    global_names: HashMap<String, u32>,
}

fn resolve_idx(map: &HashMap<String, u32>, expr: &SExpr, space: &'static str) -> Result<u32, WastParseError> {
    match expr {
        SExpr::Atom(s, pos) if s.starts_with('$') => {
            map.get(s).copied().ok_or_else(|| WastParseError::UnknownIdentifier {
                pos: *pos,
                name: s.clone(),
                space,
            })
        }
        SExpr::Atom(s, pos) => s
            .parse::<u32>()
            .map_err(|_| WastParseError::UnexpectedToken { pos: *pos, found: s.clone(), expected: "an index" }),
        other => Err(WastParseError::UnexpectedToken {
            pos: other.pos(),
            found: "list".to_string(),
            expected: "an index or $identifier",
        }),
    }
}

fn parse_value_type(expr: &SExpr) -> Result<ValueType, WastParseError> {
    let s = expr.as_atom().ok_or(WastParseError::UnexpectedToken {
        pos: expr.pos(),
        found: "list".to_string(),
        expected: "a value type",
    })?;
    match s {
        "i32" => Ok(ValueType::I32),
        "i64" => Ok(ValueType::I64),
        "f32" => Ok(ValueType::F32),
        "f64" => Ok(ValueType::F64),
        other => Err(WastParseError::UnexpectedToken { pos: expr.pos(), found: other.to_string(), expected: "a value type" }),
    }
}

/// Parse a `(func (param i32 i32) (result i32))`-style signature list
/// (used both by `(type ...)` declarations and by inline signatures on
/// `func`/`import func`) into a [`FuncType`], skipping any leading
/// `$name`/`(export ...)`/`(type ...)` fields the caller has already
/// consumed and any params'/locals' own `$name`s (signature-only; names
/// are collected separately for the encoder's local scope).
fn parse_func_signature(fields: &[&SExpr]) -> Result<FuncType, WastParseError> {
    let mut params = Vec::new();
    let mut results = Vec::new();
    for f in fields {
        if f.is_keyword_list("param") {
            let items = f.as_list().unwrap();
            // `(param $name type)` (exactly one, named) or `(param type type ...)` (positional, any count).
            if items.len() == 3 && items[1].as_atom().is_some_and(|s| s.starts_with('$')) {
                params.push(parse_value_type(&items[2])?);
            } else {
                for t in &items[1..] {
                    params.push(parse_value_type(t)?);
                }
            }
        } else if f.is_keyword_list("result") {
            let items = f.as_list().unwrap();
            for t in &items[1..] {
                results.push(parse_value_type(t)?);
            }
        }
    }
    Ok(FuncType { params, results })
}

/// Find-or-insert `ty` into `module.types`, returning its index — the
/// "implicit type deduplication" WAT does for a `func`/`call_indirect`
/// signature that isn't an explicit `(type $t)` reference.
fn dedup_type(module: &mut WasmModule, ty: FuncType) -> u32 {
    if let Some(idx) = module.types.iter().position(|t| *t == ty) {
        idx as u32
    } else {
        module.types.push(ty);
        (module.types.len() - 1) as u32
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Pass 1 — collect every index space's names and sizes.
// ─────────────────────────────────────────────────────────────────────────

fn collect_symbols(fields: &[&SExpr], ctx: &mut ModuleCtx) -> Result<(), WastParseError> {
    // `type` declarations first -- other fields' inline signatures may not
    // reference them, but explicit `(type $t)` uses need the name resolved
    // before pass 2, and it's harmless to do this uniformly first.
    for f in fields {
        if f.is_keyword_list("type") {
            let items = f.as_list().unwrap();
            let mut rest = &items[1..];
            if let Some(name) = rest.first().and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    if ctx.type_names.contains_key(name) {
                        return Err(WastParseError::DuplicateIdentifier { pos: f.pos(), name: name.to_string(), space: "type" });
                    }
                    ctx.type_names.insert(name.to_string(), ctx.module.types.len() as u32);
                    rest = &rest[1..];
                }
            }
            let func_sig = rest.first().and_then(|e| e.as_list()).unwrap_or(&[]);
            let sig_fields: Vec<&SExpr> = func_sig.iter().skip(1).collect();
            let func_type = parse_func_signature(&sig_fields)?;
            ctx.module.types.push(func_type);
        }
    }

    // Imports: always occupy the lowest indices in their respective index
    // spaces, in textual import order, regardless of interleaving with
    // non-import definitions -- the WAT spec's own rule.
    for f in fields {
        if f.is_keyword_list("import") {
            let items = f.as_list().unwrap();
            let desc = items.get(3).and_then(|e| e.as_list()).ok_or(WastParseError::UnexpectedToken {
                pos: f.pos(),
                found: "".to_string(),
                expected: "an import description",
            })?;
            let kind = desc.first().and_then(|e| e.as_atom()).unwrap_or("");
            let name = desc.get(1).and_then(|e| e.as_atom());
            match kind {
                "func" => {
                    // This loop only pushes to `ctx.module.functions` for
                    // func-kind imports, so its current length already IS
                    // the running func-space index -- no separate counter
                    // needed.
                    let idx = ctx.module.functions.len() as u32;
                    if let Some(n) = name {
                        insert_unique(&mut ctx.func_names, n, idx, f.pos(), "func")?;
                    }
                    ctx.module.functions.push(0); // placeholder type index, fixed in pass 2
                }
                "table" => {
                    let idx = ctx.table_names.len() as u32 + ctx.module.tables.len() as u32;
                    if let Some(n) = name {
                        insert_unique(&mut ctx.table_names, n, idx, f.pos(), "table")?;
                    }
                    ctx.module.tables.push(TableType { element_type: FUNCREF, limits: Limits { min: 0, max: None } });
                }
                "memory" => {
                    let idx = ctx.memory_names.len() as u32 + ctx.module.memories.len() as u32;
                    if let Some(n) = name {
                        insert_unique(&mut ctx.memory_names, n, idx, f.pos(), "memory")?;
                    }
                    ctx.module.memories.push(MemoryType { limits: Limits { min: 0, max: None } });
                }
                "global" => {
                    let idx = ctx.global_names.len() as u32 + ctx.module.globals.len() as u32;
                    if let Some(n) = name {
                        insert_unique(&mut ctx.global_names, n, idx, f.pos(), "global")?;
                    }
                    ctx.module.globals.push(Global {
                        global_type: GlobalType { value_type: ValueType::I32, mutable: false },
                        init_expr: vec![0x0B],
                    });
                }
                _ => {}
            }
            let import = build_import_shell(items, desc, kind)?;
            ctx.module.imports.push(import);
        }
    }

    // Non-import definitions, in textual order.
    for f in fields {
        if f.is_keyword_list("func") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = ctx.module.functions.len() as u32;
                    insert_unique(&mut ctx.func_names, name, idx, f.pos(), "func")?;
                }
            }
            ctx.module.functions.push(0); // fixed in pass 2
            ctx.module.code.push(FunctionBody { locals: vec![], code: vec![0x0B] }); // fixed in pass 2
        } else if f.is_keyword_list("table") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = ctx.module.tables.len() as u32;
                    insert_unique(&mut ctx.table_names, name, idx, f.pos(), "table")?;
                }
            }
            ctx.module.tables.push(TableType { element_type: FUNCREF, limits: Limits { min: 0, max: None } });
        } else if f.is_keyword_list("memory") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = ctx.module.memories.len() as u32;
                    insert_unique(&mut ctx.memory_names, name, idx, f.pos(), "memory")?;
                }
            }
            ctx.module.memories.push(MemoryType { limits: Limits { min: 0, max: None } });
        } else if f.is_keyword_list("global") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = ctx.module.globals.len() as u32;
                    insert_unique(&mut ctx.global_names, name, idx, f.pos(), "global")?;
                }
            }
            ctx.module.globals.push(Global {
                global_type: GlobalType { value_type: ValueType::I32, mutable: false },
                init_expr: vec![0x0B],
            });
        }
    }

    Ok(())
}

fn insert_unique(
    map: &mut HashMap<String, u32>,
    name: &str,
    idx: u32,
    pos: usize,
    space: &'static str,
) -> Result<(), WastParseError> {
    if map.contains_key(name) {
        return Err(WastParseError::DuplicateIdentifier { pos, name: name.to_string(), space });
    }
    map.insert(name.to_string(), idx);
    Ok(())
}

fn build_import_shell(items: &[SExpr], desc: &[SExpr], kind: &str) -> Result<Import, WastParseError> {
    let module_name = match &items[1] {
        SExpr::Str(b, _) => String::from_utf8_lossy(b).to_string(),
        other => return Err(WastParseError::UnexpectedToken { pos: other.pos(), found: "".into(), expected: "a module name string" }),
    };
    let name = match &items[2] {
        SExpr::Str(b, _) => String::from_utf8_lossy(b).to_string(),
        other => return Err(WastParseError::UnexpectedToken { pos: other.pos(), found: "".into(), expected: "an import name string" }),
    };
    let type_info = match kind {
        "func" => ImportTypeInfo::Function(0), // fixed in pass 2 once the type is known
        "table" => ImportTypeInfo::Table(TableType { element_type: FUNCREF, limits: Limits { min: 0, max: None } }),
        "memory" => ImportTypeInfo::Memory(MemoryType { limits: parse_limits(&desc[1..])? }),
        "global" => {
            let gt = desc.get(1).ok_or(WastParseError::UnexpectedEof)?;
            ImportTypeInfo::Global(parse_global_type(gt)?)
        }
        _ => return Err(WastParseError::UnexpectedToken { pos: desc[0].pos(), found: kind.to_string(), expected: "func/table/memory/global" }),
    };
    Ok(Import {
        module_name,
        name,
        kind: match kind {
            "func" => ExternalKind::Function,
            "table" => ExternalKind::Table,
            "memory" => ExternalKind::Memory,
            _ => ExternalKind::Global,
        },
        type_info,
    })
}

fn parse_limits(fields: &[SExpr]) -> Result<Limits, WastParseError> {
    let digit_atoms: Vec<&SExpr> = fields
        .iter()
        .take_while(|e| e.as_atom().is_some_and(|s| s.chars().all(|c| c.is_ascii_digit())))
        .collect();
    // A digit-only string doesn't guarantee it fits u32 -- a syntactically
    // fine but numerically out-of-range literal (e.g. 2^32) must produce a
    // clean error here, not an `.unwrap()` panic on `parse`'s `Err`.
    let nums: Vec<u32> = digit_atoms
        .iter()
        .map(|e| {
            let (s, pos) = match e {
                SExpr::Atom(s, pos) => (s.as_str(), *pos),
                _ => unreachable!("take_while already filtered to atoms"),
            };
            s.parse::<u32>().map_err(|_| WastParseError::InvalidNumericLiteralForType {
                pos,
                text: s.to_string(),
                ty: "u32 limit",
            })
        })
        .collect::<Result<_, _>>()?;
    match nums.as_slice() {
        [min] => Ok(Limits { min: *min, max: None }),
        [min, max] => Ok(Limits { min: *min, max: Some(*max) }),
        _ => Err(WastParseError::UnexpectedToken { pos: 0, found: "".into(), expected: "1 or 2 limit numbers" }),
    }
}

fn parse_global_type(expr: &SExpr) -> Result<GlobalType, WastParseError> {
    if expr.is_keyword_list("mut") {
        let items = expr.as_list().unwrap();
        Ok(GlobalType { value_type: parse_value_type(&items[1])?, mutable: true })
    } else {
        Ok(GlobalType { value_type: parse_value_type(expr)?, mutable: false })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Pass 2 — build the real encoded structures.
// ─────────────────────────────────────────────────────────────────────────

fn build(fields: &[&SExpr], ctx: &mut ModuleCtx) -> Result<(), WastParseError> {
    // Imports always occupy the lowest indices in their index space,
    // regardless of textual interleaving with non-import definitions (see
    // `collect_symbols`'s doc comment) -- so a non-import definition's real
    // index is "total imports of that kind" (a fixed count, known once
    // pass 1 has finished) plus "how many non-import definitions of that
    // kind have been seen so far in this pass." Computed once, up front,
    // rather than re-derived per iteration.
    let num_import_funcs = ctx.module.imports.iter().filter(|i| i.kind == ExternalKind::Function).count();
    let num_import_tables = ctx.module.imports.iter().filter(|i| i.kind == ExternalKind::Table).count();
    let num_import_memories = ctx.module.imports.iter().filter(|i| i.kind == ExternalKind::Memory).count();
    let num_import_globals = ctx.module.imports.iter().filter(|i| i.kind == ExternalKind::Global).count();

    let mut import_all_i = 0usize; // index into ctx.module.imports (every kind, textual order)
    let mut import_func_i = 0usize; // func-space index of the next func-kind import
    let mut func_i = 0usize;
    let mut table_i = 0usize;
    let mut memory_i = 0usize;
    let mut global_i = 0usize;

    for f in fields {
        let items = f.as_list().unwrap_or(&[]);
        let head = items.first().and_then(|e| e.as_atom()).unwrap_or("");
        match head {
            "import" => {
                let desc = items[3].as_list().unwrap();
                if desc[0].as_atom().unwrap() == "func" {
                    let type_idx = resolve_func_signature_ref(&desc[1..], ctx)?;
                    ctx.module.imports[import_all_i].type_info = ImportTypeInfo::Function(type_idx);
                    ctx.module.functions[import_func_i] = type_idx;
                    import_func_i += 1;
                }
                // table/memory/global import shells are already correct
                // from pass 1 -- no fixup needed for those kinds.
                import_all_i += 1;
            }
            "func" => {
                let name_skip = if items.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
                build_func(&items[name_skip..], ctx, num_import_funcs + func_i)?;
                func_i += 1;
            }
            "export" => {
                let name = match expect_get(items, 1)? {
                    SExpr::Str(b, _) => String::from_utf8_lossy(b).to_string(),
                    other => return Err(WastParseError::UnexpectedToken { pos: other.pos(), found: "".into(), expected: "an export name string" }),
                };
                let refd_expr = expect_get(items, 2)?;
                let refd = refd_expr.as_list().ok_or(WastParseError::UnexpectedToken {
                    pos: refd_expr.pos(),
                    found: "".into(),
                    expected: "an export target (func/table/memory/global $x)",
                })?;
                let refd_head = expect_get(refd, 0)?;
                let (kind, map) = match refd_head.as_atom().unwrap_or("") {
                    "func" => (ExternalKind::Function, &ctx.func_names),
                    "table" => (ExternalKind::Table, &ctx.table_names),
                    "memory" => (ExternalKind::Memory, &ctx.memory_names),
                    "global" => (ExternalKind::Global, &ctx.global_names),
                    other => return Err(WastParseError::UnexpectedToken { pos: refd_head.pos(), found: other.to_string(), expected: "func/table/memory/global" }),
                };
                let index = resolve_idx(map, expect_get(refd, 1)?, "export target")?;
                ctx.module.exports.push(Export { name, kind, index });
            }
            "memory" => {
                let name_skip = if items.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
                let rest = &items[name_skip..];
                let idx = num_import_memories + memory_i;
                let (limits_start, _) = handle_inline_export(rest, "memory", idx, ctx)?;
                ctx.module.memories[idx].limits = parse_limits(&rest[limits_start..])?;
                memory_i += 1;
            }
            "table" => {
                let name_skip = if items.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
                let rest = &items[name_skip..];
                let idx = num_import_tables + table_i;
                let (limits_start, _) = handle_inline_export(rest, "table", idx, ctx)?;
                ctx.module.tables[idx].limits = parse_limits(&rest[limits_start..])?;
                table_i += 1;
            }
            "global" => {
                let name_skip = if items.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
                let rest = &items[name_skip..];
                let idx = num_import_globals + global_i;
                let (type_start, _) = handle_inline_export(rest, "global", idx, ctx)?;
                let gt = parse_global_type(expect_get(rest, type_start)?)?;
                let init_instrs = rest.get(type_start + 1..).unwrap_or(&[]);
                let mut code = Vec::new();
                encode_instr_list(init_instrs, &mut InstrCtx::empty(ctx), &mut code)?;
                code.push(0x0B);
                ctx.module.globals[idx] = Global { global_type: gt, init_expr: code };
                global_i += 1;
            }
            "elem" => build_elem(&items[1..], ctx)?,
            "data" => build_data(&items[1..], ctx)?,
            "start" => {
                let idx = resolve_idx(&ctx.func_names, &items[1], "func")?;
                ctx.module.start = Some(idx);
            }
            _ => {}
        }
    }
    Ok(())
}

/// `(memory $name? (export "e")* limits)` / `(table ...)` / `(global
/// $name? (export "e")* type)` all share this "zero or more inline export
/// shorthands, then the real payload" shape. Registers each inline export
/// against `idx` and returns the position in `rest` where the real payload
/// (limits, or the global's type) starts.
fn handle_inline_export(
    rest: &[SExpr],
    kind_name: &str,
    idx: usize,
    ctx: &mut ModuleCtx,
) -> Result<(usize, ()), WastParseError> {
    let kind = match kind_name {
        "func" => ExternalKind::Function,
        "table" => ExternalKind::Table,
        "memory" => ExternalKind::Memory,
        _ => ExternalKind::Global,
    };
    let mut i = 0;
    while i < rest.len() && rest[i].is_keyword_list("export") {
        let items = rest[i].as_list().unwrap();
        if let SExpr::Str(b, _) = &items[1] {
            ctx.module.exports.push(Export { name: String::from_utf8_lossy(b).to_string(), kind, index: idx as u32 });
        }
        i += 1;
    }
    Ok((i, ()))
}

/// Resolve a `func` import description's signature (`(type $t)`, inline
/// `(param...) (result...)`, or a mix) to a type-section index, deduping
/// an inline-only signature the same way a non-import `func` would.
fn resolve_func_signature_ref(desc_rest: &[SExpr], ctx: &mut ModuleCtx) -> Result<u32, WastParseError> {
    if let Some(type_ref) = desc_rest.iter().find(|e| e.is_keyword_list("type")) {
        let items = type_ref.as_list().unwrap();
        return resolve_idx(&ctx.type_names, &items[1], "type");
    }
    let sig_fields: Vec<&SExpr> = desc_rest.iter().collect();
    let ty = parse_func_signature(&sig_fields)?;
    Ok(dedup_type(&mut ctx.module, ty))
}

fn build_func(fields: &[SExpr], ctx: &mut ModuleCtx, func_idx: usize) -> Result<(), WastParseError> {
    // Inline export shorthand: `(func $f (export "e") ...)`.
    let (after_export, _) = handle_inline_export(fields, "func", func_idx, ctx)?;
    let fields = &fields[after_export..];

    let type_idx = resolve_func_signature_ref(fields, ctx)?;
    ctx.module.functions[func_idx] = type_idx;
    let func_type = ctx.module.types[type_idx as usize].clone();

    // Local scope: params first (from the signature just resolved,
    // re-walking the param forms here only for their OPTIONAL names --
    // `parse_func_signature` already captured the types), then declared
    // `(local ...)` forms.
    let mut local_names: HashMap<String, u32> = HashMap::new();
    let mut next_local = 0u32;
    let mut locals_decl: Vec<ValueType> = Vec::new();
    let mut instr_start = 0usize;
    for (i, f) in fields.iter().enumerate() {
        if f.is_keyword_list("param") {
            let items = f.as_list().unwrap();
            if items.len() == 3 && items[1].as_atom().is_some_and(|s| s.starts_with('$')) {
                local_names.insert(items[1].as_atom().unwrap().to_string(), next_local);
                next_local += 1;
            } else {
                next_local += (items.len() - 1) as u32;
            }
            instr_start = i + 1;
        } else if f.is_keyword_list("result") || f.is_keyword_list("type") {
            instr_start = i + 1;
        } else if f.is_keyword_list("local") {
            let items = f.as_list().unwrap();
            if items.len() == 3 && items[1].as_atom().is_some_and(|s| s.starts_with('$')) {
                local_names.insert(items[1].as_atom().unwrap().to_string(), next_local);
                locals_decl.push(parse_value_type(&items[2])?);
                next_local += 1;
            } else {
                for t in &items[1..] {
                    locals_decl.push(parse_value_type(t)?);
                    next_local += 1;
                }
            }
            instr_start = i + 1;
        } else {
            break;
        }
    }
    let _ = func_type;

    let mut icx = InstrCtx { module: ctx, locals: local_names, labels: Vec::new() };
    let mut code = Vec::new();
    encode_instr_list(&fields[instr_start..], &mut icx, &mut code)?;
    code.push(0x0B);
    ctx.module.code[func_idx] = FunctionBody { locals: locals_decl, code };
    Ok(())
}

fn build_elem(fields: &[SExpr], ctx: &mut ModuleCtx) -> Result<(), WastParseError> {
    let mut i = 0;
    let table_index = if fields.first().is_some_and(|e| e.is_keyword_list("table")) {
        let table_form = fields[0].as_list().unwrap();
        let idx = resolve_idx(&ctx.table_names, expect_get(table_form, 1)?, "table")?;
        i += 1;
        idx
    } else {
        0
    };
    let offset_expr_form = expect_get(fields, i)?;
    let offset_expr = if offset_expr_form.is_keyword_list("offset") {
        let items = offset_expr_form.as_list().unwrap();
        let mut code = Vec::new();
        encode_instr_list(&items[1..], &mut InstrCtx::empty(ctx), &mut code)?;
        code.push(0x0B);
        i += 1;
        code
    } else {
        // Shorthand: a single folded instruction with no `(offset ...)` wrapper.
        let mut code = Vec::new();
        encode_instr_list(std::slice::from_ref(offset_expr_form), &mut InstrCtx::empty(ctx), &mut code)?;
        code.push(0x0B);
        i += 1;
        code
    };
    let mut function_indices = Vec::new();
    for f in fields.get(i..).unwrap_or(&[]) {
        function_indices.push(resolve_idx(&ctx.func_names, f, "func")?);
    }
    ctx.module.elements.push(Element { table_index, offset_expr, function_indices });
    Ok(())
}

fn build_data(fields: &[SExpr], ctx: &mut ModuleCtx) -> Result<(), WastParseError> {
    let mut i = 0;
    let memory_index = if fields.first().is_some_and(|e| e.is_keyword_list("memory")) {
        let memory_form = fields[0].as_list().unwrap();
        let idx = resolve_idx(&ctx.memory_names, expect_get(memory_form, 1)?, "memory")?;
        i += 1;
        idx
    } else {
        0
    };
    let offset_expr_form = expect_get(fields, i)?;
    let offset_expr = if offset_expr_form.is_keyword_list("offset") {
        let items = offset_expr_form.as_list().unwrap();
        let mut code = Vec::new();
        encode_instr_list(&items[1..], &mut InstrCtx::empty(ctx), &mut code)?;
        code.push(0x0B);
        i += 1;
        code
    } else {
        let mut code = Vec::new();
        encode_instr_list(std::slice::from_ref(offset_expr_form), &mut InstrCtx::empty(ctx), &mut code)?;
        code.push(0x0B);
        i += 1;
        code
    };
    let mut data = Vec::new();
    for f in fields.get(i..).unwrap_or(&[]) {
        if let SExpr::Str(b, _) = f {
            data.extend_from_slice(b);
        }
    }
    ctx.module.data.push(DataSegment { memory_index, offset_expr, data });
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Instruction encoding — folded or flat, identical code path.
// ─────────────────────────────────────────────────────────────────────────

struct InstrCtx<'a> {
    module: &'a mut ModuleCtx,
    locals: HashMap<String, u32>,
    /// Active label scope, innermost last. A named `block $l`/`loop
    /// $l`/`if $l` pushes `Some("$l")`; unnamed pushes `None`. `br`/`br_if`
    /// resolve either a plain depth number or a `$name` by scanning from
    /// the innermost label outward.
    labels: Vec<Option<String>>,
}

impl<'a> InstrCtx<'a> {
    fn empty(module: &'a mut ModuleCtx) -> Self {
        InstrCtx { module, locals: HashMap::new(), labels: Vec::new() }
    }

    fn resolve_label(&self, expr: &SExpr) -> Result<u32, WastParseError> {
        match expr {
            SExpr::Atom(s, pos) if s.starts_with('$') => self
                .labels
                .iter()
                .rev()
                .position(|l| l.as_deref() == Some(s.as_str()))
                .map(|depth| depth as u32)
                .ok_or_else(|| WastParseError::UnknownIdentifier { pos: *pos, name: s.clone(), space: "label" }),
            SExpr::Atom(s, pos) => {
                s.parse::<u32>().map_err(|_| WastParseError::UnexpectedToken { pos: *pos, found: s.clone(), expected: "a label" })
            }
            other => Err(WastParseError::UnexpectedToken { pos: other.pos(), found: "list".into(), expected: "a label" }),
        }
    }

    fn resolve_local(&self, expr: &SExpr) -> Result<u32, WastParseError> {
        resolve_idx(&self.locals, expr, "local")
    }
}

/// A "list" here is the flat sequence of instruction forms that appears
/// directly inside a `func`, `block`, `loop`, `if` arm, or a folded
/// instruction's argument list.
///
/// WAT has two syntactically different, semantically identical ways to
/// write the same instruction, and this function's whole job is telling
/// them apart correctly:
///
/// - **Folded**: `(i32.add (i32.const 1) (local.get 0))` — a single
///   [`SExpr::List`]. Its own `rest` items are a mix of *operand*
///   sub-expressions (recursed into first, since operands must be pushed
///   before the operator that consumes them) and this instruction's own
///   trailing *immediate* atoms (`(local.get $a)`, `(i32.load offset=8
///   ...)`) — see [`encode_flat_instr`].
/// - **Flat**: `local.get $a  local.get $b  i32.add` — three separate
///   top-level [`SExpr::Atom`]s in the SAME list. Critically, a flat
///   instruction's stack *operands* are never part of its own syntax at
///   all — whatever pushed them already ran as an earlier list element.
///   Only its own *immediates*, if any, trail it as further atoms in this
///   SAME flat sequence (`local.get` then `$a`; `i32.const` then `1`) —
///   see [`encode_stream_instr`], which is why this function walks with an
///   explicit cursor instead of a plain `for` loop: a flat instruction
///   with an immediate needs to consume more than one list element.
fn encode_instr_list(exprs: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let mut i = 0;
    while i < exprs.len() {
        i = encode_one(exprs, i, icx, out)?;
    }
    Ok(())
}

/// Encode `exprs[i]` (and, for a flat bare-atom instruction, however many
/// following elements it needs as immediates). Returns the index just past
/// what was consumed.
fn encode_one(exprs: &[SExpr], i: usize, icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<usize, WastParseError> {
    match &exprs[i] {
        SExpr::List(items, pos) => {
            let name = items.first().and_then(|it| it.as_atom()).ok_or(WastParseError::UnexpectedToken {
                pos: *pos,
                found: "".into(),
                expected: "an instruction name",
            })?;
            let rest = &items[1..];
            if matches!(name, "block" | "loop" | "if") {
                encode_structured_instr(name, rest, *pos, icx, out)?;
            } else {
                encode_flat_instr(name, rest, *pos, icx, out)?;
            }
            Ok(i + 1)
        }
        SExpr::Atom(name, pos) if matches!(name.as_str(), "block" | "loop" | "if") => {
            encode_stream_structured_instr(name, &exprs[i + 1..], *pos, icx, out)
                .map(|consumed| i + 1 + consumed)
        }
        SExpr::Atom(name, pos) => {
            let consumed = encode_stream_instr(name, &exprs[i + 1..], *pos, icx, out)?;
            Ok(i + 1 + consumed)
        }
        SExpr::Str(_, pos) => Err(WastParseError::UnexpectedToken { pos: *pos, found: "string".into(), expected: "an instruction" }),
    }
}

/// Encode a flat (bare-atom) instruction's opcode plus however many of
/// `following` it needs as immediates — never operands, which flat form
/// never carries at this position (see [`encode_instr_list`]'s doc
/// comment). Returns how many elements of `following` were consumed.
fn encode_stream_instr(
    name: &str,
    following: &[SExpr],
    pos: usize,
    icx: &mut InstrCtx,
    out: &mut Vec<u8>,
) -> Result<usize, WastParseError> {
    let info = wasm_opcodes::get_opcode_by_name(name)
        .ok_or_else(|| WastParseError::UnknownInstruction { pos, name: name.to_string() })?;
    match name {
        "local.get" | "local.set" | "local.tee" => {
            let idx = icx.resolve_local(following.first().ok_or(WastParseError::UnexpectedEof)?)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(1)
        }
        "global.get" | "global.set" => {
            let idx = resolve_idx(&icx.module.global_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "global")?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(1)
        }
        "call" => {
            let idx = resolve_idx(&icx.module.func_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "func")?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(1)
        }
        "call_indirect" => {
            // Flat form: `call_indirect (type $t)` -- the type reference
            // (and any `(param...)`/`(result...)`) trails as List elements
            // in the SAME flat sequence, not nested inside this atom.
            let sig_end = following.iter().position(|a| !is_type_or_param_or_result(a)).unwrap_or(following.len());
            let sig_fields = &following[..sig_end];
            let type_form = sig_fields.iter().find(|a| a.is_keyword_list("type"));
            let type_idx = if let Some(t) = type_form {
                resolve_idx(&icx.module.type_names, &t.as_list().unwrap()[1], "type")?
            } else {
                let refs: Vec<&SExpr> = sig_fields.iter().collect();
                let ty = parse_func_signature(&refs)?;
                dedup_type(&mut icx.module.module, ty)
            };
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(type_idx as u64));
            out.push(0x00);
            Ok(sig_end)
        }
        "br" | "br_if" => {
            let depth = icx.resolve_label(following.first().ok_or(WastParseError::UnexpectedEof)?)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(depth as u64));
            Ok(1)
        }
        "br_table" => {
            // Every following bare-atom label reference belongs to this
            // instruction (br_table takes >=1, with no other syntax to
            // mark the end) -- consume atoms until a non-label element
            // (a List, i.e. the next real instruction) or end of stream.
            let label_end = following.iter().position(|a| !matches!(a, SExpr::Atom(_, _))).unwrap_or(following.len());
            if label_end == 0 {
                return Err(WastParseError::UnexpectedEof);
            }
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned((label_end - 1) as u64));
            for l in &following[..label_end] {
                let depth = icx.resolve_label(l)?;
                out.extend(wasm_leb128::encode_unsigned(depth as u64));
            }
            Ok(label_end)
        }
        "i32.load" | "i64.load" | "f32.load" | "f64.load" | "i32.load8_s" | "i32.load8_u" | "i32.load16_s"
        | "i32.load16_u" | "i64.load8_s" | "i64.load8_u" | "i64.load16_s" | "i64.load16_u" | "i64.load32_s"
        | "i64.load32_u" | "i32.store" | "i64.store" | "f32.store" | "f64.store" | "i32.store8" | "i32.store16"
        | "i64.store8" | "i64.store16" | "i64.store32" => {
            let (memarg, consumed) = parse_memarg(following);
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(memarg.0 as u64));
            out.extend(wasm_leb128::encode_unsigned(memarg.1 as u64));
            Ok(consumed)
        }
        "memory.size" | "memory.grow" => {
            out.push(info.opcode);
            out.push(0x00);
            Ok(0)
        }
        "i32.const" => {
            let text = literal_text(following.first(), pos)?;
            let v = parse_i32(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_signed(v as i64));
            Ok(1)
        }
        "i64.const" => {
            let text = literal_text(following.first(), pos)?;
            let v = parse_i64(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_signed(v));
            Ok(1)
        }
        "f32.const" => {
            let text = literal_text(following.first(), pos)?;
            let bits = parse_f32_bits(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(bits.to_le_bytes());
            Ok(1)
        }
        "f64.const" => {
            let text = literal_text(following.first(), pos)?;
            let bits = parse_f64_bits(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(bits.to_le_bytes());
            Ok(1)
        }
        // Every other opcode (control returns/unreachable/nop, parametric
        // drop/select, and the ~150 no-immediate numeric/comparison/
        // conversion instructions) takes no immediates at all in EITHER
        // syntax -- just the bare opcode byte.
        _ => {
            out.push(info.opcode);
            Ok(0)
        }
    }
}

/// As [`encode_structured_instr`], for a flat (non-folded) `block`/`loop`/
/// `if ... end` whose body is a run of bare stream elements terminated by
/// a matching `end` atom **at this same nesting level** — a nested
/// `block`/`loop`/`if` inside the body recurses through this same
/// function via [`encode_one`], consuming its own matching `end` first, so
/// the outer scan here only ever needs to track ITS OWN terminator, never
/// nested ones.
fn encode_stream_structured_instr(
    name: &str,
    following: &[SExpr],
    pos: usize,
    icx: &mut InstrCtx,
    out: &mut Vec<u8>,
) -> Result<usize, WastParseError> {
    let opcode = wasm_opcodes::get_opcode_by_name(name).unwrap().opcode;
    let mut i = 0;
    let mut label_name = None;
    if let Some(SExpr::Atom(s, _)) = following.get(i) {
        if s.starts_with('$') {
            label_name = Some(s.clone());
            i += 1;
        }
    }
    let blocktype_byte: Vec<u8> = if let Some(r) = following.get(i).filter(|a| a.is_keyword_list("result")) {
        let items = r.as_list().unwrap();
        i += 1;
        if items.len() == 2 { vec![parse_value_type(&items[1])?.byte_tag().unwrap()] } else { vec![0x40] }
    } else {
        vec![0x40]
    };

    out.push(opcode);
    out.extend(&blocktype_byte);
    icx.labels.push(label_name);

    if name == "if" {
        // The condition was already pushed by whatever preceded `if` in
        // the flat stream (per this whole function's own contract), so the
        // body starts immediately at `i`. Walk element-by-element (not via
        // `encode_instr_list`, which has no early-stop condition) so a
        // bare `else`/`end` atom AT THIS LEVEL stops the scan, while a
        // nested nested `if`'s own `else`/`end` is consumed by that
        // recursive `encode_one` call and never seen here.
        loop {
            match following.get(i) {
                None => return Err(WastParseError::UnexpectedEof),
                Some(SExpr::Atom(s, _)) if s == "else" => {
                    i += 1;
                    out.push(0x05);
                    loop {
                        match following.get(i) {
                            None => return Err(WastParseError::UnexpectedEof),
                            Some(SExpr::Atom(s, _)) if s == "end" => {
                                i += 1;
                                break;
                            }
                            _ => i = encode_one(following, i, icx, out)?,
                        }
                    }
                    break;
                }
                Some(SExpr::Atom(s, _)) if s == "end" => {
                    i += 1;
                    break;
                }
                _ => i = encode_one(following, i, icx, out)?,
            }
        }
    } else {
        loop {
            match following.get(i) {
                None => return Err(WastParseError::UnexpectedEof),
                Some(SExpr::Atom(s, _)) if s == "end" => {
                    i += 1;
                    break;
                }
                _ => i = encode_one(following, i, icx, out)?,
            }
        }
    }

    icx.labels.pop();
    out.push(0x0B);
    let _ = pos;
    Ok(i)
}

/// Encode a **folded** instruction (`(name args...)`) — `args` mixes zero
/// or more operand sub-expressions (encoded first, recursively) with this
/// instruction's own trailing immediate atoms, per instruction kind. See
/// [`encode_instr_list`]'s doc comment for how this differs from the flat,
/// bare-atom form ([`encode_stream_instr`]).
fn encode_flat_instr(
    name: &str,
    args: &[SExpr],
    pos: usize,
    icx: &mut InstrCtx,
    out: &mut Vec<u8>,
) -> Result<(), WastParseError> {
    let info = wasm_opcodes::get_opcode_by_name(name)
        .ok_or_else(|| WastParseError::UnknownInstruction { pos, name: name.to_string() })?;

    // Special-cased immediate shapes. NOTE: in folded syntax, an
    // instruction's own immediate (index/label) is always its FIRST arg —
    // `(local.set $x (i32.const 5))`, `(call $f (i32.const 1))`, `(br
    // $label (i32.const 1))` — with any stack-operand sub-expressions
    // trailing AFTER it, not before. This is the opposite order from
    // memory ops' `align=`/`offset=` attributes, which also lead, but
    // whose "immediate" isn't a symbolic reference needing this same
    // index/label resolution -- each case below reflects its own actual
    // arg order rather than sharing one generic split.
    match name {
        "local.get" | "local.set" | "local.tee" => {
            let idx_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
            encode_instr_list(&args[1..], icx, out)?;
            out.push(info.opcode);
            let idx = icx.resolve_local(idx_expr)?;
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(())
        }
        "global.get" | "global.set" => {
            let idx_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
            encode_instr_list(&args[1..], icx, out)?;
            out.push(info.opcode);
            let idx = resolve_idx(&icx.module.global_names, idx_expr, "global")?;
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(())
        }
        "call" => {
            let idx_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
            encode_instr_list(&args[1..], icx, out)?;
            out.push(info.opcode);
            let idx = resolve_idx(&icx.module.func_names, idx_expr, "func")?;
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(())
        }
        "call_indirect" => {
            // `(call_indirect (type $t) (param...) (result...) operand-exprs...)`
            let type_form = args.iter().find(|a| a.is_keyword_list("type"));
            let operand_start = args.iter().position(|a| !is_type_or_param_or_result(a)).unwrap_or(args.len());
            encode_instr_list(&args[operand_start..], icx, out)?;
            out.push(info.opcode);
            let type_idx = if let Some(t) = type_form {
                resolve_idx(&icx.module.type_names, &t.as_list().unwrap()[1], "type")?
            } else {
                let sig_fields: Vec<&SExpr> = args[..operand_start].iter().collect();
                let ty = parse_func_signature(&sig_fields)?;
                dedup_type(&mut icx.module.module, ty)
            };
            out.extend(wasm_leb128::encode_unsigned(type_idx as u64));
            out.push(0x00); // table index, always 0 in WASM 1.0
            Ok(())
        }
        "br" | "br_if" => {
            let label_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
            encode_instr_list(&args[1..], icx, out)?;
            out.push(info.opcode);
            let depth = icx.resolve_label(label_expr)?;
            out.extend(wasm_leb128::encode_unsigned(depth as u64));
            Ok(())
        }
        "br_table" => {
            // All trailing atoms are labels (>=1); everything before that,
            // if any, is the folded index operand.
            let label_start = args.iter().rposition(|a| !is_label_atom(a)).map(|i| i + 1).unwrap_or(0);
            let labels = &args[label_start..];
            // br_table takes >=1 label per spec -- `(br_table)` or
            // `(br_table (i32.const 0))` (zero trailing label atoms) must
            // error cleanly, not underflow `labels.len() - 1` below.
            if labels.is_empty() {
                return Err(WastParseError::UnexpectedEof);
            }
            encode_instr_list(&args[..label_start], icx, out)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned((labels.len() - 1) as u64));
            for l in labels {
                let depth = icx.resolve_label(l)?;
                out.extend(wasm_leb128::encode_unsigned(depth as u64));
            }
            Ok(())
        }
        "i32.load" | "i64.load" | "f32.load" | "f64.load" | "i32.load8_s" | "i32.load8_u" | "i32.load16_s"
        | "i32.load16_u" | "i64.load8_s" | "i64.load8_u" | "i64.load16_s" | "i64.load16_u" | "i64.load32_s"
        | "i64.load32_u" | "i32.store" | "i64.store" | "f32.store" | "f64.store" | "i32.store8" | "i32.store16"
        | "i64.store8" | "i64.store16" | "i64.store32" => {
            let (memarg, operand_start) = parse_memarg(args);
            encode_instr_list(&args[operand_start..], icx, out)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(memarg.0 as u64));
            out.extend(wasm_leb128::encode_unsigned(memarg.1 as u64));
            Ok(())
        }
        "memory.size" | "memory.grow" => {
            let (operands, _) = split_operands_and_immediates(args, 0);
            encode_instr_list(operands, icx, out)?;
            out.push(info.opcode);
            out.push(0x00); // memory index, always 0
            Ok(())
        }
        "i32.const" => {
            let (_, imm) = split_operands_and_immediates(args, 1.min(args.len()));
            let text = literal_text(if args.is_empty() { None } else { Some(&imm[0]) }, pos)?;
            let v = parse_i32(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_signed(v as i64));
            Ok(())
        }
        "i64.const" => {
            let text = literal_text(args.first(), pos)?;
            let v = parse_i64(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_signed(v));
            Ok(())
        }
        "f32.const" => {
            let text = literal_text(args.first(), pos)?;
            let bits = parse_f32_bits(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(bits.to_le_bytes());
            Ok(())
        }
        "f64.const" => {
            let text = literal_text(args.first(), pos)?;
            let bits = parse_f64_bits(&text.0, text.1)?;
            out.push(info.opcode);
            out.extend(bits.to_le_bytes());
            Ok(())
        }
        "return" | "unreachable" | "nop" | "drop" | "select" => {
            encode_instr_list(args, icx, out)?;
            out.push(info.opcode);
            Ok(())
        }
        _ => {
            // Every other opcode (the ~150 no-immediate numeric/comparison/
            // conversion instructions) is: recurse into folded operands,
            // then emit the bare opcode byte.
            encode_instr_list(args, icx, out)?;
            out.push(info.opcode);
            Ok(())
        }
    }
}

fn is_type_or_param_or_result(e: &SExpr) -> bool {
    e.is_keyword_list("type") || e.is_keyword_list("param") || e.is_keyword_list("result")
}

fn is_label_atom(e: &SExpr) -> bool {
    matches!(e, SExpr::Atom(_, _))
}

fn split_operands_and_immediates(args: &[SExpr], n_immediates: usize) -> (&[SExpr], &[SExpr]) {
    if args.len() < n_immediates {
        return (args, &[]);
    }
    let split_at = args.len() - n_immediates;
    (&args[..split_at], &args[split_at..])
}

fn literal_text(expr: Option<&SExpr>, pos: usize) -> Result<(String, usize), WastParseError> {
    match expr {
        Some(SExpr::Atom(s, p)) => Ok((s.clone(), *p)),
        _ => Err(WastParseError::UnexpectedToken { pos, found: "".into(), expected: "a numeric literal" }),
    }
}

/// `align=N` / `offset=N` attributes on a load/store, in either order,
/// both optional (default align is the natural alignment, encoded here as
/// 0 -- real alignment *hints* don't affect semantics, only performance,
/// so defaulting to the loosest hint is always safe). Returns
/// `((align_log2, offset), first_non_attribute_index)`.
fn parse_memarg(args: &[SExpr]) -> ((u32, u32), usize) {
    let mut align_log2 = 0u32;
    let mut offset = 0u32;
    let mut i = 0;
    while i < args.len() {
        if let SExpr::Atom(s, _) = &args[i] {
            if let Some(v) = s.strip_prefix("offset=") {
                if let Ok(n) = v.parse::<u32>() {
                    offset = n;
                    i += 1;
                    continue;
                }
            }
            if let Some(v) = s.strip_prefix("align=") {
                if let Ok(n) = v.parse::<u32>() {
                    align_log2 = n.trailing_zeros();
                    i += 1;
                    continue;
                }
            }
        }
        break;
    }
    ((align_log2, offset), i)
}

fn encode_structured_instr(
    name: &str,
    args: &[SExpr],
    pos: usize,
    icx: &mut InstrCtx,
    out: &mut Vec<u8>,
) -> Result<(), WastParseError> {
    let opcode = wasm_opcodes::get_opcode_by_name(name).unwrap().opcode;
    let mut i = 0;
    let mut label_name = None;
    if let Some(SExpr::Atom(s, _)) = args.get(i) {
        if s.starts_with('$') {
            label_name = Some(s.clone());
            i += 1;
        }
    }
    // Optional inline result type: `(result i32)` (no params allowed on a
    // structured block's own header in WASM 1.0's non-multi-value form).
    let blocktype_byte: Vec<u8> = if let Some(r) = args.get(i).filter(|a| a.is_keyword_list("result")) {
        let items = r.as_list().unwrap();
        i += 1;
        if items.len() == 2 {
            vec![parse_value_type(&items[1])?.byte_tag().unwrap()]
        } else {
            vec![0x40]
        }
    } else {
        vec![0x40]
    };

    if name == "if" {
        // `if`'s own condition is a folded operand appearing BEFORE the
        // structured body in text, but WASM's binary form pops the
        // condition immediately before the `if` opcode -- so any leading
        // non-block operand expressions here are the condition.
        let then_start = args[i..].iter().position(|a| a.is_keyword_list("then")).map(|p| p + i);
        let cond_end = then_start.unwrap_or(args.len());
        encode_instr_list(&args[i..cond_end], icx, out)?;
        out.push(opcode);
        out.extend(&blocktype_byte);
        icx.labels.push(label_name);
        if let Some(ts) = then_start {
            let then_items = args[ts].as_list().unwrap();
            encode_instr_list(&then_items[1..], icx, out)?;
            if let Some(else_expr) = args.get(ts + 1).filter(|a| a.is_keyword_list("else")) {
                out.push(0x05); // else
                let else_items = else_expr.as_list().unwrap();
                encode_instr_list(&else_items[1..], icx, out)?;
            }
        } else {
            // Flat (non-folded) `if ... else ... end` form -- body runs
            // until a bare `else`/`end` atom at this same nesting level;
            // encode_instr_list already handles nested lists recursively,
            // so we only need to special-case top-level `else` here.
            let body = &args[i..];
            let else_pos = body.iter().position(|a| matches!(a, SExpr::Atom(s, _) if s == "else"));
            let end_pos = body.iter().position(|a| matches!(a, SExpr::Atom(s, _) if s == "end")).unwrap_or(body.len());
            match else_pos {
                Some(ep) if ep < end_pos => {
                    encode_instr_list(&body[..ep], icx, out)?;
                    out.push(0x05);
                    encode_instr_list(&body[ep + 1..end_pos], icx, out)?;
                }
                _ => {
                    encode_instr_list(&body[..end_pos], icx, out)?;
                }
            }
        }
        icx.labels.pop();
        out.push(0x0B);
        return Ok(());
    }

    // block / loop.
    out.push(opcode);
    out.extend(&blocktype_byte);
    icx.labels.push(label_name);
    let body = &args[i..];
    let end_pos = body.iter().position(|a| matches!(a, SExpr::Atom(s, _) if s == "end")).unwrap_or(body.len());
    encode_instr_list(&body[..end_pos], icx, out)?;
    icx.labels.pop();
    out.push(0x0B);
    let _ = pos;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_of(module: &WasmModule, func_idx: usize) -> &[u8] {
        &module.code[func_idx].code
    }

    #[test]
    fn empty_module_parses() {
        let m = parse_module("(module)").unwrap();
        assert_eq!(m, WasmModule::default());
    }

    #[test]
    fn simple_add_function_encodes_expected_bytes() {
        let m = parse_module(
            "(module (func (param $a i32) (param $b i32) (result i32) local.get $a local.get $b i32.add))",
        )
        .unwrap();
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.types[0], FuncType { params: vec![ValueType::I32, ValueType::I32], results: vec![ValueType::I32] });
        assert_eq!(code_of(&m, 0), &[0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]);
    }

    #[test]
    fn folded_instruction_flattens_to_postfix() {
        let m = parse_module("(module (func (result i32) (i32.add (i32.const 1) (i32.const 2))))").unwrap();
        // i32.const 1; i32.const 2; i32.add; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x01, 0x41, 0x02, 0x6A, 0x0B]);
    }

    #[test]
    fn positional_and_named_locals_share_one_index_space() {
        let m = parse_module(
            "(module (func (param i32) (local $x i32) local.get 0 local.get $x drop))",
        )
        .unwrap();
        // param is local 0, $x is local 1.
        assert_eq!(code_of(&m, 0), &[0x20, 0x00, 0x20, 0x01, 0x1A, 0x0B]);
    }

    #[test]
    fn call_resolves_forward_reference_by_name() {
        // `main` calls `helper`, declared AFTER it -- proves pass 1's
        // symbol collection runs before pass 2's encoding.
        let m = parse_module(
            "(module (func $main (result i32) (call $helper)) (func $helper (result i32) (i32.const 5)))",
        )
        .unwrap();
        assert_eq!(code_of(&m, 0), &[0x10, 0x01, 0x0B]); // call func index 1
        assert_eq!(code_of(&m, 1), &[0x41, 0x05, 0x0B]);
    }

    #[test]
    fn named_labels_resolve_to_correct_branch_depth() {
        let m = parse_module(
            "(module (func (result i32) (block $outer (result i32) (block $inner (result i32) (br $outer (i32.const 1))) (i32.const 2))))",
        )
        .unwrap();
        // block(outer) block(inner) i32.const 1 br 1(outer, depth counted from br site: inner=0,outer=1) end(inner is unreachable after br but still emitted) i32.const 2 end(outer) end(func)
        let code = code_of(&m, 0);
        // Find the br opcode (0x0C) and check its depth immediate is 1 (outer, not 0=inner).
        let br_pos = code.iter().position(|&b| b == 0x0C).unwrap();
        assert_eq!(code[br_pos + 1], 1);
    }

    #[test]
    fn if_then_else_folded_form_encodes_condition_before_if_opcode() {
        let m = parse_module(
            "(module (func (result i32) (if (result i32) (i32.const 1) (then (i32.const 2)) (else (i32.const 3)))))",
        )
        .unwrap();
        // i32.const 1 ; if (result i32) ; i32.const 2 ; else ; i32.const 3 ; end ; end
        assert_eq!(
            code_of(&m, 0),
            &[0x41, 0x01, 0x04, 0x7F, 0x41, 0x02, 0x05, 0x41, 0x03, 0x0B, 0x0B]
        );
    }

    #[test]
    fn memory_load_with_offset_attribute() {
        let m = parse_module("(module (memory 1) (func (result i32) (i32.load offset=8 (i32.const 0))))").unwrap();
        // i32.const 0 ; i32.load align=0 offset=8 ; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x28, 0x00, 0x08, 0x0B]);
    }

    #[test]
    fn global_get_and_module_scoped_global() {
        let m = parse_module("(module (global $g (mut i32) (i32.const 42)) (func (result i32) global.get $g))").unwrap();
        assert_eq!(m.globals[0].global_type, GlobalType { value_type: ValueType::I32, mutable: true });
        assert_eq!(m.globals[0].init_expr, vec![0x41, 0x2A, 0x0B]);
        assert_eq!(code_of(&m, 0), &[0x23, 0x00, 0x0B]);
    }

    #[test]
    fn inline_export_shorthand_and_top_level_export_both_work() {
        let m = parse_module(
            "(module (func $f (export \"a\") (result i32) (i32.const 1)) (export \"b\" (func $f)))",
        )
        .unwrap();
        assert_eq!(m.exports.len(), 2);
        assert!(m.exports.iter().all(|e| e.index == 0));
        assert_eq!(m.exports[0].name, "a");
        assert_eq!(m.exports[1].name, "b");
    }

    #[test]
    fn implicit_type_dedup_reuses_structurally_equal_signature() {
        let m = parse_module(
            "(module (func (param i32) (result i32) (local.get 0)) (func (param i32) (result i32) (local.get 0)))",
        )
        .unwrap();
        assert_eq!(m.types.len(), 1, "two structurally-identical inline signatures must share one type entry");
        assert_eq!(m.functions, vec![0, 0]);
    }

    #[test]
    fn explicit_type_declaration_is_not_deduped_against_inline() {
        let m = parse_module(
            "(module (type $t (func (param i32) (result i32))) (func (type $t) (param i32) (result i32) (local.get 0)))",
        )
        .unwrap();
        assert_eq!(m.types.len(), 1);
        assert_eq!(m.functions, vec![0]);
    }

    #[test]
    fn data_segment_with_string_literal() {
        let m = parse_module(r#"(module (memory 1) (data (i32.const 0) "hi"))"#).unwrap();
        assert_eq!(m.data.len(), 1);
        assert_eq!(m.data[0].data, b"hi");
        assert_eq!(m.data[0].offset_expr, vec![0x41, 0x00, 0x0B]);
    }

    #[test]
    fn unknown_instruction_is_a_clear_error() {
        let err = parse_module("(module (func (nonexistent.op)))").unwrap_err();
        assert!(matches!(err, WastParseError::UnknownInstruction { .. }));
    }

    #[test]
    fn unknown_local_identifier_is_a_clear_error() {
        let err = parse_module("(module (func (result i32) local.get $nope))").unwrap_err();
        assert!(matches!(err, WastParseError::UnknownIdentifier { .. }));
    }

    // ── Security-review regressions: malformed-but-syntactically-parseable
    // input must produce a clean Err, never panic. ────────────────────────

    #[test]
    fn memory_limit_number_out_of_u32_range_errors_cleanly_not_panics() {
        // 2^32 -- an all-digit atom (passes the take_while filter) that
        // doesn't fit u32 (used to unwrap-panic inside parse_limits).
        let err = parse_module("(module (memory 4294967296))").unwrap_err();
        assert!(matches!(err, WastParseError::InvalidNumericLiteralForType { .. }));
    }

    #[test]
    fn folded_br_table_with_no_labels_errors_cleanly_not_panics() {
        // Used to underflow `labels.len() - 1` (0 - 1 on usize).
        let err = parse_module("(module (func (br_table)))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn export_missing_target_errors_cleanly_not_panics() {
        let err = parse_module(r#"(module (export "e"))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn global_with_only_inline_export_and_no_type_errors_cleanly_not_panics() {
        let err = parse_module(r#"(module (global (export "g")))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn empty_elem_and_data_segments_error_cleanly_not_panic() {
        assert!(matches!(parse_module("(module (elem))"), Err(WastParseError::UnexpectedEof)));
        assert!(matches!(parse_module("(module (data))"), Err(WastParseError::UnexpectedEof)));
    }

    #[test]
    fn deeply_nested_parens_error_cleanly_not_stack_overflow() {
        // MAX_NESTING_DEPTH levels' worth of opens, deliberately unclosed
        // (doesn't matter -- depth is checked as each '(' is consumed,
        // before the parser would ever look for a matching close).
        let src = "(".repeat(crate::sexpr::MAX_NESTING_DEPTH + 1);
        let err = parse_module(&src).unwrap_err();
        assert!(matches!(err, WastParseError::TooDeeplyNested { .. }));
    }

    /// Folded `call` with arguments -- catches the exact bug found during
    /// development: the callee index is the FIRST arg, argument
    /// expressions trail it, not the other way around.
    #[test]
    fn folded_call_with_arguments_encodes_index_first_then_argument_operands() {
        let m = parse_module(
            "(module (func $add (param i32 i32) (result i32) (i32.add (local.get 0) (local.get 1))) \
              (func (result i32) (call $add (i32.const 3) (i32.const 4))))",
        )
        .unwrap();
        // i32.const 3 ; i32.const 4 ; call 0 ; end
        assert_eq!(code_of(&m, 1), &[0x41, 0x03, 0x41, 0x04, 0x10, 0x00, 0x0B]);
    }

    /// Folded `br` carrying a value -- same "immediate first, operand
    /// after" shape as `call`, proven observably: the pushed value must
    /// come from the folded `(i32.const 9)` operand, encoded BEFORE `br`'s
    /// own opcode+depth bytes.
    #[test]
    fn folded_br_with_value_encodes_label_first_then_value_operand() {
        let m = parse_module(
            "(module (func (result i32) (block $b (result i32) (br $b (i32.const 9)) (i32.const 0))))",
        )
        .unwrap();
        // block(result i32) ; i32.const 9 ; br 0 ; i32.const 0 ; end ; end
        assert_eq!(
            code_of(&m, 0),
            &[0x02, 0x7F, 0x41, 0x09, 0x0C, 0x00, 0x41, 0x00, 0x0B, 0x0B]
        );
    }

    /// Flat (non-folded) `call` with argument operands preceding it in the
    /// stream, mirroring how real testsuite files write it -- distinct
    /// code path from the folded case above (`encode_stream_instr` vs.
    /// `encode_flat_instr`).
    #[test]
    fn flat_call_with_preceding_argument_operands() {
        let m = parse_module(
            "(module (func $add (param i32 i32) (result i32) local.get 0 local.get 1 i32.add) \
              (func (result i32) i32.const 3 i32.const 4 call $add))",
        )
        .unwrap();
        assert_eq!(code_of(&m, 1), &[0x41, 0x03, 0x41, 0x04, 0x10, 0x00, 0x0B]);
    }

    /// Flat, nested nested `block`/`loop` -- proves the stream-form
    /// terminator scan finds only ITS OWN `end`, never a nested block's,
    /// since the nested block recurses through `encode_one` and consumes
    /// its own `end` first.
    #[test]
    fn flat_nested_blocks_each_consume_their_own_end() {
        // Literal values deliberately avoid 0x02 (the block opcode) and
        // 0x0B (the end opcode) in their own LEB128 encoding, so the raw
        // byte-count assertions below can't collide with incidental data
        // bytes -- 1 and 9 both encode as single bytes 0x01/0x09.
        // Only `block`/`loop`/`if` use a textual `end` keyword; the
        // enclosing `(func ...)` form's own closing paren ends the
        // function body, with no third `end` atom needed or valid.
        let m = parse_module(
            "(module (func (result i32) \
                block $outer (result i32) \
                  block $inner (result i32) i32.const 1 br $outer end \
                  i32.const 9 \
                end))",
        )
        .unwrap();
        let code = code_of(&m, 0);
        // Two block opcodes (0x02), two end opcodes (0x0B) for the blocks
        // plus one more for the function itself = 3 total 0x0B.
        assert_eq!(code.iter().filter(|&&b| b == 0x02).count(), 2);
        assert_eq!(code.iter().filter(|&&b| b == 0x0B).count(), 3);
        // br's depth must resolve to 1 (outer), not 0 (inner).
        let br_pos = code.iter().position(|&b| b == 0x0C).unwrap();
        assert_eq!(code[br_pos + 1], 1);
    }

    /// Flat `if`/`else`/`end`, condition pushed by a preceding flat
    /// instruction (not folded into the `if` itself).
    #[test]
    fn flat_if_else_end() {
        let m = parse_module(
            "(module (func (result i32) i32.const 1 if (result i32) i32.const 2 else i32.const 3 end))",
        )
        .unwrap();
        assert_eq!(
            code_of(&m, 0),
            &[0x41, 0x01, 0x04, 0x7F, 0x41, 0x02, 0x05, 0x41, 0x03, 0x0B, 0x0B]
        );
    }
}
