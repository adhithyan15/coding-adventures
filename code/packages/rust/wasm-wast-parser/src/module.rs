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

use crate::numeric;
use crate::numeric::{parse_f32_bits, parse_f64_bits, parse_i16, parse_i32, parse_i64, parse_i8};
use crate::sexpr::{expect_get, parse_source, SExpr};
use crate::WastParseError;
use std::collections::HashMap;
use wasm_types::{
    DataSegment, Element, Export, ExternalKind, FuncType, FunctionBody, Global, GlobalType,
    Import, ImportTypeInfo, Limits, MemoryType, TableType, ValueType, WasmModule, FUNCREF,
};

/// Parse a whole WAT source text: the plain-`.wat` entry point, and also
/// what a `.wast` script's `(module quote "...")` directive re-parses its
/// concatenated string content as (see `script.rs`'s own doc comment on
/// eager vs. lazy module building).
///
/// The WAT text format allows a source to be written two ways, and this
/// accepts both:
/// - **Explicit**: a single top-level `(module ...)` form (what every other
///   caller in this crate already produces).
/// - **Abbreviated**: the enclosing `module` keyword omitted entirely, with
///   the module's fields written directly at the top level. This is the
///   form the official spec testsuite's `comments.wast`/`block.wast`/etc.
///   use for their `(module quote "(func ...)" "(func ...)" ...)`
///   directives — the concatenated text is `(func ...)(func ...)`, not
///   `(module (func ...)(func ...))`. Other files (`align.wast`,
///   `global.wast`) instead quote the explicit form. Both are real,
///   independently-valid WAT — treating the concatenated text as "exactly
///   one `(module ...)` form" (the old behavior) silently discarded every
///   abbreviated-form field as an unrecognized top-level item, producing an
///   empty module that trivially "passed" validation while every export it
///   was supposed to have was simply missing.
pub fn parse_module(src: &str) -> Result<WasmModule, WastParseError> {
    let exprs = parse_source(src)?;
    if let [single] = exprs.as_slice() {
        if single.as_list().and_then(|items| items.first()).and_then(|i| i.as_atom()) == Some("module") {
            return parse_module_expr(single);
        }
    }
    if exprs.is_empty() {
        return Err(WastParseError::UnexpectedEof);
    }
    let owned_fields: Vec<&SExpr> = exprs.iter().collect();
    let desugared = desugar_inline_imports(&owned_fields);
    let fields: Vec<&SExpr> = desugared.iter().collect();
    let mut ctx = ModuleCtx::default();
    collect_symbols(&fields, &mut ctx)?;
    build(&fields, &mut ctx)?;
    Ok(ctx.module)
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
    let owned_fields: Vec<&SExpr> = items
        .iter()
        .skip(1)
        .skip_while(|e| matches!(e, SExpr::Atom(s, _) if s.starts_with('$')))
        .collect();
    let desugared = desugar_inline_imports(&owned_fields);
    let fields: Vec<&SExpr> = desugared.iter().collect();

    let mut ctx = ModuleCtx::default();
    collect_symbols(&fields, &mut ctx)?;
    build(&fields, &mut ctx)?;
    Ok(ctx.module)
}

/// Desugars WAT's **inline-import shorthand** — `(func $f? (import "m" "n")
/// ...rest)`, and the same for `table`/`memory`/`global` — into the
/// equivalent explicit form, `(import "m" "n" (func $f? ...rest))`, so
/// `collect_symbols`/`build` only ever have to understand ONE import shape.
/// This is a pure syntactic rewrite per the WAT spec (§ module abbreviations):
/// `(func $f (import "m" "n") (type $t))` means EXACTLY `(import "m" "n"
/// (func $f (type $t)))`, not a real function body.
///
/// Only recognizes the import form immediately following an optional
/// `$name` (i.e. as the field's very first substantive item) — the shape
/// every inline-import case in the vendored testsuite actually uses.
/// Combining inline import with inline export on the same field (`(func
/// (export "e") (import "m" "n") ...)`, also spec-legal) isn't handled;
/// no vendored file currently needs it, and it isn't the gap this fixes.
fn desugar_inline_imports(fields: &[&SExpr]) -> Vec<SExpr> {
    fields.iter().map(|f| desugar_one_inline_import(f)).collect()
}

fn desugar_one_inline_import(f: &SExpr) -> SExpr {
    let Some(items) = f.as_list() else { return (*f).clone() };
    let pos = f.pos();
    let kind = items.first().and_then(|e| e.as_atom()).unwrap_or("");
    if !matches!(kind, "func" | "table" | "memory" | "global") {
        return (*f).clone();
    }
    let mut i = 1;
    if matches!(items.get(i), Some(SExpr::Atom(s, _)) if s.starts_with('$')) {
        i += 1;
    }
    match items.get(i) {
        Some(candidate) if candidate.is_keyword_list("import") => {
            let import_items = candidate.as_list().unwrap();
            if import_items.len() != 3 {
                return (*f).clone(); // malformed; let normal parsing surface a real error
            }
            let module_name = import_items[1].clone();
            let field_name = import_items[2].clone();
            let mut desc_items: Vec<SExpr> = items[..i].to_vec(); // kind atom + optional $name
            desc_items.extend(items[i + 1..].iter().cloned()); // everything after (import ...)
            let desc = SExpr::List(desc_items, pos);
            SExpr::List(vec![SExpr::Atom("import".to_string(), pos), module_name, field_name, desc], pos)
        }
        _ => (*f).clone(),
    }
}

#[derive(Default)]
struct ModuleCtx {
    module: WasmModule,
    type_names: HashMap<String, u32>,
    func_names: HashMap<String, u32>,
    table_names: HashMap<String, u32>,
    memory_names: HashMap<String, u32>,
    global_names: HashMap<String, u32>,
    /// A data segment's own `$id` -> its index in `module.data` (task #95).
    /// `memory.init`/`data.drop` reference a data segment by this same
    /// name/index space, mirroring every other `*_names` map here.
    data_names: HashMap<String, u32>,
    /// An element segment's own `$id` -> its index in `module.elements`
    /// (task #97). `table.init`/`elem.drop` reference an element segment
    /// by this same name/index space, mirroring `data_names` above.
    elem_names: HashMap<String, u32>,
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
    // `(ref null func)` / `(ref null extern)` -- the fully-spelled-out
    // nullable abstract-heap-type syntax, semantically identical to the bare
    // `funcref`/`externref` keywords below (WASM17; found in the real
    // corpus's `br_table.wast`, e.g. `(result (ref null func))`). Non-null
    // `(ref func)` and concrete `(ref null $t)` / `(ref $t)` forms are
    // deliberately NOT recognized -- see `code/specs/
    // W08-wasm-funcref-externref.md`'s "explicitly out of scope" section.
    if let Some(items) = expr.as_list() {
        if items.len() == 3 && items[0].as_atom() == Some("ref") && items[1].as_atom() == Some("null") {
            return match items[2].as_atom() {
                Some("func") => Ok(ValueType::Funcref),
                Some("extern") => Ok(ValueType::Externref),
                _ => Err(WastParseError::UnexpectedToken { pos: expr.pos(), found: "list".to_string(), expected: "a value type" }),
            };
        }
        return Err(WastParseError::UnexpectedToken { pos: expr.pos(), found: "list".to_string(), expected: "a value type" });
    }
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
        "v128" => Ok(ValueType::V128),
        "funcref" => Ok(ValueType::Funcref),
        "externref" => Ok(ValueType::Externref),
        other => Err(WastParseError::UnexpectedToken { pos: expr.pos(), found: other.to_string(), expected: "a value type" }),
    }
}

/// Parse a `ref.null` heap-type immediate keyword (`func` or `extern`) into
/// its binary heap-type byte. This crate only recognizes the two abstract
/// heap types `funcref`/`externref` need (WASM17); a concrete `$t` heap
/// type is deliberately out of scope (see `parse_value_type`'s doc comment).
fn parse_ref_null_heap_type(expr: &SExpr) -> Result<u8, WastParseError> {
    let s = expr.as_atom().ok_or(WastParseError::UnexpectedToken {
        pos: expr.pos(),
        found: "list".to_string(),
        expected: "a heap type (func or extern)",
    })?;
    match s {
        "func" => Ok(0x70),
        "extern" => Ok(0x6F),
        other => Err(WastParseError::UnexpectedToken { pos: expr.pos(), found: other.to_string(), expected: "a heap type (func or extern)" }),
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
    //
    // `ctx.module.functions`/`tables`/`memories`/`globals` mirror the real
    // WASM BINARY format's function/table/memory/global SECTIONS, which
    // never include imports (an import's type info lives solely in
    // `ctx.module.imports`; see `wasm-module-parser`'s own section parsers,
    // which never touch these arrays while parsing the import section
    // either). So this loop must NOT push placeholder entries into them --
    // only `func_i`/`table_i`/`memory_i`/`global_i` below (dedicated,
    // per-kind counters, since imports of different kinds interleave in
    // one textual pass) track "how many imports of this kind have been
    // seen so far," which IS the func-space/table-space/etc. index imports
    // occupy (they're always the lowest indices, so the k-th import of a
    // kind gets index k directly, no offset).
    let mut import_func_i = 0u32;
    let mut import_table_i = 0u32;
    let mut import_memory_i = 0u32;
    let mut import_global_i = 0u32;
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
                    if let Some(n) = name {
                        insert_unique(&mut ctx.func_names, n, import_func_i, f.pos(), "func")?;
                    }
                    import_func_i += 1;
                }
                "table" => {
                    if let Some(n) = name {
                        insert_unique(&mut ctx.table_names, n, import_table_i, f.pos(), "table")?;
                    }
                    import_table_i += 1;
                }
                "memory" => {
                    if let Some(n) = name {
                        insert_unique(&mut ctx.memory_names, n, import_memory_i, f.pos(), "memory")?;
                    }
                    import_memory_i += 1;
                }
                "global" => {
                    if let Some(n) = name {
                        insert_unique(&mut ctx.global_names, n, import_global_i, f.pos(), "global")?;
                    }
                    import_global_i += 1;
                }
                _ => {}
            }
            let import = build_import_shell(items, desc, kind)?;
            ctx.module.imports.push(import);
        }
    }

    // Non-import definitions, in textual order. Each one's NAME resolves to
    // a func-space/table-space/etc. index (`num_import_X + ctx.module.X.len()`
    // -- the fixed import count from the pass above, plus how many REAL
    // definitions of this kind have been pushed to the (now import-free)
    // storage array so far), but the storage array itself is indexed
    // real-definitions-only, matching the binary format `wasm-module-parser`
    // itself produces -- `build`'s pass 2 relies on this same split (see
    // `build_func`'s own doc comment).
    let num_import_funcs = import_func_i;
    let num_import_tables = import_table_i;
    let num_import_memories = import_memory_i;
    let num_import_globals = import_global_i;
    // Data segments (task #95) have no import counterpart, so their index
    // space is just textual declaration order -- this counter tracks the
    // same index `build_data`'s later `ctx.module.data.push(...)` will
    // assign, since both loops walk `fields` in the same order.
    let mut data_i = 0u32;
    // Element segments (task #97) have the same "no import counterpart,
    // index space is just textual declaration order" shape as data
    // segments above -- this counter tracks the same index `build_elem`'s
    // later `ctx.module.elements.push(...)` will assign.
    let mut elem_i = 0u32;
    for f in fields {
        if f.is_keyword_list("func") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = num_import_funcs + ctx.module.functions.len() as u32;
                    insert_unique(&mut ctx.func_names, name, idx, f.pos(), "func")?;
                }
            }
            ctx.module.functions.push(0); // fixed in pass 2
            ctx.module.code.push(FunctionBody { locals: vec![], code: vec![0x0B] }); // fixed in pass 2
        } else if f.is_keyword_list("table") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = num_import_tables + ctx.module.tables.len() as u32;
                    insert_unique(&mut ctx.table_names, name, idx, f.pos(), "table")?;
                }
            }
            ctx.module.tables.push(TableType { element_type: FUNCREF, limits: Limits { min: 0, max: None } });
        } else if f.is_keyword_list("memory") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = num_import_memories + ctx.module.memories.len() as u32;
                    insert_unique(&mut ctx.memory_names, name, idx, f.pos(), "memory")?;
                }
            }
            ctx.module.memories.push(MemoryType { limits: Limits { min: 0, max: None }, shared: false }); // fixed in pass 2
        } else if f.is_keyword_list("global") {
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    let idx = num_import_globals + ctx.module.globals.len() as u32;
                    insert_unique(&mut ctx.global_names, name, idx, f.pos(), "global")?;
                }
            }
            ctx.module.globals.push(Global {
                global_type: GlobalType { value_type: ValueType::I32, mutable: false },
                init_expr: vec![0x0B],
            });
        } else if f.is_keyword_list("data") {
            // No placeholder pushed here (unlike func/table/memory/global
            // above): `build_data` (pass 2) is the only thing that appends
            // to `ctx.module.data`, so this just registers `$id -> data_i`
            // ahead of time -- `data_i` alone tracks the index, not
            // `ctx.module.data.len()` (which stays 0 through all of pass 1).
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    insert_unique(&mut ctx.data_names, name, data_i, f.pos(), "data")?;
                }
            }
            data_i += 1;
        } else if f.is_keyword_list("elem") {
            // No placeholder pushed here (same reasoning as the "data"
            // branch above): `build_elem` (pass 2) is the only thing
            // that appends to `ctx.module.elements`, so this just
            // registers `$id -> elem_i` ahead of time.
            let items = f.as_list().unwrap();
            if let Some(name) = items.get(1).and_then(|e| e.as_atom()) {
                if name.starts_with('$') {
                    insert_unique(&mut ctx.elem_names, name, elem_i, f.pos(), "elem")?;
                }
            }
            elem_i += 1;
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
        "table" => {
            // `desc` is `(table $name? limits reftype)` -- was previously
            // discarded entirely (a `FUNCREF`/zero-limits placeholder,
            // never fixed up afterward), same class of bug as the
            // declared-table case above (task #96): an imported table's
            // REAL limits and element type must come from the source, not
            // a stub.
            let limits_start = if desc.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
            let rest = &desc[limits_start..];
            // Task #99: same hex-aware shape check as the declared-table
            // and `parse_limits` call sites -- an imported table with a
            // hex limit (`(import "m" "t" (table 0x10 funcref))`) hit the
            // identical ascii-digit-only bug here too.
            let digit_count = rest.iter().take_while(|e| e.as_atom().is_some_and(looks_like_uint_literal)).count();
            let limits = parse_limits(&rest[..digit_count])?;
            let reftype = expect_get(rest, digit_count)?;
            let element_type = match reftype.as_atom() {
                Some("funcref") => wasm_types::FUNCREF,
                Some("externref") => wasm_types::EXTERNREF,
                _ => {
                    return Err(WastParseError::UnexpectedToken {
                        pos: reftype.pos(),
                        found: reftype.as_atom().unwrap_or("list").to_string(),
                        expected: "funcref or externref",
                    })
                }
            };
            ImportTypeInfo::Table(TableType { element_type, limits })
        }
        "memory" => {
            // `desc` is `(memory $name? limits... shared?)` -- same
            // optional-`$name`-at-index-1 shape `global`'s arm below
            // handles, and the same class of bug WASM19 fixed there: a
            // named inline-import memory (`(memory $m (import "m" "n")
            // 1 1)`) desugars to `desc = [memory, $m, 1, 1]`, and
            // `desc[1..]` alone would hand `parse_memory_limits` a
            // leading `$m` atom, which its `take_while(digit)` scan
            // would immediately stop on -- "expected 1 or 2 limit
            // numbers" on a syntactically valid, real WAT form. Found
            // while adding WASM18's `shared` keyword support.
            let limits_start = if desc.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
            let (limits, shared) = parse_memory_limits(&desc[limits_start..])?;
            ImportTypeInfo::Memory(MemoryType { limits, shared })
        }
        "global" => {
            // `desc` is `(global $name? <type>)` -- an optional `$name`
            // (from either the desugared inline-import shorthand or a real
            // explicit `(import "m" "n" (global $g <type>))`) can sit at
            // index 1, pushing the actual type field to index 2. Every
            // other import kind's `desc` either doesn't carry a name at
            // all (table/memory don't read `desc` here) or doesn't need to
            // skip past one (func's type is resolved later) -- `global` is
            // the only kind whose type info lives IN `desc` at a
            // name-dependent offset. Found via the real corpus's
            // `global.wast`, whose `(global $g0 (import "G" "g") i32)`
            // (a NAMED inline import) previously mis-read `$g0` itself as
            // the value type ("expected a value type, found \"$g0\"") --
            // only the unnamed shorthand `(global (import ...) i32)`
            // worked before this fix.
            let type_field = if desc.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) {
                desc.get(2)
            } else {
                desc.get(1)
            }
            .ok_or(WastParseError::UnexpectedEof)?;
            ImportTypeInfo::Global(parse_global_type(type_field)?)
        }
        // `desc` can be an empty list (`(import "m" "n" ())`), in which case
        // `kind` is already "" from `unwrap_or("")` above -- fall back to
        // the enclosing `(import ...)` form's own position (always present,
        // `items` is that form's own list) rather than indexing `desc[0]`.
        _ => return Err(WastParseError::UnexpectedToken { pos: desc.first().map(|e| e.pos()).unwrap_or_else(|| items[0].pos()), found: kind.to_string(), expected: "func/table/memory/global" }),
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

/// Structural (not numeric) check: does this atom look like a limit
/// literal, decimal or `0x`/`0X`-hex, `_`-separated? Used only to decide
/// where the leading run of limit numbers ends and any trailing keyword
/// (`shared` for memory, a reftype for table) begins -- the ACTUAL value
/// (and its own range validation) comes from [`numeric::parse_u32`]
/// below, not from this shape check.
fn looks_like_uint_literal(s: &str) -> bool {
    let s = s.strip_prefix('+').unwrap_or(s);
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit() || c == '_'),
        None => !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '_'),
    }
}

fn parse_limits(fields: &[SExpr]) -> Result<Limits, WastParseError> {
    // Task #99: the real testsuite's own `table.wast` uses hex limits like
    // `(table 0xffff_ffff funcref)` -- this used to filter to ascii-digit-
    // only atoms, silently dropping a hex atom out of the limits list
    // entirely (wrong `Limits`, not even a clean parse error) instead of
    // parsing it. `numeric::parse_u32` already has the real hex-aware
    // magnitude parser `parse_i32`/`parse_i8`/etc. all share.
    let digit_atoms: Vec<&SExpr> = fields.iter().take_while(|e| e.as_atom().is_some_and(looks_like_uint_literal)).collect();
    let nums: Vec<u32> = digit_atoms
        .iter()
        .map(|e| {
            let (s, pos) = match e {
                SExpr::Atom(s, pos) => (s.as_str(), *pos),
                _ => unreachable!("take_while already filtered to atoms"),
            };
            numeric::parse_u32(s, pos)
        })
        .collect::<Result<_, _>>()?;
    match nums.as_slice() {
        [min] => Ok(Limits { min: *min, max: None }),
        [min, max] => Ok(Limits { min: *min, max: Some(*max) }),
        _ => Err(WastParseError::UnexpectedToken { pos: 0, found: "".into(), expected: "1 or 2 limit numbers" }),
    }
}

/// A memory's limits, plus WASM18's `shared` trailing keyword
/// (`(memory 1 1 shared)`) -- tables never carry this, so `parse_limits`
/// itself stays shared/table-agnostic; this wrapper is memory-specific.
/// `parse_limits`'s own digit-scanning `take_while` already stops at the
/// first non-digit atom, so a trailing `shared` keyword never confuses
/// the numeric parse -- this just also checks whether it's present.
fn parse_memory_limits(fields: &[SExpr]) -> Result<(Limits, bool), WastParseError> {
    let limits = parse_limits(fields)?;
    let shared = fields.iter().any(|e| e.as_atom() == Some("shared"));
    Ok((limits, shared))
}

