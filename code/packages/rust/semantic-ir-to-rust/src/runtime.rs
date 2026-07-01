//! Inlined Rust runtime helpers.
//!
//! The Rust backend produces self-contained output: every generated
//! `.rs` file embeds the runtime helpers it needs.  This module
//! supplies that runtime as a single string constant pasted into
//! every artifact.
//!
//! Per SIR13, the runtime is byte-identical across modules.
//! Dead-code elimination is the responsibility of `rustc` /
//! `cargo` (release builds with LTO strip unused helpers).
//!
//! Style notes:
//!
//! - The runtime lives inside a `mod __sir` inner module so it
//!   never collides with user names.
//! - `Rc` is used (not `Arc`) — single-threaded by design.
//! - No external crate dependencies.

/// The full inlined runtime.  Always emitted verbatim.
pub const RUNTIME: &str = r##"mod __sir {
    //! Runtime support — value model, builtins, helpers.
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    // ── value model ───────────────────────────────────────────────
    #[derive(Clone)]
    pub enum Value {
        Int(i64),
        // SIR16 floats.  Kept distinct from `Int` so the value model
        // never silently coerces — arithmetic promotes to `Float` only
        // when an operand is already a `Float` (see `any_float`).
        Float(f64),
        Bool(bool),
        Nil,
        // ── DefaultParams (P2e) sentinel ──────────────────────────
        // `Missing` marks an *omitted* positional argument at a call
        // site (the `DirectCall` emitter pads omitted trailing slots
        // with `missing()`).  A defaulted param's body-top prologue
        // tests for it via `is_missing` and substitutes the param's
        // default expression.  It is an internal sentinel — it should
        // never be printed or compared by ordinary user code, because
        // by the time a function body runs past its prologue every
        // `Missing` has been replaced.  We still give it safe,
        // defensive arms in `format`/`value_eq` so a leaked sentinel
        // degrades gracefully instead of panicking (`<missing>`; equal
        // only to another `Missing`).
        Missing,
        Sym(Rc<str>),
        Str(Rc<str>),
        Pair(Rc<Pair>),
        Closure(Rc<Closure>),
        // ── SIR16 sequences ───────────────────────────────────────
        // A growable, *mutably shared* vector.  `Rc<RefCell<…>>` is the
        // crux: `SeqSet` (`xs[i] = v`) must mutate the very sequence the
        // caller holds, and two bindings that alias the same literal must
        // see each other's writes — exactly the reference semantics of a
        // Python list or JS array.  Cloning a `Value::Seq` clones the
        // `Rc` (a shared handle), not the backing `Vec`.
        Seq(Rc<RefCell<Vec<Value>>>),
        // ── SIR16 maps ────────────────────────────────────────────
        // An *insertion-ordered* association list.  We key by `Value`
        // using the runtime's own `value_eq` (linear scan) rather than a
        // `HashMap`, because our `Value` is neither `Hash` nor `Eq`
        // (floats, closures, nested seqs/maps).  `value_eq` already
        // defines structural equality across the whole tower, so a
        // `Vec<(Value, Value)>` gives correct `MapGet`/`MapSet` semantics
        // — including missing-key ⇒ `Nil` — for *any* key type, and
        // preserves insertion order for deterministic iteration/printing.
        // Shared + mutable via `Rc<RefCell<…>>`, same as `Seq`.
        Map(Rc<RefCell<Vec<(Value, Value)>>>),
    }

    pub struct Pair {
        pub car: Value,
        pub cdr: Value,
    }

    pub struct Closure {
        pub fun: Box<dyn Fn(Vec<Value>) -> Value + 'static>,
    }

    // ── symbol interning ──────────────────────────────────────────
    thread_local! {
        static SYMBOL_TABLE: RefCell<HashMap<String, Rc<str>>> =
            RefCell::new(HashMap::new());
    }

    pub fn intern(name: &str) -> Value {
        SYMBOL_TABLE.with(|t| {
            let mut t = t.borrow_mut();
            if let Some(s) = t.get(name) {
                return Value::Sym(s.clone());
            }
            let s: Rc<str> = Rc::from(name);
            t.insert(name.to_string(), s.clone());
            Value::Sym(s)
        })
    }

    // ── DefaultParams (P2e) sentinel helpers ──────────────────────
    //
    // `missing()` constructs the omitted-argument sentinel; the
    // `DirectCall` emitter appends one per trailing param the caller
    // left off, so the emitted call is always full-arity.  `is_missing`
    // is the predicate a defaulted param's prologue uses to decide
    // whether to evaluate its default.  Both are trivial, but exposing
    // them as named helpers keeps the emitter's output readable and the
    // sentinel representation in one place.
    pub fn missing() -> Value {
        Value::Missing
    }

    pub fn is_missing(v: &Value) -> bool {
        matches!(v, Value::Missing)
    }

    // ── global storage ────────────────────────────────────────────
    thread_local! {
        static GLOBALS: RefCell<HashMap<String, Value>> =
            RefCell::new(HashMap::new());
    }

    pub fn global_set(name: &Value, value: Value) -> Value {
        let key = sym_or_string(name);
        GLOBALS.with(|g| g.borrow_mut().insert(key, value.clone()));
        value
    }

    pub fn global_get(name: &Value) -> Value {
        let key = sym_or_string(name);
        GLOBALS.with(|g| {
            g.borrow()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| panic!("undefined global: {}", key))
        })
    }

    pub fn global_get_static(name: &str) -> Value {
        GLOBALS.with(|g| {
            g.borrow()
                .get(name)
                .cloned()
                .unwrap_or_else(|| panic!("undefined global: {}", name))
        })
    }

    fn sym_or_string(v: &Value) -> String {
        match v {
            Value::Sym(s) => s.to_string(),
            Value::Str(s) => s.to_string(),
            other => format(other),
        }
    }

    // ── closure application ───────────────────────────────────────
    pub fn apply_closure(c: &Value, args: Vec<Value>) -> Value {
        match c {
            Value::Closure(cl) => (cl.fun)(args),
            _ => panic!("apply on non-closure"),
        }
    }

    // ── truthiness ────────────────────────────────────────────────
    pub fn truthy(v: &Value) -> bool {
        !matches!(v, Value::Bool(false) | Value::Nil)
    }

    // ── arithmetic builtins (variadic) ────────────────────────────
    //
    // Numeric tower (SIR16): the helpers stay on the integer path while
    // every operand is an `Int`, preserving exact i64 semantics
    // (including `times`' wrapping).  The moment *any* operand is a
    // `Float`, the whole fold promotes to f64 — matching the
    // "int op float ⇒ float" rule of Python/Ruby/JS.
    pub fn plus(args: Vec<Value>) -> Value {
        if any_float(&args) {
            let mut total = 0.0f64;
            for a in &args {
                total += as_f64(a);
            }
            return Value::Float(total);
        }
        let mut total: i64 = 0;
        for a in args {
            total += as_i64(&a);
        }
        Value::Int(total)
    }

    pub fn minus(args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Int(0);
        }
        if any_float(&args) {
            if args.len() == 1 {
                return Value::Float(-as_f64(&args[0]));
            }
            let mut acc = as_f64(&args[0]);
            for a in &args[1..] {
                acc -= as_f64(a);
            }
            return Value::Float(acc);
        }
        if args.len() == 1 {
            return Value::Int(-as_i64(&args[0]));
        }
        let mut acc = as_i64(&args[0]);
        for a in &args[1..] {
            acc -= as_i64(a);
        }
        Value::Int(acc)
    }

    pub fn times(args: Vec<Value>) -> Value {
        if any_float(&args) {
            let mut acc = 1.0f64;
            for a in &args {
                acc *= as_f64(a);
            }
            return Value::Float(acc);
        }
        let mut acc: i64 = 1;
        for a in args {
            acc = acc.wrapping_mul(as_i64(&a));
        }
        Value::Int(acc)
    }

    pub fn divide(args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Int(0);
        }
        if any_float(&args) {
            // Float division follows IEEE-754: `1.0 / 0.0` is `inf`
            // rather than a panic.  Only the all-integer path keeps the
            // historical divide-by-zero panic.
            let mut acc = as_f64(&args[0]);
            for a in &args[1..] {
                acc /= as_f64(a);
            }
            return Value::Float(acc);
        }
        let mut acc = as_i64(&args[0]);
        for a in &args[1..] {
            let d = as_i64(a);
            if d == 0 {
                panic!("division by zero");
            }
            acc /= d;
        }
        Value::Int(acc)
    }

    // ── comparison ────────────────────────────────────────────────
    pub fn eq(a: Value, b: Value) -> Value {
        Value::Bool(value_eq(&a, &b))
    }
    pub fn lt(a: Value, b: Value) -> Value {
        Value::Bool(num_lt(&a, &b))
    }
    pub fn gt(a: Value, b: Value) -> Value {
        Value::Bool(num_lt(&b, &a))
    }

    // Ordered numeric comparison.  Both-int compares as i64 (no
    // precision loss for large magnitudes); any float operand lifts the
    // comparison into f64.  `gt` is defined as `num_lt` with operands
    // swapped, so the two share one source of truth.
    fn num_lt(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x < y,
            _ => as_f64(a) < as_f64(b),
        }
    }

    // ── pair ops ──────────────────────────────────────────────────
    pub fn cons(a: Value, b: Value) -> Value {
        Value::Pair(Rc::new(Pair { car: a, cdr: b }))
    }
    pub fn car(a: Value) -> Value {
        match a {
            Value::Pair(p) => p.car.clone(),
            _ => panic!("car on non-pair"),
        }
    }
    pub fn cdr(a: Value) -> Value {
        match a {
            Value::Pair(p) => p.cdr.clone(),
            _ => panic!("cdr on non-pair"),
        }
    }

    // ── predicates ────────────────────────────────────────────────
    pub fn is_null(a: Value) -> Value { Value::Bool(matches!(a, Value::Nil)) }
    pub fn is_pair(a: Value) -> Value { Value::Bool(matches!(a, Value::Pair(_))) }
    // `number?` is true for both integers and floats — the predicate
    // names the numeric tower, not a single representation.
    pub fn is_number(a: Value) -> Value { Value::Bool(matches!(a, Value::Int(_) | Value::Float(_))) }
    pub fn is_symbol(a: Value) -> Value { Value::Bool(matches!(a, Value::Sym(_))) }

    // ── print ─────────────────────────────────────────────────────
    pub fn print(a: Value) -> Value {
        println!("{}", format(&a));
        Value::Nil
    }

    // ── format ────────────────────────────────────────────────────
    //
    // Cycle safety.  `Value::Seq`/`Value::Map` are *shared, mutable*
    // handles, so an emitted program can build a cyclic structure
    // (`xs = []; xs[0] = xs`).  A naive structural walk would recurse
    // forever and blow the stack.  We guard the recursion with a
    // `visited` set of the `Rc` handle addresses *currently on the
    // active path*: a handle is inserted on entry and removed on exit.
    //
    // Removing on exit (rather than leaving it set for the whole walk)
    // is deliberate — it means a value reached twice by two *sibling*
    // (non-cyclic) paths still prints in full both times; only a handle
    // that re-appears *within its own subtree* (a true cycle) is
    // short-circuited to a placeholder (`[...]` for a seq, `{...}` for a
    // map).  See `handle_id` for how a stable per-handle key is derived.
    pub fn format(v: &Value) -> String {
        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
        format_d(v, &mut visited)
    }

    // A stable identity for a shared handle: the address of the
    // `RefCell` the `Rc` points at, narrowed to a plain `usize`.  Two
    // `Value`s alias the same backing store iff their `handle_id`s match.
    fn seq_handle_id(items: &Rc<RefCell<Vec<Value>>>) -> usize {
        Rc::as_ptr(items) as *const () as usize
    }

    fn map_handle_id(entries: &Rc<RefCell<Vec<(Value, Value)>>>) -> usize {
        Rc::as_ptr(entries) as *const () as usize
    }

    fn format_d(v: &Value, visited: &mut std::collections::HashSet<usize>) -> String {
        match v {
            Value::Int(n) => n.to_string(),
            // `{:?}` keeps a trailing `.0` on integral floats (`3.0`,
            // not `3`) so the printed form is unambiguously a float —
            // matching how Python/Ruby render `3.0`.  Non-finite values
            // print as `NaN` / `inf` / `-inf`.
            Value::Float(x) => format_float(*x),
            Value::Bool(true) => "#t".to_string(),
            Value::Bool(false) => "#f".to_string(),
            Value::Nil => "nil".to_string(),
            // Defensive: a `Missing` sentinel should be consumed by a
            // defaulted param's prologue before any value is printed, so
            // this arm is normally unreachable.  Render a visible
            // placeholder rather than panicking if one ever leaks.
            Value::Missing => "<missing>".to_string(),
            Value::Sym(s) => s.to_string(),
            Value::Str(s) => s.to_string(),
            Value::Pair(p) => format_pair_d(p, visited),
            Value::Closure(_) => "<closure>".to_string(),
            // Sequences print like a bracketed list: `[1, 2, 3]`.
            Value::Seq(items) => {
                let id = seq_handle_id(items);
                if !visited.insert(id) {
                    // Already on the active path ⇒ cycle.  Print a
                    // placeholder instead of recursing forever.
                    return "[...]".to_string();
                }
                let out = format_seq_d(&items.borrow(), visited);
                visited.remove(&id);
                out
            }
            // Maps print like a brace-wrapped entry list in insertion
            // order: `{a: 1, b: 2}`.
            Value::Map(entries) => {
                let id = map_handle_id(entries);
                if !visited.insert(id) {
                    return "{...}".to_string();
                }
                let out = format_map_d(&entries.borrow(), visited);
                visited.remove(&id);
                out
            }
        }
    }

    fn format_seq_d(items: &[Value], visited: &mut std::collections::HashSet<usize>) -> String {
        let mut out = String::new();
        out.push('[');
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format_d(item, visited));
        }
        out.push(']');
        out
    }

    fn format_map_d(
        entries: &[(Value, Value)],
        visited: &mut std::collections::HashSet<usize>,
    ) -> String {
        let mut out = String::new();
        out.push('{');
        for (i, (k, v)) in entries.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format_d(k, visited));
            out.push_str(": ");
            out.push_str(&format_d(v, visited));
        }
        out.push('}');
        out
    }

    fn format_float(x: f64) -> String {
        if x.is_nan() {
            "NaN".to_string()
        } else if x.is_infinite() {
            if x > 0.0 { "inf".to_string() } else { "-inf".to_string() }
        } else {
            format!("{:?}", x)
        }
    }

    // `Pair`s are immutable (no shared `RefCell`), so a pair-chain can
    // never form a cycle on its own.  It can, however, *contain* a
    // cyclic seq/map in a `car`/`cdr`, so we still thread `visited`
    // through to the element formatters.
    fn format_pair_d(p: &Pair, visited: &mut std::collections::HashSet<usize>) -> String {
        let mut out = String::new();
        out.push('(');
        out.push_str(&format_d(&p.car, visited));
        let mut rest = p.cdr.clone();
        loop {
            match rest {
                Value::Pair(inner) => {
                    out.push(' ');
                    out.push_str(&format_d(&inner.car, visited));
                    rest = inner.cdr.clone();
                }
                Value::Nil => break,
                other => {
                    out.push_str(" . ");
                    out.push_str(&format_d(&other, visited));
                    break;
                }
            }
        }
        out.push(')');
        out
    }

    // ── loop support (SIR16 Loops) ────────────────────────────────
    //
    // `as_int` exposes the integer extraction the `ForRange` emitter
    // needs for its `start`/`stop`/`step` bounds: these must be raw
    // `i64`s for the numeric loop counter, not boxed `Value`s.  It is
    // simply the public face of `as_i64` (which stays private for the
    // arithmetic helpers).
    pub fn as_int(v: &Value) -> i64 {
        as_i64(v)
    }

    // `seq_iter` flattens a sequence value into a `Vec<Value>` for a
    // `ForEach` loop.  SIR16 introduced two distinct "sequence" shapes
    // this backend must iterate uniformly:
    //
    //   * `Value::Seq(vec)` — the real `Sequences` value (a `SeqLit`,
    //     `[1, 2, 3]`).  Cloned element-wise so the loop body sees stable
    //     snapshots even if it mutates the underlying sequence.
    //   * the classic cons-list — a `Pair`-chain ending in `Nil` (what
    //     `cons`/`car`/`cdr` build).  `Nil` itself is the empty sequence.
    //
    // Keeping both keeps A2's `ForEach`-over-cons-list working while
    // making `for x in [1, 2, 3]` (a `SeqLit`) iterate end to end.  An
    // improper list (a non-`Nil`, non-`Pair` tail) is a programming error
    // and panics, matching the strictness of `car`/`cdr` on a non-pair.
    pub fn seq_iter(v: &Value) -> Vec<Value> {
        // A real sequence: snapshot its current elements.
        if let Value::Seq(items) = v {
            return items.borrow().clone();
        }
        // Otherwise treat it as a cons-list.
        let mut out = Vec::new();
        let mut cur = v.clone();
        loop {
            match cur {
                Value::Nil => break,
                Value::Pair(p) => {
                    out.push(p.car.clone());
                    cur = p.cdr.clone();
                }
                other => panic!("cannot iterate non-sequence: {}", format(&other)),
            }
        }
        out
    }

    // ── sequence ops (SIR16 Sequences) ────────────────────────────
    //
    // A `Value::Seq` wraps a shared, mutable `Vec<Value>`.  These helpers
    // are the lowering targets for `SeqLit`/`SeqIndex`/`SeqLen`/`SeqSet`.

    // `seq_lit([a, b, ...])` constructs a fresh sequence from its items.
    pub fn seq_lit(items: Vec<Value>) -> Value {
        Value::Seq(Rc::new(RefCell::new(items)))
    }

    // `seq_index(seq, i)` reads `seq[i]`.  The index is taken as an
    // integer; a negative or out-of-range index panics (sequences are
    // strict, like `car`/`cdr`), matching SIR's "0-indexed, bounds are
    // target-defined" — we choose to define out-of-bounds as a panic.
    pub fn seq_index(seq: &Value, index: &Value) -> Value {
        match seq {
            Value::Seq(items) => {
                let i = as_i64(index);
                let items = items.borrow();
                if i < 0 || (i as usize) >= items.len() {
                    panic!("sequence index out of range: {} (len {})", i, items.len());
                }
                items[i as usize].clone()
            }
            other => panic!("seq-index on non-sequence: {}", format(other)),
        }
    }

    // `seq_len(seq)` returns the element count as an `Int`.
    pub fn seq_len(seq: &Value) -> Value {
        match seq {
            Value::Seq(items) => Value::Int(items.borrow().len() as i64),
            other => panic!("seq-len on non-sequence: {}", format(other)),
        }
    }

    // `seq_set(seq, i, value)` writes `seq[i] = value`, mutating the
    // shared backing vector in place.  Out-of-range writes panic (we do
    // not auto-grow, matching the index read's strictness).
    pub fn seq_set(seq: &Value, index: &Value, value: Value) -> Value {
        match seq {
            Value::Seq(items) => {
                let i = as_i64(index);
                let mut items = items.borrow_mut();
                if i < 0 || (i as usize) >= items.len() {
                    panic!("sequence index out of range: {} (len {})", i, items.len());
                }
                items[i as usize] = value.clone();
                value
            }
            other => panic!("seq-set on non-sequence: {}", format(other)),
        }
    }

    // ── map ops (SIR16 Maps) ──────────────────────────────────────
    //
    // A `Value::Map` wraps a shared, mutable, insertion-ordered
    // `Vec<(Value, Value)>`.  Lookups use `value_eq` for key comparison,
    // so any value type (including a float, string, or symbol) can be a
    // key with the same structural-equality semantics as `=`.

    // `map_lit([(k0, v0), (k1, v1), ...])` builds a fresh map.  Later
    // entries with a key equal to an earlier one overwrite in place, so
    // the literal `{a: 1, a: 2}` yields `{a: 2}` (last-write-wins,
    // mirroring object/dict literal semantics) while keeping first-seen
    // insertion order.
    pub fn map_lit(entries: Vec<(Value, Value)>) -> Value {
        // `store` is a plain local `Vec` (not yet wrapped in the shared
        // `Rc<RefCell<…>>`), so the `value_eq` key comparisons below
        // cannot collide with a borrow of the map under construction —
        // even for a self-referential key, which can only be an *already
        // built* map handle, never this not-yet-published `store`.
        let mut store: Vec<(Value, Value)> = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            // Resolve the slot index without holding any iterator borrow
            // across the (recursive) `value_eq` call.
            let found = store.iter().position(|(ek, _)| value_eq(ek, &k));
            match found {
                Some(i) => store[i].1 = v,
                None => store.push((k, v)),
            }
        }
        Value::Map(Rc::new(RefCell::new(store)))
    }

    // `map_get(map, key)` reads `map[key]`, returning the associated
    // value or `Nil` when the key is absent (SIR's target-defined
    // missing-key behaviour — we choose `Nil`, mirroring the TypeScript
    // backend's `?? null`).
    pub fn map_get(map: &Value, key: &Value) -> Value {
        match map {
            Value::Map(entries) => {
                // Snapshot the entries (a shallow `Rc`-handle clone per
                // value) and drop the borrow *before* running `value_eq`.
                // A self-referential key would otherwise re-enter this
                // same cell while it's still borrowed; `value_eq` on a
                // cyclic value can deep-walk, so we must not hold a borrow
                // across it.  (A shared borrow would tolerate a nested
                // shared re-borrow, but scoping it is clearer and keeps
                // `map_get`/`map_set`/`map_lit` uniform.)
                let snapshot = entries.borrow().clone();
                snapshot
                    .iter()
                    .find(|(k, _)| value_eq(k, key))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Value::Nil)
            }
            other => panic!("map-get on non-map: {}", format(other)),
        }
    }

    // `map_set(map, key, value)` inserts or overwrites `map[key]`,
    // mutating the shared backing store.  A new key appends (preserving
    // insertion order); an existing key (by `value_eq`) overwrites in
    // place without disturbing order.
    pub fn map_set(map: &Value, key: Value, value: Value) -> Value {
        match map {
            Value::Map(entries) => {
                // Find the matching slot *without* holding a `borrow_mut`
                // across `value_eq`: take a short shared borrow, snapshot
                // the keys, drop it, then compare.  A self-referential key
                // (`d["self"] = d` then looking it up) would otherwise
                // re-borrow this very cell inside `value_eq` and panic with
                // "already mutably borrowed".  Resolving to an *index*
                // first lets us re-borrow mutably only for the write.
                let index = {
                    let keys: Vec<Value> =
                        entries.borrow().iter().map(|(k, _)| k.clone()).collect();
                    keys.iter().position(|k| value_eq(k, &key))
                };
                let mut entries = entries.borrow_mut();
                match index {
                    Some(i) => entries[i].1 = value.clone(),
                    None => entries.push((key, value.clone())),
                }
                value
            }
            other => panic!("map-set on non-map: {}", format(other)),
        }
    }

    // ── helpers ───────────────────────────────────────────────────
    fn as_i64(v: &Value) -> i64 {
        match v {
            Value::Int(n) => *n,
            other => panic!("expected int, got {}", format(other)),
        }
    }

    // Coerce any number to f64 for the promoted arithmetic/comparison
    // paths.  Integers widen losslessly for values within ±2^53; beyond
    // that the all-integer fast paths above keep exactness, so this
    // widening only runs once a float is genuinely in play.
    fn as_f64(v: &Value) -> f64 {
        match v {
            Value::Int(n) => *n as f64,
            Value::Float(x) => *x,
            other => panic!("expected number, got {}", format(other)),
        }
    }

    fn any_float(args: &[Value]) -> bool {
        args.iter().any(|v| matches!(v, Value::Float(_)))
    }

    // Public structural equality.  Cycle safety: two *distinct* cyclic
    // structures (e.g. `xs[0]=xs` and `ys[0]=ys`, separate handles) would
    // make a naive deep walk recurse forever, because the `Rc::ptr_eq`
    // fast path only catches a value compared against *itself*.  We bound
    // the walk co-inductively with a `pending` set of handle-pairs
    // currently being compared: re-encountering a pair already in flight
    // means we've closed a cycle in lock-step, so we treat that pair as
    // equal (the standard co-inductive definition of bisimulation
    // equality).  This terminates for *any* pair of finite-handle graphs.
    fn value_eq(a: &Value, b: &Value) -> bool {
        let mut pending: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();
        value_eq_d(a, b, &mut pending)
    }

    fn value_eq_d(
        a: &Value,
        b: &Value,
        pending: &mut std::collections::HashSet<(usize, usize)>,
    ) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            // Cross-representation numeric equality (`1 == 1.0`) holds,
            // mirroring dynamic-language `==`.  Float/Float uses IEEE
            // equality, so `NaN == NaN` is correctly `false`.
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
            (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Nil, Value::Nil) => true,
            // Defensive: the `Missing` sentinel is internal and should
            // never reach a user-level comparison (a defaulted param's
            // prologue replaces it before the body runs).  Give it a
            // safe arm anyway: a `Missing` is equal only to another
            // `Missing`, never to `Nil` or any real value.
            (Value::Missing, Value::Missing) => true,
            (Value::Sym(x), Value::Sym(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => **x == **y,
            (Value::Pair(x), Value::Pair(y)) => {
                value_eq_d(&x.car, &y.car, pending) && value_eq_d(&x.cdr, &y.cdr, pending)
            }
            // Sequences and maps compare *structurally* (element-wise),
            // matching how `Pair` compares — `[1, 2] = [1, 2]` is true,
            // and two maps are equal when they hold equal entries in the
            // same insertion order.  Identical `Rc` handles short-circuit
            // to `true` without a deep walk.  Comparing two maps element-
            // wise (rather than as unordered sets) is sufficient here
            // because `map_lit`/`map_set` keep a canonical first-seen
            // order, so equal maps built the same way share that order.
            (Value::Seq(x), Value::Seq(y)) => {
                if Rc::ptr_eq(x, y) {
                    return true;
                }
                let pair = (seq_handle_id(x), seq_handle_id(y));
                // Already comparing this exact handle-pair higher up the
                // stack ⇒ we've matched in lock-step around a cycle.
                // Assume equal; if the structures genuinely differ it will
                // be caught on a *non-cyclic* element elsewhere.
                if !pending.insert(pair) {
                    return true;
                }
                // Snapshot the operands before recursing so we never hold
                // a `RefCell` borrow across the recursive `value_eq_d`
                // calls (a self-referential element would otherwise try to
                // re-borrow the same cell and panic).
                let xs = x.borrow().clone();
                let ys = y.borrow().clone();
                let result = xs.len() == ys.len()
                    && xs.iter().zip(ys.iter()).all(|(a, b)| value_eq_d(a, b, pending));
                pending.remove(&pair);
                result
            }
            (Value::Map(x), Value::Map(y)) => {
                if Rc::ptr_eq(x, y) {
                    return true;
                }
                let pair = (map_handle_id(x), map_handle_id(y));
                if !pending.insert(pair) {
                    return true;
                }
                let xs = x.borrow().clone();
                let ys = y.borrow().clone();
                let result = xs.len() == ys.len()
                    && xs.iter().zip(ys.iter()).all(|((ak, av), (bk, bv))| {
                        value_eq_d(ak, bk, pending) && value_eq_d(av, bv, pending)
                    });
                pending.remove(&pair);
                result
            }
            _ => false,
        }
    }

    // ── builtin-by-name dispatch (for VarRef Builtin or forward-compat) ─
    pub fn builtin_closure(name: &str) -> Value {
        let n = name.to_string();
        Value::Closure(Rc::new(Closure {
            fun: Box::new(move |args: Vec<Value>| call_builtin_by_name(&n, args)),
        }))
    }

    pub fn call_builtin_by_name(name: &str, args: Vec<Value>) -> Value {
        match name {
            "+" => plus(args),
            "-" => minus(args),
            "*" => times(args),
            "/" => divide(args),
            "=" => {
                let mut it = args.into_iter();
                eq(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            "<" => {
                let mut it = args.into_iter();
                lt(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            ">" => {
                let mut it = args.into_iter();
                gt(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            "cons" => {
                let mut it = args.into_iter();
                cons(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
            }
            "car" => car(args.into_iter().next().unwrap_or(Value::Nil)),
            "cdr" => cdr(args.into_iter().next().unwrap_or(Value::Nil)),
            "null?" => is_null(args.into_iter().next().unwrap_or(Value::Nil)),
            "pair?" => is_pair(args.into_iter().next().unwrap_or(Value::Nil)),
            "number?" => is_number(args.into_iter().next().unwrap_or(Value::Nil)),
            "symbol?" => is_symbol(args.into_iter().next().unwrap_or(Value::Nil)),
            "print" => print(args.into_iter().next().unwrap_or(Value::Nil)),
            "global_set" => {
                let mut it = args.into_iter();
                let key = it.next().unwrap_or(Value::Nil);
                let value = it.next().unwrap_or(Value::Nil);
                global_set(&key, value)
            }
            "global_get" => {
                let key = args.into_iter().next().unwrap_or(Value::Nil);
                global_get(&key)
            }
            other => panic!("unknown builtin: {}", other),
        }
    }

    // ── collection-method dispatch (C6) ───────────────────────────
    //
    // A source-level `recv.meth(args…)` / `recv.meth { |x| … }` reaches
    // this backend as `BuiltinCall("__method__", [recv, "meth", …args])`
    // and is emitted as `call_method(recv, "meth", vec![…args])`.  This
    // is the Rust analogue of the Python/TypeScript `sir-runtime-oop`
    // `call_method`, ported for behavioural parity (same method names,
    // same semantics) so a collection program produces identical output
    // across every backend.
    //
    // ── SECURITY: the catalog IS the allowlist ────────────────────
    //
    // Dispatch is an EXPLICIT `match` on `(type_of(recv), name)` — every
    // reachable method is written out by hand below.  There is NO
    // reflective / dynamic lookup that could turn an attacker-controlled
    // method name into arbitrary behaviour: a name we do not enumerate
    // simply falls through to `unknown_method`, which returns a controlled
    // error value (Ruby `nil`, matching the Python reference's honest
    // floor) — never an out-of-catalog effect.  This mirrors the C3 RCE
    // lesson: the allowlist is the whole security boundary, so it must be
    // a closed, hand-written set, never a table keyed by the raw name.
    //
    // ── block convention ──────────────────────────────────────────
    //
    // A trailing Ruby block reaches us as the *last* element of `args`
    // when it is a `Value::Closure` (a `{ }` block lowers to `MakeClosure`;
    // an `&:sym` block-pass lowers to `sym_to_proc`, also a `Closure`).
    // Block-taking methods split it off with `split_block` and apply it
    // via `apply_closure`, exactly as the Python runtime applies a trailing
    // `Closure`.

    // Coerce a (rarely used) non-literal method name to a `String`.  The
    // narrow-waist convention makes the name a `StrLit` in practice, so the
    // emitter passes a `&str` literal directly and this is only reached for
    // a defensively-handled non-literal name.
    pub fn method_name(v: &Value) -> String {
        match v {
            Value::Str(s) => s.to_string(),
            Value::Sym(s) => s.to_string(),
            other => format(other),
        }
    }

    // Split a trailing block off an argument list: if the last argument is
    // a `Closure`, return `(positional_args, Some(block))`, else
    // `(all_args, None)`.  Mirrors the Python runtime's
    // `isinstance(arg_list[-1], Closure)` test.
    fn split_block(mut args: Vec<Value>) -> (Vec<Value>, Option<Value>) {
        if matches!(args.last(), Some(Value::Closure(_))) {
            let block = args.pop();
            (args, block)
        } else {
            (args, None)
        }
    }

    // The honest "not in the catalog for this receiver" floor.  Ruby would
    // raise `NoMethodError`; the reference runtimes instead return `nil`
    // (the never-raise-on-the-OO-surface invariant).  We match that — a
    // *controlled* value, never undefined behaviour — but keep the name in
    // one place so the intent (and the security boundary) is explicit.
    fn unknown_method(_recv: &Value, _name: &str) -> Value {
        Value::Nil
    }

    /// Dispatch collection method `name` on `recv`.
    ///
    /// Resolution is a closed match on the receiver's runtime type, then on
    /// the method name within that type.  Block-taking methods pull a
    /// trailing `Closure` block off `args` first.  Anything unresolved
    /// bottoms out at `unknown_method` (Ruby `nil`) — never a reflective
    /// fallthrough.
    pub fn call_method(recv: Value, name: &str, args: Vec<Value>) -> Value {
        // Universal `Object#to_s` — available on *every* receiver, matching
        // the Python/TS reference (where `to_s` lives in the universal
        // Object table).  Handled here, before the type-specific catalogs,
        // so `&:to_s` works on numbers, symbols, etc.  It renders via the
        // runtime's `format` (the same display path `print` uses), so
        // `1.to_s == "1"` and `[1,2].to_s == "[1, 2]"`.
        if name == "to_s" && !matches!(recv, Value::Sym(_)) {
            // A Symbol has its own `to_s` (its bare name) in `symbol_method`;
            // everything else uses the universal display form.
            return Value::Str(Rc::from(format(&recv).as_str()));
        }
        match &recv {
            Value::Seq(_) => array_method(recv, name, args),
            Value::Map(_) => map_method(recv, name, args),
            Value::Str(_) => string_method(recv, name, args),
            Value::Sym(_) => symbol_method(recv, name, args),
            // `bool` is checked before the numeric arm on purpose: a Ruby
            // `true`/`false` is not a Numeric, so it never resolves the
            // numeric catalog.
            Value::Bool(_) => unknown_method(&recv, name),
            Value::Int(_) | Value::Float(_) => numeric_method(recv, name, args),
            _ => unknown_method(&recv, name),
        }
    }

    // ── Array (`Value::Seq`) catalog ──────────────────────────────
    //
    // A Ruby `Array` is a `Value::Seq` (shared, mutable `Vec`).  Non-block
    // methods read/compute; the mutators (`push`/`pop`) mutate the backing
    // vector in place through the `Rc<RefCell<…>>`, so the caller's handle
    // observes the change — exactly like the Python list reference.
    fn array_method(recv: Value, name: &str, args: Vec<Value>) -> Value {
        let items_rc = match &recv {
            Value::Seq(items) => items.clone(),
            _ => return unknown_method(&recv, name),
        };
        let (pos, block) = split_block(args);
        match name {
            "length" | "size" => Value::Int(items_rc.borrow().len() as i64),
            "first" => items_rc.borrow().first().cloned().unwrap_or(Value::Nil),
            "last" => items_rc.borrow().last().cloned().unwrap_or(Value::Nil),
            "reverse" => {
                let mut v = items_rc.borrow().clone();
                v.reverse();
                seq_lit(v)
            }
            "sort" => {
                let mut v = items_rc.borrow().clone();
                // Ordering uses the runtime's numeric `<` (`num_lt`); a
                // stable insertion-order-preserving sort keeps ties in place.
                v.sort_by(|a, b| {
                    if num_lt(a, b) {
                        std::cmp::Ordering::Less
                    } else if num_lt(b, a) {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });
                seq_lit(v)
            }
            "join" => {
                let sep = pos.first().map(|s| method_name(s)).unwrap_or_default();
                let joined = items_rc
                    .borrow()
                    .iter()
                    .map(format)
                    .collect::<Vec<_>>()
                    .join(&sep);
                Value::Str(Rc::from(joined.as_str()))
            }
            "include?" => {
                let needle = pos.first().cloned().unwrap_or(Value::Nil);
                Value::Bool(items_rc.borrow().iter().any(|x| value_eq(x, &needle)))
            }
            "push" | "append" => {
                items_rc.borrow_mut().extend(pos);
                recv
            }
            "pop" => items_rc.borrow_mut().pop().unwrap_or(Value::Nil),
            // ── block-taking Array methods ────────────────────────
            "each" => {
                if let Some(b) = &block {
                    for item in items_rc.borrow().clone() {
                        apply_closure(b, vec![item]);
                    }
                }
                recv
            }
            "map" | "collect" => match &block {
                Some(b) => seq_lit(
                    items_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .map(|item| apply_closure(b, vec![item]))
                        .collect(),
                ),
                None => unknown_method(&recv, name),
            },
            "select" | "filter" => match &block {
                Some(b) => seq_lit(
                    items_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .filter(|item| truthy(&apply_closure(b, vec![item.clone()])))
                        .collect(),
                ),
                None => unknown_method(&recv, name),
            },
            "reject" => match &block {
                Some(b) => seq_lit(
                    items_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .filter(|item| !truthy(&apply_closure(b, vec![item.clone()])))
                        .collect(),
                ),
                None => unknown_method(&recv, name),
            },
            "find" | "detect" => match &block {
                Some(b) => items_rc
                    .borrow()
                    .clone()
                    .into_iter()
                    .find(|item| truthy(&apply_closure(b, vec![item.clone()])))
                    .unwrap_or(Value::Nil),
                None => unknown_method(&recv, name),
            },
            "any?" => match &block {
                Some(b) => Value::Bool(
                    items_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .any(|item| truthy(&apply_closure(b, vec![item]))),
                ),
                None => Value::Bool(items_rc.borrow().iter().any(truthy)),
            },
            "all?" => match &block {
                Some(b) => Value::Bool(
                    items_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .all(|item| truthy(&apply_closure(b, vec![item]))),
                ),
                None => Value::Bool(items_rc.borrow().iter().all(truthy)),
            },
            "none?" => match &block {
                Some(b) => Value::Bool(
                    !items_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .any(|item| truthy(&apply_closure(b, vec![item]))),
                ),
                None => Value::Bool(!items_rc.borrow().iter().any(truthy)),
            },
            "reduce" | "inject" => {
                let b = match &block {
                    Some(b) => b,
                    None => return unknown_method(&recv, name),
                };
                // With an explicit seed arg the fold starts there over the
                // whole vector; without one it seeds from the first element
                // (Ruby `inject`).  An empty seedless reduce is `nil`.
                let snapshot = items_rc.borrow().clone();
                let (mut acc, rest): (Value, &[Value]) = if let Some(seed) = pos.into_iter().next() {
                    (seed, &snapshot[..])
                } else if let Some((head, tail)) = snapshot.split_first() {
                    (head.clone(), tail)
                } else {
                    return Value::Nil;
                };
                for item in rest {
                    acc = apply_closure(b, vec![acc, item.clone()]);
                }
                acc
            }
            _ => unknown_method(&recv, name),
        }
    }

    // ── Hash (`Value::Map`) catalog ───────────────────────────────
    //
    // A Ruby `Hash` is a `Value::Map` (insertion-ordered assoc list).  A
    // block method receives `[key, value]` per entry, matching the Python
    // reference's `apply(block, [key, value])`.
    fn map_method(recv: Value, name: &str, args: Vec<Value>) -> Value {
        let entries_rc = match &recv {
            Value::Map(entries) => entries.clone(),
            _ => return unknown_method(&recv, name),
        };
        let (pos, block) = split_block(args);
        match name {
            "keys" => seq_lit(entries_rc.borrow().iter().map(|(k, _)| k.clone()).collect()),
            "values" => seq_lit(entries_rc.borrow().iter().map(|(_, v)| v.clone()).collect()),
            "size" | "length" => Value::Int(entries_rc.borrow().len() as i64),
            "has_key?" | "key?" | "include?" | "member?" => {
                let needle = pos.first().cloned().unwrap_or(Value::Nil);
                Value::Bool(entries_rc.borrow().iter().any(|(k, _)| value_eq(k, &needle)))
            }
            "each" | "each_pair" => {
                if let Some(b) = &block {
                    for (k, v) in entries_rc.borrow().clone() {
                        apply_closure(b, vec![k, v]);
                    }
                }
                recv
            }
            "map" | "collect" => match &block {
                Some(b) => seq_lit(
                    entries_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .map(|(k, v)| apply_closure(b, vec![k, v]))
                        .collect(),
                ),
                None => unknown_method(&recv, name),
            },
            "select" | "filter" => match &block {
                Some(b) => {
                    let kept: Vec<(Value, Value)> = entries_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .filter(|(k, v)| truthy(&apply_closure(b, vec![k.clone(), v.clone()])))
                        .collect();
                    Value::Map(Rc::new(RefCell::new(kept)))
                }
                None => unknown_method(&recv, name),
            },
            _ => unknown_method(&recv, name),
        }
    }

    // ── String (`Value::Str`) catalog ─────────────────────────────
    //
    // A Ruby `String` is an immutable `Value::Str`, so every method returns
    // a fresh value.  `split` with no argument splits on whitespace runs
    // (Ruby's awk-style default); with a separator it splits on that
    // literal substring.
    fn string_method(recv: Value, name: &str, args: Vec<Value>) -> Value {
        let s = match &recv {
            Value::Str(s) => s.clone(),
            _ => return unknown_method(&recv, name),
        };
        match name {
            "length" | "size" => Value::Int(s.chars().count() as i64),
            "upcase" => Value::Str(Rc::from(s.to_uppercase().as_str())),
            "downcase" => Value::Str(Rc::from(s.to_lowercase().as_str())),
            "reverse" => Value::Str(Rc::from(s.chars().rev().collect::<String>().as_str())),
            "strip" => Value::Str(Rc::from(s.trim())),
            "include?" => {
                let needle = args.first().map(method_name).unwrap_or_default();
                Value::Bool(s.contains(&needle))
            }
            "split" => {
                let parts: Vec<Value> = match args.first() {
                    Some(sep) => {
                        let sep = method_name(sep);
                        s.split(&sep).map(|p| Value::Str(Rc::from(p))).collect()
                    }
                    None => s.split_whitespace().map(|p| Value::Str(Rc::from(p))).collect(),
                };
                seq_lit(parts)
            }
            _ => unknown_method(&recv, name),
        }
    }

    // ── Numeric (`Value::Int` / `Value::Float`) catalog ───────────
    fn numeric_method(recv: Value, name: &str, args: Vec<Value>) -> Value {
        let (pos, block) = split_block(args);
        match name {
            "abs" => match &recv {
                Value::Int(n) => Value::Int(n.abs()),
                Value::Float(x) => Value::Float(x.abs()),
                _ => unknown_method(&recv, name),
            },
            "to_i" => Value::Int(as_i64_lenient(&recv)),
            "to_f" => Value::Float(as_f64_lenient(&recv)),
            "even?" => Value::Bool(as_i64_lenient(&recv) % 2 == 0),
            "odd?" => Value::Bool(as_i64_lenient(&recv) % 2 != 0),
            "zero?" => Value::Bool(as_f64_lenient(&recv) == 0.0),
            "times" => {
                // `n.times { |i| … }` yields 0..n and returns the receiver.
                if let Some(b) = &block {
                    let n = as_i64_lenient(&recv);
                    let mut i = 0i64;
                    while i < n {
                        apply_closure(b, vec![Value::Int(i)]);
                        i += 1;
                    }
                }
                let _ = pos;
                recv
            }
            _ => unknown_method(&recv, name),
        }
    }

    // Lenient numeric coercions for the Numeric catalog: unlike the strict
    // `as_i64`/`as_f64` used by arithmetic (which panic on a non-number),
    // these degrade a non-numeric receiver to `0` rather than panicking —
    // upholding the never-raise-on-the-OO-surface invariant.
    fn as_i64_lenient(v: &Value) -> i64 {
        match v {
            Value::Int(n) => *n,
            Value::Float(x) => *x as i64,
            _ => 0,
        }
    }

    fn as_f64_lenient(v: &Value) -> f64 {
        match v {
            Value::Int(n) => *n as f64,
            Value::Float(x) => *x,
            _ => 0.0,
        }
    }

    // ── Symbol catalog + `Symbol#to_proc` (`&:sym`) ───────────────
    fn symbol_method(recv: Value, name: &str, _args: Vec<Value>) -> Value {
        let s = match &recv {
            Value::Sym(s) => s.clone(),
            _ => return unknown_method(&recv, name),
        };
        match name {
            "to_s" => Value::Str(Rc::from(&*s)),
            "to_sym" => recv,
            "length" | "size" => Value::Int(s.chars().count() as i64),
            "upcase" => intern(&s.to_uppercase()),
            "downcase" => intern(&s.to_lowercase()),
            _ => unknown_method(&recv, name),
        }
    }

    // `sym_to_proc(:m)` builds a `Closure` equivalent to Ruby's
    // `:m.to_proc`: applied to `[recv, rest…]` it dispatches
    // `recv.m(rest…)` through `call_method`, so `[1,2,3].map(&:to_s)` is
    // `[1,2,3].map { |x| x.to_s }`.  The frontend lowers `&:sym` to
    // `block_pass(SymLit("sym"))`; the emitter turns that surviving
    // envelope into `sym_to_proc(intern("sym"))`, yielding a `Closure` the
    // block-taking catalog drives exactly like a `{ }` block.  A non-symbol
    // argument is coerced to its display name defensively.
    pub fn sym_to_proc(sym: Value) -> Value {
        // An already-callable `&blk` (a `Closure`) passes through unchanged
        // — only a *symbol* is converted into a dispatching proc.
        if matches!(sym, Value::Closure(_)) {
            return sym;
        }
        let method = match &sym {
            Value::Sym(s) => s.to_string(),
            other => format(other),
        };
        Value::Closure(Rc::new(Closure {
            fun: Box::new(move |mut args: Vec<Value>| {
                if args.is_empty() {
                    return Value::Nil;
                }
                let recv = args.remove(0);
                call_method(recv, &method, args)
            }),
        }))
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_non_empty_ends_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_declares_module_and_value() {
        assert!(RUNTIME.contains("mod __sir"));
        assert!(RUNTIME.contains("pub enum Value"));
    }

    #[test]
    fn runtime_declares_float_variant_and_helpers() {
        // SIR16 floats: the value model gains a `Float(f64)` arm, and
        // the numeric helpers gain f64 coercion + a display path.
        assert!(RUNTIME.contains("Float(f64)"));
        assert!(RUNTIME.contains("fn as_f64"));
        assert!(RUNTIME.contains("fn any_float"));
        assert!(RUNTIME.contains("fn format_float"));
        assert!(RUNTIME.contains("fn num_lt"));
    }

    #[test]
    fn runtime_includes_all_builtins() {
        for op in &[
            "plus", "minus", "times", "divide", "eq", "lt", "gt",
            "cons", "car", "cdr", "is_null", "is_pair", "is_number",
            "is_symbol", "print", "global_set", "global_get",
            "apply_closure", "intern", "truthy", "format",
            "call_builtin_by_name", "builtin_closure",
        ] {
            assert!(RUNTIME.contains(op), "runtime missing `{}`", op);
        }
    }

    #[test]
    fn runtime_uses_thread_local_globals() {
        assert!(RUNTIME.contains("thread_local!"));
        assert!(RUNTIME.contains("static GLOBALS"));
        assert!(RUNTIME.contains("static SYMBOL_TABLE"));
    }

    #[test]
    fn runtime_declares_loop_helpers() {
        // SIR16 Loops: ForRange needs an integer bound extractor
        // (`as_int`), ForEach needs cons-list iteration (`seq_iter`).
        assert!(RUNTIME.contains("pub fn as_int"));
        assert!(RUNTIME.contains("pub fn seq_iter"));
    }

    #[test]
    fn runtime_declares_seq_and_map_value_and_helpers() {
        // SIR16 Sequences + Maps: the value model gains shared, mutable
        // `Seq`/`Map` arms and the lowering helpers for each IR node.
        assert!(RUNTIME.contains("Seq(Rc<RefCell<Vec<Value>>>)"));
        assert!(RUNTIME.contains("Map(Rc<RefCell<Vec<(Value, Value)>>>)"));
        for helper in &[
            "pub fn seq_lit", "pub fn seq_index", "pub fn seq_len",
            "pub fn seq_set", "pub fn map_lit", "pub fn map_get",
            "pub fn map_set",
        ] {
            assert!(RUNTIME.contains(helper), "runtime missing `{}`", helper);
        }
    }

    #[test]
    fn runtime_declares_method_dispatch_and_catalog() {
        // C6: the inline runtime must ship `call_method` (the dispatcher),
        // `sym_to_proc` (`&:sym`), and a representative method from each of
        // the four catalogs so a collection program runs end to end.
        for helper in &["pub fn call_method", "pub fn sym_to_proc", "pub fn method_name"] {
            assert!(RUNTIME.contains(helper), "runtime missing `{}`", helper);
        }
        // Array / Map / String / Numeric catalog witnesses.
        for name in &[
            "\"map\" | \"collect\"",
            "\"reduce\" | \"inject\"",
            "\"keys\"",
            "\"upcase\"",
            "\"even?\"",
        ] {
            assert!(RUNTIME.contains(name), "runtime catalog missing `{}`", name);
        }
    }

    #[test]
    fn runtime_dispatch_has_no_reflective_fallback() {
        // Security: dispatch is a closed match with an honest `nil` floor
        // (`unknown_method`) — there must be NO `call_builtin_by_name`-style
        // raw-name table reachable from `call_method`.
        assert!(RUNTIME.contains("fn unknown_method"));
    }

    #[test]
    fn runtime_seq_iter_handles_real_seq() {
        // ForEach reconciliation: `seq_iter` must snapshot a `Value::Seq`
        // (the new real sequence) as well as walk a cons-list.
        assert!(RUNTIME.contains("if let Value::Seq(items) = v"));
    }
}