fn parse_global_type(expr: &SExpr) -> Result<GlobalType, WastParseError> {
    if expr.is_keyword_list("mut") {
        let items = expr.as_list().unwrap();
        // `(mut)` with no trailing value type is syntactically a valid
        // keyword-list (arity isn't checked by `is_keyword_list`) but has
        // no second element.
        Ok(GlobalType { value_type: parse_value_type(expect_get(items, 1)?)?, mutable: true })
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
    let mut func_i = 0usize;
    let mut table_i = 0usize;
    let mut memory_i = 0usize;
    let mut global_i = 0usize;

    for f in fields {
        let items = f.as_list().unwrap_or(&[]);
        let head = items.first().and_then(|e| e.as_atom()).unwrap_or("");
        match head {
            "import" => {
                // Safe: `collect_symbols` (pass 1) already ran to completion
                // over this exact same `fields` list before `build` (pass 2)
                // is ever called -- and pass 1's own `build_import_shell`
                // already returns a clean `Err` (not a panic) unless
                // `items[3]` is a list AND its first element is an atom
                // matching one of func/table/memory/global. Reaching this
                // arm at all is proof both hold for this form.
                let desc = items[3].as_list().unwrap();
                if desc[0].as_atom().unwrap() == "func" {
                    // A func import's resolved type lives ONLY in its own
                    // `ctx.module.imports[..].type_info` -- `ctx.module.functions`
                    // never has an entry for it (that array mirrors the
                    // binary format's function section, which is real-funcs-only;
                    // see `collect_symbols`'s doc comment on this same split).
                    let type_idx = resolve_func_signature_ref(&desc[1..], ctx)?;
                    ctx.module.imports[import_all_i].type_info = ImportTypeInfo::Function(type_idx);
                }
                // table/memory/global import shells are already correct
                // from pass 1 -- no fixup needed for those kinds.
                import_all_i += 1;
            }
            "func" => {
                let name_skip = if items.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
                build_func(&items[name_skip..], ctx, num_import_funcs + func_i, func_i)?;
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
                // `space_idx` (imports counted in) is what exports address
                // this memory by; `memory_i` (real-only) is where its entry
                // actually lives in `ctx.module.memories` -- see
                // `collect_symbols`'s doc comment on why these differ once
                // a module has any memory import.
                let space_idx = num_import_memories + memory_i;
                let (limits_start, _) = handle_inline_export(rest, "memory", space_idx, ctx)?;
                let (limits, shared) = parse_memory_limits(&rest[limits_start..])?;
                ctx.module.memories[memory_i].limits = limits;
                ctx.module.memories[memory_i].shared = shared;
                memory_i += 1;
            }
            "table" => {
                let name_skip = if items.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
                let rest = &items[name_skip..];
                let space_idx = num_import_tables + table_i;
                let (limits_start, _) = handle_inline_export(rest, "table", space_idx, ctx)?;
                build_table_limits_and_elements(&rest[limits_start..], space_idx as u32, table_i as u32, ctx)?;
                table_i += 1;
            }
            "global" => {
                let name_skip = if items.get(1).and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) { 2 } else { 1 };
                let rest = &items[name_skip..];
                let space_idx = num_import_globals + global_i;
                let (type_start, _) = handle_inline_export(rest, "global", space_idx, ctx)?;
                let gt = parse_global_type(expect_get(rest, type_start)?)?;
                let init_instrs = rest.get(type_start + 1..).unwrap_or(&[]);
                let mut code = Vec::new();
                encode_instr_list(init_instrs, &mut InstrCtx::empty(ctx), &mut code)?;
                code.push(0x0B);
                ctx.module.globals[global_i] = Global { global_type: gt, init_expr: code };
                global_i += 1;
            }
            "elem" => build_elem(&items[1..], ctx)?,
            "data" => build_data(&items[1..], ctx)?,
            "start" => {
                let idx = resolve_idx(&ctx.func_names, expect_get(items, 1)?, "func")?;
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
        // `(export)` with no trailing name string is syntactically a valid
        // keyword-list but has no second element.
        if let SExpr::Str(b, _) = expect_get(items, 1)? {
            ctx.module.exports.push(Export { name: String::from_utf8_lossy(b).to_string(), kind, index: idx as u32 });
        }
        i += 1;
    }
    Ok((i, ()))
}

/// Resolve a `func` import description's signature (`(type $t)`, inline
/// `(param...) (result...)`, or a mix) to a type-section index, deduping
/// an inline-only signature the same way a non-import `func` would.
///
/// Only scans the LEADING run of `param`/`result`/`type` fields (via
/// `is_leading_field`, the same boundary `build_func`'s own mismatch
/// pre-scan uses) after skipping an optional leading `$name` atom, not all
/// of `desc_rest` unbounded. `build_func` already strips a func's own
/// `$name` before calling this, but the import call site's `desc[1..]`
/// does not, so both must be handled here. For an import description
/// every field after that optional name IS in the leading run anyway, so
/// the bounding is a no-op there -- but `build_func` calls this with the
/// WHOLE remaining function body, and a flat (non-folded) `block`/`loop`/
/// `if`'s own multi-value blocktype (WASM06/WASM04) puts UNNESTED
/// `(param ...)`/`(result ...)` sibling fields later in that same slice
/// (folded syntax nests them inside one `(block ...)` list instead, which
/// is naturally immune). Scanning unbounded picked up a later block's
/// blocktype fields as if they were part of the FUNC's own signature,
/// corrupting `param_count` for the mismatch check right below this
/// function's call site and silently rejecting perfectly valid functions.
fn resolve_func_signature_ref(desc_rest: &[SExpr], ctx: &mut ModuleCtx) -> Result<u32, WastParseError> {
    let start = if matches!(desc_rest.first(), Some(SExpr::Atom(s, _)) if s.starts_with('$')) { 1 } else { 0 };
    let sig_end = start + desc_rest[start..].iter().take_while(|f| is_leading_field(f)).count();
    let leading = &desc_rest[start..sig_end];
    if let Some(type_ref) = leading.iter().find(|e| e.is_keyword_list("type")) {
        let items = type_ref.as_list().unwrap();
        return resolve_idx(&ctx.type_names, expect_get(items, 1)?, "type");
    }
    let sig_fields: Vec<&SExpr> = leading.iter().collect();
    let ty = parse_func_signature(&sig_fields)?;
    Ok(dedup_type(&mut ctx.module, ty))
}

/// Whether `f` belongs to a WASM function's leading `param`/`result`/
/// `type`/`local` region -- shared verbatim by `build_func`'s mismatch
/// pre-scan and its main index-assignment loop so the two can never
/// silently disagree on where that region ends (a round-3 security
/// review found they'd drifted apart once already).
fn is_leading_field(f: &SExpr) -> bool {
    f.is_keyword_list("param") || f.is_keyword_list("result") || f.is_keyword_list("type") || f.is_keyword_list("local")
}

/// Count how many params one `(param ...)` s-expression's `items` (the
/// full list, including the leading `"param"` atom) declares, and the
/// name of the ONE it declares if it's the named form -- `(param $x i32)`
/// is a single named param (a name + its type), not two; `(param i32
/// i32)` is two unnamed ones. Shared by `build_func`'s mismatch pre-scan
/// and its main index-assignment loop for the same reason `is_leading_field`
/// is: a round-4 security review flagged two independently-maintained
/// copies of this same arithmetic as exactly the kind of drift that
/// produced this bug's first two rounds.
fn count_literal_param(items: &[SExpr]) -> (u32, Option<String>) {
    if items.len() == 3 && items[1].as_atom().is_some_and(|s| s.starts_with('$')) {
        (1, Some(items[1].as_atom().unwrap().to_string()))
    } else {
        ((items.len() - 1) as u32, None)
    }
}

/// `func_idx` is this function's index in the **func space** (imports,
/// which occupy the lowest indices, counted in) -- what exports, calls, and
/// `ctx.module.functions` itself all address a function by. `code_idx` is
/// separate: `ctx.module.code` holds bodies for REAL functions only (an
/// import has no body to encode), so it's indexed 0.. among just those,
/// with no offset for however many func imports precede this one. The two
/// coincide (`code_idx == func_idx`) in every module with zero func
/// imports, which is why this distinction went unexercised until inline
/// import shorthand (`(func $f (import "m" "n") ...)`, desugared by
/// `desugar_inline_imports`) gave a real module both a func import AND a
/// subsequent real function body for the first time in this crate's own
/// tests and the vendored corpus alike -- `ctx.module.code[func_idx]` panics
/// with an out-of-bounds index the moment that combination occurs.
fn build_func(fields: &[SExpr], ctx: &mut ModuleCtx, func_idx: usize, code_idx: usize) -> Result<(), WastParseError> {
    // Inline export shorthand: `(func $f (export "e") ...)`.
    let (after_export, _) = handle_inline_export(fields, "func", func_idx, ctx)?;
    let fields = &fields[after_export..];

    let type_idx = resolve_func_signature_ref(fields, ctx)?;
    ctx.module.functions[code_idx] = type_idx;

    // Local scope: params first, then declared `(local ...)` forms right
    // after them. The FIRST declared local's index must be the function's
    // REAL param count -- `ctx.module.types[type_idx].params.len()` --
    // not a count built by re-walking this function's own literal
    // `(param ...)` forms below. Those two can differ: a function that
    // references an out-of-line signature via `(type $sig)` (`func.wast`'s
    // own "type-use-1".."type-use-5" cases, none of which repeat `(param
    // ...)` inline) has ZERO literal `(param ...)` forms in `fields` at
    // all, even though its real type has params occupying local indices
    // 0..N. Seeding the local-index counter from 0 in that case makes the
    // first declared `(local ...)` silently alias parameter index 0
    // instead of starting after the params -- a real, previously-wrong
    // computed VALUE (not a trap), since `local.get $var` then reads the
    // param's value instead of the local's own zero-initialized default.
    // The loop below still walks literal `(param ...)` forms (when
    // present) to capture their OPTIONAL `$name`s at the correct
    // POSITIONAL index, which is unaffected by this fix either way.
    //
    // `resolved_type` is `None` for an out-of-range numeric `(type N)`
    // reference (no `(type ...)` section entry at all) -- a real, already
    // regression-tested case (`func_with_out_of_range_numeric_type_reference_does_not_panic`)
    // this text-level parser deliberately does NOT reject: bounds-checking
    // a type index is `wasm-validator`'s job, not this parser's, and this
    // already-structurally-invalid module will fail validation regardless,
    // for the missing type, not for anything computed here. `param_count`
    // falls back to 0 for local-index purposes in that case (unaffected
    // either way, since there's no real type to disagree with).
    //
    // A round-4 security review found the mismatch check just below this
    // had DRIFTED from that same documented contract: it compared against
    // `param_count`'s 0-fallback too, so `(func (type 0) (param i32))`
    // with NO `(type ...)` section at all got hard-rejected at parse time
    // instead of the "still parses, fails validation instead" behavior
    // the crate promises for an out-of-range type index. Gating the check
    // on `resolved_type.is_some()` restores that contract: an unresolvable
    // type reference is `wasm-validator`'s problem either way, never this
    // check's.
    let resolved_type = ctx.module.types.get(type_idx as usize);
    let param_count = resolved_type.map(|t| t.params.len()).unwrap_or(0) as u32;

    // Reject a func that gives BOTH an explicit `(type $sig)` reference
    // AND its own literal `(param ...)` forms whose arity disagrees with
    // `$sig`'s real params -- see `WastParseError::TypeUseParamCountMismatch`'s
    // own doc comment for why local-index computation below depends on
    // this invariant holding, not just on it being the common case. When
    // a func has NO `(type ...)` reference at all, `resolve_func_signature_ref`
    // synthesizes its type directly FROM these same literal params, so
    // `param_count` and `literal_param_count` are equal by construction
    // and this is always a no-op in that case.
    //
    // A round-2 security review found this pre-scan's original "stop"
    // condition (break on the first field that isn't `param`/`result`/
    // `type`) DIVERGED from the main assignment loop below, which also
    // treats `local` as part of the same leading region (this text-level
    // parser doesn't enforce that `(param ...)` forms all precede `(local
    // ...)` forms -- that's `wasm-validator`'s job, same division of
    // responsibility documented throughout this file). A `func` with a
    // `(local ...)` BEFORE some of its trailing `(param ...)` forms made
    // this scan stop early, undercounting `literal_param_count` (or never
    // even setting `saw_literal_param`) and silently skipping the check
    // below -- while the main loop still processed those later params,
    // seeding `next_local` from a stale, too-small count. `is_leading_field`
    // below is shared verbatim by both this pre-scan and the main loop's
    // own `else if`/`else { break }` structure, so the two can no longer
    // silently disagree on where the leading region ends. `count_literal_param`
    // is likewise the SAME function both this pre-scan and the main loop
    // call for the named-vs-unnamed counting logic, for the same reason:
    // two independently-maintained copies of "the same" arithmetic is
    // exactly the pattern that produced this bug's first two rounds.
    let mut literal_param_count = 0u32;
    let mut saw_literal_param = false;
    for f in fields.iter().take_while(|f| is_leading_field(f)) {
        if f.is_keyword_list("param") {
            saw_literal_param = true;
            literal_param_count += count_literal_param(f.as_list().unwrap()).0;
        }
    }
    if resolved_type.is_some() && saw_literal_param && literal_param_count != param_count {
        return Err(WastParseError::TypeUseParamCountMismatch {
            pos: fields.first().map(|f| f.pos()).unwrap_or(0),
            declared: literal_param_count as usize,
            referenced: param_count as usize,
        });
    }

    let mut local_names: HashMap<String, u32> = HashMap::new();
    let mut param_position = 0u32;
    // `next_local` isn't seeded until the FIRST `(local ...)` form is
    // actually reached -- see below for why.
    let mut next_local: Option<u32> = None;
    let mut locals_decl: Vec<ValueType> = Vec::new();
    let mut instr_start = 0usize;
    for (i, f) in fields.iter().enumerate() {
        if f.is_keyword_list("param") {
            let (count, name) = count_literal_param(f.as_list().unwrap());
            if let Some(name) = name {
                local_names.insert(name, param_position);
            }
            param_position += count;
            instr_start = i + 1;
        } else if f.is_keyword_list("result") || f.is_keyword_list("type") {
            instr_start = i + 1;
        } else if f.is_keyword_list("local") {
            // A security review found a residual edge case in the fix
            // above: `param_position` (literal `(param ...)` forms
            // counted as written) and `param_count` (the type's real,
            // resolved param count) are only guaranteed to agree when a
            // function's literal params match its `(type $sig)`
            // reference exactly -- `resolve_func_signature_ref` doesn't
            // enforce that itself (that's `wasm-validator`'s job, same
            // division of responsibility as the out-of-range `(type N)`
            // case above). A syntactically-valid but semantically
            // inconsistent module (literal params disagreeing in count
            // with a same-function `(type $sig)` reference) could
            // otherwise make a declared local alias whichever of the two
            // counts was smaller. Seeding from `max` the first time a
            // `(local ...)` is actually reached (not the loop iteration
            // that resolves `type_idx`) guarantees a declared local can
            // never collide with a position either count considers a
            // parameter, in every case -- including the ordinary one
            // this fix exists for, where `param_position` stays 0.
            let next_local = next_local.get_or_insert_with(|| param_position.max(param_count));
            let items = f.as_list().unwrap();
            if items.len() == 3 && items[1].as_atom().is_some_and(|s| s.starts_with('$')) {
                local_names.insert(items[1].as_atom().unwrap().to_string(), *next_local);
                locals_decl.push(parse_value_type(&items[2])?);
                *next_local += 1;
            } else {
                for t in &items[1..] {
                    locals_decl.push(parse_value_type(t)?);
                    *next_local += 1;
                }
            }
            instr_start = i + 1;
        } else {
            // A round-5 security review found `is_leading_field`'s own
            // doc comment overclaimed this dispatch actually CALLS it --
            // it re-implements the identical 4-way test inline instead,
            // which happened to still agree today but is exactly the kind
            // of silent-drift risk rounds 2-4 spent closing for the
            // arity-counting logic. This assertion makes that claim
            // actually true (not just documented): any future edit that
            // adds a leading-field kind to one without the other trips
            // this in every `cargo test` run, immediately, rather than
            // waiting for a security review to notice again.
            debug_assert!(
                !is_leading_field(f),
                "is_leading_field and this dispatch's own param/result/type/local arms must stay in sync"
            );
            break;
        }
    }

    let mut icx = InstrCtx { module: ctx, locals: local_names, labels: Vec::new(), depth: 0 };
    let mut code = Vec::new();
    encode_instr_list(&fields[instr_start..], &mut icx, &mut code)?;
    code.push(0x0B);
    ctx.module.code[code_idx] = FunctionBody { locals: locals_decl, code };
    Ok(())
}

/// A `table` declaration's payload (after any name/inline-export fields
/// are already stripped) is one of two forms:
/// - `limits reftype` — explicit `min [max]` numbers, e.g. `(table 1
///   funcref)` or `(table 1 10 funcref)`.
/// - `reftype (elem elem*)` — no explicit numbers at all; the table's
///   size is implied by the inline element list, which is sugar for
///   `min = max = element count` plus an elem segment initializing the
///   table with those elements starting at offset 0. Found missing while
///   running the real WebAssembly/testsuite corpus (e.g. `br.wast`,
///   `call_indirect.wast`) -- every one of them uses this shorthand for
///   their auxiliary function-pointer table.
///
/// `table_idx` is the table-space index (imports counted in) -- what an
/// `Element`'s own `table_index` field addresses, matching how the binary
/// format's element section references a table. `storage_idx` is separate:
/// `ctx.module.tables` holds entries for REAL (non-import) tables only, so
/// it's indexed 0.. among just those, with no offset for however many table
/// imports precede this one -- the same split `build_func`'s own doc
/// comment explains for functions/code, needed here for the identical
/// reason once a module can combine a table import with a real table.
fn build_table_limits_and_elements(rest: &[SExpr], table_idx: u32, storage_idx: u32, ctx: &mut ModuleCtx) -> Result<(), WastParseError> {
    // Task #99: this used the same ascii-digit-only check `parse_limits`
    // itself used to have, so a hex limit like `(table 0x10 0xffff_ffff
    // funcref)` fell into the WRONG branch below entirely (mistaking the
    // limits for a missing-limits `[reftype, (elem e*)]` form), not just
    // mis-parsing the number. `looks_like_uint_literal` is the same hex-
    // aware shape check `parse_limits` now uses.
    let starts_with_limit_number = rest.first().and_then(|e| e.as_atom()).is_some_and(looks_like_uint_literal);
    if starts_with_limit_number {
        // Task #96: `(table $t 2 externref)` -- `parse_limits`'s own
        // digit-scanning `take_while` only consumes the leading numeric
        // limit(s), leaving the REQUIRED trailing reftype keyword
        // (`funcref`/`externref`) unconsumed right after them. Previously
        // this reftype was silently discarded entirely, leaving every
        // table's `element_type` at its `TableType::default()`-style
        // FUNCREF placeholder regardless of what the source actually
        // declared -- real (not just multi-table) corpus files with an
        // `externref` table hit this.
        let digit_count = rest.iter().take_while(|e| e.as_atom().is_some_and(looks_like_uint_literal)).count();
        ctx.module.tables[storage_idx as usize].limits = parse_limits(&rest[..digit_count])?;
        let reftype = expect_get(rest, digit_count)?;
        ctx.module.tables[storage_idx as usize].element_type = match reftype.as_atom() {
            Some("funcref") => wasm_types::FUNCREF,
            Some("externref") => wasm_types::EXTERNREF,
            _ => {
                return Err(WastParseError::UnexpectedToken {
                    pos: reftype.pos(),
                    found: reftype.as_atom().unwrap_or("list").to_string(),
                    expected: "funcref or externref",
                })
            }
        };
        return Ok(());
    }

    // `[reftype, (elem e*)]` -- skip the reftype keyword (this crate only
    // tracks FUNCREF tables, matching every MVP-era table declaration),
    // then resolve the elem list's own function references. `(table
    // funcref ())` -- a syntactically valid but EMPTY inner list -- has no
    // "elem" head atom at all, so this must confirm one is actually
    // present before slicing `[1..]`, not just that the list is non-empty.
    let elem_items = expect_get(rest, 1)?
        .as_list()
        .ok_or(WastParseError::UnexpectedEof)?;
    if expect_get(elem_items, 0)?.as_atom() != Some("elem") {
        return Err(WastParseError::UnexpectedToken {
            pos: elem_items[0].pos(),
            found: "list".to_string(),
            expected: "an (elem ...) form",
        });
    }
    let function_indices: Vec<Option<u32>> = elem_items[1..]
        .iter()
        .map(|f| resolve_idx(&ctx.func_names, f, "func").map(Some))
        .collect::<Result<_, _>>()?;
    let count = function_indices.len() as u32;
    ctx.module.tables[storage_idx as usize].limits = Limits { min: count, max: Some(count) };
    ctx.module.elements.push(Element {
        table_index: table_idx,
        offset_expr: vec![0x41, 0x00, 0x0B], // i32.const 0; end
        function_indices,
        is_passive: false,
    });
    Ok(())
}

fn build_elem(fields: &[SExpr], ctx: &mut ModuleCtx) -> Result<(), WastParseError> {
    let mut i = 0;
    // Optional leading `$id` (task #97): already registered into
    // `ctx.elem_names` by `collect_symbols`'s pass-1 pre-pass (mirrors
    // `data_names`'s own task #95 pattern), so this is a skip only.
    if fields.first().and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) {
        i += 1;
    }
    let has_table_clause = fields.get(i).is_some_and(|e| e.is_keyword_list("table"));
    let table_index = if has_table_clause {
        let table_form = fields[i].as_list().unwrap();
        let idx = resolve_idx(&ctx.table_names, expect_get(table_form, 1)?, "table")?;
        i += 1;
        idx
    } else {
        0
    };
    // Passive segment (bulk-table proposal, task #97): no `(table ...)`
    // clause AND the next field is the `func` keyword or a reftype
    // keyword (`funcref`/`externref`) rather than an offset expression --
    // same "check the shape of the next field" style `build_data`'s own
    // `is_passive` detection uses (task #95). A `(table ...)` clause
    // alone already proves this segment is active (a passive segment has
    // no target table to name at declaration time -- `table.init`
    // supplies one per-call instead).
    let is_passive = !has_table_clause
        && fields.get(i).and_then(|e| e.as_atom()).is_some_and(|s| s == "func" || s == "funcref" || s == "externref");
    let offset_expr = if is_passive {
        Vec::new()
    } else {
        let offset_expr_form = expect_get(fields, i)?;
        let offset_expr = if offset_expr_form.is_keyword_list("offset") {
            let items = offset_expr_form.as_list().unwrap();
            let mut code = Vec::new();
            encode_instr_list(&items[1..], &mut InstrCtx::empty(ctx), &mut code)?;
            code.push(0x0B);
            code
        } else {
            // Shorthand: a single folded instruction with no `(offset ...)` wrapper.
            let mut code = Vec::new();
            encode_instr_list(std::slice::from_ref(offset_expr_form), &mut InstrCtx::empty(ctx), &mut code)?;
            code.push(0x0B);
            code
        };
        i += 1;
        offset_expr
    };
    // Reftype-vs-funcidx-list disambiguation (task #97): a leading
    // `funcref`/`externref` keyword means every following entry is a
    // real encoded expression (`(ref.func N)`/`(ref.null T)`, binary
    // exprs-list mode 5 -- this repo only ever produces `funcref`,
    // matching `parse_element_section`'s own scoped decode). Otherwise,
    // an OPTIONAL leading bare `func` keyword atom (task #96's own
    // established skip, now shared by both active and passive
    // funcidx-list segments) is simply skipped -- it doesn't affect
    // encoding, this repo only supports funcref elem segments either way.
    let use_exprs = if fields.get(i).and_then(|e| e.as_atom()).is_some_and(|s| s == "funcref" || s == "externref") {
        i += 1;
        true
    } else {
        if fields.get(i).is_some_and(|f| matches!(f, SExpr::Atom(s, _) if s == "func")) {
            i += 1;
        }
        false
    };
    // `/security-review` finding: an ACTIVE segment using the exprs-list
    // form (binary modes 4/6) is out of scope -- the real spec's own
    // W17-wasm-bulk-table-ops.md census found no vendored corpus file
    // needs it, and `parse_element_section`'s binary decoder already
    // structurally can't produce this combination (its `match flags`
    // only has arms for 0/1/2/5, with everything else, including 4/6,
    // falling into a hard reject). Without this check, `Element.is_
    // passive: false` combined with a `None` (`ref.null`) entry would
    // silently reach `wasm-module-encoder::encode_element`, whose active-
    // segment branch does `func_index.unwrap_or(0)` -- turning a real
    // `ref.null` into a real reference to function 0 on re-encode, wrong
    // table contents with no error. Reject at parse time instead, same
    // "loud, not silent" discipline as every other out-of-scope shape
    // this parser rejects.
    if !is_passive && use_exprs {
        return Err(WastParseError::UnexpectedToken {
            pos: fields.first().map(|f| f.pos()).unwrap_or(0),
            found: "an active element segment using the exprs-list (funcref/externref) form".to_string(),
            expected: "an active segment to use a plain function-index list instead (exprs-list is only supported for passive segments)",
        });
    }
    let mut function_indices = Vec::new();
    for f in fields.get(i..).unwrap_or(&[]) {
        if use_exprs {
            function_indices.push(resolve_elem_expr_entry(f, ctx)?);
        } else {
            function_indices.push(Some(resolve_idx(&ctx.func_names, f, "func")?));
        }
    }
    ctx.module.elements.push(Element { table_index, offset_expr, function_indices, is_passive });
    Ok(())
}

/// Resolve one exprs-list element-segment entry -- `(ref.func $name_or_
/// idx)` (→ `Some(idx)`) or `(ref.null func)`/`(ref.null extern)` (→
/// `None`, a real null table slot) -- the same two shapes `parse_
/// element_section`'s own `read_elem_expr_entry` decodes from binary
/// (task #97). Any other expression is a clean parse error, matching
/// this crate's scoped-modes discipline.
fn resolve_elem_expr_entry(expr: &SExpr, ctx: &ModuleCtx) -> Result<Option<u32>, WastParseError> {
    let items = expr.as_list().ok_or_else(|| WastParseError::UnexpectedToken {
        pos: expr.pos(),
        found: "atom".to_string(),
        expected: "(ref.func ...) or (ref.null ...)",
    })?;
    match expect_get(items, 0)?.as_atom() {
        Some("ref.func") => {
            let idx = resolve_idx(&ctx.func_names, expect_get(items, 1)?, "func")?;
            Ok(Some(idx))
        }
        Some("ref.null") => {
            // Heap type is validated for shape but otherwise unused --
            // this repo's `WasmValue::Ref(None)` representation doesn't
            // distinguish a null funcref from a null externref (see its
            // own doc comment), so the specific keyword doesn't change
            // anything downstream.
            let _ = parse_ref_null_heap_type(expect_get(items, 1)?)?;
            Ok(None)
        }
        _ => Err(WastParseError::UnexpectedToken {
            pos: expr.pos(),
            found: "list".to_string(),
            expected: "(ref.func ...) or (ref.null ...)",
        }),
    }
}

fn build_data(fields: &[SExpr], ctx: &mut ModuleCtx) -> Result<(), WastParseError> {
    let mut i = 0;
    // Optional leading `$id` (task #95): same leading position as every
    // other named module field -- already registered into `ctx.data_names`
    // by `collect_symbols`'s pass-1 pre-pass, so this is a skip only.
    if fields.first().and_then(|e| e.as_atom()).is_some_and(|s| s.starts_with('$')) {
        i += 1;
    }
    let has_memory_clause = fields.get(i).is_some_and(|e| e.is_keyword_list("memory"));
    let memory_index = if has_memory_clause {
        let memory_form = fields[i].as_list().unwrap();
        let idx = resolve_idx(&ctx.memory_names, expect_get(memory_form, 1)?, "memory")?;
        i += 1;
        idx
    } else {
        0
    };
    // Passive segment (bulk-memory proposal, task #95): declared with no
    // offset expression at all -- `(data $d "bytes")` vs. an active
    // segment's `(data (i32.const 0) "bytes")`/`(data (memory $m) (offset
    // ...) "bytes")`. A `(memory ...)` clause is only ever legal on an
    // active segment (a passive segment has no target memory to name at
    // declaration time -- `memory.init` supplies one per-call instead), so
    // its presence alone already proves this segment is active; otherwise,
    // the next field being the data payload directly (a string literal, or
    // nothing at all) rather than an offset expression is what marks it
    // passive.
    let is_passive = !has_memory_clause && matches!(fields.get(i), Some(SExpr::Str(..)) | None);
    let offset_expr = if is_passive {
        Vec::new()
    } else {
        let offset_expr_form = expect_get(fields, i)?;
        let offset_expr = if offset_expr_form.is_keyword_list("offset") {
            let items = offset_expr_form.as_list().unwrap();
            let mut code = Vec::new();
            encode_instr_list(&items[1..], &mut InstrCtx::empty(ctx), &mut code)?;
            code.push(0x0B);
            code
        } else {
            let mut code = Vec::new();
            encode_instr_list(std::slice::from_ref(offset_expr_form), &mut InstrCtx::empty(ctx), &mut code)?;
            code.push(0x0B);
            code
        };
        i += 1;
        offset_expr
    };
    let mut data = Vec::new();
    for f in fields.get(i..).unwrap_or(&[]) {
        if let SExpr::Str(b, _) = f {
            data.extend_from_slice(b);
        }
    }
    ctx.module.data.push(DataSegment { memory_index, offset_expr, data, is_passive });
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Instruction encoding — folded or flat, identical code path.
// ─────────────────────────────────────────────────────────────────────────

/// Nesting depth ceiling for [`encode_one`]'s own recursion — deliberately
/// much lower than [`crate::sexpr::MAX_NESTING_DEPTH`] (512). That guard
/// bounds `(...)` nesting in the lightweight S-expression tree-builder;
/// `encode_one` and the functions it dispatches to recurse through a
/// heavier call chain (several locals per level, including an owned
/// `Vec<u8>`/`Option<String>` in the `block`/`loop`/`if` path), so 512
/// levels of THIS recursion measurably overflows a real thread's stack
/// well before the counter would ever stop it -- empirically, deeply
/// nested folded arithmetic (`(i32.add (i32.add ...) ...)`) aborted with a
/// real stack overflow around depth ~165-170, and deeply nested
/// `block`/`loop`/`if` bodies around depth ~487, both on a standard
/// `cargo test` worker thread.
///
/// This was originally set to 100. Adding W18's memarg-carrying
/// load/store match arm (task #92/#110) -- itself sitting directly in
/// `encode_flat_instr`, on this exact recursion's hot path -- measurably
/// pushed the real folded-arithmetic overflow point down far enough that
/// 100 stopped being a safe ceiling: `deeply_nested_folded_arithmetic_
/// errors_cleanly_not_stack_overflow` started aborting with a genuine
/// SIGABRT (a real OS stack overflow) instead of the intended clean
/// `TooDeeplyNested` error, even though that test never touches
/// `i32.load` itself -- a debug build sizes a function's stack frame for
/// the union of every match arm's locals, not just the arm actually
/// taken, so growing ANY arm's locals in a function on this recursion
/// path shrinks the real margin for ALL of it. Lowered to 60 and
/// reverified with repeated (8x) full-suite runs showing zero flakiness,
/// restoring real margin under the new, lower measured overflow point --
/// still far above any real hand-written or official-testsuite `.wat`
/// file's actual nesting (a few dozen levels at most, even for a
/// deliberately deep expression or control-flow tower). Because this
/// ceiling is calibrated against an empirically-measured OS stack limit
/// rather than a mathematical bound, ANY future change to a function on
/// this recursion path (`encode_one` / `encode_one_inner` /
/// `encode_flat_instr` / `encode_structured_instr` / `encode_instr_list`)
/// should re-run this file's stack-overflow regression tests a few times
/// before assuming the margin still holds.
const MAX_INSTR_NESTING_DEPTH: usize = 60;

struct InstrCtx<'a> {
    module: &'a mut ModuleCtx,
    locals: HashMap<String, u32>,
    /// Active label scope, innermost last. A named `block $l`/`loop
    /// $l`/`if $l` pushes `Some("$l")`; unnamed pushes `None`. `br`/`br_if`
    /// resolve either a plain depth number or a `$name` by scanning from
    /// the innermost label outward.
    labels: Vec<Option<String>>,
    /// Instruction-encoding recursion depth, incremented/decremented once
    /// per [`encode_one`] call (every single instruction, not just
    /// `block`/`loop`/`if`). See [`InstrCtx::enter_block`].
    depth: usize,
}

impl<'a> InstrCtx<'a> {
    fn empty(module: &'a mut ModuleCtx) -> Self {
        InstrCtx { module, locals: HashMap::new(), labels: Vec::new(), depth: 0 }
    }

    /// Guard against unbounded Rust call-stack recursion through nested
    /// instructions. `sexpr::MAX_NESTING_DEPTH` only bounds S-expression
    /// `(...)` nesting -- but a *folded* operand (`(i32.add (i32.add ...)
    /// ...)`) recurses through this crate's own encoder, not the
    /// S-expression tree-builder, and WAT's **flat** `block`/`loop`/`if`
    /// syntax (`block block block ... end end end`, all sibling atoms in
    /// one unnested list) drives that SAME encoder recursion with no
    /// parentheses at all for the S-expression guard to see in the first
    /// place. `encode_one` is the single point every one of these funnels
    /// through, so this counter, called once per `encode_one` invocation,
    /// bounds all of them uniformly with one mechanism.
    fn enter_block(&mut self, pos: usize) -> Result<(), WastParseError> {
        if self.depth >= MAX_INSTR_NESTING_DEPTH {
            return Err(WastParseError::TooDeeplyNested { pos });
        }
        self.depth += 1;
        Ok(())
    }

    fn exit_block(&mut self) {
        self.depth -= 1;
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
///
/// This is the single point every form of instruction nesting funnels
/// through -- a folded operand (`(i32.add (i32.add ...) ...)`), a folded
/// `block`/`loop`/`if` body, and a flat `block`/`loop`/`if` body (whose
/// own recursion happens one level up, in
/// `encode_stream_structured_instr`, but which reaches back into this
/// function for every element of its body) all recurse by calling this
/// function again. So this is also the ONE place a depth guard needs to
/// live to bound Rust call-stack recursion for ALL of them uniformly --
/// see [`InstrCtx::enter_block`].
fn encode_one(exprs: &[SExpr], i: usize, icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<usize, WastParseError> {
    icx.enter_block(exprs[i].pos())?;
    let result = encode_one_inner(exprs, i, icx, out);
    icx.exit_block();
    result
}

fn encode_one_inner(exprs: &[SExpr], i: usize, icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<usize, WastParseError> {
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
/// The `0xFC` sub-opcode for one of the "non-trapping float-to-int
/// conversions" proposal's 8 `trunc_sat` instructions, or `None` for any
/// other name. Order matches the spec's own table exactly: `i32` before
/// `i64`, `f32` before `f64`, `_s` before `_u`.
fn trunc_sat_sub_opcode(name: &str) -> Option<u8> {
    match name {
        "i32.trunc_sat_f32_s" => Some(0x00),
        "i32.trunc_sat_f32_u" => Some(0x01),
        "i32.trunc_sat_f64_s" => Some(0x02),
        "i32.trunc_sat_f64_u" => Some(0x03),
        "i64.trunc_sat_f32_s" => Some(0x04),
        "i64.trunc_sat_f32_u" => Some(0x05),
        "i64.trunc_sat_f64_s" => Some(0x06),
        "i64.trunc_sat_f64_u" => Some(0x07),
        _ => None,
    }
}

fn encode_stream_instr(
    name: &str,
    following: &[SExpr],
    pos: usize,
    icx: &mut InstrCtx,
    out: &mut Vec<u8>,
) -> Result<usize, WastParseError> {
    // `i32/i64.trunc_sat_f32/f64_s/u` -- the "non-trapping float-to-int
    // conversions" proposal's 8 opcodes, encoded as the two-byte `0xFC
    // <sub-opcode>` prefix form (see this crate's own module doc comment
    // for why `wasm_opcodes` deliberately doesn't model 0xFC opcodes: they
    // don't fit its single-byte `OpcodeInfo` table). Intercepted before the
    // `get_opcode_by_name` lookup below, which would otherwise reject them
    // as unknown. No immediates beyond the sub-opcode byte itself, so this
    // mirrors the default (no-immediate) arm at the bottom of this match.
    if let Some(sub) = trunc_sat_sub_opcode(name) {
        out.push(0xFC);
        out.push(sub);
        return Ok(0);
    }
    // `memory.copy` / `memory.fill` (bulk-memory proposal, task #94): same
    // 0xFC-prefixed shape as trunc_sat, so intercepted here too, before the
    // `get_opcode_by_name` lookup. Neither takes an immediate beyond its
    // memory-index byte(s) -- their three i32 operands are ordinary stack
    // values, not immediates, so this mirrors the default (no-immediate)
    // arm below except for the extra index byte(s). No vendored corpus
    // file uses an explicit non-default memory index for either (bulk-
    // memory predates multi-memory), so -- same deferral as the
    // `"memory.size" | "memory.grow"` arm below -- this always emits
    // index 0x00, not a real lookup.
    if name == "memory.copy" {
        out.push(0xFC);
        out.push(0x0A);
        out.push(0x00); // destination memory index
        out.push(0x00); // source memory index
        return Ok(0);
    }
    if name == "memory.fill" {
        out.push(0xFC);
        out.push(0x0B);
        out.push(0x00); // memory index
        return Ok(0);
    }
    // `memory.init`/`data.drop` (bulk-memory proposal, task #95): same
    // 0xFC-prefixed shape, intercepted here too. Unlike memory.copy/fill
    // above, these DO take a real immediate -- a data-segment index,
    // resolved through `icx.module.data_names` exactly like `global.get`'s
    // `global_names` lookup above -- so, like `local.get`/`global.get`,
    // this consumes ONE following token. `memory.init` also carries a
    // trailing memory-index byte (same single-memory-only deferral as
    // memory.copy/fill: always `0x00`, no explicit-memory-index lookup
    // yet); `data.drop` has no memory concept at all, so no such byte.
    if name == "memory.init" {
        let idx = resolve_idx(&icx.module.data_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "data")?;
        out.push(0xFC);
        out.push(0x08);
        out.extend(wasm_leb128::encode_unsigned(idx as u64));
        out.push(0x00); // memory index
        return Ok(1);
    }
    if name == "data.drop" {
        let idx = resolve_idx(&icx.module.data_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "data")?;
        out.push(0xFC);
        out.push(0x09);
        out.extend(wasm_leb128::encode_unsigned(idx as u64));
        return Ok(1);
    }
    // `table.grow` / `table.size` / `table.fill` (bulk-table proposal,
    // task #98): same 0xFC-prefixed shape as memory.copy/fill above, and
    // the same "no vendored corpus file exercises a non-default table
    // index through this flat/stream form" deferral `"memory.size" |
    // "memory.grow"`'s own arm below documents -- always table 0 here,
    // not a real lookup. The FOLDED form (`encode_flat_instr`) DOES
    // resolve a real leading `$t` table index (table_size.wast/
    // table_fill.wast only ever use folded syntax).
    if name == "table.grow" {
        out.push(0xFC);
        out.push(0x0F);
        out.extend(wasm_leb128::encode_unsigned(0));
        return Ok(0);
    }
    if name == "table.size" {
        out.push(0xFC);
        out.push(0x10);
        out.extend(wasm_leb128::encode_unsigned(0));
        return Ok(0);
    }
    if name == "table.fill" {
        out.push(0xFC);
        out.push(0x11);
        out.extend(wasm_leb128::encode_unsigned(0));
        return Ok(0);
    }
    // `table.init` / `table.copy` / `elem.drop` (bulk-table proposal,
    // task #97): same "no vendored corpus file exercises this flat/
    // stream form" deferral as `table.grow`/`table.size`/`table.fill`
    // above -- `table_init.wast`/`table_copy.wast` only ever use folded
    // syntax (confirmed by direct census, `code/specs/
    // W17-wasm-bulk-table-ops.md`). `table.init` still needs a real
    // elem-segment index (unlike table/memory indices, WHICH segment to
    // read from can't be hardcoded), so this consumes ONE following
    // token even in this deferred form; `table.copy` needs no immediate
    // token at all (both table operands are 0); `elem.drop` needs a real
    // elem-segment index, same shape as `data.drop`.
    if name == "table.init" {
        let idx = resolve_idx(&icx.module.elem_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "elem")?;
        out.push(0xFC);
        out.push(0x0C);
        out.extend(wasm_leb128::encode_unsigned(idx as u64));
        out.push(0x00); // table index
        return Ok(1);
    }
    if name == "table.copy" {
        out.push(0xFC);
        out.push(0x0E);
        out.push(0x00); // destination table index
        out.push(0x00); // source table index
        return Ok(0);
    }
    if name == "elem.drop" {
        let idx = resolve_idx(&icx.module.elem_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "elem")?;
        out.push(0xFC);
        out.push(0x0D);
        out.extend(wasm_leb128::encode_unsigned(idx as u64));
        return Ok(1);
    }
    // `ref.null <heaptype>` / `ref.is_null` (reference-types proposal,
    // WASM17): neither is registered in `wasm_opcodes::OPCODES` (see
    // `code/specs/W08-wasm-funcref-externref.md` -- both already have real
    // runtime/validator handlers from before WASM17, but this crate never
    // had a metadata entry for either), so both must be intercepted here
    // before the `get_opcode_by_name` lookup below, exactly like trunc_sat.
    if name == "ref.null" {
        let heap_byte = parse_ref_null_heap_type(following.first().ok_or(WastParseError::UnexpectedEof)?)?;
        out.push(0xD0);
        out.push(heap_byte);
        return Ok(1);
    }
    if name == "ref.is_null" {
        out.push(0xD1);
        return Ok(0);
    }
    // Atomic memory operations (`0xFE`-prefixed, threads proposal --
    // WASM18): like `ref.null`/`ref.is_null` above, these aren't in
    // `wasm_opcodes::OPCODES` (a two-byte prefix encoding, same reason
    // 0xFB/0xFC aren't either) -- `wasm_opcodes::ATOMIC_OPS` is the real
    // lookup table. `atomic.fence` takes no immediate at all; every other
    // atomic op takes the identical `memarg` (align + offset) shape the
    // 20 existing MVP load/store instructions already use, just under
    // the `0xFE` prefix instead of a bare single byte. See `code/specs/
    // W09-wasm-atomics-plain.md`.
    if name == "atomic.fence" {
        out.push(0xFE);
        out.push(0x03);
        return Ok(0);
    }
    if let Some(atomic_op) = wasm_opcodes::get_atomic_op_by_name(name) {
        let (memarg, consumed) = parse_memarg(following, atomic_op.natural_align.trailing_zeros());
        out.push(0xFE);
        out.push(atomic_op.sub_opcode);
        out.extend(wasm_leb128::encode_unsigned(memarg.0 as u64));
        out.extend(wasm_leb128::encode_unsigned(memarg.1 as u64));
        return Ok(consumed);
    }
    // SIMD (`0xFD`-prefixed, SIMD PR1b-2 -- v128 first slice, see
    // `code/specs/W13-wasm-simd-v128-first-slice.md`): same two-byte-
    // prefix shape as atomics above, but the sub-opcode is a LEB128 `u32`
    // (not a raw byte) -- `wasm_opcodes::SimdOpInfo::sub_opcode`'s own doc
    // comment explains why (`i32x4.add`'s real value, 174, needs the
    // 2-byte LEB128 continuation encoding). `v128.const` carries a 16-byte
    // literal (`<shape> <lane0> ... <laneN-1>`, all 6 shapes supported --
    // see `parse_v128_const`'s doc comment for why, even though this
    // slice's own opcodes only execute the `i32x4` one);
    // `i32x4.extract_lane` carries a single raw (non-LEB128) lane-index
    // byte; every other opcode in this slice takes no immediate at all.
    if let Some(simd_op) = wasm_opcodes::get_simd_op_by_name(name) {
        match simd_op.kind {
            wasm_opcodes::SimdOpKind::Const => {
                let (bytes, consumed) = parse_v128_const(following, pos)?;
                out.push(0xFD);
                out.extend(wasm_leb128::encode_unsigned(simd_op.sub_opcode as u64));
                out.extend(bytes);
                return Ok(consumed);
            }
            wasm_opcodes::SimdOpKind::ExtractLane => {
                let (text, lpos) = literal_text(following.first(), pos)?;
                let lane = parse_lane_index(&text, lpos)?;
                out.push(0xFD);
                out.extend(wasm_leb128::encode_unsigned(simd_op.sub_opcode as u64));
                out.push(lane);
                return Ok(1);
            }
            wasm_opcodes::SimdOpKind::Splat
            | wasm_opcodes::SimdOpKind::Add
            | wasm_opcodes::SimdOpKind::Sub
            | wasm_opcodes::SimdOpKind::Mul
            | wasm_opcodes::SimdOpKind::Neg
            | wasm_opcodes::SimdOpKind::Abs
            | wasm_opcodes::SimdOpKind::MinS
            | wasm_opcodes::SimdOpKind::MinU
            | wasm_opcodes::SimdOpKind::MaxS
            | wasm_opcodes::SimdOpKind::MaxU
            | wasm_opcodes::SimdOpKind::Eq
            | wasm_opcodes::SimdOpKind::Ne
            | wasm_opcodes::SimdOpKind::LtS
            | wasm_opcodes::SimdOpKind::LtU
            | wasm_opcodes::SimdOpKind::GtS
            | wasm_opcodes::SimdOpKind::GtU
            | wasm_opcodes::SimdOpKind::LeS
            | wasm_opcodes::SimdOpKind::LeU
            | wasm_opcodes::SimdOpKind::GeS
            | wasm_opcodes::SimdOpKind::GeU
            | wasm_opcodes::SimdOpKind::EqI16x8
            | wasm_opcodes::SimdOpKind::NeI16x8
            | wasm_opcodes::SimdOpKind::LtSI16x8
            | wasm_opcodes::SimdOpKind::LtUI16x8
            | wasm_opcodes::SimdOpKind::GtSI16x8
            | wasm_opcodes::SimdOpKind::GtUI16x8
            | wasm_opcodes::SimdOpKind::LeSI16x8
            | wasm_opcodes::SimdOpKind::LeUI16x8
            | wasm_opcodes::SimdOpKind::GeSI16x8
            | wasm_opcodes::SimdOpKind::GeUI16x8
            | wasm_opcodes::SimdOpKind::EqI8x16
            | wasm_opcodes::SimdOpKind::NeI8x16
            | wasm_opcodes::SimdOpKind::LtSI8x16
            | wasm_opcodes::SimdOpKind::LtUI8x16
            | wasm_opcodes::SimdOpKind::GtSI8x16
            | wasm_opcodes::SimdOpKind::GtUI8x16
            | wasm_opcodes::SimdOpKind::LeSI8x16
            | wasm_opcodes::SimdOpKind::LeUI8x16
            | wasm_opcodes::SimdOpKind::GeSI8x16
            | wasm_opcodes::SimdOpKind::GeUI8x16
            | wasm_opcodes::SimdOpKind::AbsI8x16
            | wasm_opcodes::SimdOpKind::PopcntI8x16
            | wasm_opcodes::SimdOpKind::MinSI8x16
            | wasm_opcodes::SimdOpKind::MinUI8x16
            | wasm_opcodes::SimdOpKind::MaxSI8x16
            | wasm_opcodes::SimdOpKind::MaxUI8x16
            | wasm_opcodes::SimdOpKind::AvgrUI8x16
            | wasm_opcodes::SimdOpKind::AbsI16x8
            | wasm_opcodes::SimdOpKind::MinSI16x8
            | wasm_opcodes::SimdOpKind::MinUI16x8
            | wasm_opcodes::SimdOpKind::MaxSI16x8
            | wasm_opcodes::SimdOpKind::MaxUI16x8
            | wasm_opcodes::SimdOpKind::AvgrUI16x8
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI8x16S
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI8x16U
            | wasm_opcodes::SimdOpKind::ExtmulLowI8x16S
            | wasm_opcodes::SimdOpKind::ExtmulHighI8x16S
            | wasm_opcodes::SimdOpKind::ExtmulLowI8x16U
            | wasm_opcodes::SimdOpKind::ExtmulHighI8x16U
            | wasm_opcodes::SimdOpKind::Not
            | wasm_opcodes::SimdOpKind::And
            | wasm_opcodes::SimdOpKind::AndNot
            | wasm_opcodes::SimdOpKind::Or
            | wasm_opcodes::SimdOpKind::Xor
            | wasm_opcodes::SimdOpKind::Bitselect
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI16x8S
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI16x8U
            | wasm_opcodes::SimdOpKind::DotI16x8S
            | wasm_opcodes::SimdOpKind::ExtmulLowI16x8S
            | wasm_opcodes::SimdOpKind::ExtmulHighI16x8S
            | wasm_opcodes::SimdOpKind::ExtmulLowI16x8U
            | wasm_opcodes::SimdOpKind::ExtmulHighI16x8U
            | wasm_opcodes::SimdOpKind::AddI8x16
            | wasm_opcodes::SimdOpKind::SubI8x16
            | wasm_opcodes::SimdOpKind::NegI8x16
            | wasm_opcodes::SimdOpKind::AddI16x8
            | wasm_opcodes::SimdOpKind::SubI16x8
            | wasm_opcodes::SimdOpKind::MulI16x8
            | wasm_opcodes::SimdOpKind::NegI16x8 => {
                // All of these take NO immediate beyond the opcode byte
                // itself -- their operands are ordinary stack values,
                // pushed by whatever preceding instruction(s) already ran
                // (folded or flat), same as `Splat`/`Add`/`Eq` above.
                out.push(0xFD);
                out.extend(wasm_leb128::encode_unsigned(simd_op.sub_opcode as u64));
                return Ok(0);
            }
        }
    }
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
        "table.get" | "table.set" => {
            let idx = resolve_idx(&icx.module.table_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "table")?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(1)
        }
        "ref.func" => {
            let idx = resolve_idx(&icx.module.func_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "func")?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(1)
        }
        "call" | "return_call" => {
            let idx = resolve_idx(&icx.module.func_names, following.first().ok_or(WastParseError::UnexpectedEof)?, "func")?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(1)
        }
        "call_indirect" | "return_call_indirect" => {
            // Flat form: `call_indirect (type $t)` -- the type reference
            // (and any `(param...)`/`(result...)`) trails as List elements
            // in the SAME flat sequence, not nested inside this atom.
            // `return_call_indirect` (WASM16) shares this exact shape --
            // same immediates (typeidx, tableidx), only the opcode byte
            // itself differs, already selected via `info.opcode`.
            //
            // Optional leading table token (task #107): the reference-
            // types proposal's explicit-table-index text form is
            // `call_indirect $table (type $t) ...` -- a bare atom BEFORE
            // the `(type ...)` list, not itself a type/param/result
            // keyword list. Defaults to table 0 when absent, matching
            // every other MVP module (real corpus precedent:
            // `table_copy.wast`/`table_init.wast` use this form
            // pervasively in FOLDED syntax; no vendored file needs it in
            // this flat/stream form, but the parse shape is identical).
            let has_table_token = following.first().is_some_and(|a| a.as_atom().is_some());
            let table_idx = if has_table_token {
                resolve_idx(&icx.module.table_names, &following[0], "table")?
            } else {
                0
            };
            let sig_start = if has_table_token { 1 } else { 0 };
            let sig_end = sig_start
                + following[sig_start..]
                    .iter()
                    .position(|a| !is_type_or_param_or_result(a))
                    .unwrap_or(following.len() - sig_start);
            let sig_fields = &following[sig_start..sig_end];
            let type_form = sig_fields.iter().find(|a| a.is_keyword_list("type"));
            let type_idx = if let Some(t) = type_form {
                resolve_idx(&icx.module.type_names, expect_get(t.as_list().unwrap(), 1)?, "type")?
            } else {
                let refs: Vec<&SExpr> = sig_fields.iter().collect();
                let ty = parse_func_signature(&refs)?;
                dedup_type(&mut icx.module.module, ty)
            };
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(type_idx as u64));
            out.extend(wasm_leb128::encode_unsigned(table_idx as u64));
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
            let (memarg, consumed) = parse_memarg(following, 0);
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(memarg.0 as u64));
            out.extend(wasm_leb128::encode_unsigned(memarg.1 as u64));
            Ok(consumed)
        }
        "memory.size" | "memory.grow" => {
            // Multi-memory (W16, task #85): deliberately NOT given the
            // same leading-memidx-token support the folded form (`encode_
            // instr`'s matching arm) got. In THIS flat/stream form, the
            // token right after "memory.grow" is ambiguous -- it's either
            // an explicit memidx OR the start of the FOLLOWING
            // instruction's own keyword (both are plain, unparenthesized
            // atoms here; the folded form's operands are always
            // parenthesized, so `Atom` vs `List` disambiguates cleanly
            // there but not here) -- resolving that needs a real
            // instruction-keyword lookahead check this crate doesn't have
            // yet. No vendored corpus file exercises a non-default memory
            // index through this flat form (only the folded form, per
            // `code/specs/W16-wasm-multi-memory-first-slice.md`), so this
            // stays a named, deferred scope boundary rather than a rushed
            // fix.
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
    let (blocktype_byte, consumed) = encode_blocktype(&following[i..], icx)?;
    i += consumed;

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

/// `table.grow $t? (init)(delta)` (task #98) — folded form. Kept as its
/// own `#[inline(never)]` function rather than inlined into
/// `encode_flat_instr` directly: see that call site's own comment for why
/// (a debug-build stack frame bloat that broke a stack-overflow-guard
/// test). The table-index immediate is OPTIONAL, defaulting to table 0
/// -- same disambiguation `"table.get" | "table.set"` already uses
/// elsewhere in this file (task #96): an explicit index is always a bare
/// `Atom` here, while every real operand is always a nested, parenthesized
/// instruction (`SExpr::List`) in folded form.
#[inline(never)]
fn encode_table_grow_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let (table_idx, operands) = match args.first() {
        Some(expr @ SExpr::Atom(..)) => (resolve_idx(&icx.module.table_names, expr, "table")?, &args[1..]),
        _ => (0, args),
    };
    encode_instr_list(operands, icx, out)?;
    out.push(0xFC);
    out.push(0x0F);
    out.extend(wasm_leb128::encode_unsigned(table_idx as u64));
    Ok(())
}

/// `table.size $t?` (task #98) — folded form. See
/// [`encode_table_grow_flat`]'s doc comment for why this is its own
/// function.
#[inline(never)]
fn encode_table_size_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let table_idx = match args.first() {
        Some(expr @ SExpr::Atom(..)) => resolve_idx(&icx.module.table_names, expr, "table")?,
        _ => 0,
    };
    out.push(0xFC);
    out.push(0x10);
    out.extend(wasm_leb128::encode_unsigned(table_idx as u64));
    Ok(())
}

/// `table.fill $t? (dest)(value)(len)` (task #98) — folded form. See
/// [`encode_table_grow_flat`]'s doc comment for why this is its own
/// function.
#[inline(never)]
fn encode_table_fill_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let (table_idx, operands) = match args.first() {
        Some(expr @ SExpr::Atom(..)) => (resolve_idx(&icx.module.table_names, expr, "table")?, &args[1..]),
        _ => (0, args),
    };
    encode_instr_list(operands, icx, out)?;
    out.push(0xFC);
    out.push(0x11);
    out.extend(wasm_leb128::encode_unsigned(table_idx as u64));
    Ok(())
}

/// `table.init $t $e (dest)(src)(len)` (task #97) — folded form. Kept as
/// its own `#[inline(never)]` function for the same stack-frame-bloat
/// reason `encode_table_grow_flat`'s own doc comment explains (task
/// #98). LEADING immediates, matching `"call" | "return_call"`'s own
/// folded shape -- unlike `table.grow`/`table.size`/`table.fill`'s
/// single OPTIONAL table index, `table.init` always takes TWO required
/// leading tokens: the target table (`$t`) THEN the source elem segment
/// (`$e`), in that TEXT order -- but the real BINARY encoding is
/// `elemidx` FIRST, `tableidx` SECOND (`0xFC 0x0C <elemidx> <tableidx>`,
/// same "segment index, then target index" convention `memory.init`'s
/// own `0xFC 0x08 <dataidx> <memidx>` already establishes), so the two
/// resolved indices are written in the OPPOSITE order from how they're
/// read here.
#[inline(never)]
fn encode_table_init_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let table_idx = resolve_idx(&icx.module.table_names, expect_get(args, 0)?, "table")?;
    let elem_idx = resolve_idx(&icx.module.elem_names, expect_get(args, 1)?, "elem")?;
    encode_instr_list(&args[2..], icx, out)?;
    out.push(0xFC);
    out.push(0x0C);
    out.extend(wasm_leb128::encode_unsigned(elem_idx as u64));
    out.extend(wasm_leb128::encode_unsigned(table_idx as u64));
    Ok(())
}

/// `table.copy $dst $src (dest)(src)(len)` (task #97) — folded form. See
/// [`encode_table_init_flat`]'s doc comment for why this is its own
/// function. Unlike `table.init`, the TEXT and BINARY operand orders
/// match (`0xFC 0x0E <dst_tableidx> <src_tableidx>`, destination first
/// both times) -- no reordering needed.
#[inline(never)]
fn encode_table_copy_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let dst_table_idx = resolve_idx(&icx.module.table_names, expect_get(args, 0)?, "table")?;
    let src_table_idx = resolve_idx(&icx.module.table_names, expect_get(args, 1)?, "table")?;
    encode_instr_list(&args[2..], icx, out)?;
    out.push(0xFC);
    out.push(0x0E);
    out.extend(wasm_leb128::encode_unsigned(dst_table_idx as u64));
    out.extend(wasm_leb128::encode_unsigned(src_table_idx as u64));
    Ok(())
}

/// `elem.drop $e` (task #97) — folded form. See
/// [`encode_table_init_flat`]'s doc comment for why this is its own
/// function. No stack operands at all, same shape `data.drop`'s own
/// folded arm uses (task #95).
#[inline(never)]
fn encode_elem_drop_flat(args: &[SExpr], icx: &mut InstrCtx, out: &mut Vec<u8>) -> Result<(), WastParseError> {
    let elem_idx = resolve_idx(&icx.module.elem_names, expect_get(args, 0)?, "elem")?;
    out.push(0xFC);
    out.push(0x0D);
    out.extend(wasm_leb128::encode_unsigned(elem_idx as u64));
    Ok(())
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
    // See `encode_stream_instr`'s matching comment: trunc_sat's 0xFC-prefixed
    // encoding doesn't fit `wasm_opcodes`' single-byte table, so it's
    // intercepted here too, before the `get_opcode_by_name` lookup. Its one
    // folded operand recurses through `encode_instr_list` exactly like the
    // default (no-immediate) arm below, just emitting two opcode bytes.
    if let Some(sub) = trunc_sat_sub_opcode(name) {
        encode_instr_list(args, icx, out)?;
        out.push(0xFC);
        out.push(sub);
        return Ok(());
    }
    // `memory.copy` / `memory.fill` (task #94): see the matching comment in
    // `encode_stream_instr`. Both take their full operand list folded --
    // `(memory.copy (dest) (src) (size))` / `(memory.fill (dest) (value)
    // (size))` -- recursed in written order via `encode_instr_list`, same
    // as the default (no-immediate) arm below, before the memory-index
    // byte(s).
    if name == "memory.copy" {
        encode_instr_list(args, icx, out)?;
        out.push(0xFC);
        out.push(0x0A);
        out.push(0x00); // destination memory index
        out.push(0x00); // source memory index
        return Ok(());
    }
    if name == "memory.fill" {
        // W18 (task #92/#111): an optional LEADING memidx token --
        // `(memory.fill $mem1 (dest)(value)(len))` -- same shape as
        // `memory.size`/`memory.grow`'s own leading token, reusing the
        // same helper.
        let (memidx, _has_memidx, rest) = resolve_leading_memidx_token(args, icx)?;
        encode_instr_list(rest, icx, out)?;
        out.push(0xFC);
        out.push(0x0B);
        out.extend(wasm_leb128::encode_unsigned(memidx as u64));
        return Ok(());
    }
    // `memory.init`/`data.drop` (task #95): see the matching comment in
    // `encode_stream_instr`. Unlike memory.copy/fill, the data-segment
    // index is a real immediate -- `(memory.init $d (dest)(src)(len))` --
    // and, matching `"call" | "return_call"`'s own folded shape above, it
    // LEADS the operand list rather than trailing it.
    if name == "memory.init" {
        // W18 (task #92/#111): the data-segment index is REQUIRED, but an
        // OPTIONAL memidx token can precede it -- `(memory.init $mem1 $d
        // ...)`. Both are bare identifier/index atoms, so unlike
        // `i32.load`'s leading token (disambiguated from its address
        // operand by shape: an `Atom` vs a nested `List`), these can't be
        // told apart by shape alone -- only by COUNT of leading atoms
        // before the first parenthesized operand: one atom means "just
        // the required dataidx" (memidx defaults to 0, matching
        // `memory_init.wast`'s existing single-memory form from task
        // #95); two means "memidx then dataidx" (matching memory-multi.wast).
        let leading_atoms = args.iter().take_while(|e| matches!(e, SExpr::Atom(..))).count();
        let (memidx, idx_expr, rest): (u32, &SExpr, &[SExpr]) = match leading_atoms {
            0 => return Err(WastParseError::UnexpectedEof),
            1 => (0, &args[0], &args[1..]),
            _ => (resolve_idx(&icx.module.memory_names, &args[0], "memory")?, &args[1], &args[2..]),
        };
        encode_instr_list(rest, icx, out)?;
        let idx = resolve_idx(&icx.module.data_names, idx_expr, "data")?;
        out.push(0xFC);
        out.push(0x08);
        out.extend(wasm_leb128::encode_unsigned(idx as u64));
        out.extend(wasm_leb128::encode_unsigned(memidx as u64));
        return Ok(());
    }
    if name == "data.drop" {
        let idx_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
        let idx = resolve_idx(&icx.module.data_names, idx_expr, "data")?;
        out.push(0xFC);
        out.push(0x09);
        out.extend(wasm_leb128::encode_unsigned(idx as u64));
        return Ok(());
    }
    // `table.grow` / `table.size` / `table.fill` (bulk-table proposal,
    // task #98): same 0xFC-prefixed shape as memory.init/data.drop above
    // (neither is in `wasm_opcodes::OPCODES`, so all three are
    // intercepted here before the `get_opcode_by_name` lookup). Factored
    // out into their own `#[inline(never)]` functions rather than inlined
    // here directly: `encode_flat_instr` is the recursive descent's own
    // funnel (`encode_flat_instr` -> `encode_instr_list` -> `encode_one`
    // -> ...), and a debug-build stack frame sizes to the union of ALL of
    // a function's locals across every branch, not just the one actually
    // taken. Inlining these 3 arms here once measurably pushed ordinary
    // deeply-nested `i32.add` recursion -- which never touches this code
    // at all -- past the real stack limit before `MAX_INSTR_NESTING_
    // DEPTH`'s own guard could fire, breaking `deeply_nested_folded_
    // arithmetic_errors_cleanly_not_stack_overflow`.
    if name == "table.grow" {
        return encode_table_grow_flat(args, icx, out);
    }
    if name == "table.size" {
        return encode_table_size_flat(args, icx, out);
    }
    if name == "table.fill" {
        return encode_table_fill_flat(args, icx, out);
    }
    // `table.init` / `table.copy` / `elem.drop` (bulk-table proposal,
    // task #97): same `#[inline(never)]`-factored-out shape as `table.
    // grow`/`table.size`/`table.fill` above, for the identical stack-
    // frame-bloat reason -- `table_init.wast`/`table_copy.wast` (unlike
    // `table_grow.wast`) exclusively use FOLDED syntax for these three,
    // confirmed by direct census (`code/specs/W17-wasm-bulk-table-ops.
    // md`), so this is where the real work happens.
    if name == "table.init" {
        return encode_table_init_flat(args, icx, out);
    }
    if name == "table.copy" {
        return encode_table_copy_flat(args, icx, out);
    }
    if name == "elem.drop" {
        return encode_elem_drop_flat(args, icx, out);
    }
    // `ref.null <heaptype>` / `ref.is_null` (reference-types proposal,
    // WASM17): see the matching comment in `encode_stream_instr` -- neither
    // is registered in `wasm_opcodes::OPCODES`, so both are intercepted
    // here before the `get_opcode_by_name` lookup, exactly like trunc_sat.
    // Both take zero stack operands, so unlike every other folded
    // instruction below there is no `args[1..]`/`args` sub-expression list
    // to recurse into for `ref.null` (its one arg is the heap-type keyword,
    // not an operand) -- `ref.is_null` DOES take one stack operand (the
    // reference being tested), which is why it still recurses into `args`.
    if name == "ref.null" {
        let heap_byte = parse_ref_null_heap_type(args.first().ok_or(WastParseError::UnexpectedEof)?)?;
        out.push(0xD0);
        out.push(heap_byte);
        return Ok(());
    }
    if name == "ref.is_null" {
        encode_instr_list(args, icx, out)?;
        out.push(0xD1);
        return Ok(());
    }
    // Atomic memory operations (WASM18): see the matching comment in
    // `encode_stream_instr`. `atomic.fence` takes no operands at all
    // (like `ref.null`, nothing to recurse into); every other atomic op
    // follows the SAME "memarg leads, operands trail" folded shape the
    // existing `i32.load`/`i32.store`-family arms below already use.
    if name == "atomic.fence" {
        out.push(0xFE);
        out.push(0x03);
        return Ok(());
    }
    if let Some(atomic_op) = wasm_opcodes::get_atomic_op_by_name(name) {
        let (memarg, operand_start) = parse_memarg(args, atomic_op.natural_align.trailing_zeros());
        encode_instr_list(&args[operand_start..], icx, out)?;
        out.push(0xFE);
        out.push(atomic_op.sub_opcode);
        out.extend(wasm_leb128::encode_unsigned(memarg.0 as u64));
        out.extend(wasm_leb128::encode_unsigned(memarg.1 as u64));
        return Ok(());
    }
    // SIMD: see the matching comment in `encode_stream_instr`. `v128.const`
    // takes no stack operand at all (its whole `args` list is the literal
    // itself, like `ref.null`'s heap-type keyword); `i32x4.extract_lane`'s
    // lane index leads, its one stack operand (the `v128` being read)
    // trails, same "immediate leads, operands trail" convention as
    // `local.get`/`global.get`/etc. above; the remaining ops take no
    // immediate, only operands.
    if let Some(simd_op) = wasm_opcodes::get_simd_op_by_name(name) {
        match simd_op.kind {
            wasm_opcodes::SimdOpKind::Const => {
                let (bytes, _consumed) = parse_v128_const(args, pos)?;
                out.push(0xFD);
                out.extend(wasm_leb128::encode_unsigned(simd_op.sub_opcode as u64));
                out.extend(bytes);
                return Ok(());
            }
            wasm_opcodes::SimdOpKind::ExtractLane => {
                let lane_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
                encode_instr_list(&args[1..], icx, out)?;
                let (text, lpos) = literal_text(Some(lane_expr), pos)?;
                let lane = parse_lane_index(&text, lpos)?;
                out.push(0xFD);
                out.extend(wasm_leb128::encode_unsigned(simd_op.sub_opcode as u64));
                out.push(lane);
                return Ok(());
            }
            wasm_opcodes::SimdOpKind::Splat
            | wasm_opcodes::SimdOpKind::Add
            | wasm_opcodes::SimdOpKind::Sub
            | wasm_opcodes::SimdOpKind::Mul
            | wasm_opcodes::SimdOpKind::Neg
            | wasm_opcodes::SimdOpKind::Abs
            | wasm_opcodes::SimdOpKind::MinS
            | wasm_opcodes::SimdOpKind::MinU
            | wasm_opcodes::SimdOpKind::MaxS
            | wasm_opcodes::SimdOpKind::MaxU
            | wasm_opcodes::SimdOpKind::Eq
            | wasm_opcodes::SimdOpKind::Ne
            | wasm_opcodes::SimdOpKind::LtS
            | wasm_opcodes::SimdOpKind::LtU
            | wasm_opcodes::SimdOpKind::GtS
            | wasm_opcodes::SimdOpKind::GtU
            | wasm_opcodes::SimdOpKind::LeS
            | wasm_opcodes::SimdOpKind::LeU
            | wasm_opcodes::SimdOpKind::GeS
            | wasm_opcodes::SimdOpKind::GeU
            | wasm_opcodes::SimdOpKind::EqI16x8
            | wasm_opcodes::SimdOpKind::NeI16x8
            | wasm_opcodes::SimdOpKind::LtSI16x8
            | wasm_opcodes::SimdOpKind::LtUI16x8
            | wasm_opcodes::SimdOpKind::GtSI16x8
            | wasm_opcodes::SimdOpKind::GtUI16x8
            | wasm_opcodes::SimdOpKind::LeSI16x8
            | wasm_opcodes::SimdOpKind::LeUI16x8
            | wasm_opcodes::SimdOpKind::GeSI16x8
            | wasm_opcodes::SimdOpKind::GeUI16x8
            | wasm_opcodes::SimdOpKind::EqI8x16
            | wasm_opcodes::SimdOpKind::NeI8x16
            | wasm_opcodes::SimdOpKind::LtSI8x16
            | wasm_opcodes::SimdOpKind::LtUI8x16
            | wasm_opcodes::SimdOpKind::GtSI8x16
            | wasm_opcodes::SimdOpKind::GtUI8x16
            | wasm_opcodes::SimdOpKind::LeSI8x16
            | wasm_opcodes::SimdOpKind::LeUI8x16
            | wasm_opcodes::SimdOpKind::GeSI8x16
            | wasm_opcodes::SimdOpKind::GeUI8x16
            | wasm_opcodes::SimdOpKind::AbsI8x16
            | wasm_opcodes::SimdOpKind::PopcntI8x16
            | wasm_opcodes::SimdOpKind::MinSI8x16
            | wasm_opcodes::SimdOpKind::MinUI8x16
            | wasm_opcodes::SimdOpKind::MaxSI8x16
            | wasm_opcodes::SimdOpKind::MaxUI8x16
            | wasm_opcodes::SimdOpKind::AvgrUI8x16
            | wasm_opcodes::SimdOpKind::AbsI16x8
            | wasm_opcodes::SimdOpKind::MinSI16x8
            | wasm_opcodes::SimdOpKind::MinUI16x8
            | wasm_opcodes::SimdOpKind::MaxSI16x8
            | wasm_opcodes::SimdOpKind::MaxUI16x8
            | wasm_opcodes::SimdOpKind::AvgrUI16x8
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI8x16S
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI8x16U
            | wasm_opcodes::SimdOpKind::ExtmulLowI8x16S
            | wasm_opcodes::SimdOpKind::ExtmulHighI8x16S
            | wasm_opcodes::SimdOpKind::ExtmulLowI8x16U
            | wasm_opcodes::SimdOpKind::ExtmulHighI8x16U
            | wasm_opcodes::SimdOpKind::Not
            | wasm_opcodes::SimdOpKind::And
            | wasm_opcodes::SimdOpKind::AndNot
            | wasm_opcodes::SimdOpKind::Or
            | wasm_opcodes::SimdOpKind::Xor
            | wasm_opcodes::SimdOpKind::Bitselect
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI16x8S
            | wasm_opcodes::SimdOpKind::ExtaddPairwiseI16x8U
            | wasm_opcodes::SimdOpKind::DotI16x8S
            | wasm_opcodes::SimdOpKind::ExtmulLowI16x8S
            | wasm_opcodes::SimdOpKind::ExtmulHighI16x8S
            | wasm_opcodes::SimdOpKind::ExtmulLowI16x8U
            | wasm_opcodes::SimdOpKind::ExtmulHighI16x8U
            | wasm_opcodes::SimdOpKind::AddI8x16
            | wasm_opcodes::SimdOpKind::SubI8x16
            | wasm_opcodes::SimdOpKind::NegI8x16
            | wasm_opcodes::SimdOpKind::AddI16x8
            | wasm_opcodes::SimdOpKind::SubI16x8
            | wasm_opcodes::SimdOpKind::MulI16x8
            | wasm_opcodes::SimdOpKind::NegI16x8 => {
                encode_instr_list(args, icx, out)?;
                out.push(0xFD);
                out.extend(wasm_leb128::encode_unsigned(simd_op.sub_opcode as u64));
                return Ok(());
            }
        }
    }
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
        "table.get" | "table.set" => {
            // Task #96: the table-index immediate is OPTIONAL, defaulting
            // to table 0 when omitted -- `(table.get (local.get $i))` is
            // just as legal as `(table.get $t2 (local.get $i))`. Same
            // disambiguation as W16's `memory.size`/`memory.grow`: an
            // explicit index is always a bare `Atom` here, while every
            // operand (the index-to-look-up for `table.get`, or the
            // index+value pair for `table.set`) is always a nested,
            // parenthesized instruction (`SExpr::List`) in folded form.
            let (idx, operands) = match args.first() {
                Some(expr @ SExpr::Atom(..)) => (resolve_idx(&icx.module.table_names, expr, "table")?, &args[1..]),
                _ => (0, args),
            };
            encode_instr_list(operands, icx, out)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(())
        }
        "ref.func" => {
            let idx_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
            encode_instr_list(&args[1..], icx, out)?;
            out.push(info.opcode);
            let idx = resolve_idx(&icx.module.func_names, idx_expr, "func")?;
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(())
        }
        "call" | "return_call" => {
            let idx_expr = args.first().ok_or(WastParseError::UnexpectedEof)?;
            encode_instr_list(&args[1..], icx, out)?;
            out.push(info.opcode);
            let idx = resolve_idx(&icx.module.func_names, idx_expr, "func")?;
            out.extend(wasm_leb128::encode_unsigned(idx as u64));
            Ok(())
        }
        "call_indirect" | "return_call_indirect" => {
            // `(call_indirect (type $t) (param...) (result...) operand-exprs...)`
            // `return_call_indirect` (WASM16) shares this exact shape --
            // same immediates, only the opcode byte differs.
            //
            // Optional leading table token (task #107): a bare atom
            // BEFORE the `(type ...)`/`(param...)`/`(result...)` fields
            // names an explicit non-default table -- pervasive in the
            // real corpus (`table_init.wast`/`table_copy.wast` use
            // `(call_indirect $t0 (type 0) ...)` throughout). Same
            // detection shape as the flat-form arm above: a bare atom
            // isn't itself a type/param/result keyword list, so without
            // this check the type/param/result scan below would
            // misidentify the table token as the start of the operand-
            // expression list. Defaults to table 0 when absent.
            let has_table_token = args.first().is_some_and(|a| a.as_atom().is_some());
            let table_idx = if has_table_token {
                resolve_idx(&icx.module.table_names, &args[0], "table")?
            } else {
                0
            };
            let sig_start = if has_table_token { 1 } else { 0 };
            let type_form = args[sig_start..].iter().find(|a| a.is_keyword_list("type"));
            let operand_start = sig_start
                + args[sig_start..]
                    .iter()
                    .position(|a| !is_type_or_param_or_result(a))
                    .unwrap_or(args.len() - sig_start);
            encode_instr_list(&args[operand_start..], icx, out)?;
            out.push(info.opcode);
            let type_idx = if let Some(t) = type_form {
                resolve_idx(&icx.module.type_names, expect_get(t.as_list().unwrap(), 1)?, "type")?
            } else {
                let sig_fields: Vec<&SExpr> = args[sig_start..operand_start].iter().collect();
                let ty = parse_func_signature(&sig_fields)?;
                dedup_type(&mut icx.module.module, ty)
            };
            out.extend(wasm_leb128::encode_unsigned(type_idx as u64));
            out.extend(wasm_leb128::encode_unsigned(table_idx as u64));
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
            // Opposite of every other instruction's own "immediates trail
            // operands" split: `br_table`'s folded grammar lists all label
            // targets FIRST (bare atoms), then an OPTIONAL folded index
            // operand LAST -- `(br_table $a $b (i32.const 0))`, not
            // `(br_table (i32.const 0) $a $b)`.
            let label_end = args.iter().position(|a| !is_label_atom(a)).unwrap_or(args.len());
            let labels = &args[..label_end];
            // br_table takes >=1 label per spec -- `(br_table)` or
            // `(br_table (i32.const 0))` (zero leading label atoms) must
            // error cleanly, not underflow `labels.len() - 1` below.
            if labels.is_empty() {
                return Err(WastParseError::UnexpectedEof);
            }
            encode_instr_list(&args[label_end..], icx, out)?;
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
            // Multi-memory (W18, task #92/#110): an optional LEADING
            // memory-index token before the offset=/align= attributes --
            // `(i32.load $mem1 (i32.const 1))`. Resolving it is pulled
            // into its own function (`resolve_leading_memidx_token`)
            // rather than inlined here on purpose: this match arm lives
            // inside `encode_flat_instr`, which sits on the hot
            // recursive path every folded instruction funnels through
            // (`encode_one` -> `encode_flat_instr` -> `encode_instr_list`
            // -> `encode_one` ...), guarded only by
            // `MAX_INSTR_NESTING_DEPTH`'s software counter. In a debug
            // build a function's stack frame is sized for the union of
            // every match arm's locals, so adding locals inline here
            // once measurably grew `encode_flat_instr`'s own per-call
            // frame enough to overflow the REAL OS stack before the
            // depth counter ever reached its limit -- confirmed by
            // `deeply_nested_folded_arithmetic_errors_cleanly_not_
            // stack_overflow` (which never even touches `i32.load`)
            // failing with a genuine SIGABRT once these locals lived
            // directly in this arm. Keeping them in a separate,
            // non-hot-path function keeps this arm's cost off every
            // other instruction's recursion budget.
            let (memidx, has_memidx, rest) = resolve_leading_memidx_token(args, icx)?;
            let (memarg, operand_start) = parse_memarg(rest, 0);
            encode_instr_list(&rest[operand_start..], icx, out)?;
            out.push(info.opcode);
            const MULTI_MEMORY_FLAG: u32 = 0x40;
            let flag_byte = memarg.0 | if has_memidx { MULTI_MEMORY_FLAG } else { 0 };
            out.extend(wasm_leb128::encode_unsigned(flag_byte as u64));
            out.extend(wasm_leb128::encode_unsigned(memarg.1 as u64));
            if has_memidx {
                out.extend(wasm_leb128::encode_unsigned(memidx as u64));
            }
            Ok(())
        }
        "memory.size" | "memory.grow" => {
            // Multi-memory (W16, task #85): an optional LEADING memory-
            // index token -- `(memory.size $mem1)`, `(memory.grow $mem1
            // (i32.const 1))` -- distinguished from `memory.grow`'s value
            // operand (always a nested, parenthesized instruction in
            // folded form, so always an `SExpr::List`) by checking
            // whether the first arg is an `Atom` at all. Same
            // leading-immediate-then-operands shape as `"call" |
            // "return_call"` above, not a new pattern.
            let (memidx, operands) = match args.first() {
                Some(expr @ SExpr::Atom(..)) => (resolve_idx(&icx.module.memory_names, expr, "memory")?, &args[1..]),
                _ => (0, args),
            };
            encode_instr_list(operands, icx, out)?;
            out.push(info.opcode);
            out.extend(wasm_leb128::encode_unsigned(memidx as u64));
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

/// Parse and encode a `block`/`loop`/`if` header's **blocktype** from the
/// items starting at `items[0]` (immediately after any optional `$label`,
/// which the caller has already consumed) — WASM06.
///
/// A blocktype is one of, in the order checked:
/// - No `(type ...)`/`(param ...)`/`(result ...)` present at all: the
///   empty-blocktype byte `0x40`.
/// - An explicit `(type $t)` reference: resolved via `type_names`, same
///   lookup `call_indirect`'s own explicit `(type $t)` form already uses.
/// - Otherwise, an inline `(param ...)*(result ...)*` signature, parsed
///   the same way `call_indirect`'s inline signature already is. If it
///   has no params and at most one result — the overwhelming common
///   case — encoded as the single value-type byte shorthand (or `0x40`
///   for zero results), matching the WASM 1.0 encoding exactly. Otherwise
///   (any params, or more than one result) the binary format has no
///   "anonymous inline blocktype" encoding at all — it MUST be a real
///   type-section index — so the signature is `dedup_type`'d the same way
///   an anonymous `func`/`call_indirect` signature already is, and the
///   index is emitted as a signed LEB128 (distinguishable from the
///   negative-valued single-byte shorthands by construction, since a real
///   type index is always non-negative).
///
/// Returns the encoded blocktype bytes and how many leading elements of
/// `items` were consumed — advancing the caller's cursor correctly past
/// whatever was consumed here is the actual bug this function fixes: the
/// old code left an unconsumed `(param ...)` for the body encoder to
/// mis-read as an instruction named `"param"`.
fn encode_blocktype(items: &[SExpr], icx: &mut InstrCtx) -> Result<(Vec<u8>, usize), WastParseError> {
    let sig_end = items.iter().position(|a| !is_type_or_param_or_result(a)).unwrap_or(items.len());
    let sig_fields = &items[..sig_end];
    if sig_fields.is_empty() {
        return Ok((vec![0x40], 0));
    }
    if let Some(t) = sig_fields.iter().find(|a| a.is_keyword_list("type")) {
        let type_idx = resolve_idx(&icx.module.type_names, expect_get(t.as_list().unwrap(), 1)?, "type")?;
        return Ok((wasm_leb128::encode_signed(type_idx as i64), sig_end));
    }
    let refs: Vec<&SExpr> = sig_fields.iter().collect();
    let ty = parse_func_signature(&refs)?;
    if ty.params.is_empty() && ty.results.len() <= 1 {
        let byte = ty.results.first().map(|t| t.byte_tag().unwrap()).unwrap_or(0x40);
        return Ok((vec![byte], sig_end));
    }
    let type_idx = dedup_type(&mut icx.module.module, ty);
    Ok((wasm_leb128::encode_signed(type_idx as i64), sig_end))
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

/// `i32x4.extract_lane`'s lane-index immediate: a single raw (non-LEB128)
/// byte, 0-3 for this slice's only lane-typed shape. Real corpus files
/// only ever write a small plain-decimal literal here (`(i32x4.extract_lane
/// 0 ...)`), so a plain `u8` parse (not the full `parse_i32`-style
/// signed/hex/underscore grammar numeric literals elsewhere in this crate
/// support) is enough; out-of-range lane indices (>3) are rejected at
/// execution time by `wasm-execution`'s own bounds check, not here.
fn parse_lane_index(text: &str, pos: usize) -> Result<u8, WastParseError> {
    text.parse::<u8>()
        .map_err(|_| WastParseError::InvalidNumericLiteral { pos, text: text.to_string() })
}

/// `v128.const <shape> <lane0> ... <laneN-1>` -- the SIMD proposal's
/// literal syntax. `<shape>` picks both the lane width and count (16×i8,
/// 8×i16, 4×i32, 2×i64, 4×f32, or 2×f64); every shape packs down to the
/// same 16 raw bytes regardless of how it's spelled in source.
///
/// This slice's own executable opcodes (SIMD PR1a) only run `i32x4`-shaped
/// ops, but the *literal syntax* itself is shape-agnostic once encoded, so
/// all 6 shapes are supported here even though nothing in this slice
/// computes with the other 5. That's not scope creep: real corpus files
/// mix shapes even when testing a single op -- e.g. `simd_splat.wast`
/// exercises `i8x16.splat`/`f64x2.splat`/etc. alongside `i32x4.splat`, each
/// compared against a `v128.const` of its OWN shape -- so parsing only
/// `i32x4` would leave those files unparseable as a whole (one bad literal
/// fails the whole module's parse, per this crate's parser design), not
/// just missing coverage for the unimplemented shapes' own instructions.
///
/// Returns `(bytes, consumed)`, where `consumed` counts the shape keyword
/// itself plus every lane literal (`1 + lane_count`) -- used by the stream
/// (bare-atom) caller to advance its cursor; the folded caller already
/// knows its whole `args` list is exactly this literal, so it ignores it.
///
/// `pub(crate)`, not private: `script.rs`'s `parse_const_value` reuses this
/// directly for `assert_return`/`invoke`'s own `(v128.const ...)` argument/
/// expected-value syntax (SIMD PR1b-3) -- same grammar, no reason to
/// duplicate the shape/lane-width table there.
pub(crate) fn parse_v128_const(operands: &[SExpr], pos: usize) -> Result<([u8; 16], usize), WastParseError> {
    let (shape, shape_pos) = literal_text(operands.first(), pos)?;
    let lane_count: usize = match shape.as_str() {
        "i8x16" => 16,
        "i16x8" => 8,
        "i32x4" => 4,
        "i64x2" => 2,
        "f32x4" => 4,
        "f64x2" => 2,
        _ => {
            return Err(WastParseError::UnexpectedToken {
                pos: shape_pos,
                found: shape,
                expected: "a v128 shape (i8x16/i16x8/i32x4/i64x2/f32x4/f64x2)",
            })
        }
    };
    let lanes = operands.get(1..).unwrap_or(&[]);
    if lanes.len() < lane_count {
        return Err(WastParseError::UnexpectedEof);
    }
    let mut bytes = [0u8; 16];
    let mut offset = 0usize;
    for lane_expr in &lanes[..lane_count] {
        let (text, lpos) = literal_text(Some(lane_expr), pos)?;
        let lane_bytes: Vec<u8> = match shape.as_str() {
            "i8x16" => vec![parse_i8(&text, lpos)? as u8],
            "i16x8" => parse_i16(&text, lpos)?.to_le_bytes().to_vec(),
            "i32x4" => parse_i32(&text, lpos)?.to_le_bytes().to_vec(),
            "i64x2" => parse_i64(&text, lpos)?.to_le_bytes().to_vec(),
            "f32x4" => parse_f32_bits(&text, lpos)?.to_le_bytes().to_vec(),
            "f64x2" => parse_f64_bits(&text, lpos)?.to_le_bytes().to_vec(),
            _ => unreachable!("shape already validated above"),
        };
        bytes[offset..offset + lane_bytes.len()].copy_from_slice(&lane_bytes);
        offset += lane_bytes.len();
    }
    Ok((bytes, 1 + lane_count))
}

/// Split an optional LEADING memory-index token off a memarg-carrying
/// load/store's folded argument list -- `(i32.load $mem1 (i32.const 1))`
/// -- the same shape as `memory.size`/`memory.grow`'s own leading token
/// (see that arm in `encode_flat_instr`): only a bare `Atom` that isn't
/// itself an `offset=`/`align=` attribute counts (the address operand is
/// always a nested, parenthesized instruction in folded form, so always
/// an `SExpr::List`). Returns `(memidx, was_a_token_present, remaining_args)`.
///
/// Deliberately its own function, not inlined into the match arm that
/// calls it: that arm lives in `encode_flat_instr`, on the hot recursive
/// path every folded instruction funnels through (`encode_one` ->
/// `encode_flat_instr` -> `encode_instr_list` -> `encode_one` ...),
/// guarded only by `MAX_INSTR_NESTING_DEPTH`'s software depth counter. In
/// a debug build a function's stack frame is sized for the union of every
/// match arm's own locals, so adding these locals directly inline once
/// measurably grew `encode_flat_instr`'s per-call frame enough to
/// overflow the real OS stack before the depth counter ever tripped --
/// caught by `deeply_nested_folded_arithmetic_errors_cleanly_not_stack_
/// overflow` (which never even touches `i32.load`) failing with a
/// genuine SIGABRT. Keeping this logic in a separate function keeps its
/// cost off every other instruction's recursion budget.
fn resolve_leading_memidx_token<'e>(args: &'e [SExpr], icx: &InstrCtx) -> Result<(u32, bool, &'e [SExpr]), WastParseError> {
    match args.first() {
        Some(expr @ SExpr::Atom(s, _)) if !s.starts_with("offset=") && !s.starts_with("align=") => {
            Ok((resolve_idx(&icx.module.memory_names, expr, "memory")?, true, &args[1..]))
        }
        _ => Ok((0, false, args)),
    }
}

/// `align=N` / `offset=N` attributes on a load/store, in either order,
/// both optional (default align is the natural alignment, encoded here as
/// 0 -- real alignment *hints* don't affect semantics, only performance,
/// so defaulting to the loosest hint is always safe). Returns
/// `((align_log2, offset), first_non_attribute_index)`.
/// Parse a `memarg` (`align=N offset=N`, either/both/neither, in either
/// order... actually always `offset=` then `align=` per real WAT
/// convention, but this scans generically). `default_align_log2` is what
/// `align_log2` becomes when NO `align=` token is present at all --
/// plain loads/stores pass `0` (their existing convention: a missing
/// align is just a weaker hint, never semantically required to match
/// anything). Atomic ops (WASM18) pass their own natural alignment's
/// log2 instead, since the real corpus's atomic instructions are
/// typically written WITHOUT an explicit `align=` at all, relying on
/// "defaults to natural" -- and unlike plain ops, atomic ops REQUIRE
/// exact natural alignment, so silently defaulting to `0` (1-byte) would
/// make every real atomic op without an explicit `align=` fail
/// validation. An align=N that a real test case DOES write out
/// explicitly (e.g. deliberately mis-aligned, to test rejection) always
/// takes precedence over this default either way.
fn parse_memarg(args: &[SExpr], default_align_log2: u32) -> ((u32, u32), usize) {
    let mut align_log2 = default_align_log2;
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
    // Blocktype: `(type $t)` / `(param ...)*(result ...)*` / a single
    // `(result T)` / nothing at all -- see `encode_blocktype`'s own doc
    // comment for the full resolution order (WASM06).
    let (blocktype_byte, consumed) = encode_blocktype(&args[i..], icx)?;
    i += consumed;

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
    fn func_inline_import_shorthand_desugars_to_a_real_import() {
        // `(func $f (import "m" "n") (type $t))` means EXACTLY
        // `(import "m" "n" (func $f (type $t)))` -- matches the official
        // testsuite's func_ptrs.wast shape ($print).
        let m = parse_module(
            r#"(module
                 (type $t (func (param i32)))
                 (func $print (import "spectest" "print_i32") (type $t)))"#,
        )
        .unwrap();
        assert_eq!(m.functions.len(), 0, "the import has no entry in the real-funcs-only function section");
        assert_eq!(m.code.len(), 0);
        assert_eq!(m.imports.len(), 1);
        assert_eq!(m.imports[0].module_name, "spectest");
        assert_eq!(m.imports[0].name, "print_i32");
        assert!(matches!(m.imports[0].kind, ExternalKind::Function));
        assert!(matches!(m.imports[0].type_info, ImportTypeInfo::Function(0)));
    }

    #[test]
    fn func_import_followed_by_a_real_func_resolves_both_correctly() {
        // The regression this crate's own conformance run against func_ptrs.wast
        // caught: `ctx.module.functions`/`code` used to be indexed by the
        // COMBINED func-space index (imports counted in) even though those
        // arrays only ever hold REAL functions -- a module with one func
        // import followed by any real func panicked with an out-of-bounds
        // index the first time this combination was ever exercised.
        let m = parse_module(
            r#"(module
                 (func $print (import "spectest" "print_i32") (param i32))
                 (func $real (export "real") (result i32) (i32.const 7))
                 (func (export "call_print") (call $print (i32.const 1))))"#,
        )
        .unwrap();
        assert_eq!(m.imports.len(), 1);
        assert_eq!(m.functions.len(), 2, "one real-funcs-only entry per non-import func");
        assert_eq!(m.code.len(), 2);
        assert_eq!(code_of(&m, 0), &[0x41, 0x07, 0x0B]); // $real: i32.const 7; end
        // `call $print` must resolve to func-space index 0 (the import,
        // the lowest index in its space) -- 0x10 = call, 0x00 = funcidx 0.
        assert_eq!(code_of(&m, 1), &[0x41, 0x01, 0x10, 0x00, 0x0B]);
        let real_export = m.exports.iter().find(|e| e.name == "real").unwrap();
        assert_eq!(real_export.index, 1, "the real func's own func-space index (after the 1 import)");
    }

    #[test]
    fn global_inline_import_shorthand_followed_by_a_real_global_resolves_both_correctly() {
        // Matches the official testsuite's global.wast shape: unnamed
        // inline-import globals (no `$name`) followed by real globals.
        let m = parse_module(
            r#"(module
                 (global (import "spectest" "global_i32") i32)
                 (global $g (export "g") i32 (i32.const 42)))"#,
        )
        .unwrap();
        assert_eq!(m.imports.len(), 1);
        assert!(matches!(m.imports[0].type_info, ImportTypeInfo::Global(GlobalType { value_type: ValueType::I32, mutable: false })));
        assert_eq!(m.globals.len(), 1, "one real-globals-only entry, not one per import too");
        assert_eq!(m.globals[0].init_expr, vec![0x41, 0x2A, 0x0B]); // i32.const 42; end
        let export = m.exports.iter().find(|e| e.name == "g").unwrap();
        assert_eq!(export.index, 1, "the real global's own global-space index (after the 1 import)");
    }

    #[test]
    fn named_global_inline_import_shorthand_resolves_its_type_and_index() {
        // WASM19: `(global $g0 (import "m" "n") i32)` -- the NAMED form,
        // matching the real corpus's global.wast. Only the unnamed form
        // (no `$name` between `global` and `(import ...)`) worked before
        // this fix: `build_import_shell`'s "global" arm read `desc.get(1)`
        // unconditionally as the value type, but when a `$name` is present
        // it desugars into that very slot instead (`desc` = `[global,
        // $name, type]`, not `[global, type]`), so `$g0` itself was
        // mis-parsed as a value type ("expected a value type, found
        // \"$g0\"").
        let m = parse_module(
            r#"(module
                 (global $g0 (import "m" "n") i32)
                 (func (export "get") (result i32) (global.get $g0)))"#,
        )
        .unwrap();
        assert_eq!(m.imports.len(), 1);
        assert!(matches!(m.imports[0].type_info, ImportTypeInfo::Global(GlobalType { value_type: ValueType::I32, mutable: false })));
        // `$g0` must resolve to the import's own index (0), not fail as an
        // unknown identifier.
        assert_eq!(code_of(&m, 0), &[0x23, 0x00, 0x0B]); // global.get 0; end
    }

    #[test]
    fn i32_load_without_a_leading_memidx_token_keeps_the_plain_mvp_encoding() {
        // Regression guard: a plain `i32.load` with no memory reference
        // must still encode exactly as before this feature -- align byte
        // WITHOUT the multi-memory flag bit (0x40) set, no trailing
        // memidx byte at all.
        let m = parse_module(r#"(module (memory 1) (func (i32.load (i32.const 0))))"#).unwrap();
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x28, 0x00, 0x00, 0x0B]); // i32.const 0; i32.load align=0 offset=0; end
    }

    #[test]
    fn i32_load_with_a_leading_memidx_token_encodes_the_multi_memory_flag_and_index() {
        // W18 (task #92/#110): `(i32.load $mem1 (i32.const 0))` must
        // resolve `$mem1` against the module's memory index space and
        // encode it using the real multi-memory binary form: the
        // align byte gets its top bit (0x40) set, followed by offset,
        // followed by a trailing memidx LEB128 -- not silently dropped
        // to memory 0 like before this fix.
        let m = parse_module(
            r#"(module
                 (memory $mem0 1)
                 (memory $mem1 1)
                 (func (i32.load $mem1 (i32.const 0))))"#,
        )
        .unwrap();
        // i32.const 0; i32.load align=(0x40|0) offset=0 memidx=1; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x28, 0x40, 0x00, 0x01, 0x0B]);
    }

    #[test]
    fn i32_store_with_offset_and_a_leading_memidx_token_orders_flag_offset_then_memidx() {
        // Same feature, but proves the leading memidx token composes
        // correctly with an explicit `offset=` attribute that follows
        // it -- `parse_memarg` must see the REMAINDER after the memidx
        // token is stripped, not the raw arg list (which would
        // misinterpret `$mem1` itself as an out-of-place operand).
        let m = parse_module(
            r#"(module
                 (memory $mem0 1)
                 (memory $mem1 1)
                 (func (i32.store $mem1 offset=8 (i32.const 0) (i32.const 1))))"#,
        )
        .unwrap();
        // i32.const 0; i32.const 1; i32.store align=(0x40|0) offset=8 memidx=1; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x41, 0x01, 0x36, 0x40, 0x08, 0x01, 0x0B]);
    }

    #[test]
    fn memory_fill_without_a_leading_memidx_token_keeps_the_plain_mvp_encoding() {
        // Regression guard: `memory.fill`'s pre-W18 single-memory form
        // (task #94) must still encode its memory-index byte as 0.
        let m = parse_module(r#"(module (memory 1) (func (memory.fill (i32.const 0) (i32.const 7) (i32.const 4))))"#).unwrap();
        // i32.const 0; i32.const 7; i32.const 4; memory.fill memidx=0; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x41, 0x07, 0x41, 0x04, 0xFC, 0x0B, 0x00, 0x0B]);
    }

    #[test]
    fn memory_fill_with_a_leading_memidx_token_encodes_the_real_index() {
        // W18 (task #92/#111): `(memory.fill $mem1 ...)` must resolve
        // `$mem1` and encode it as the real trailing memidx byte, matching
        // `memory-multi.wast`'s own usage.
        let m = parse_module(
            r#"(module
                 (memory $mem0 1)
                 (memory $mem1 1)
                 (func (memory.fill $mem1 (i32.const 0) (i32.const 7) (i32.const 4))))"#,
        )
        .unwrap();
        // i32.const 0; i32.const 7; i32.const 4; memory.fill memidx=1; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x41, 0x07, 0x41, 0x04, 0xFC, 0x0B, 0x01, 0x0B]);
    }

    #[test]
    fn memory_init_without_a_leading_memidx_token_keeps_the_plain_mvp_encoding() {
        // Regression guard: `memory.init`'s pre-W18 single-memory form
        // (task #95) -- ONE leading atom (just the dataidx) -- must still
        // encode its trailing memory-index byte as 0.
        let m = parse_module(
            r#"(module (memory 1) (data $d "hi")
                 (func (memory.init $d (i32.const 0) (i32.const 0) (i32.const 2))))"#,
        )
        .unwrap();
        // i32.const 0; i32.const 0; i32.const 2; memory.init dataidx=0 memidx=0; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x41, 0x00, 0x41, 0x02, 0xFC, 0x08, 0x00, 0x00, 0x0B]);
    }

    #[test]
    fn memory_init_with_a_leading_memidx_token_encodes_memidx_then_dataidx() {
        // W18 (task #92/#111): `(memory.init $mem1 $d ...)` -- TWO
        // leading atoms -- must resolve `$mem1` against the memory index
        // space and `$d` against the data-segment index space (in that
        // order), matching `memory-multi.wast`'s own usage. Binary order
        // is dataidx THEN memidx (not the same order they appear in text).
        let m = parse_module(
            r#"(module
                 (memory $mem0 1)
                 (memory $mem1 1)
                 (data $d "hi")
                 (func (memory.init $mem1 $d (i32.const 0) (i32.const 0) (i32.const 2))))"#,
        )
        .unwrap();
        // i32.const 0; i32.const 0; i32.const 2; memory.init dataidx=0 memidx=1; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0x41, 0x00, 0x41, 0x02, 0xFC, 0x08, 0x00, 0x01, 0x0B]);
    }

    #[test]
    fn named_mutable_global_inline_import_shorthand_resolves_correctly() {
        let m = parse_module(
            r#"(module (global $mg (import "m" "n") (mut f64)))"#,
        )
        .unwrap();
        assert!(matches!(
            m.imports[0].type_info,
            ImportTypeInfo::Global(GlobalType { value_type: ValueType::F64, mutable: true })
        ));
    }

    #[test]
    fn table_import_followed_by_a_real_table_resolves_both_correctly() {
        // Same storage-vs-space-index split as func/global, exercised for
        // table since `build_table_limits_and_elements` shares the identical
        // code path this fix touches.
        let m = parse_module(
            r#"(module
                 (table (import "spectest" "table") 1 funcref)
                 (table $t 2 funcref))"#,
        )
        .unwrap();
        assert_eq!(m.imports.len(), 1);
        assert_eq!(m.tables.len(), 1, "one real-tables-only entry");
        assert_eq!(m.tables[0].limits.min, 2);
    }

    #[test]
    fn abbreviated_module_form_with_no_wrapper_parses_its_bare_fields() {
        // The WAT text format allows the enclosing `(module ...)` to be
        // omitted entirely when the fields are the ONLY top-level content --
        // exactly how the official spec testsuite's comments.wast/block.wast
        // write their `(module quote "...")` directive's concatenated text
        // (this is what `parse_module` is re-parsing that content as).
        let m = parse_module(r#"(func (export "f") (result i32) (i32.const 42))"#).unwrap();
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.exports[0].name, "f");
    }

    #[test]
    fn abbreviated_module_form_with_multiple_top_level_funcs() {
        let m = parse_module(
            r#"(func (export "a") (result i32) (i32.const 1))(func (export "b") (result i32) (i32.const 2))"#,
        )
        .unwrap();
        assert_eq!(m.functions.len(), 2);
        assert_eq!(m.exports.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
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

    // ── Multi-table (task #96) ──────────────────────────────────────────

    /// Real bug found vendoring table_get.wast: a declared table's own
    /// `funcref`/`externref` reftype keyword was silently discarded
    /// entirely -- `parse_limits`'s digit-only scan correctly stops
    /// before it, but nothing ever read it afterward, leaving every
    /// table's `element_type` at its hardcoded FUNCREF construction-time
    /// default regardless of what the source actually declared.
    #[test]
    fn declared_table_reads_its_own_reftype_not_a_hardcoded_funcref_default() {
        let m = parse_module("(module (table $t2 2 externref) (table $t3 3 funcref))").unwrap();
        assert_eq!(m.tables[0].element_type, wasm_types::EXTERNREF);
        assert_eq!(m.tables[1].element_type, wasm_types::FUNCREF);
    }

    /// Same class of bug, for an imported table: `(import "m" "n" (table
    /// N reftype))`'s real limits/reftype were discarded in favor of an
    /// unconditional `{element_type: FUNCREF, limits: {min: 0, max: None}}`
    /// placeholder that was never fixed up afterward (unlike function
    /// imports, which ARE fixed up in a documented "pass 2").
    #[test]
    fn imported_table_reads_its_own_limits_and_reftype_not_a_zero_funcref_placeholder() {
        let m = parse_module(r#"(module (import "m" "t" (table 3 10 externref)))"#).unwrap();
        let ImportTypeInfo::Table(tt) = &m.imports[0].type_info else {
            panic!("expected a table import, got {:?}", m.imports[0].type_info);
        };
        assert_eq!(tt.element_type, wasm_types::EXTERNREF);
        assert_eq!(tt.limits, Limits { min: 3, max: Some(10) });
    }

    /// Task #99: table (and memory) limits accept hex literals, not just
    /// decimal -- the real testsuite's own `table.wast` uses `0xffff_ffff`
    /// (with a `_` digit separator). `parse_limits` used to filter to
    /// ascii-digit-only atoms, silently dropping a hex atom out of the
    /// limits list entirely instead of parsing it.
    #[test]
    fn table_limits_accept_hex_literals_with_underscore_separators() {
        let m = parse_module("(module (table 0x10 0xffff_ffff funcref))").unwrap();
        assert_eq!(m.tables[0].limits, Limits { min: 0x10, max: Some(0xFFFF_FFFF) });
    }

    #[test]
    fn memory_limits_accept_hex_literals() {
        let m = parse_module("(module (memory 0x1 0x2))").unwrap();
        assert_eq!(m.memories[0].limits, Limits { min: 1, max: Some(2) });
    }

    /// The same hex-literal gap, at a THIRD independent call site: an
    /// imported table's limits (`build_import_shell`'s own digit-scanning
    /// `take_while`, separate from `build_table_limits_and_elements`'s).
    #[test]
    fn imported_table_limits_accept_hex_literals() {
        let m = parse_module(r#"(module (import "m" "t" (table 0x10 funcref)))"#).unwrap();
        let ImportTypeInfo::Table(tt) = &m.imports[0].type_info else {
            panic!("expected a table import, got {:?}", m.imports[0].type_info);
        };
        assert_eq!(tt.limits, Limits { min: 0x10, max: None });
    }

    // ── Multi-memory (W16, task #85) ──────────────────────────────────────

    #[test]
    fn memory_size_and_grow_default_to_memory_zero_when_no_index_is_given() {
        let m = parse_module("(module (memory 1) (func (result i32) (memory.size)))").unwrap();
        assert_eq!(code_of(&m, 0), &[0x3F, 0x00, 0x0B]); // memory.size memidx=0; end
    }

    #[test]
    fn memory_grow_resolves_a_named_memory_index_in_folded_form() {
        let m = parse_module(
            "(module (memory $mem0 1) (memory $mem1 1) \
             (func (param i32) (result i32) (memory.grow $mem1 (local.get 0))))",
        )
        .unwrap();
        // local.get 0; memory.grow memidx=1; end
        assert_eq!(code_of(&m, 0), &[0x20, 0x00, 0x40, 0x01, 0x0B]);
    }

    #[test]
    fn memory_size_resolves_a_numeric_memory_index_in_folded_form() {
        let m = parse_module("(module (memory 1) (memory 1) (func (result i32) (memory.size 1)))").unwrap();
        assert_eq!(code_of(&m, 0), &[0x3F, 0x01, 0x0B]); // memory.size memidx=1; end
    }

    // ── Multi-value blocktype (WASM06/WASM04) ────────────────────────────

    #[test]
    fn empty_and_single_result_blocktypes_are_unchanged() {
        // Sanity: the existing single-byte shorthand path must still work
        // exactly as before once it's routed through `encode_blocktype`.
        let empty = parse_module("(module (func (block (nop))))").unwrap();
        // block 0x40; nop; end (block); end (func)
        assert_eq!(code_of(&empty, 0), &[0x02, 0x40, 0x01, 0x0B, 0x0B]);

        let single_result = parse_module("(module (func (result i32) (block (result i32) (i32.const 1))))").unwrap();
        // block 0x7F (single i32 result); i32.const 1; end; end
        assert_eq!(code_of(&single_result, 0), &[0x02, 0x7F, 0x41, 0x01, 0x0B, 0x0B]);
    }

    #[test]
    fn param_only_block_encodes_a_deduped_type_index_and_body_position_is_correct() {
        // The actual regression: before WASM06/WASM04, a leading `(param
        // i32)` was never consumed, and the block's real body (`i32.add`)
        // would have been probed as an instruction named "param" -- so the
        // key assertion here isn't just "it produces bytes," it's that the
        // body encodes starting at the CORRECT position (drop is right
        // after the blocktype bytes, not swallowed by them).
        let m = parse_module(
            "(module (func (param i32) (result i32) (local.get 0) (block (param i32) (drop))))",
        )
        .unwrap();
        assert_eq!(m.types.len(), 2, "func's own type, plus the deduped (param i32) (result) block type");
        assert_eq!(m.types[1], FuncType { params: vec![ValueType::I32], results: vec![] });
        // local.get 0; block <type_idx=1 as SLEB128>; drop; end (block); end (func)
        assert_eq!(code_of(&m, 0), &[0x20, 0x00, 0x02, 0x01, 0x1A, 0x0B, 0x0B]);
    }

    #[test]
    fn param_and_multi_result_block_dedupes_against_an_identical_later_block() {
        // The func's own signature deliberately does NOT match either
        // block's signature (an unused trailing f32 param), so this test
        // isolates block-to-block deduplication specifically, without the
        // func's own inferred type accidentally colliding with it too.
        let m = parse_module(
            "(module (func (param f32) (result i32 i64)
                 (block (param) (result i32 i64) (i32.const 1) (i64.const 2))
                 (drop) (drop)
                 (block (result i32 i64) (i32.const 3) (i64.const 4))))",
        )
        .unwrap();
        // The first block's (param) (result i32 i64) and a later plain
        // (result i32 i64) block are the SAME signature (an explicit empty
        // param list yields zero params, same as an implicit one) -- both
        // should dedupe to ONE shared new type entry, proving `dedup_type`
        // (not a fresh entry per block) is really being used.
        assert_eq!(m.types.len(), 2, "func's own (different) type, plus ONE deduped (result i32 i64) type shared by both blocks");
        assert_eq!(m.types[1], FuncType { params: vec![], results: vec![ValueType::I32, ValueType::I64] });
    }

    #[test]
    fn flat_form_multi_value_loop_header_encodes_correctly() {
        // Flat (non-folded) syntax goes through `encode_stream_structured_instr`,
        // a separate code path from the folded-form test above -- both must
        // be fixed, so both get their own test. The func's own signature
        // has an extra unused i32 param so it can't accidentally dedupe
        // against the loop's (param i64) (result i64) blocktype.
        let m = parse_module(
            "(module (func (param i64 i32) (result i64)
                 local.get 0
                 loop (param i64) (result i64)
                   drop
                   i64.const 42
                 end))",
        )
        .unwrap();
        assert_eq!(m.types.len(), 2);
        assert_eq!(m.types[1], FuncType { params: vec![ValueType::I64], results: vec![ValueType::I64] });
        // local.get 0; loop <type_idx=1>; drop; i64.const 42; end (loop); end (func)
        assert_eq!(code_of(&m, 0), &[0x20, 0x00, 0x03, 0x01, 0x1A, 0x42, 0x2A, 0x0B, 0x0B]);
    }

    #[test]
    fn if_with_multi_value_blocktype_encodes_correctly() {
        // The func's own signature has an extra unused i64 param so it
        // can't accidentally dedupe against the if's own (param i32)
        // (result i32) blocktype.
        let m = parse_module(
            "(module (func (param i32 i64) (result i32)
                 (i32.const 1)
                 (if (param i32) (result i32) (local.get 0)
                   (then (i32.const 2) (i32.add))
                   (else (i32.const -2) (i32.add)))))",
        )
        .unwrap();
        assert_eq!(m.types.len(), 2, "func's own (different) type, plus the deduped (param i32) (result i32) if-blocktype");
        assert_eq!(m.types[1], FuncType { params: vec![ValueType::I32], results: vec![ValueType::I32] });
    }

    #[test]
    fn explicit_type_reference_blocktype_resolves_via_type_names() {
        let m = parse_module(
            "(module
               (type $t (func (param i32) (result i32)))
               (func (param i32) (result i32)
                 (local.get 0)
                 (block (type $t) (drop) (i32.const 9))))",
        )
        .unwrap();
        // No NEW type should be created -- the explicit reference resolves
        // to the already-declared $t (index 0), not a deduped/synthesized
        // second entry.
        assert_eq!(m.types.len(), 1);
        assert_eq!(code_of(&m, 0), &[0x20, 0x00, 0x02, 0x00, 0x1A, 0x41, 0x09, 0x0B, 0x0B]);
    }

    #[test]
    fn sign_extension_opcodes_encode_flat_and_folded_the_same_as_any_other_no_immediate_instruction() {
        // Real single-byte opcodes (0xC0-0xC4) go through the exact same
        // `wasm_opcodes::get_opcode_by_name` path every other no-immediate
        // instruction does -- no special-casing needed once WASM03 adds
        // them to the opcode table.
        let flat = parse_module("(module (func (param i32) (result i32) local.get 0 i32.extend8_s))").unwrap();
        assert_eq!(code_of(&flat, 0), &[0x20, 0x00, 0xC0, 0x0B]);

        let folded = parse_module("(module (func (param i64) (result i64) (i64.extend32_s (local.get 0))))").unwrap();
        assert_eq!(code_of(&folded, 0), &[0x20, 0x00, 0xC4, 0x0B]);
    }

    #[test]
    fn trunc_sat_opcodes_encode_as_the_0xfc_prefixed_two_byte_form() {
        // Unlike sign-extension, trunc_sat isn't in `wasm_opcodes`' table at
        // all (deliberately -- see that crate's own module doc comment) --
        // this crate's `desugar`-adjacent interception in `encode_stream_instr`/
        // `encode_flat_instr` must emit the real `0xFC <sub>` bytes.
        let flat = parse_module("(module (func (param f32) (result i32) local.get 0 i32.trunc_sat_f32_s))").unwrap();
        assert_eq!(code_of(&flat, 0), &[0x20, 0x00, 0xFC, 0x00, 0x0B]);

        let folded = parse_module("(module (func (param f64) (result i64) (i64.trunc_sat_f64_u (local.get 0))))").unwrap();
        assert_eq!(code_of(&folded, 0), &[0x20, 0x00, 0xFC, 0x07, 0x0B]);
    }

    #[test]
    fn memory_copy_and_memory_fill_encode_as_the_0xfc_prefixed_form_with_memidx_bytes(
    ) {
        // Task #94: same "0xFC isn't in wasm_opcodes' table" shape as
        // trunc_sat, but with a trailing memory-index byte (memory.copy:
        // two -- dest then source; memory.fill: one) after the sub-opcode.
        let flat = parse_module(
            "(module (memory 1) (func i32.const 0 i32.const 1 i32.const 2 memory.copy))",
        )
        .unwrap();
        assert_eq!(
            code_of(&flat, 0),
            &[0x41, 0x00, 0x41, 0x01, 0x41, 0x02, 0xFC, 0x0A, 0x00, 0x00, 0x0B]
        );

        let folded = parse_module(
            "(module (memory 1) (func (memory.copy (i32.const 0) (i32.const 1) (i32.const 2))))",
        )
        .unwrap();
        assert_eq!(
            code_of(&folded, 0),
            &[0x41, 0x00, 0x41, 0x01, 0x41, 0x02, 0xFC, 0x0A, 0x00, 0x00, 0x0B]
        );

        let flat_fill = parse_module(
            "(module (memory 1) (func i32.const 0 i32.const 0xAA i32.const 5 memory.fill))",
        )
        .unwrap();
        assert_eq!(
            code_of(&flat_fill, 0),
            &[0x41, 0x00, 0x41, 0xAA, 0x01, 0x41, 0x05, 0xFC, 0x0B, 0x00, 0x0B]
        );

        let folded_fill = parse_module(
            "(module (memory 1) (func (memory.fill (i32.const 0) (i32.const 0xAA) (i32.const 5))))",
        )
        .unwrap();
        assert_eq!(
            code_of(&folded_fill, 0),
            &[0x41, 0x00, 0x41, 0xAA, 0x01, 0x41, 0x05, 0xFC, 0x0B, 0x00, 0x0B]
        );
    }

    #[test]
    fn memory_init_and_data_drop_encode_as_the_0xfc_prefixed_form_with_a_real_dataidx(
    ) {
        // Task #95: unlike memory.copy/fill, the data-segment index here is
        // a REAL immediate resolved via `data_names`, not a hardcoded byte
        // -- flat form places it as the following token (`local.get`/
        // `global.get`'s own shape), folded form as the leading arg
        // (`call`'s own shape).
        let flat = parse_module(
            r#"(module (memory 1) (data $d "hi") (func i32.const 0 i32.const 0 i32.const 2 memory.init $d))"#,
        )
        .unwrap();
        assert_eq!(
            code_of(&flat, 0),
            &[0x41, 0x00, 0x41, 0x00, 0x41, 0x02, 0xFC, 0x08, 0x00, 0x00, 0x0B]
        );

        let folded = parse_module(
            r#"(module (memory 1) (data $d "hi") (func (memory.init $d (i32.const 0) (i32.const 0) (i32.const 2))))"#,
        )
        .unwrap();
        assert_eq!(
            code_of(&folded, 0),
            &[0x41, 0x00, 0x41, 0x00, 0x41, 0x02, 0xFC, 0x08, 0x00, 0x00, 0x0B]
        );

        let flat_drop = parse_module(r#"(module (data $d "hi") (func data.drop $d))"#).unwrap();
        assert_eq!(code_of(&flat_drop, 0), &[0xFC, 0x09, 0x00, 0x0B]);

        let folded_drop = parse_module(r#"(module (data $d "hi") (func (data.drop $d)))"#).unwrap();
        assert_eq!(code_of(&folded_drop, 0), &[0xFC, 0x09, 0x00, 0x0B]);
    }

    /// Task #97: an ACTIVE element segment with an explicit table and a
    /// `func`-keyword-prefixed funcidx-list -- the exact shape `table_
    /// init.wast`/`table_copy.wast` use throughout (`(elem (table $t0)
    /// (i32.const 2) func 3 1 4 1)`).
    #[test]
    fn active_element_segment_explicit_table_funcidx_list() {
        let m = parse_module(
            r#"(module
                 (func) (func) (func) (func) (func)
                 (table $t0 30 30 funcref)
                 (elem (table $t0) (i32.const 2) func 3 1 4 1))"#,
        )
        .unwrap();
        assert_eq!(m.elements.len(), 1);
        assert!(!m.elements[0].is_passive);
        assert_eq!(m.elements[0].table_index, 0);
        assert_eq!(m.elements[0].function_indices, vec![Some(3), Some(1), Some(4), Some(1)]);
    }

    /// Task #97: a PASSIVE element segment using the exprs-list form,
    /// mixing real `ref.func` entries with a `ref.null` entry -- the
    /// `ref.null` case is the whole reason `Element.function_indices`
    /// widened to `Vec<Option<u32>>`.
    #[test]
    fn passive_element_segment_exprs_list_with_ref_func_and_ref_null() {
        let m = parse_module(
            r#"(module
                 (func) (func) (func)
                 (elem funcref (ref.func 0) (ref.null func) (ref.func 2)))"#,
        )
        .unwrap();
        assert_eq!(m.elements.len(), 1);
        assert!(m.elements[0].is_passive);
        assert!(m.elements[0].offset_expr.is_empty());
        assert_eq!(m.elements[0].function_indices, vec![Some(0), None, Some(2)]);
    }

    /// `/security-review` finding: an ACTIVE segment using the exprs-list
    /// form (binary modes 4/6) must be rejected at parse time, not
    /// silently accepted -- `wasm-module-encoder::encode_element`'s
    /// active-segment branch does `func_index.unwrap_or(0)`, which would
    /// silently turn a `ref.null` entry into a real reference to function
    /// 0 on re-encode if this combination ever reached it. The binary
    /// decoder (`parse_element_section`) already structurally can't
    /// produce this shape (only mode flags 0/1/2/5 are handled); this
    /// test confirms the text parser enforces the same restriction.
    #[test]
    fn active_element_segment_using_exprs_list_form_is_rejected() {
        let err = parse_module(
            r#"(module
                 (func) (table $t 30 30 funcref)
                 (elem (table $t) (i32.const 0) funcref (ref.func 0) (ref.null func)))"#,
        )
        .unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedToken { .. }), "{err:?}");
    }

    /// Task #97: `table.init`/`table.copy`/`elem.drop`'s folded-form
    /// encoding, using the exact real syntax `table_init.wast`/
    /// `table_copy.wast` use throughout -- `(table.init $t 0 (dest)(src)
    /// (len))`, `(table.copy $dst $src (dest)(src)(len))`, `(elem.drop
    /// 0)`. `table.init`'s binary operand order is elemidx-THEN-tableidx
    /// (opposite of its own text order); `table.copy`'s binary order
    /// matches its text order (dst-then-src both times).
    #[test]
    fn table_init_copy_elem_drop_folded_form_encode_correctly() {
        let init = parse_module(
            r#"(module
                 (func) (table $t 30 30 funcref) (elem $e (table $t) (i32.const 0) func 0)
                 (func (table.init $t $e (i32.const 1) (i32.const 0) (i32.const 4))))"#,
        )
        .unwrap();
        assert_eq!(
            code_of(&init, 1),
            &[
                0x41, 0x01, // i32.const 1 (dest)
                0x41, 0x00, // i32.const 0 (src)
                0x41, 0x04, // i32.const 4 (len)
                0xFC, 0x0C, 0x00, 0x00, // table.init elemidx=0 tableidx=0
                0x0B,
            ]
        );

        let copy = parse_module(
            r#"(module
                 (table $t0 30 30 funcref) (table $t1 30 30 funcref)
                 (func (table.copy $t1 $t0 (i32.const 20) (i32.const 15) (i32.const 5))))"#,
        )
        .unwrap();
        assert_eq!(
            code_of(&copy, 0),
            &[
                0x41, 20, // i32.const 20 (dest)
                0x41, 15, // i32.const 15 (src)
                0x41, 5, // i32.const 5 (len)
                0xFC, 0x0E, 0x01, 0x00, // table.copy dst=1 src=0
                0x0B,
            ]
        );

        let drop = parse_module(
            r#"(module (func) (elem $e (i32.const 0) func 0) (func (elem.drop $e)))"#,
        )
        .unwrap();
        assert_eq!(code_of(&drop, 1), &[0xFC, 0x0D, 0x00, 0x0B]);
    }

    /// Task #98: `table.size`'s folded form resolves a REAL leading `$t`
    /// table index (defaulting to 0 when omitted), same disambiguation
    /// `table.get`/`table.set` already use.
    #[test]
    fn table_size_folded_form_resolves_a_named_table_index_or_defaults_to_zero() {
        let default_idx = parse_module(r#"(module (table $t 1 funcref) (func (result i32) (table.size)))"#).unwrap();
        assert_eq!(code_of(&default_idx, 0), &[0xFC, 0x10, 0x00, 0x0B]);

        let explicit_idx = parse_module(
            r#"(module (table $t0 1 funcref) (table $t1 1 funcref) (func (result i32) (table.size $t1)))"#,
        )
        .unwrap();
        assert_eq!(code_of(&explicit_idx, 0), &[0xFC, 0x10, 0x01, 0x0B]);
    }

    /// Task #98: `table.grow`'s folded form encodes its two operands
    /// (init value, then delta) before the `0xFC 0x0F <table_idx>` bytes,
    /// same operand-then-opcode order as `memory.fill`'s own folded arm.
    #[test]
    fn table_grow_folded_form_encodes_operands_then_the_real_table_index() {
        let m = parse_module(
            r#"(module (table $t0 1 funcref) (table $t1 1 funcref)
                 (func (result i32) (table.grow $t1 (ref.null func) (i32.const 2))))"#,
        )
        .unwrap();
        assert_eq!(
            code_of(&m, 0),
            &[0xD0, 0x70, /* ref.null func */ 0x41, 0x02, /* i32.const 2 */ 0xFC, 0x0F, 0x01, 0x0B]
        );
    }

    /// Task #98: `table.fill`'s folded form encodes its three operands
    /// (dest, value, len) before the `0xFC 0x11 <table_idx>` bytes, same
    /// shape as `table.grow` above but with one more operand.
    #[test]
    fn table_fill_folded_form_encodes_operands_then_the_real_table_index() {
        let m = parse_module(
            r#"(module (table $t 5 funcref)
                 (func (table.fill $t (i32.const 1) (ref.null func) (i32.const 2))))"#,
        )
        .unwrap();
        assert_eq!(
            code_of(&m, 0),
            &[
                0x41, 0x01, // i32.const 1
                0xD0, 0x70, // ref.null func
                0x41, 0x02, // i32.const 2
                0xFC, 0x11, 0x00, 0x0B
            ]
        );
    }

    /// Task #98: the FLAT/stream form defers to table 0 always, same
    /// documented scope boundary as `"memory.size" | "memory.grow"`'s own
    /// flat-form arm -- no vendored corpus file exercises a non-default
    /// table index through this form.
    #[test]
    fn table_grow_size_fill_flat_form_always_defaults_to_table_zero() {
        let m = parse_module(r#"(module (table $t 1 funcref) (func (result i32) table.size))"#).unwrap();
        assert_eq!(code_of(&m, 0), &[0xFC, 0x10, 0x00, 0x0B]);
    }

    #[test]
    fn all_eight_trunc_sat_names_map_to_the_spec_assigned_sub_opcode() {
        let expected = [
            ("i32.trunc_sat_f32_s", 0x00u8),
            ("i32.trunc_sat_f32_u", 0x01),
            ("i32.trunc_sat_f64_s", 0x02),
            ("i32.trunc_sat_f64_u", 0x03),
            ("i64.trunc_sat_f32_s", 0x04),
            ("i64.trunc_sat_f32_u", 0x05),
            ("i64.trunc_sat_f64_s", 0x06),
            ("i64.trunc_sat_f64_u", 0x07),
        ];
        for (name, sub) in expected {
            assert_eq!(trunc_sat_sub_opcode(name), Some(sub), "{name}");
        }
        assert_eq!(trunc_sat_sub_opcode("i32.trunc_f32_s"), None, "the TRAPPING variant must not match");
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

    /// WASM14: a function that references its signature via `(type $sig)`
    /// and has ZERO literal `(param ...)` forms of its own must still seed
    /// the local-index counter from the referenced type's REAL param count,
    /// not from 0 -- otherwise a declared `(local ...)` silently aliases
    /// parameter index 0 instead of starting right after the params. This
    /// is exactly `func.wast`'s own "type-use-1" shape from the official
    /// spec testsuite (`(func (export "f") (type $sig) (local $var i32)
    /// (local.get $var))` returning the local's own zero-initialized
    /// default, 0, not the param's value, 42), found because that real
    /// case was returning the wrong VALUE (not trapping) -- the kind of
    /// bug an inline-`(param ...)` test like the one above can't catch.
    #[test]
    fn local_declared_after_a_type_only_referenced_param_gets_the_next_free_index() {
        let m = parse_module(
            "(module
               (type $sig (func (param i32) (result i32)))
               (func (export \"f\") (type $sig) (local $var i32) (local.get $var)))",
        )
        .unwrap();
        // The param occupies local 0 (from $sig, never spelled out here);
        // $var must be local 1, NOT local 0 again.
        assert_eq!(code_of(&m, 0), &[0x20, 0x01, 0x0B]);
    }

    /// Two rounds of security review on the WASM14 fix above chased the
    /// same underlying issue through two increasingly narrow patches: a
    /// func that gives BOTH an explicit `(type $sig)` reference AND its
    /// own literal `(param ...)` forms with a DIFFERENT arity is something
    /// `resolve_func_signature_ref` never rejects, so `param_count` (from
    /// the real type) and this function's own literal param count can
    /// disagree. Round 1's fix (seed local indices from `param_count`)
    /// could make a declared local collide with (alias the storage of) an
    /// "extra" literal param. Round 2's fix (seed from `max` of the two
    /// counts) closed that collision but, since the compiled
    /// `FunctionBody` and real function type only ever account for
    /// `param_count` real params, an "extra" literal param's `local.get`/
    /// `.set`/`.tee` still encoded an index past the function's real local
    /// array -- confirmed via `wasm-execution`'s raw, unchecked
    /// `ctx.typed_locals[index]` panicking once such a module actually ran
    /// (not memory-unsafe, but a real crash/DoS surface). The real fix is
    /// upstream of both patches: REJECT the mismatch at parse time
    /// (`WastParseError::TypeUseParamCountMismatch`) rather than silently
    /// accepting it and hoping every later index computation stays safe.
    /// This is also the spec-correct behavior — a real `.wat` file's
    /// literal params, when given alongside a type reference, must always
    /// already match it exactly. The round-1/round-2 `max()`-based local
    /// index seeding stays in place as defense in depth (harmless — once
    /// this check passes, `param_position` and `param_count` are always
    /// equal whenever literal params were given), but this parse-time
    /// rejection is what actually makes the invariant hold.
    #[test]
    fn func_rejects_type_reference_disagreeing_with_its_own_literal_params() {
        let result = parse_module(
            "(module
               (type $sig (func (param i32) (result i32)))
               (func (export \"f\") (type $sig) (param i32) (param i32) (local $x i32)
                 (local.get $x)))",
        );
        assert!(matches!(
            result,
            Err(WastParseError::TypeUseParamCountMismatch { declared: 2, referenced: 1, .. })
        ));
    }

    /// The legitimate counterpart to the rejection test above: a func that
    /// repeats its `(param ...)` forms AGREEING with a `(type $sig)`
    /// reference (purely for naming — `func.wast`'s own `"type-use-6"`
    /// does exactly this: `(func (export "type-use-6") (type $sig-3)
    /// (param i32))`) must still parse and index locals correctly.
    #[test]
    fn func_accepts_type_reference_agreeing_with_its_own_literal_params() {
        let m = parse_module(
            "(module
               (type $sig (func (param i32) (result i32)))
               (func (export \"f\") (type $sig) (param $x i32) (local $y i32)
                 local.get $x local.get $y drop))",
        )
        .unwrap();
        // $x (from the matching literal param) is local 0, $y is local 1.
        assert_eq!(code_of(&m, 0), &[0x20, 0x00, 0x20, 0x01, 0x1A, 0x0B]);
    }

    /// A THIRD round of security review found round 3's own mismatch
    /// check could itself be bypassed: its pre-scan originally stopped at
    /// the first field that wasn't `param`/`result`/`type`, but this
    /// text-level parser doesn't otherwise enforce that a func's `(local
    /// ...)` forms all come after its `(param ...)` forms (that ordering
    /// check is `wasm-validator`'s job too). A func with a `(local ...)`
    /// BEFORE some of its trailing `(param ...)` forms made the pre-scan
    /// stop before ever counting those later params -- silently skipping
    /// the mismatch check entirely (`saw_literal_param` could even end up
    /// `false`) while the main assignment loop still processed them,
    /// reproducing the exact out-of-bounds local index round 2's finding
    /// was about, just via reordering instead of an outright arity
    /// mismatch. Fixed by giving the pre-scan the SAME leading-region
    /// membership test (`is_leading_field`) the main loop already uses --
    /// `param`/`result`/`type`/`local` are ALL "still in the prefix,"
    /// only a true instruction ends it -- so the two can no longer
    /// disagree on how far to scan.
    #[test]
    fn func_rejects_type_reference_disagreeing_even_with_a_local_field_reordered_before_the_extra_params() {
        let result = parse_module(
            "(module
               (type $sig (func (param i32)))
               (func (export \"f\") (type $sig)
                 (local $a i32)
                 (param $b i32) (param $c i32) (param $d i32)
                 (local.get $d)))",
        );
        assert!(matches!(
            result,
            Err(WastParseError::TypeUseParamCountMismatch { declared: 3, referenced: 1, .. })
        ));
    }

    /// A FOURTH round of security review found the `TypeUseParamCountMismatch`
    /// check itself had drifted from this file's own documented contract:
    /// an out-of-range numeric `(type N)` reference (no `(type ...)`
    /// section entry at all) must NOT be rejected here -- see
    /// `func_with_out_of_range_numeric_type_reference_does_not_panic`
    /// above, which is this exact promise, already regression-tested for
    /// the no-literal-params case. But `(func (type 0) (param i32))`
    /// (ordinary, spec-legal literal params, alongside an unresolvable
    /// type reference) compared against `param_count`'s `0` FALLBACK for
    /// the missing type and got hard-rejected -- a real functional
    /// regression, not a security bug (it only makes the parser MORE
    /// conservative), but still a false positive against input this
    /// crate's own design says it should still accept and pass through to
    /// `wasm-validator`. Fixed by gating the check on the type reference
    /// actually having resolved to a real type in the first place.
    #[test]
    fn out_of_range_type_reference_with_ordinary_literal_params_still_does_not_panic_or_reject() {
        let result = parse_module("(module (func (type 0) (param i32)))");
        assert!(result.is_ok());
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
        assert!(!m.data[0].is_passive);
    }

    /// Task #95: a passive segment has no offset expression at all --
    /// `(data $d "bytes")`, not `(data (i32.const 0) "bytes")`. Also
    /// exercises `$id` resolution: `data.drop`/`memory.init` need to look
    /// this segment up by name via `ctx.data_names`.
    #[test]
    fn passive_data_segment_has_no_offset_and_registers_its_id() {
        let m = parse_module(r#"(module (data $d "hello passive"))"#).unwrap();
        assert_eq!(m.data.len(), 1);
        assert!(m.data[0].is_passive);
        assert!(m.data[0].offset_expr.is_empty());
        assert_eq!(m.data[0].data, b"hello passive");
    }

    /// An unnamed active segment declared AFTER a named passive one must
    /// still get the right index -- proves `data_i`'s pass-1 counter (used
    /// to register `$id`s) and pass-2's real `ctx.module.data.push` stay in
    /// lockstep even when only some segments are named.
    #[test]
    fn mixed_passive_and_active_data_segments_keep_consistent_indices() {
        let m = parse_module(
            r#"(module (memory 1) (data $passive "one") (data (i32.const 0) "two"))"#,
        )
        .unwrap();
        assert_eq!(m.data.len(), 2);
        assert!(m.data[0].is_passive);
        assert_eq!(m.data[0].data, b"one");
        assert!(!m.data[1].is_passive);
        assert_eq!(m.data[1].data, b"two");
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

    #[test]
    fn select_with_explicit_result_type_annotation_is_a_known_gap() {
        // `select (result funcref)` -- opcode 0x1C, a SEPARATE opcode from
        // plain `select` (0x1B), added by the same reference-types
        // proposal WASM17 otherwise implements (needed because a bare
        // `select`'s type can't always be inferred from the stack alone
        // when reference types are ambiguous). Deliberately NOT implemented
        // here -- `select`'s existing folded-arm handling recurses into
        // `(result funcref)` as if it were an operand sub-expression,
        // producing a clean `UnknownInstruction` on "result" rather than a
        // real parse. Documented honestly as a known limitation (see the
        // real corpus's `select.wast`) rather than silently mishandled.
        let err = parse_module(
            r#"(module (func (param funcref funcref i32) (result funcref)
                 (select (result funcref) (local.get 0) (local.get 1) (local.get 2))))"#,
        )
        .unwrap_err();
        assert!(matches!(err, WastParseError::UnknownInstruction { .. }));
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

    /// Found running the real WebAssembly/testsuite corpus (align.wast):
    /// folded `br_table`'s label targets come FIRST, with an OPTIONAL
    /// folded index operand LAST -- `(br_table $a $b (i32.const 0))` --
    /// the opposite order from every other instruction's own
    /// "immediates trail operands" convention. The original
    /// implementation searched from the END of `args` for the first
    /// non-atom element (assuming trailing atoms were the labels), which
    /// finds the folded operand's own position and treats everything
    /// after it (nothing) as the labels, mis-encoding a zero-label
    /// `br_table` and silently dropping `$a`/`$b` instead of resolving
    /// them.
    #[test]
    fn folded_br_table_with_multiple_labels_and_trailing_folded_operand() {
        let m = parse_module(
            "(module
               (func $f
                 (block $a
                   (block $b
                     (br_table $a $b (i32.const 0))
                   )
                 )
               )
             )",
        )
        .unwrap();
        let code = &m.code[0].code;
        // br_table opcode 0x0E, count=1 (2 labels - 1), then each label's
        // depth in WRITTEN order: $a (outer, depth 1), $b (inner, depth 0).
        assert!(code.windows(4).any(|w| w == [0x0E, 0x01, 0x01, 0x00]), "code: {code:02x?}");
    }

    /// Found running the real WebAssembly/testsuite corpus (`br.wast`,
    /// `call_indirect.wast`): `(table reftype (elem e*))` -- no explicit
    /// numeric limits at all, table size implied by the element list --
    /// was completely unhandled, always erroring "expected 1 or 2 limit
    /// numbers" because `funcref` (the reftype keyword) isn't a digit atom.
    #[test]
    fn table_with_size_implied_by_inline_elem_list() {
        let m = parse_module(
            "(module
               (func $a (result i32) i32.const 1)
               (func $b (result i32) i32.const 2)
               (table funcref (elem $a $b))
             )",
        )
        .unwrap();
        assert_eq!(m.tables.len(), 1);
        assert_eq!(m.tables[0].limits, Limits { min: 2, max: Some(2) });
        assert_eq!(m.elements.len(), 1);
        assert_eq!(m.elements[0].table_index, 0);
        assert_eq!(m.elements[0].function_indices, vec![Some(0), Some(1)]);
        assert_eq!(m.elements[0].offset_expr, vec![0x41, 0x00, 0x0B]);
    }

    /// Found via security review of the round-4 `wasm-conformance` PR,
    /// empirically confirmed with a real panic before this fix:
    /// `(table funcref ())` has a syntactically valid but EMPTY inner
    /// list -- no "elem" head atom at all -- so indexing `elem_form[1..]`
    /// without first confirming the list is non-empty AND actually headed
    /// by "elem" panicked with a slice-range-out-of-bounds.
    #[test]
    fn table_with_empty_inline_list_errors_cleanly_not_panics() {
        let err = parse_module("(module (table funcref ()))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn table_with_wrong_keyword_in_inline_list_errors_cleanly_not_panics() {
        let err = parse_module("(module (table funcref (notelem)))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedToken { .. }));
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
    fn empty_elem_segment_errors_cleanly_not_panic() {
        // `elem` has no passive-segment support yet (task #97), so `(elem)`
        // alone is still unconditionally an error, not a valid empty
        // passive segment.
        assert!(matches!(parse_module("(module (elem))"), Err(WastParseError::UnexpectedEof)));
    }

    /// Task #95: `(data)` alone -- no `$id`, no `(memory ...)` clause, no
    /// offset expression, no string literal -- used to be a parse error
    /// (there was no such thing as a passive segment, so "no offset" was
    /// always malformed). Per the real WAT grammar, this is now a
    /// perfectly legal, empty PASSIVE segment (`datasegment ::= '(' 'data'
    /// id? datastring* ')'` -- zero `datastring`s is fine), not an error.
    #[test]
    fn empty_data_segment_is_a_valid_empty_passive_segment() {
        let module = parse_module("(module (data))").unwrap();
        assert_eq!(module.data.len(), 1);
        assert!(module.data[0].is_passive);
        assert!(module.data[0].data.is_empty());
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

    // ── Round 2 security-review regressions ─────────────────────────────

    #[test]
    fn import_with_empty_description_errors_cleanly_not_panics() {
        // `desc` is `()` -- `kind` falls through to "" via unwrap_or, which
        // used to index `desc[0]` on an empty slice to build the error.
        let err = parse_module(r#"(module (import "m" "n" ()))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn global_mut_with_no_value_type_errors_cleanly_not_panics() {
        let err = parse_module("(module (global (mut)))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn import_global_mut_with_no_value_type_errors_cleanly_not_panics() {
        let err = parse_module(r#"(module (import "m" "g" (global (mut))))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn start_with_no_function_reference_errors_cleanly_not_panics() {
        let err = parse_module("(module (start))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn inline_export_with_no_name_errors_cleanly_not_panics() {
        let err = parse_module("(module (func (export) (result i32) i32.const 1))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn bare_type_reference_with_no_index_errors_cleanly_not_panics() {
        // Import func description: `(type)` with no trailing index/name.
        let err = parse_module(r#"(module (import "m" "f" (func (type))))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn flat_call_indirect_with_bare_type_reference_errors_cleanly_not_panics() {
        let err = parse_module("(module (table 1 funcref) (func call_indirect (type)))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn folded_call_indirect_with_bare_type_reference_errors_cleanly_not_panics() {
        let err = parse_module("(module (table 1 funcref) (func (call_indirect (type))))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn func_with_out_of_range_numeric_type_reference_does_not_panic() {
        // `(type 0)` with no `(type ...)` declared anywhere in the module
        // used to index `ctx.module.types[0]` on an empty Vec while
        // fetching a value (`func_type`) that was never actually used --
        // dead code whose only effect was a panic. Bounds-checking a type
        // index is `wasm-validator`'s job (structural "index bounds"
        // validation), not this text-parser's -- so the fix is simply to
        // stop indexing at parse time, not to duplicate that validation
        // here. This module still parses to a (structurally invalid)
        // `WasmModule`, which is the correct division of responsibility.
        let result = parse_module("(module (func (type 0)))");
        assert!(result.is_ok());
    }

    // ── Round 4 security-review regression ──────────────────────────────

    #[test]
    fn deeply_nested_flat_blocks_error_cleanly_not_stack_overflow() {
        // WAT's FLAT instruction syntax lets `block`/`loop`/`if` nest with
        // no parentheses at all (`block block block ... end end end`, all
        // sibling atoms in one unnested list) -- each one drives one more
        // level of `encode_one` <-> `encode_stream_structured_instr`
        // recursion that `sexpr::MAX_NESTING_DEPTH` (which only bounds
        // `(...)` nesting) never sees. `InstrCtx::enter_block`/`exit_block`
        // is a second, independent depth counter for exactly this case.
        let body = "block ".repeat(MAX_INSTR_NESTING_DEPTH + 1);
        let src = format!("(module (func {body}))");
        let err = parse_module(&src).unwrap_err();
        assert!(matches!(err, WastParseError::TooDeeplyNested { .. }));
    }

    // ── Round 5 security-review regressions ─────────────────────────────

    #[test]
    fn deeply_nested_folded_arithmetic_errors_cleanly_not_stack_overflow() {
        // Round 4's fix only guarded `block`/`loop`/`if` recursion --
        // deeply nested FOLDED operands of an ordinary instruction
        // (`(i32.add (i32.add (i32.add ...) ...) ...)`) recurse through
        // `encode_flat_instr` -> `encode_instr_list` -> `encode_one` with
        // no depth guard at all, and empirically aborted with a real stack
        // overflow around depth ~165 -- well below `sexpr::MAX_NESTING_
        // DEPTH` (512), so that guard never tripped first either. The
        // `MAX_INSTR_NESTING_DEPTH` guard now lives in `encode_one` itself,
        // the single funnel every form of instruction recursion passes
        // through, so it catches this the same way it catches flat
        // `block`/`loop`/`if` nesting.
        let mut src = "(module (func (result i32) ".to_string();
        for _ in 0..MAX_INSTR_NESTING_DEPTH + 1 {
            src.push_str("(i32.add (i32.const 1) ");
        }
        src.push_str("(i32.const 1)");
        for _ in 0..MAX_INSTR_NESTING_DEPTH + 1 {
            src.push(')');
        }
        src.push_str("))");
        let err = parse_module(&src).unwrap_err();
        assert!(matches!(err, WastParseError::TooDeeplyNested { .. }));
    }

    #[test]
    fn long_flat_non_nested_instruction_sequence_does_not_trip_depth_guard() {
        // The depth guard must track NESTING, not total instruction count
        // -- a long flat (sibling, not nested) sequence well past
        // `MAX_INSTR_NESTING_DEPTH` in length is completely ordinary WAT
        // (e.g. a function that pushes many constants) and must still
        // parse successfully.
        let mut src = "(module (func (result i32) i32.const 0".to_string();
        for _ in 0..(MAX_INSTR_NESTING_DEPTH * 3) {
            src.push_str(" i32.const 1 i32.add");
        }
        src.push_str("))");
        assert!(parse_module(&src).is_ok());
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

    // ── WASM16: tail calls ────────────────────────────────────────────────

    /// Folded `return_call` -- same "index first, argument operands
    /// after" shape as `call`, just opcode 0x12 instead of 0x10.
    #[test]
    fn folded_return_call_encodes_index_first_then_argument_operands() {
        let m = parse_module(
            "(module (func $add (param i32 i32) (result i32) (i32.add (local.get 0) (local.get 1))) \
              (func (result i32) (return_call $add (i32.const 3) (i32.const 4))))",
        )
        .unwrap();
        // i32.const 3 ; i32.const 4 ; return_call 0 ; end
        assert_eq!(code_of(&m, 1), &[0x41, 0x03, 0x41, 0x04, 0x12, 0x00, 0x0B]);
    }

    /// Flat (non-folded) `return_call`, distinct code path from the folded
    /// case above.
    #[test]
    fn flat_return_call_with_preceding_argument_operands() {
        let m = parse_module(
            "(module (func $add (param i32 i32) (result i32) local.get 0 local.get 1 i32.add) \
              (func (result i32) i32.const 3 i32.const 4 return_call $add))",
        )
        .unwrap();
        assert_eq!(code_of(&m, 1), &[0x41, 0x03, 0x41, 0x04, 0x12, 0x00, 0x0B]);
    }

    /// Folded `return_call_indirect` with an explicit `(type $t)` --
    /// same shape as `call_indirect`, opcode 0x13 instead of 0x11.
    #[test]
    fn folded_return_call_indirect_encodes_type_and_table_index() {
        let m = parse_module(
            "(module (type $t (func (param i32) (result i32))) \
              (table 1 funcref) \
              (func (result i32) (return_call_indirect (type $t) (i32.const 5) (i32.const 0))))",
        )
        .unwrap();
        // i32.const 5 ; i32.const 0 ; return_call_indirect type=0 table=0 ; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x05, 0x41, 0x00, 0x13, 0x00, 0x00, 0x0B]);
    }

    /// Task #107: `call_indirect`/`return_call_indirect` with an EXPLICIT,
    /// non-default table index -- the reference-types proposal's own text
    /// syntax (`(call_indirect $t0 (type 0) ...)`), pervasive throughout
    /// `table_init.wast`/`table_copy.wast`. Two tables declared; targets
    /// the SECOND one by name, confirming the table token is resolved and
    /// encoded, not silently ignored the way it used to be.
    #[test]
    fn folded_call_indirect_with_explicit_nonzero_table_index() {
        let m = parse_module(
            "(module (type $t (func (param i32) (result i32))) \
              (table $t0 1 funcref) (table $t1 1 funcref) \
              (func (result i32) (call_indirect $t1 (type $t) (i32.const 5) (i32.const 0))))",
        )
        .unwrap();
        // i32.const 5 ; i32.const 0 ; call_indirect type=0 table=1 ; end
        assert_eq!(code_of(&m, 0), &[0x41, 0x05, 0x41, 0x00, 0x11, 0x00, 0x01, 0x0B]);
    }

    /// Same as above, but in the FLAT (non-folded) form, and for
    /// `return_call_indirect` -- confirms both encoder arms got the fix,
    /// not just the folded `call_indirect` one.
    #[test]
    fn flat_return_call_indirect_with_explicit_nonzero_table_index() {
        let m = parse_module(
            "(module (type $t (func (param i32) (result i32))) \
              (table $t0 1 funcref) (table $t1 1 funcref) \
              (func (result i32) i32.const 5 i32.const 0 return_call_indirect $t1 (type $t)))",
        )
        .unwrap();
        assert_eq!(code_of(&m, 0), &[0x41, 0x05, 0x41, 0x00, 0x13, 0x00, 0x01, 0x0B]);
    }

    /// Flat (non-folded) `return_call_indirect`.
    #[test]
    fn flat_return_call_indirect_with_preceding_argument_operands() {
        let m = parse_module(
            "(module (type $t (func (param i32) (result i32))) \
              (table 1 funcref) \
              (func (result i32) i32.const 5 i32.const 0 return_call_indirect (type $t)))",
        )
        .unwrap();
        assert_eq!(code_of(&m, 0), &[0x41, 0x05, 0x41, 0x00, 0x13, 0x00, 0x00, 0x0B]);
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

    // ── WASM17: funcref/externref, ref.null/ref.is_null/ref.func, table.get/set ──

    #[test]
    fn funcref_externref_recognized_as_value_types() {
        let m = parse_module(
            r#"(module
                 (func $f (param $p funcref) (result externref)
                   (local $l externref)
                   unreachable))"#,
        )
        .unwrap();
        assert_eq!(m.types[0].params, vec![ValueType::Funcref]);
        assert_eq!(m.types[0].results, vec![ValueType::Externref]);
    }

    #[test]
    fn verbose_ref_null_func_extern_form_matches_bare_keyword() {
        // `(ref null func)`/`(ref null extern)` -- the fully-spelled-out
        // syntax found in the real corpus (br_table.wast) -- must parse to
        // the exact same ValueType as the bare `funcref`/`externref`
        // keyword.
        let m = parse_module(
            r#"(module (func (param $p (ref null func)) (result (ref null extern))
                 unreachable))"#,
        )
        .unwrap();
        assert_eq!(m.types[0].params, vec![ValueType::Funcref]);
        assert_eq!(m.types[0].results, vec![ValueType::Externref]);
    }

    #[test]
    fn ref_null_func_folded_and_flat_emit_same_bytes() {
        let folded = parse_module("(module (func (result funcref) (ref.null func)))").unwrap();
        let flat = parse_module("(module (func (result funcref) ref.null func))").unwrap();
        // ref.null (0xD0) + heap-type byte 0x70 (func); end (0x0B).
        assert_eq!(code_of(&folded, 0), &[0xD0, 0x70, 0x0B]);
        assert_eq!(code_of(&flat, 0), &[0xD0, 0x70, 0x0B]);
    }

    #[test]
    fn ref_null_extern_emits_heap_type_byte_0x6f() {
        let m = parse_module("(module (func (result externref) (ref.null extern)))").unwrap();
        assert_eq!(code_of(&m, 0), &[0xD0, 0x6F, 0x0B]);
    }

    #[test]
    fn ref_null_unknown_heap_type_is_a_clean_error_not_a_panic() {
        // `$t` (a concrete heap type) is deliberately out of scope -- must
        // error, not panic or silently accept it.
        let err = parse_module("(module (func (result funcref) (ref.null $t)))").unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn ref_is_null_folded_and_flat_pop_one_push_i32() {
        let folded = parse_module(
            "(module (func (param funcref) (result i32) (ref.is_null (local.get 0))))",
        )
        .unwrap();
        let flat = parse_module(
            "(module (func (param funcref) (result i32) local.get 0 ref.is_null))",
        )
        .unwrap();
        // local.get 0 (0x20 0x00); ref.is_null (0xD1); end.
        assert_eq!(code_of(&folded, 0), &[0x20, 0x00, 0xD1, 0x0B]);
        assert_eq!(code_of(&flat, 0), &[0x20, 0x00, 0xD1, 0x0B]);
    }

    #[test]
    fn ref_func_resolves_named_index_folded_and_flat() {
        let folded = parse_module(
            r#"(module (func $tf) (func (result funcref) (ref.func $tf)))"#,
        )
        .unwrap();
        let flat = parse_module(r#"(module (func $tf) (func (result funcref) ref.func $tf))"#).unwrap();
        // ref.func (0xD2) + funcidx 0 (the only function, itself, is
        // func-space index 0 since `$tf` comes first); end.
        assert_eq!(code_of(&folded, 1), &[0xD2, 0x00, 0x0B]);
        assert_eq!(code_of(&flat, 1), &[0xD2, 0x00, 0x0B]);
    }

    #[test]
    fn table_get_set_resolve_named_table_folded_and_flat() {
        let folded = parse_module(
            r#"(module
                 (table $t 1 funcref)
                 (func (param funcref)
                   (table.set $t (i32.const 0) (local.get 0))
                   (drop (table.get $t (i32.const 0)))))"#,
        )
        .unwrap();
        // table.set (0x26) + tableidx 0; drop needs table.get (0x25) +
        // tableidx 0 first.
        let code = code_of(&folded, 0);
        assert!(code.windows(2).any(|w| w == [0x26, 0x00]), "table.set $t -> tableidx 0: {code:?}");
        assert!(code.windows(2).any(|w| w == [0x25, 0x00]), "table.get $t -> tableidx 0: {code:?}");

        let flat = parse_module(
            r#"(module
                 (table $t 1 funcref)
                 (func (param funcref)
                   i32.const 0 local.get 0 table.set $t
                   i32.const 0 table.get $t drop))"#,
        )
        .unwrap();
        let code = code_of(&flat, 0);
        assert!(code.windows(2).any(|w| w == [0x26, 0x00]));
        assert!(code.windows(2).any(|w| w == [0x25, 0x00]));
    }

    #[test]
    fn table_get_unknown_table_name_is_a_clean_error() {
        let err = parse_module(
            r#"(module (table 1 funcref) (func (drop (table.get $nope (i32.const 0)))))"#,
        )
        .unwrap_err();
        assert!(matches!(err, WastParseError::UnknownIdentifier { space: "table", .. }));
    }

    // ── WASM18: shared memories, atomic load/store/RMW/cmpxchg/fence ────────

    #[test]
    fn shared_memory_keyword_parses_and_sets_the_flag() {
        let m = parse_module(r#"(module (memory 1 1 shared))"#).unwrap();
        assert!(m.memories[0].shared);
        assert_eq!(m.memories[0].limits, Limits { min: 1, max: Some(1) });
    }

    #[test]
    fn non_shared_memory_defaults_shared_to_false() {
        let m = parse_module(r#"(module (memory 1 1))"#).unwrap();
        assert!(!m.memories[0].shared);
    }

    #[test]
    fn named_shared_memory_import_shorthand_sets_the_flag() {
        let m = parse_module(r#"(module (memory $m (import "m" "n") 1 1 shared))"#).unwrap();
        assert_eq!(m.imports.len(), 1);
        match &m.imports[0].type_info {
            ImportTypeInfo::Memory(mt) => assert!(mt.shared),
            other => panic!("expected a memory import, got {other:?}"),
        }
    }

    #[test]
    fn atomic_load_store_folded_and_flat_emit_the_0xfe_prefix() {
        let folded = parse_module(
            r#"(module (memory 1 1 shared)
                 (func (i32.atomic.store (i32.const 0) (i32.atomic.load (i32.const 0)))))"#,
        )
        .unwrap();
        let flat = parse_module(
            r#"(module (memory 1 1 shared)
                 (func i32.const 0 i32.const 0 i32.atomic.load i32.atomic.store))"#,
        )
        .unwrap();
        // i32.atomic.load: 0xFE 0x10 <align> <offset>; i32.atomic.store:
        // 0xFE 0x17 <align> <offset>.
        for code in [code_of(&folded, 0), code_of(&flat, 0)] {
            assert!(code.windows(2).any(|w| w == [0xFE, 0x10]), "missing i32.atomic.load: {code:?}");
            assert!(code.windows(2).any(|w| w == [0xFE, 0x17]), "missing i32.atomic.store: {code:?}");
        }
    }

    #[test]
    fn atomic_rmw_and_cmpxchg_emit_correct_sub_opcodes() {
        let m = parse_module(
            r#"(module (memory 1 1 shared)
                 (func (param i32 i32 i32)
                   (drop (i32.atomic.rmw.add (local.get 0) (local.get 1)))
                   (drop (i32.atomic.rmw.cmpxchg (local.get 0) (local.get 1) (local.get 2)))))"#,
        )
        .unwrap();
        let code = code_of(&m, 0);
        assert!(code.windows(2).any(|w| w == [0xFE, 0x1E]), "missing i32.atomic.rmw.add: {code:?}");
        assert!(code.windows(2).any(|w| w == [0xFE, 0x48]), "missing i32.atomic.rmw.cmpxchg: {code:?}");
    }

    #[test]
    fn atomic_fence_folded_and_flat_take_no_immediate() {
        let folded = parse_module(r#"(module (func (atomic.fence)))"#).unwrap();
        let flat = parse_module(r#"(module (func atomic.fence))"#).unwrap();
        assert_eq!(code_of(&folded, 0), &[0xFE, 0x03, 0x0B]);
        assert_eq!(code_of(&flat, 0), &[0xFE, 0x03, 0x0B]);
    }

    #[test]
    fn atomic_load_honors_explicit_align_and_offset() {
        let m = parse_module(
            r#"(module (memory 1 1 shared)
                 (func (result i32) (i32.atomic.load offset=4 align=4 (i32.const 0))))"#,
        )
        .unwrap();
        // 0xFE 0x10 <align_log2=2> <offset=4>
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0xFE, 0x10, 0x02, 0x04, 0x0B]);
    }

    #[test]
    fn atomic_op_with_no_explicit_align_defaults_to_natural_not_zero() {
        // The real corpus's atomic instructions are typically written
        // WITHOUT an explicit `align=` at all (unlike plain loads/stores,
        // which are semantically fine defaulting to a 1-byte hint since
        // their align check only enforces an upper bound). Atomic ops
        // require EXACT natural alignment, so silently defaulting the
        // missing align to 0 (log2 of 1 byte) would make ordinary,
        // spec-legal atomic instructions fail validation. Regression
        // test for that exact bug, caught while adding wasm-validator's
        // exact-alignment check.
        let m = parse_module(
            r#"(module (memory 1 1 shared)
                 (func (result i64) (i64.atomic.load (i32.const 0))))"#,
        )
        .unwrap();
        // i64.atomic.load's natural align is 8 bytes -> log2 = 3, NOT 0.
        assert_eq!(code_of(&m, 0), &[0x41, 0x00, 0xFE, 0x11, 0x03, 0x00, 0x0B]);
    }

    #[test]
    fn atomic_notify_and_wait_parse_and_emit_correct_sub_opcodes() {
        // Implementation-time correction of the (already-merged) W09
        // spec's "deliberately absent" claim -- notify/wait have real,
        // deterministic single-agent semantics, see wasm-opcodes'
        // AtomicOpKind::Notify/Wait doc comments.
        let m = parse_module(
            r#"(module
                 (func (drop (memory.atomic.notify (i32.const 0) (i32.const 0))))
                 (func (drop (memory.atomic.wait32 (i32.const 0) (i32.const 0) (i64.const 0))))
                 (func (drop (memory.atomic.wait64 (i32.const 0) (i64.const 0) (i64.const 0)))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFE, 0x00]), "notify");
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFE, 0x01]), "wait32");
        assert!(code_of(&m, 2).windows(2).any(|w| w == [0xFE, 0x02]), "wait64");
    }

    #[test]
    fn unknown_atomic_instruction_name_is_a_clean_error() {
        let err = parse_module(r#"(module (func (drop (i32.atomic.nonsense (i32.const 0)))))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnknownInstruction { .. }));
    }

    // ══════════════════════════════════════════════════════════════════════
    // SIMD PR1b-2: v128.const text syntax + the 4 other first-slice opcodes
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn v128_const_folded_and_flat_emit_the_0xfd_prefix_and_real_16_bytes() {
        let folded = parse_module(r#"(module (func (drop (v128.const i32x4 1 2 3 4))))"#).unwrap();
        let flat = parse_module(r#"(module (func v128.const i32x4 1 2 3 4 drop))"#).unwrap();
        let mut expected = vec![0xFDu8, 0x0C];
        for lane in [1i32, 2, 3, 4] {
            expected.extend(lane.to_le_bytes());
        }
        for code in [code_of(&folded, 0), code_of(&flat, 0)] {
            assert!(code.windows(expected.len()).any(|w| w == expected.as_slice()), "missing v128.const i32x4 1 2 3 4 bytes: {code:?}");
        }
    }

    #[test]
    fn v128_const_supports_all_six_shapes() {
        // Real corpus files (e.g. simd_splat.wast) mix all 6 shapes even
        // when testing a single op -- see `parse_v128_const`'s doc
        // comment. None of these shapes' own instructions (i8x16.splat
        // etc.) are executable yet; this only proves the LITERAL syntax
        // itself parses to the correct 16 bytes for every shape.
        let cases: &[(&str, &str, [u8; 16])] = &[
            ("i8x16", "i8x16 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16", [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            ("i16x8", "i16x8 1 2 3 4 5 6 7 8", {
                let mut b = [0u8; 16];
                for (i, lane) in [1i16, 2, 3, 4, 5, 6, 7, 8].into_iter().enumerate() {
                    b[i * 2..i * 2 + 2].copy_from_slice(&lane.to_le_bytes());
                }
                b
            }),
            ("i64x2", "i64x2 1 2", {
                let mut b = [0u8; 16];
                b[0..8].copy_from_slice(&1i64.to_le_bytes());
                b[8..16].copy_from_slice(&2i64.to_le_bytes());
                b
            }),
            ("f32x4", "f32x4 1.5 1.5 1.5 1.5", {
                let mut b = [0u8; 16];
                for i in 0..4 {
                    b[i * 4..i * 4 + 4].copy_from_slice(&1.5f32.to_le_bytes());
                }
                b
            }),
            ("f64x2", "f64x2 1.5 1.5", {
                let mut b = [0u8; 16];
                b[0..8].copy_from_slice(&1.5f64.to_le_bytes());
                b[8..16].copy_from_slice(&1.5f64.to_le_bytes());
                b
            }),
        ];
        for (shape, literal, expected_bytes) in cases {
            let src = format!("(module (func (drop (v128.const {literal}))))");
            let m = parse_module(&src).unwrap_or_else(|e| panic!("{shape} failed to parse: {e:?}"));
            let code = code_of(&m, 0);
            assert!(
                code.windows(expected_bytes.len()).any(|w| w == expected_bytes.as_slice()),
                "{shape}: expected bytes {expected_bytes:?} not found in {code:?}"
            );
        }
    }

    #[test]
    fn simd_binops_use_the_real_sub_opcode_leb128_encoding() {
        // i32x4.add's real sub-opcode is 174 (0xAE) -- past the 1-byte
        // LEB128 range, so it MUST take the 2-byte continuation encoding
        // [0xAE, 0x01], exercising the same multi-byte path SIMD PR1a's
        // own decoder test proved (see wasm-execution's CHANGELOG). eq's
        // sub-opcode (0x37 = 55) fits in one byte, no continuation bit.
        let m = parse_module(
            r#"(module
                 (func (param v128 v128) (result v128) (i32x4.add (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.eq (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(3).any(|w| w == [0xFD, 0xAE, 0x01]), "i32x4.add: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x37]), "i32x4.eq: {:?}", code_of(&m, 1));
    }

    #[test]
    fn simd_i32x4_arith_and_cmp_widening_encodes_the_real_sub_opcodes() {
        // SIMD widening (task #113-117): a representative sample of the
        // new arithmetic (Sub/Mul/Neg, including the one UNARY kind) and
        // comparison-family opcodes, each real sub-opcode verified
        // against wasm-opcodes' own CHANGELOG entry. `i32x4.neg` takes
        // only ONE folded operand, unlike every binary op here.
        let m = parse_module(
            r#"(module
                 (func (param v128 v128) (result v128) (i32x4.sub (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.mul (local.get 0) (local.get 1)))
                 (func (param v128) (result v128) (i32x4.neg (local.get 0)))
                 (func (param v128 v128) (result v128) (i32x4.lt_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.ge_s (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(3).any(|w| w == [0xFD, 0xB1, 0x01]), "i32x4.sub: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(3).any(|w| w == [0xFD, 0xB5, 0x01]), "i32x4.mul: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(3).any(|w| w == [0xFD, 0xA1, 0x01]), "i32x4.neg: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(2).any(|w| w == [0xFD, 0x3A]), "i32x4.lt_u: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(2).any(|w| w == [0xFD, 0x3F]), "i32x4.ge_s: {:?}", code_of(&m, 4));
    }

    #[test]
    fn simd_i32x4_arith2_widening_encodes_the_real_sub_opcodes() {
        // SIMD widening (task #118-120): i32x4.abs (the one UNARY kind
        // here) and the min/max family, each real sub-opcode verified
        // against wasm-opcodes' own CHANGELOG entry.
        let m = parse_module(
            r#"(module
                 (func (param v128) (result v128) (i32x4.abs (local.get 0)))
                 (func (param v128 v128) (result v128) (i32x4.min_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.min_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.max_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.max_u (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(3).any(|w| w == [0xFD, 0xA0, 0x01]), "i32x4.abs: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(3).any(|w| w == [0xFD, 0xB6, 0x01]), "i32x4.min_s: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(3).any(|w| w == [0xFD, 0xB7, 0x01]), "i32x4.min_u: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(3).any(|w| w == [0xFD, 0xB8, 0x01]), "i32x4.max_s: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(3).any(|w| w == [0xFD, 0xB9, 0x01]), "i32x4.max_u: {:?}", code_of(&m, 4));
    }

    #[test]
    fn simd_i32x4_from_i16x8_widening_encodes_the_real_sub_opcodes() {
        // SIMD widening (task #121-124): i32x4.extadd_pairwise_i16x8_s/_u
        // (sub-opcodes < 128, single-byte LEB128 -- unlike every prior
        // arm's entries here, no continuation byte), i32x4.dot_i16x8_s,
        // and i32x4.extmul_low/high_i16x8_s/_u (sub-opcodes >= 128, same
        // 2-byte LEB128 shape as min_s/max_u above), each real sub-opcode
        // verified against wasm-opcodes' own CHANGELOG entry.
        let m = parse_module(
            r#"(module
                 (func (param v128) (result v128) (i32x4.extadd_pairwise_i16x8_s (local.get 0)))
                 (func (param v128) (result v128) (i32x4.extadd_pairwise_i16x8_u (local.get 0)))
                 (func (param v128 v128) (result v128) (i32x4.dot_i16x8_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.extmul_low_i16x8_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.extmul_high_i16x8_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.extmul_low_i16x8_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i32x4.extmul_high_i16x8_u (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x7E]), "i32x4.extadd_pairwise_i16x8_s: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x7F]), "i32x4.extadd_pairwise_i16x8_u: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(3).any(|w| w == [0xFD, 0xBA, 0x01]), "i32x4.dot_i16x8_s: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(3).any(|w| w == [0xFD, 0xBC, 0x01]), "i32x4.extmul_low_i16x8_s: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(3).any(|w| w == [0xFD, 0xBD, 0x01]), "i32x4.extmul_high_i16x8_s: {:?}", code_of(&m, 4));
        assert!(code_of(&m, 5).windows(3).any(|w| w == [0xFD, 0xBE, 0x01]), "i32x4.extmul_low_i16x8_u: {:?}", code_of(&m, 5));
        assert!(code_of(&m, 6).windows(3).any(|w| w == [0xFD, 0xBF, 0x01]), "i32x4.extmul_high_i16x8_u: {:?}", code_of(&m, 6));
    }

    #[test]
    fn simd_i8x16_first_slice_encodes_the_real_sub_opcodes() {
        // SIMD widen PR4: i8x16.add/sub/neg -- this lane width's first
        // slice. All three sub-opcodes are < 128, so single-byte LEB128
        // (no continuation byte), same as i32x4.extadd_pairwise_i16x8_s/_u
        // above -- unlike i32x4.add's own (0xAE, needs a continuation
        // byte). Each real sub-opcode verified against wasm-opcodes' own
        // CHANGELOG entry.
        let m = parse_module(
            r#"(module
                 (func (param v128 v128) (result v128) (i8x16.add (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.sub (local.get 0) (local.get 1)))
                 (func (param v128) (result v128) (i8x16.neg (local.get 0))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x6E]), "i8x16.add: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x71]), "i8x16.sub: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(2).any(|w| w == [0xFD, 0x61]), "i8x16.neg: {:?}", code_of(&m, 2));
    }

    #[test]
    fn simd_i16x8_first_slice_encodes_the_real_sub_opcodes() {
        // SIMD widen PR5: i16x8.add/sub/mul/neg -- the first opcodes
        // where i16x8 is a PRIMARY lane width. All four sub-opcodes are
        // >= 128, so 2-byte LEB128 (needs the continuation byte), same
        // shape as i32x4.add's own (0xAE) -- unlike i8x16.add/sub/neg
        // above (all < 128, single-byte). Each real sub-opcode verified
        // against wasm-opcodes' own CHANGELOG entry.
        let m = parse_module(
            r#"(module
                 (func (param v128 v128) (result v128) (i16x8.add (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.sub (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.mul (local.get 0) (local.get 1)))
                 (func (param v128) (result v128) (i16x8.neg (local.get 0))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(3).any(|w| w == [0xFD, 0x8E, 0x01]), "i16x8.add: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(3).any(|w| w == [0xFD, 0x91, 0x01]), "i16x8.sub: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(3).any(|w| w == [0xFD, 0x95, 0x01]), "i16x8.mul: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(3).any(|w| w == [0xFD, 0x81, 0x01]), "i16x8.neg: {:?}", code_of(&m, 3));
    }

    #[test]
    fn simd_i16x8_cmp_family_encodes_the_real_sub_opcodes() {
        // SIMD widen PR6: i16x8's own comparison family. All ten sub-
        // opcodes are < 128, so single-byte LEB128 (no continuation
        // byte), same shape as i32x4.eq's own (0x37) -- unlike
        // i16x8.add/sub/mul/neg above (all >= 128, 2-byte LEB128). Each
        // real sub-opcode verified against wasm-opcodes' own CHANGELOG
        // entry.
        let m = parse_module(
            r#"(module
                 (func (param v128 v128) (result v128) (i16x8.eq (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.ne (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.lt_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.lt_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.gt_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.gt_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.le_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.le_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.ge_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.ge_u (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x2D]), "i16x8.eq: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x2E]), "i16x8.ne: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(2).any(|w| w == [0xFD, 0x2F]), "i16x8.lt_s: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(2).any(|w| w == [0xFD, 0x30]), "i16x8.lt_u: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(2).any(|w| w == [0xFD, 0x31]), "i16x8.gt_s: {:?}", code_of(&m, 4));
        assert!(code_of(&m, 5).windows(2).any(|w| w == [0xFD, 0x32]), "i16x8.gt_u: {:?}", code_of(&m, 5));
        assert!(code_of(&m, 6).windows(2).any(|w| w == [0xFD, 0x33]), "i16x8.le_s: {:?}", code_of(&m, 6));
        assert!(code_of(&m, 7).windows(2).any(|w| w == [0xFD, 0x34]), "i16x8.le_u: {:?}", code_of(&m, 7));
        assert!(code_of(&m, 8).windows(2).any(|w| w == [0xFD, 0x35]), "i16x8.ge_s: {:?}", code_of(&m, 8));
        assert!(code_of(&m, 9).windows(2).any(|w| w == [0xFD, 0x36]), "i16x8.ge_u: {:?}", code_of(&m, 9));
    }

    #[test]
    fn simd_i8x16_cmp_family_encodes_the_real_sub_opcodes() {
        // SIMD widen PR7: i8x16's own comparison family, closing the same
        // gap PR6 closed for i16x8. All ten sub-opcodes are < 128, so
        // single-byte LEB128 (no continuation byte) -- same shape as
        // i16x8's own comparison family above, unlike i8x16.add/sub/neg
        // (which use lower sub-opcodes but happen to be < 128 too; the
        // point of contrast is with i16x8.add/sub/mul/neg's >= 128
        // 2-byte encoding). Each real sub-opcode verified against
        // wasm-opcodes' own CHANGELOG entry.
        let m = parse_module(
            r#"(module
                 (func (param v128 v128) (result v128) (i8x16.eq (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.ne (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.lt_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.lt_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.gt_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.gt_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.le_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.le_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.ge_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.ge_u (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x23]), "i8x16.eq: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x24]), "i8x16.ne: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(2).any(|w| w == [0xFD, 0x25]), "i8x16.lt_s: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(2).any(|w| w == [0xFD, 0x26]), "i8x16.lt_u: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(2).any(|w| w == [0xFD, 0x27]), "i8x16.gt_s: {:?}", code_of(&m, 4));
        assert!(code_of(&m, 5).windows(2).any(|w| w == [0xFD, 0x28]), "i8x16.gt_u: {:?}", code_of(&m, 5));
        assert!(code_of(&m, 6).windows(2).any(|w| w == [0xFD, 0x29]), "i8x16.le_s: {:?}", code_of(&m, 6));
        assert!(code_of(&m, 7).windows(2).any(|w| w == [0xFD, 0x2A]), "i8x16.le_u: {:?}", code_of(&m, 7));
        assert!(code_of(&m, 8).windows(2).any(|w| w == [0xFD, 0x2B]), "i8x16.ge_s: {:?}", code_of(&m, 8));
        assert!(code_of(&m, 9).windows(2).any(|w| w == [0xFD, 0x2C]), "i8x16.ge_u: {:?}", code_of(&m, 9));
    }

    #[test]
    fn simd_i8x16_arith2_family_encodes_the_real_sub_opcodes() {
        // SIMD widen PR8: i8x16's own abs/popcnt/min_s/min_u/max_s/
        // max_u/avgr_u family. All seven sub-opcodes are < 128, so
        // single-byte LEB128 (no continuation byte). Each real
        // sub-opcode verified against wasm-opcodes' own CHANGELOG entry.
        let m = parse_module(
            r#"(module
                 (func (param v128) (result v128) (i8x16.abs (local.get 0)))
                 (func (param v128) (result v128) (i8x16.popcnt (local.get 0)))
                 (func (param v128 v128) (result v128) (i8x16.min_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.min_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.max_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.max_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i8x16.avgr_u (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x60]), "i8x16.abs: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x62]), "i8x16.popcnt: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(2).any(|w| w == [0xFD, 0x76]), "i8x16.min_s: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(2).any(|w| w == [0xFD, 0x77]), "i8x16.min_u: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(2).any(|w| w == [0xFD, 0x78]), "i8x16.max_s: {:?}", code_of(&m, 4));
        assert!(code_of(&m, 5).windows(2).any(|w| w == [0xFD, 0x79]), "i8x16.max_u: {:?}", code_of(&m, 5));
        assert!(code_of(&m, 6).windows(2).any(|w| w == [0xFD, 0x7B]), "i8x16.avgr_u: {:?}", code_of(&m, 6));
    }

    #[test]
    fn simd_i16x8_arith2_family_encodes_the_real_sub_opcodes() {
        // SIMD widen PR9: i16x8's own abs/min_s/min_u/max_s/max_u/
        // avgr_u family, closing the same "arith2" gap PR8 just closed
        // for i8x16 (no i16x8.popcnt -- WASM SIMD only defines popcnt
        // for i8x16). All six sub-opcodes are >= 128, so 2-byte LEB128
        // (a continuation byte) -- same shape as i16x8's own add/sub/
        // mul/neg, unlike i8x16's own arith2 family (all < 128). Each
        // real sub-opcode verified against wasm-opcodes' own CHANGELOG
        // entry.
        let m = parse_module(
            r#"(module
                 (func (param v128) (result v128) (i16x8.abs (local.get 0)))
                 (func (param v128 v128) (result v128) (i16x8.min_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.min_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.max_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.max_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.avgr_u (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(3).any(|w| w == [0xFD, 0x80, 0x01]), "i16x8.abs: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(3).any(|w| w == [0xFD, 0x96, 0x01]), "i16x8.min_s: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(3).any(|w| w == [0xFD, 0x97, 0x01]), "i16x8.min_u: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(3).any(|w| w == [0xFD, 0x98, 0x01]), "i16x8.max_s: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(3).any(|w| w == [0xFD, 0x99, 0x01]), "i16x8.max_u: {:?}", code_of(&m, 4));
        assert!(code_of(&m, 5).windows(3).any(|w| w == [0xFD, 0x9B, 0x01]), "i16x8.avgr_u: {:?}", code_of(&m, 5));
    }

    #[test]
    fn simd_i16x8_from_i8x16_widening_encodes_the_real_sub_opcodes() {
        // SIMD widen PR10: i16x8.extadd_pairwise_i8x16_s/_u (sub-opcodes
        // < 128, single-byte LEB128 -- unlike i16x8's own extmul_low/
        // high_i8x16_s/_u below) and i16x8.extmul_low/high_i8x16_s/_u
        // (sub-opcodes >= 128, same 2-byte LEB128 shape as min_s/max_u
        // above). Mirrors `simd_i32x4_from_i16x8_widening_encodes_the_
        // real_sub_opcodes` one lane width down; no
        // i16x8.dot_i8x16_s -- WASM SIMD does not define a dot-product
        // for this pair. Each real sub-opcode verified against
        // wasm-opcodes' own CHANGELOG entry.
        let m = parse_module(
            r#"(module
                 (func (param v128) (result v128) (i16x8.extadd_pairwise_i8x16_s (local.get 0)))
                 (func (param v128) (result v128) (i16x8.extadd_pairwise_i8x16_u (local.get 0)))
                 (func (param v128 v128) (result v128) (i16x8.extmul_low_i8x16_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.extmul_high_i8x16_s (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.extmul_low_i8x16_u (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (i16x8.extmul_high_i8x16_u (local.get 0) (local.get 1))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x7C]), "i16x8.extadd_pairwise_i8x16_s: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x7D]), "i16x8.extadd_pairwise_i8x16_u: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(3).any(|w| w == [0xFD, 0x9C, 0x01]), "i16x8.extmul_low_i8x16_s: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(3).any(|w| w == [0xFD, 0x9D, 0x01]), "i16x8.extmul_high_i8x16_s: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(3).any(|w| w == [0xFD, 0x9E, 0x01]), "i16x8.extmul_low_i8x16_u: {:?}", code_of(&m, 4));
        assert!(code_of(&m, 5).windows(3).any(|w| w == [0xFD, 0x9F, 0x01]), "i16x8.extmul_high_i8x16_u: {:?}", code_of(&m, 5));
    }

    #[test]
    fn simd_bitwise_family_encodes_the_real_sub_opcodes() {
        // SIMD widen PR11: v128.not/and/andnot/or/xor/bitselect
        // (sub-opcodes 0x4D-0x52, all < 128 so single-byte LEB128).
        // Lane-width-agnostic -- these operate on raw bytes, not
        // typed lanes, so there's only ever one `v128.*` spelling
        // per op (no i8x16/i16x8/i32x4 suffix family). bitselect is
        // the first ternary SIMD op (3 v128 operands); the "no
        // immediate beyond the opcode byte itself" encoder bucket
        // handles it with zero special-casing since operand count
        // is driven by the S-expression recursion, not the encoder.
        let m = parse_module(
            r#"(module
                 (func (param v128) (result v128) (v128.not (local.get 0)))
                 (func (param v128 v128) (result v128) (v128.and (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (v128.andnot (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (v128.or (local.get 0) (local.get 1)))
                 (func (param v128 v128) (result v128) (v128.xor (local.get 0) (local.get 1)))
                 (func (param v128 v128 v128) (result v128) (v128.bitselect (local.get 0) (local.get 1) (local.get 2))))"#,
        )
        .unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x4D]), "v128.not: {:?}", code_of(&m, 0));
        assert!(code_of(&m, 1).windows(2).any(|w| w == [0xFD, 0x4E]), "v128.and: {:?}", code_of(&m, 1));
        assert!(code_of(&m, 2).windows(2).any(|w| w == [0xFD, 0x4F]), "v128.andnot: {:?}", code_of(&m, 2));
        assert!(code_of(&m, 3).windows(2).any(|w| w == [0xFD, 0x50]), "v128.or: {:?}", code_of(&m, 3));
        assert!(code_of(&m, 4).windows(2).any(|w| w == [0xFD, 0x51]), "v128.xor: {:?}", code_of(&m, 4));
        assert!(code_of(&m, 5).windows(2).any(|w| w == [0xFD, 0x52]), "v128.bitselect: {:?}", code_of(&m, 5));
    }

    #[test]
    fn i32x4_splat_emits_the_prefix_and_no_immediate() {
        let m = parse_module(r#"(module (func (param i32) (result v128) (i32x4.splat (local.get 0))))"#).unwrap();
        assert!(code_of(&m, 0).windows(2).any(|w| w == [0xFD, 0x11]), "{:?}", code_of(&m, 0));
    }

    #[test]
    fn i32x4_extract_lane_folded_and_flat_lane_index_leads() {
        let folded = parse_module(
            r#"(module (func (result i32) (i32x4.extract_lane 2 (v128.const i32x4 10 20 30 40))))"#,
        )
        .unwrap();
        let flat = parse_module(
            r#"(module (func (result i32) v128.const i32x4 10 20 30 40 i32x4.extract_lane 2))"#,
        )
        .unwrap();
        // sub-opcode 0x1B (27, single-byte LEB128) then the raw lane index.
        for code in [code_of(&folded, 0), code_of(&flat, 0)] {
            assert!(code.windows(3).any(|w| w == [0xFD, 0x1B, 0x02]), "missing extract_lane lane=2: {code:?}");
        }
    }

    #[test]
    fn v128_const_unknown_shape_is_a_clear_error() {
        let err = parse_module(r#"(module (func (drop (v128.const bogus 1 2 3 4))))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn v128_const_too_few_lanes_is_a_clear_error() {
        let err = parse_module(r#"(module (func (drop (v128.const i32x4 1 2 3))))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }
}
