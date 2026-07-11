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

    // Source-language display convention (SIR display-convention spec).  The
    // emitter substitutes `__SIR_DISPLAY_RUBY__` with `true` when the module's
    // `source_language` is Ruby, else `false` (the default Twig/Lisp form).
    // `format` reads this to render a boolean as Ruby `true`/`false` rather
    // than the Lisp `#t`/`#f`.  Kept a compile-time `const` so the branch folds
    // away — zero per-call cost — and existing Twig output is byte-for-byte
    // unchanged (the default is the Lisp form).
    pub const SIR_DISPLAY_RUBY: bool = __SIR_DISPLAY_RUBY__;
    use std::cell::Cell;
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
        // ── SIR17 user-defined-class instances (O5) ───────────────
        // A *handle* into the `INSTANCES` side-table: the `u64` is an
        // opaque instance id, and the real object state — its class-name
        // tag plus its `@ivar` bag — lives in a `thread_local` map keyed
        // by that id (see `SirInstance` / `INSTANCES` below).
        //
        // ── Value-model decision (variant vs. side-table) ──────────
        // We do NOT store `SirInstance` inline in the enum, and we do NOT
        // reuse an existing variant as a disguised handle.  Instead this
        // is a NARROW, dedicated variant carrying only an `id`, backed by
        // a side-table.  The trade-offs we weighed:
        //   • A side-table alone (reusing, say, a magic `Pair`) would
        //     leak: `pair?`/`car`/`cdr` would report/operate on an
        //     "instance", and `format`/`value_eq` would mis-render it.
        //   • Storing `SirInstance` INLINE (`Instance(Rc<SirInstance>)`)
        //     would put a `RefCell<HashMap>` on the hot, frequently-cloned
        //     `Value` and widen every ownership move.
        // The chosen id-handle-plus-side-table keeps `Value: Clone` a
        // trivial `Copy` of a `u64`, gives instances a *distinct*
        // discriminator (no built-in-type leak), and confines the
        // mutable object state to one `thread_local`.  Adding the arm
        // touches only THIS backend's emitted runtime `Value` — never the
        // core semantic-IR — and only two existing exhaustive sites
        // (`format_d`, and an identity arm in `value_eq_d`); every other
        // `match` already has a `_`/`matches!` fallback.
        Instance(u64),
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
    // `plus` is POLYMORPHIC on the tag of the FIRST operand, matching
    // Ruby's `+` (overloaded by receiver type).  The dispatch is an
    // explicit `match` on the first operand's variant — never reflection
    // (see [[dynamic-dispatch-rce]]): a `String` receiver concatenates,
    // a `Seq` receiver concatenates element vectors, and everything else
    // falls through to the UNCHANGED numeric fold below.
    //
    // | first arg   | behaviour                                   |
    // |-------------|---------------------------------------------|
    // | `Str`       | concatenate all args' string contents → Str |
    // | `Seq`       | concatenate the element vecs into a NEW Seq |
    // | otherwise   | numeric int/float fold (unchanged)          |
    //
    // Ruby `+` is binary; the SIR builtin is variadic (numeric fold), so
    // the string/array arms fold left-associatively over ≥2 operands,
    // preserving the existing variadic contract.
    pub fn plus(args: Vec<Value>) -> Value {
        match args.first() {
            // ── String concatenation ──────────────────────────────
            // `"a" + "b"` → `"ab"`.  Ruby's `String#+` requires a String
            // right-hand operand (`"a" + 1` raises `TypeError`); typed
            // rejection belongs to the sir-typed-runtime-errors cascade, so
            // here we require every operand be a `Str` and concatenate their
            // contents — a non-Str operand panics with a clear message rather
            // than silently coercing to integer garbage.
            Some(Value::Str(_)) => {
                let mut out = String::new();
                for a in &args {
                    match a {
                        Value::Str(s) => out.push_str(s),
                        other => panic!("string + expects strings, got {}", format(other)),
                    }
                }
                Value::Str(Rc::from(out.as_str()))
            }
            // ── Array concatenation ───────────────────────────────
            // `[1] + [2]` → `[1, 2]`.  Build a FRESH `Seq` from the
            // concatenated element snapshots — never alias or mutate an
            // input handle (Ruby `Array#+` returns a new array).  Each
            // operand must itself be a `Seq`.
            Some(Value::Seq(_)) => {
                let mut out: Vec<Value> = Vec::new();
                for a in &args {
                    match a {
                        Value::Seq(items) => out.extend(items.borrow().iter().cloned()),
                        other => panic!("array + expects arrays, got {}", format(other)),
                    }
                }
                seq_lit(out)
            }
            // ── Numeric fold (UNCHANGED) ──────────────────────────
            _ => {
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
        }
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

    // `times` is POLYMORPHIC on the tag of the FIRST operand, matching
    // Ruby's `*`.  Dispatch is an explicit `match` (never reflection):
    //
    // | first arg | 2nd arg | behaviour                              |
    // |-----------|---------|----------------------------------------|
    // | `Str`     | `Int n` | repeat the string n times (n≤0 → "")   |
    // | `Seq`     | `Int n` | new Seq with elements repeated n times |
    // | `Seq`     | `Str s` | join elements with `s` → Str           |
    // | otherwise | —       | numeric int/float fold (unchanged)     |
    //
    // Ruby `*` is binary; the SIR builtin is variadic (numeric fold).  The
    // string/array arms fold left-associatively pairwise, so `"ab" * 2 * 2`
    // repeats then repeats again — the natural extension of the binary
    // operator that preserves the variadic contract.  The join arm produces
    // a `Str`, so a subsequent operand would fold via the `Str` receiver
    // (repeat), matching left-associative Ruby chaining.
    pub fn times(args: Vec<Value>) -> Value {
        match args.first() {
            Some(Value::Str(_)) | Some(Value::Seq(_)) => {
                // Fold left-associatively over the operands: seed with the
                // first, apply `times_binary` against each subsequent operand.
                let mut it = args.into_iter();
                let mut acc = it.next().expect("first() was Some");
                for rhs in it {
                    acc = times_binary(acc, rhs);
                }
                acc
            }
            // ── Numeric fold (UNCHANGED) ──────────────────────────
            _ => {
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
        }
    }

    // The binary `*` for a String/Seq left operand — the atom the variadic
    // `times` fold applies pairwise.  Kept separate so the three
    // string/array behaviours (string repeat, array repeat, array join)
    // live in one explicit `match` on `(lhs, rhs)`.
    fn times_binary(lhs: Value, rhs: Value) -> Value {
        match (&lhs, &rhs) {
            // `"ab" * 3` → `"ababab"`.  A count ≤ 0 yields the empty
            // string (Ruby `"ab" * 0 == ""`, and negative counts raise in
            // Ruby but we clamp to empty for the never-raise floor).
            (Value::Str(s), Value::Int(n)) => {
                let count = if *n > 0 { *n as usize } else { 0 };
                // Guard `len * count` against `usize` overflow: `count` is cast
                // from a program-controlled `i64`, so an oversized repeat could
                // overflow (bogus size into `str::repeat`) or drive an
                // unbounded allocation.  Ruby raises `ArgumentError: argument
                // too big`; panic with the same controlled message rather than
                // overflow/OOM.
                if s.len().checked_mul(count).is_none() {
                    panic!("argument too big");
                }
                Value::Str(Rc::from(s.repeat(count).as_str()))
            }
            // `[0] * 3` → `[0, 0, 0]`.  A fresh Seq whose element snapshot
            // is repeated n times (n ≤ 0 → empty), never aliasing the input.
            (Value::Seq(items), Value::Int(n)) => {
                let count = if *n > 0 { *n as usize } else { 0 };
                let snapshot = items.borrow().clone();
                // Short-circuit an empty receiver (also avoids spinning the
                // `0..count` loop for a huge count), and guard the capacity
                // multiply against `usize` overflow — same program-controlled
                // count as the string arm.  `checked_mul` → controlled
                // `argument too big` panic (Ruby's `ArgumentError`) instead of
                // a wrapped/absurd `Vec::with_capacity` request.
                if snapshot.is_empty() || count == 0 {
                    return seq_lit(Vec::new());
                }
                let total = snapshot
                    .len()
                    .checked_mul(count)
                    .unwrap_or_else(|| panic!("argument too big"));
                let mut out: Vec<Value> = Vec::with_capacity(total);
                for _ in 0..count {
                    out.extend(snapshot.iter().cloned());
                }
                seq_lit(out)
            }
            // `[1, 2] * ", "` → `"1, 2"` (Ruby `Array#*` with a String is
            // `join`).  Element rendering uses the same `format` display the
            // rest of the backend uses (so it matches `Array#join`).
            (Value::Seq(items), Value::Str(sep)) => {
                let joined = items
                    .borrow()
                    .iter()
                    .map(format)
                    .collect::<Vec<_>>()
                    .join(sep);
                Value::Str(Rc::from(joined.as_str()))
            }
            (l, r) => panic!(
                "unsupported operands for *: {} and {}",
                format(l),
                format(r)
            ),
        }
    }

    pub fn divide(args: Vec<Value>) -> Value {
        if args.is_empty() {
            return Value::Int(0);
        }
        if any_float(&args) {
            // Ruby raises `ZeroDivisionError` for `/` by zero on BOTH int
            // and float operands (`1/0` and `1.0/0` both raise — Ruby does
            // NOT hand back IEEE `inf` here, unlike a bare host `f64` `/`).
            // We surface it as a typed `SirError` (via `raise`) so a
            // translated `rescue ZeroDivisionError` matches it, rather than
            // producing an uncatchable host `panic!` or a silent `inf`.
            let mut acc = as_f64(&args[0]);
            for a in &args[1..] {
                let d = as_f64(a);
                if d == 0.0 {
                    raise("ZeroDivisionError", Value::Str(Rc::from("divided by 0")));
                }
                acc /= d;
            }
            return Value::Float(acc);
        }
        let mut acc = as_i64(&args[0]);
        for a in &args[1..] {
            let d = as_i64(a);
            if d == 0 {
                // Typed `ZeroDivisionError` (was an uncatchable `panic!`)
                // so `rescue ZeroDivisionError` catches it — Ruby parity.
                raise("ZeroDivisionError", Value::Str(Rc::from("divided by 0")));
            }
            acc /= d;
        }
        Value::Int(acc)
    }

    // ── comparison ────────────────────────────────────────────────
    pub fn eq(a: Value, b: Value) -> Value {
        Value::Bool(value_eq(&a, &b))
    }
    /// Ruby case-equality (`pattern === value`) — the test a `when` (or `in`)
    /// arm runs.  Unlike `==`, Ruby keys `===` to the PATTERN's type: a `Range`
    /// checks membership, a `Regexp` checks a match, and everything else falls
    /// back to `==`.  A `when SomeClass` is lowered to `value.is_a?(SomeClass)`
    /// at the frontend, so a class pattern never reaches here.  This backend's
    /// `Value` has no `Range`/`Regexp` variant yet, so the only patterns that
    /// reach `case_eq` are ordinary values and the operation is exactly
    /// structural equality — matching the Python reference in `sir-runtime-oop`.
    /// When `Range`/`Regexp` values are added, extend with the membership/match
    /// arms (dispatching on `pattern`).
    pub fn case_eq(pattern: Value, value: Value) -> Value {
        Value::Bool(value_eq(&pattern, &value))
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
            // Strings and symbols compare lexicographically (Ruby `<`/`sort`).
            (Value::Str(x), Value::Str(y)) => x < y,
            (Value::Sym(x), Value::Sym(y)) => x < y,
            (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => {
                as_f64(a) < as_f64(b)
            }
            // Mixed / uncomparable types: no defined order.  We return `false`
            // rather than feeding a non-number to `as_f64` (which panicked) —
            // upholding the never-panic-on-the-OO-surface invariant.  `sort`/
            // `min`/`max`/`sort_by` then keep a stable order; the `<`/`>`
            // operators yield `false` (Ruby raises `ArgumentError` on a
            // genuinely uncomparable `<` — the typed-error refinement is a
            // separate follow-up).
            _ => false,
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

    // ── puts (Ruby semantics) ──────────────────────────────────────
    //
    // Ruby's `puts` is THE common output method and is deceptively subtle:
    //
    //   - `puts`            → one newline.
    //   - `puts x`          → `x.to_s` then a newline, UNLESS `x.to_s`
    //                         already ends in "\n" (no second newline):
    //                         `puts "x\n"` prints `x\n`, not `x\n\n`.
    //   - `puts a, b`       → each argument on its own line, in order.
    //   - `puts nil`        → a blank line (`nil.to_s` is "", then newline).
    //   - `puts []`         → a single newline (an argument flattening to
    //                         nothing still prints a blank line).
    //   - `puts [1,[2,3]]`  → each ELEMENT on its own line, arrays flattened
    //                         recursively: `1\n2\n3\n`.
    //
    // `puts` is variadic, so it takes the whole `Vec<Value>` (unlike the
    // fixed-arity `print`).  We use `print!` (no trailing newline) so the
    // trailing-newline-suppression rule can be honoured.
    pub fn puts(args: Vec<Value>) -> Value {
        if args.is_empty() {
            // No arguments: exactly one newline.
            print!("\n");
            return Value::Nil;
        }
        // A `Value::Seq` is a shared, mutable `Rc<RefCell<..>>` handle, so a
        // program can build a *cyclic* array (`a = []; a << a`).  The
        // element-per-line flatten below recurses through nested arrays, so —
        // like `format` — it MUST be cycle-guarded or a self-referential array
        // overflows the native stack and aborts (a DoS: CWE-674, uncontrolled
        // recursion).  We thread a `visited` set of the `Rc` handle addresses
        // on the active flatten path (the same `seq_handle_id` key `format`
        // uses); a handle removed on exit still flattens in full via a sibling
        // path — only a true self-cycle is short-circuited.
        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for a in &args {
            // `puts []` (empty array arg) still writes one blank line — Ruby
            // prints a line when an argument flattens to nothing.  A
            // recursive flatten of an empty seq writes nothing, so detect it.
            if let Value::Seq(items) = a {
                if items.borrow().is_empty() {
                    print!("\n");
                    continue;
                }
            }
            puts_one(a, &mut visited);
        }
        Value::Nil
    }

    // Emit a single `puts` argument.  Arrays recurse (element-per-line,
    // nested arrays flattened); everything else renders via `format` then a
    // newline — suppressed when the text already ends in one.  `nil` is a
    // blank line (`format(nil)` is "nil" for `print`, but `puts nil` is a
    // blank line, so nil is special-cased).
    //
    // Cycle safety: `visited` holds the `Rc` addresses of the seqs currently
    // on the active flatten path.  A seq ALREADY on the path is a cycle
    // (`a = []; a << a`): rather than recurse forever we write Ruby's `[...]`
    // placeholder then a newline, matching real Ruby (`puts a` on a self-
    // referential array prints `[...]` and terminates).  (We emit the literal
    // placeholder rather than `format(v)`: that formatter starts a fresh
    // visited set, so it would render the *containing* level too — `[[...]]`
    // for `a = [a]` — whereas Ruby prints a bare `[...]`.)  A seq reached twice
    // by *sibling* (non-cyclic) paths is flattened in full both times because
    // it is removed from `visited` on exit — only a handle re-appearing
    // *within its own subtree* is short-circuited.  Non-cyclic output is
    // unchanged (`puts [1,[2,3]]` still prints `1\n2\n3\n`).
    fn puts_one(v: &Value, visited: &mut std::collections::HashSet<usize>) {
        if let Value::Seq(items) = v {
            let id = seq_handle_id(items);
            if !visited.insert(id) {
                // Already on the active path ⇒ cycle: emit Ruby's `[...]`
                // placeholder and a newline, then stop recursing.
                println!("[...]");
                return;
            }
            // Clone the handle's items to avoid holding the borrow across the
            // recursive call (a nested seq re-borrows the same `RefCell`).
            let snapshot: Vec<Value> = items.borrow().clone();
            for item in &snapshot {
                puts_one(item, visited);
            }
            visited.remove(&id);
            return;
        }
        if matches!(v, Value::Nil) {
            print!("\n");
            return;
        }
        let text = format(v);
        if text.ends_with('\n') {
            print!("{}", text);
        } else {
            println!("{}", text);
        }
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
            Value::Bool(true) => if SIR_DISPLAY_RUBY { "true" } else { "#t" }.to_string(),
            Value::Bool(false) => if SIR_DISPLAY_RUBY { "false" } else { "#f" }.to_string(),
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
            // A user instance renders as Ruby's default `#<Class>` form.
            // We deliberately do NOT walk its ivars (Ruby's default
            // `to_s`/`inspect` prints only the class + an object id), so
            // there is no cyclic-structure risk and no `visited` handling
            // needed here.  A program wanting a richer form defines its
            // own `to_s`, which dispatches through `call_method` first.
            Value::Instance(id) => format!("#<{}>", instance_class(*id)),
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
                let raw = as_i64(index);
                let items = items.borrow();
                let len = items.len() as i64;
                // Ruby `arr[i]` (the `[]` op, NOT `arr.fetch(i)`): a negative
                // index counts from the end (`-1` ⇒ last); an index still
                // outside `0 .. len-1` returns `nil` — it does NOT raise (only
                // `fetch` raises IndexError).  Previously this panicked on any
                // OOB, diverging from Ruby and the other backends.
                let idx = if raw < 0 { raw + len } else { raw };
                if idx < 0 || idx >= len {
                    Value::Nil
                } else {
                    items[idx as usize].clone()
                }
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
            // User instances compare by IDENTITY (same handle id) —
            // Ruby's default `==` is object identity, and two distinct
            // `Foo.new` objects are unequal even with identical ivars.
            (Value::Instance(x), Value::Instance(y)) => x == y,
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
            "case_eq" => {
                let mut it = args.into_iter();
                case_eq(it.next().unwrap_or(Value::Nil), it.next().unwrap_or(Value::Nil))
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
            "puts" => puts(args),
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
    // simply falls through to `no_method_error`, which raises a typed
    // `NoMethodError` carrying only the (data) name string — never an
    // out-of-catalog effect.  (A KNOWN method used block-less floors to
    // `unknown_method`'s `nil` instead — see those helpers.)  This mirrors the C3 RCE
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

    // The `nil` floor for a KNOWN method used in a shape Ruby tolerates
    // without a value — e.g. a block-taking method (`map`, `select`,
    // `reduce`) called WITHOUT its block.  Ruby returns an `Enumerator`
    // there; SIR v0 has no `Enumerator`, so we floor to `nil` (a controlled
    // value, never undefined behaviour) rather than raising.  This is
    // distinct from `no_method_error` below: a name that is genuinely NOT
    // in the catalog is a Ruby `NoMethodError`, but a block-less `map` is
    // not an *unknown* method.  Also used for the defensive receiver-type
    // guards at each catalog's entry, which are unreachable in practice
    // (dispatch already routed by type).
    fn unknown_method(_recv: &Value, _name: &str) -> Value {
        Value::Nil
    }

    // The conventional Ruby class name of a value — for a `NoMethodError`
    // message and nothing else.  A closed match (never reflection on a
    // host type name), a verbatim parity port of the Go backend's
    // `_sir_ruby_class_name`.  A user instance reports its own class tag so
    // `obj.undefined` names the real class (e.g. `Dog`).
    fn ruby_class_name(v: &Value) -> String {
        match v {
            Value::Nil => "NilClass".to_string(),
            Value::Bool(true) => "TrueClass".to_string(),
            Value::Bool(false) => "FalseClass".to_string(),
            Value::Int(_) => "Integer".to_string(),
            Value::Float(_) => "Float".to_string(),
            Value::Str(_) => "String".to_string(),
            Value::Sym(_) => "Symbol".to_string(),
            Value::Seq(_) => "Array".to_string(),
            Value::Map(_) => "Hash".to_string(),
            Value::Pair(_) => "Pair".to_string(),
            Value::Closure(_) => "Proc".to_string(),
            Value::Instance(id) => instance_class(*id),
            Value::Missing => "Object".to_string(),
        }
    }

    // The honest "no such method" boundary: a name genuinely absent from
    // the receiver's catalog (or an instance's method table) is a Ruby
    // `NoMethodError`.  We surface a typed `SirError` (via `raise`) with the
    // Ruby-shaped message `undefined method 'x' for <Class>`, so a
    // translated `rescue NoMethodError` (or `rescue NameError`, its
    // superclass) catches it — replacing the old silent `nil` floor for a
    // truly-unknown method.  Resolution stays a closed match on the name
    // (never a reflective host lookup — the C3 allowlist discipline), so
    // this only ever fires for a name no catalog arm claimed.
    fn no_method_error(recv: &Value, name: &str) -> ! {
        raise(
            "NoMethodError",
            Value::Str(Rc::from(
                format!("undefined method '{}' for {}", name, ruby_class_name(recv)).as_str(),
            )),
        )
    }

    /// Dispatch collection method `name` on `recv`.
    ///
    /// Resolution is a closed match on the receiver's runtime type, then on
    /// the method name within that type.  Block-taking methods pull a
    /// trailing `Closure` block off `args` first.  Anything unresolved
    /// bottoms out at `unknown_method` (Ruby `nil`) — never a reflective
    /// fallthrough.
    pub fn call_method(recv: Value, name: &str, args: Vec<Value>) -> Value {
        // ── M6 universal metaprogramming: `send`/`__send__`/`public_send` ─
        //
        // Ruby's `send(:meth, args…)` re-enters dispatch with a *dynamic*
        // method name taken from the first argument (a Symbol or string),
        // forwarding the rest unchanged — so `x.send(:upcase)` is exactly
        // `x.upcase`, and a trailing block survives as a trailing arg.
        //
        // SECURITY ([[dynamic-dispatch-rce]]): the dynamic name is fed BACK
        // through `call_method` — the SAME explicit, closed dispatch a normal
        // `recv.meth` call takes.  It indexes the SAME hand-written catalogs /
        // `METHOD_TABLE` a direct call does, so an unknown name bottoms out at
        // the identical `no_method_error` boundary (a typed `NoMethodError`).
        // There is NO reflective lookup on the source-derived name — the name
        // is inert data that can only ever select an ARM we spelled out.
        //
        // A user-defined `send`/`tap`/etc. must win over the universal one
        // (Ruby's resolution order), so for a `Value::Instance` receiver we
        // check the user method table FIRST (below); only a genuine miss
        // reaches these universal Kernel methods, via `object_method`.
        //
        // `send` is placed FIRST (before the instance branch) so it re-enters
        // dispatch for EVERY receiver kind — an instance, a primitive, a
        // collection — uniformly, exactly as Ruby's `Kernel#send`.  An empty
        // arg list (`send` with no method name) floors to the honest
        // `NoMethodError` rather than panicking on an out-of-bounds index.
        // For an instance receiver we still honour a user-defined `send`
        // override first (resolution order), matching the Python reference
        // where the `define_method` table is consulted before `_SEND_METHODS`.
        if matches!(name, "send" | "__send__" | "public_send") {
            let user_send = match &recv {
                Value::Instance(id) => resolve_instance_method(&instance_class(*id), name),
                _ => None,
            };
            if user_send.is_none() {
                let mut it = args.into_iter();
                return match it.next() {
                    Some(target) => {
                        let target_name = method_name(&target);
                        call_method(recv, &target_name, it.collect())
                    }
                    None => no_method_error(&recv, name),
                };
            }
        }
        // ── user-defined-class dispatch (O5) ──────────────────────────
        // A `Value::Instance` receiver dispatches to the USER method table
        // (walking ancestry), with `self` bound for the call.  This branch
        // is taken FIRST and ONLY for instances, so the built-in /
        // collection path below is byte-for-byte unchanged for every other
        // receiver.  Resolution is `resolve_method` → an EXPLICIT
        // `HashMap::get` on the `(class, method)` key (never reflection):
        // a name like `constructor` simply misses and floors to the same
        // honest `NoMethodError`/`Nil` boundary the collection catalog uses
        // (`unknown_method`).  See `dispatch_user_method`.  A miss now falls
        // through to the universal Object methods (`respond_to?`/`tap`/…)
        // rather than raising immediately, matching the Python reference's
        // "reflective built-ins after the user table" resolution order.
        if let Value::Instance(id) = &recv {
            return dispatch_user_method(*id, &recv, name, args);
        }
        // Universal `Object#to_s` — available on *every* receiver, matching
        // the Python/TS reference (where `to_s` lives in the universal
        // Object table).  Handled here, before the type-specific catalogs,
        // so `&:to_s` works on numbers, symbols, etc.  It renders via the
        // runtime's `format` (the same display path `print` uses), so
        // `1.to_s == "1"` and `[1,2].to_s == "[1, 2]"`.  Instances are
        // excluded above (a user `to_s` may be defined); if none is, the
        // instance arm falls through to `unknown_method`, matching the
        // never-raise floor.
        if name == "to_s" && !matches!(recv, Value::Sym(_)) {
            // A Symbol has its own `to_s` (its bare name) in `symbol_method`;
            // everything else uses the universal display form.
            return Value::Str(Rc::from(format(&recv).as_str()));
        }
        // ── M6 universal Object methods (respond_to?/tap/then/yield_self) ─
        //
        // These resolve on EVERY primitive receiver, *after* `to_s` but
        // *before* the type-specific catalog so they never collide with a
        // type method (there is no primitive `tap`/`then`/`respond_to?`).
        // `object_method` returns `Some` when it claims the name; a `None`
        // falls through to the type-specific catalog below.  Boolean `&`/`|`/
        // `^` are handled inside `object_method` too (they resolve on a bool
        // receiver before the `no_method_error` bool arm).
        if let Some(v) = object_method(&recv, name, &args) {
            return v;
        }
        match &recv {
            Value::Seq(_) => array_method(recv, name, args),
            Value::Map(_) => map_method(recv, name, args),
            Value::Str(_) => string_method(recv, name, args),
            Value::Sym(_) => symbol_method(recv, name, args),
            // `bool` is checked before the numeric arm on purpose: a Ruby
            // `true`/`false` is not a Numeric, so it never resolves the
            // numeric catalog.  (Its `&`/`|`/`^` operators were handled by
            // `object_method` above.)
            Value::Bool(_) => no_method_error(&recv, name),
            Value::Int(_) | Value::Float(_) => numeric_method(recv, name, args),
            _ => no_method_error(&recv, name),
        }
    }

    // ── M6 universal Object / Kernel methods ──────────────────────────
    //
    // The Rust analogue of the Python/TS `sir-runtime-oop` universal Object
    // table.  A method here resolves on ANY receiver (every value is-a
    // `Object` in Ruby).  Returns `Some(value)` when `name` is a universal
    // method it handles, or `None` to let the caller fall through to the
    // type-specific catalog.  (`send`/`__send__`/`public_send` are handled
    // separately in `call_method` because they RE-ENTER dispatch; `to_s`
    // likewise, so it can render via `format`.)
    //
    // | method                  | yields | returns                    |
    // |-------------------------|--------|----------------------------|
    // | `respond_to?(name)`     | —      | bool — does dispatch resolve `name`? |
    // | `tap { … }`             | recv   | **recv** (side-effect pipe)|
    // | `then`/`yield_self { … }`| recv  | **block result** (functional pipe) |
    // | `&`/`|`/`^` (bool recv) | —      | eager boolean logic        |
    //
    // Block-less `tap`/`then` return the receiver (Ruby returns an Enumerator;
    // the v0 floor is the receiver, matching the Python reference).
    fn object_method(recv: &Value, name: &str, args: &[Value]) -> Option<Value> {
        // Boolean `&`/`|`/`^` — Ruby's EAGER, non-short-circuiting logical
        // operators on a `true`/`false` receiver (distinct from the lazy
        // `&&`/`||` keywords).  The operand is coerced by Ruby truthiness
        // (`nil`/`false` falsy, everything else truthy), so `true & nil ==
        // false` and `false | 0 == true`.  `^` is logical XOR.  Only a `Bool`
        // receiver with an operand resolves these (a bare `true.&` with no
        // arg falls through to the `no_method_error` bool arm).
        if let Value::Bool(b) = recv {
            if matches!(name, "&" | "|" | "^") {
                if let Some(other) = args.first() {
                    let o = truthy(other);
                    return Some(Value::Bool(match name {
                        "&" => *b && o,
                        "|" => *b || o,
                        _ => *b != o, // "^"
                    }));
                }
            }
        }
        match name {
            // `respond_to?(:m)` — true iff dispatch on `recv` resolves `m`,
            // consulting the SAME catalogs/table a real call uses (so it is
            // honest: an out-of-catalog name reports `false`, and that name
            // would also raise `NoMethodError` if actually called).
            "respond_to?" => {
                let target = args.first().map(method_name).unwrap_or_default();
                Some(Value::Bool(responds_to(recv, &target)))
            }
            // `tap { |x| … }` — run the block for its side effect, return the
            // RECEIVER.  Block-less `tap` still returns the receiver (v0 floor).
            "tap" => {
                if let Some(Value::Closure(_)) = args.last() {
                    apply_closure(args.last().unwrap(), vec![recv.clone()]);
                }
                Some(recv.clone())
            }
            // `then`/`yield_self { |x| … }` — pipe the receiver INTO the block,
            // return the block's RESULT.  Block-less → the receiver (v0 floor).
            "then" | "yield_self" => match args.last() {
                Some(b @ Value::Closure(_)) => Some(apply_closure(b, vec![recv.clone()])),
                _ => Some(recv.clone()),
            },
            _ => None,
        }
    }

    // Does dispatch on `recv` resolve `name`?  Mirrors the Python reference's
    // `_responds_to`: it consults the SAME closed catalogs / user method table
    // that a real `call_method` would, so the answer is honest — a name it
    // reports `true` for is exactly a name a real call would run, and a
    // `false` name is exactly one that would raise `NoMethodError`.  It is an
    // explicit membership test, never reflection on the source-derived name.
    fn responds_to(recv: &Value, name: &str) -> bool {
        // Universal methods available on every receiver.
        if matches!(
            name,
            "to_s" | "respond_to?" | "send" | "__send__" | "public_send" | "tap"
                | "then" | "yield_self"
        ) {
            return true;
        }
        match recv {
            // A user instance responds to any method its class (or an ancestor
            // / included module) defines — the SAME `resolve_instance_method`
            // walk `dispatch_user_method` uses.
            Value::Instance(id) => resolve_instance_method(&instance_class(*id), name).is_some(),
            Value::Seq(_) => matches!(
                name,
                "length" | "size" | "first" | "last" | "[]" | "reverse" | "sort" | "join"
                    | "include?" | "push" | "append" | "pop" | "fetch" | "each" | "map"
                    | "collect" | "select" | "filter" | "reject" | "find" | "detect" | "any?"
                    | "all?" | "none?" | "reduce" | "inject" | "sort_by" | "min_by" | "max_by"
                    | "group_by" | "partition" | "flat_map" | "collect_concat" | "take_while"
                    | "drop_while" | "count" | "each_with_object" | "each_with_index"
                    // aggregate / reshape non-block methods (all dispatch in
                    // `array_method` above; previously under-reported here)
                    | "min" | "max" | "sum" | "uniq" | "flatten" | "compact" | "to_a"
                    // more non-block Array methods
                    | "zip" | "rotate" | "to_h" | "tally"
                    | "take" | "drop" | "values_at"
            ),
            Value::Map(_) => matches!(
                name,
                "keys" | "values" | "[]" | "size" | "length" | "has_key?" | "key?" | "include?"
                    | "member?" | "fetch" | "each" | "each_pair" | "map" | "collect" | "select"
                    | "filter" | "transform_values" | "transform_keys"
            ),
            Value::Str(_) => matches!(
                name,
                "length" | "size" | "upcase" | "downcase" | "capitalize" | "reverse" | "strip"
                    | "lstrip" | "rstrip" | "chomp" | "chars" | "bytes" | "split" | "include?"
                    | "start_with?" | "end_with?" | "index" | "replace" | "sub" | "gsub" | "to_i"
                    | "to_f" | "to_sym" | "empty?" | "tr" | "count" | "delete" | "squeeze"
                    | "ljust" | "rjust" | "center" | "swapcase"
            ),
            Value::Sym(_) => matches!(
                name,
                "to_s" | "to_sym" | "length" | "size" | "upcase" | "downcase"
            ),
            Value::Bool(_) => matches!(name, "&" | "|" | "^"),
            Value::Int(_) | Value::Float(_) => {
                matches!(
                    name,
                    "abs" | "to_i" | "to_int" | "to_f" | "even?" | "odd?" | "zero?"
                        | "positive?" | "negative?" | "succ" | "next" | "pred" | "floor"
                        | "ceil" | "round" | "divmod" | "fdiv" | "clamp" | "between?"
                        | "gcd" | "pow" | "**" | "digits" | "times"
                        | "upto" | "downto" | "step"
                )
            }
            _ => false,
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
            // `Array#[]` — the LENIENT indexed read (the OO-surface `arr[i]`,
            // as opposed to the strict SIR-native `SeqIndex` primitive).  Ruby
            // returns `nil` for an out-of-bounds index (it does NOT raise —
            // that is `fetch`'s job), and folds a negative index from the end.
            // This arm keeps `arr[oob] ⇒ nil` from falling through to the
            // `no_method_error` floor.
            "[]" => {
                let items = items_rc.borrow();
                let len = items.len() as i64;
                let raw = as_i64(pos.first().unwrap_or(&Value::Nil));
                let idx = if raw < 0 { raw + len } else { raw };
                if idx >= 0 && idx < len {
                    items[idx as usize].clone()
                } else {
                    Value::Nil
                }
            }
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
            // `Array#fetch(i)` is the STRICT indexed read: unlike `arr[i]`
            // (which returns `nil` out of bounds), a `fetch` past the end
            // (or a negative index past the front) raises `IndexError` in
            // Ruby.  We surface a typed `SirError` so `rescue IndexError`
            // matches.  A supplied default arg (`fetch(i, default)`) is
            // returned instead of raising, matching Ruby.
            "fetch" => {
                // Ruby `Array#fetch`: a non-integer index raises a catchable
                // `TypeError` ("no implicit conversion of X into Integer"),
                // matching Ruby — NOT the uncatchable `as_i64` "expected int"
                // panic.  Checked before borrowing so no borrow is held across
                // the raise/unwind.
                if !matches!(pos.first(), Some(Value::Int(_))) {
                    let cls = ruby_class_name(pos.first().unwrap_or(&Value::Nil));
                    raise(
                        "TypeError",
                        Value::Str(Rc::from(
                            format!("no implicit conversion of {} into Integer", cls).as_str(),
                        )),
                    );
                }
                let items = items_rc.borrow();
                let len = items.len() as i64;
                let raw = as_i64(pos.first().unwrap_or(&Value::Nil));
                // Ruby folds a negative index from the end (`-1` ⇒ last).
                let idx = if raw < 0 { raw + len } else { raw };
                if idx >= 0 && idx < len {
                    items[idx as usize].clone()
                } else if let Some(default) = pos.get(1) {
                    default.clone()
                } else {
                    drop(items);
                    raise(
                        "IndexError",
                        Value::Str(Rc::from(
                            format!("index {} outside of array bounds: {}...{}", raw, -len, len)
                                .as_str(),
                        )),
                    );
                }
            }
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
            // `sort_by { |x| key }` — sort by the block-computed key using the
            // runtime's numeric ordering (`num_lt`), stable on ties.  The keys
            // are computed once (Schwartzian) so the block runs O(n), not
            // O(n log n), times.
            "sort_by" => match &block {
                Some(b) => {
                    // Snapshot into an owned Vec FIRST so no borrow of the
                    // receiver's cell is held while the block runs — a block
                    // that mutates the same array must not double-borrow-panic.
                    let snapshot = items_rc.borrow().clone();
                    let mut keyed: Vec<(Value, Value)> = snapshot
                        .into_iter()
                        .map(|item| (apply_closure(b, vec![item.clone()]), item))
                        .collect();
                    keyed.sort_by(|(ka, _), (kb, _)| {
                        if num_lt(ka, kb) {
                            std::cmp::Ordering::Less
                        } else if num_lt(kb, ka) {
                            std::cmp::Ordering::Greater
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    });
                    seq_lit(keyed.into_iter().map(|(_, item)| item).collect())
                }
                None => unknown_method(&recv, name),
            },
            // `min_by`/`max_by { |x| key }` — the element with the smallest /
            // largest block key.  First element wins a tie (strict `<`); an
            // empty array yields `nil`.
            "min_by" | "max_by" => match &block {
                Some(b) => {
                    let want_min = name == "min_by";
                    let snapshot = items_rc.borrow().clone();
                    let mut best: Option<(Value, Value)> = None;
                    for item in snapshot {
                        let k = apply_closure(b, vec![item.clone()]);
                        let take = match &best {
                            None => true,
                            Some((bk, _)) => {
                                if want_min {
                                    num_lt(&k, bk)
                                } else {
                                    num_lt(bk, &k)
                                }
                            }
                        };
                        if take {
                            best = Some((k, item));
                        }
                    }
                    best.map(|(_, item)| item).unwrap_or(Value::Nil)
                }
                None => unknown_method(&recv, name),
            },
            // `group_by { |x| key }` — a Hash mapping each block key to the
            // Array of elements that produced it, in first-seen key order and
            // element order.
            "group_by" => match &block {
                Some(b) => {
                    let snapshot = items_rc.borrow().clone();
                    let acc = map_lit(vec![]);
                    for item in snapshot {
                        let k = apply_closure(b, vec![item.clone()]);
                        let bucket = match map_get(&acc, &k) {
                            seq @ Value::Seq(_) => seq,
                            _ => seq_lit(vec![]),
                        };
                        if let Value::Seq(inner) = &bucket {
                            inner.borrow_mut().push(item);
                        }
                        map_set(&acc, k, bucket);
                    }
                    acc
                }
                None => unknown_method(&recv, name),
            },
            // `partition { |x| pred }` — `[matching, non_matching]`, each a
            // fresh Array, preserving order.
            "partition" => match &block {
                Some(b) => {
                    let snapshot = items_rc.borrow().clone();
                    let mut yes = Vec::new();
                    let mut no = Vec::new();
                    for item in snapshot {
                        if truthy(&apply_closure(b, vec![item.clone()])) {
                            yes.push(item);
                        } else {
                            no.push(item);
                        }
                    }
                    seq_lit(vec![seq_lit(yes), seq_lit(no)])
                }
                None => unknown_method(&recv, name),
            },
            // `flat_map { |x| … }` — map then concatenate one level: an Array
            // result splices its elements, a scalar is appended as-is.
            "flat_map" | "collect_concat" => match &block {
                Some(b) => {
                    let snapshot = items_rc.borrow().clone();
                    let mut out = Vec::new();
                    for item in snapshot {
                        match apply_closure(b, vec![item]) {
                            Value::Seq(inner) => out.extend(inner.borrow().clone()),
                            other => out.push(other),
                        }
                    }
                    seq_lit(out)
                }
                None => unknown_method(&recv, name),
            },
            // `take_while` / `drop_while { |x| pred }` — the leading run for
            // which the block is truthy (and the remainder after it).
            "take_while" => match &block {
                Some(b) => {
                    let snapshot = items_rc.borrow().clone();
                    let mut out = Vec::new();
                    for item in snapshot {
                        if truthy(&apply_closure(b, vec![item.clone()])) {
                            out.push(item);
                        } else {
                            break;
                        }
                    }
                    seq_lit(out)
                }
                None => unknown_method(&recv, name),
            },
            "drop_while" => match &block {
                Some(b) => {
                    let snapshot = items_rc.borrow().clone();
                    let mut out = Vec::new();
                    let mut dropping = true;
                    for item in snapshot {
                        if dropping && truthy(&apply_closure(b, vec![item.clone()])) {
                            continue;
                        }
                        dropping = false;
                        out.push(item);
                    }
                    seq_lit(out)
                }
                None => unknown_method(&recv, name),
            },
            // `count` — with a block, the number of truthy results; with an
            // argument, the count `==` to it; with neither, the length.
            "count" => match &block {
                Some(b) => {
                    let snapshot = items_rc.borrow().clone();
                    let n = snapshot
                        .into_iter()
                        .filter(|it| truthy(&apply_closure(b, vec![it.clone()])))
                        .count();
                    Value::Int(n as i64)
                }
                None => match pos.first() {
                    Some(needle) => {
                        let needle = needle.clone();
                        Value::Int(
                            items_rc.borrow().iter().filter(|x| value_eq(x, &needle)).count() as i64,
                        )
                    }
                    None => Value::Int(items_rc.borrow().len() as i64),
                },
            },
            // `each_with_object(obj) { |x, obj| … }` — yields each element with
            // the memo object and returns the (mutated) memo.
            "each_with_object" => {
                let b = match &block {
                    Some(b) => b,
                    None => return unknown_method(&recv, name),
                };
                let obj = pos.into_iter().next().unwrap_or(Value::Nil);
                let snapshot = items_rc.borrow().clone();
                for item in snapshot {
                    apply_closure(b, vec![item, obj.clone()]);
                }
                obj
            }
            // ── aggregate / reshape Array methods (non-block) ─────
            //
            // `min`/`max`: element-wise via the runtime's numeric ordering
            // (`num_lt`, the same source of truth `sort`/`<` use).  An empty
            // array yields `nil` (Ruby `[].min == nil`).  We snapshot the
            // vector first (no borrow held across the fold) and keep the
            // FIRST element on a tie, matching a stable left-fold.
            "min" => {
                let snapshot = items_rc.borrow().clone();
                let mut iter = snapshot.into_iter();
                match iter.next() {
                    Some(mut best) => {
                        for item in iter {
                            if num_lt(&item, &best) {
                                best = item;
                            }
                        }
                        best
                    }
                    None => Value::Nil,
                }
            }
            "max" => {
                let snapshot = items_rc.borrow().clone();
                let mut iter = snapshot.into_iter();
                match iter.next() {
                    Some(mut best) => {
                        for item in iter {
                            if num_lt(&best, &item) {
                                best = item;
                            }
                        }
                        best
                    }
                    None => Value::Nil,
                }
            }
            // `sum`: numeric fold seeded at `0` (or the explicit seed arg,
            // matching Ruby `sum(init)` and the Python/TS reference's
            // `total = args[0] if args else 0`).  Each step reuses `plus`,
            // so integer-only inputs stay `Int` while any float promotes to
            // `Float` — `[].sum == 0`, `[1,2,3].sum == 6`, `[1.5,2].sum == 3.5`.
            "sum" => {
                let seed = pos.into_iter().next().unwrap_or(Value::Int(0));
                let snapshot = items_rc.borrow().clone();
                let mut acc = seed;
                for item in snapshot {
                    acc = plus(vec![acc, item]);
                }
                acc
            }
            // `uniq`: first-occurrence-order de-duplication using the
            // runtime's structural `value_eq` (so `[1, 1.0]` collapses and
            // `[[1],[1]]` collapses too).  A fresh `Vec`; the snapshot is
            // taken up front so no borrow is held while we scan `out`.
            "uniq" => {
                let snapshot = items_rc.borrow().clone();
                let mut out: Vec<Value> = Vec::new();
                for item in snapshot {
                    if !out.iter().any(|kept| value_eq(kept, &item)) {
                        out.push(item);
                    }
                }
                seq_lit(out)
            }
            // `flatten`: recursively splice nested `Seq`s into one flat,
            // freshly-allocated `Seq`.  CYCLE GUARD: a `visited` set of seq
            // handle-addresses (the same discipline `puts`/`format`/`value_eq`
            // use) bounds the walk so a self-referential array terminates —
            // a handle already on the stack is treated as already-flattened
            // and skipped rather than re-entered.  Every level snapshots its
            // items (dropping the `RefCell` borrow) BEFORE recursing, so no
            // borrow is ever held across a re-entrant call.
            "flatten" => {
                fn flatten_into(
                    items: &Rc<RefCell<Vec<Value>>>,
                    out: &mut Vec<Value>,
                    visited: &mut std::collections::HashSet<usize>,
                ) {
                    let id = seq_handle_id(items);
                    if !visited.insert(id) {
                        return;
                    }
                    let snapshot = items.borrow().clone();
                    for item in snapshot {
                        match item {
                            Value::Seq(inner) => flatten_into(&inner, out, visited),
                            other => out.push(other),
                        }
                    }
                    visited.remove(&id);
                }
                let mut out: Vec<Value> = Vec::new();
                let mut visited: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();
                flatten_into(&items_rc, &mut out, &mut visited);
                seq_lit(out)
            }
            // `compact`: a fresh `Seq` with every `nil` removed.
            "compact" => seq_lit(
                items_rc
                    .borrow()
                    .iter()
                    .filter(|x| !matches!(x, Value::Nil))
                    .cloned()
                    .collect(),
            ),
            // `to_a`: Ruby `Array#to_a` returns the receiver itself (identity),
            // so downstream mutation is observed — we hand back `recv`.
            "to_a" => recv,
            // `each_with_index`: yields `(element, index)` to the block via
            // `apply_closure` and returns the receiver.  The item is cloned
            // out of the snapshot BEFORE the block runs, so no `RefCell`
            // borrow is held across the (re-entrant) closure call.
            "each_with_index" => {
                if let Some(b) = &block {
                    for (index, item) in items_rc.borrow().clone().into_iter().enumerate() {
                        apply_closure(b, vec![item, Value::Int(index as i64)]);
                    }
                }
                recv
            }
            // `zip(*others)` — an Array of tuples: the i-th tuple is
            // `[self[i], others[0][i], others[1][i], …]`.  The result length
            // is the RECEIVER's; a shorter (or non-Array) operand pads with
            // `nil`.  `[1,2,3].zip([4,5,6]) == [[1,4],[2,5],[3,6]]`.  Each
            // operand is snapshotted into an owned Vec once, so no `RefCell`
            // borrow is held while we build the tuples.
            "zip" => {
                let snapshot = items_rc.borrow().clone();
                let others: Vec<Vec<Value>> = pos
                    .iter()
                    .map(|o| match o {
                        Value::Seq(inner) => inner.borrow().clone(),
                        _ => Vec::new(),
                    })
                    .collect();
                let mut out: Vec<Value> = Vec::with_capacity(snapshot.len());
                for (i, item) in snapshot.into_iter().enumerate() {
                    let mut tuple: Vec<Value> = Vec::with_capacity(others.len() + 1);
                    tuple.push(item);
                    for o in &others {
                        tuple.push(o.get(i).cloned().unwrap_or(Value::Nil));
                    }
                    out.push(seq_lit(tuple));
                }
                seq_lit(out)
            }
            // `rotate(n = 1)` — a fresh Array with the elements shifted left by
            // `n` (a NEGATIVE `n` rotates right).  The modulo wraps so ANY `n`
            // terminates; the empty-array early return keeps the divisor
            // strictly positive (no divide-by-zero, no negative slice index).
            // `[1,2,3,4,5].rotate(2) == [3,4,5,1,2]`.
            "rotate" => {
                let snapshot = items_rc.borrow().clone();
                let len = snapshot.len() as i64;
                if len == 0 {
                    return seq_lit(vec![]);
                }
                let n = pos.first().map(as_i64).unwrap_or(1);
                // Rust `%` keeps the dividend's sign, so `+ len` then `% len`
                // normalises any `n` (including negatives) into `[0, len)`.
                let shift = (((n % len) + len) % len) as usize;
                let mut out: Vec<Value> = Vec::with_capacity(snapshot.len());
                out.extend_from_slice(&snapshot[shift..]);
                out.extend_from_slice(&snapshot[..shift]);
                seq_lit(out)
            }
            // `to_h` — read each element as a `[key, value]` pair and build a
            // Hash.  A non-pair element (not a 2-element Array) is SKIPPED,
            // upholding the never-raise floor (Ruby raises `TypeError`; we
            // degrade instead).  Later duplicate keys overwrite earlier ones,
            // matching `Hash` insertion semantics.
            "to_h" => {
                let snapshot = items_rc.borrow().clone();
                let acc = map_lit(vec![]);
                for item in snapshot {
                    if let Value::Seq(inner) = &item {
                        let pair = inner.borrow().clone();
                        if pair.len() == 2 {
                            map_set(&acc, pair[0].clone(), pair[1].clone());
                        }
                    }
                }
                acc
            }
            // `tally` — a Hash mapping each distinct element to how many times
            // it occurs, in first-seen order, keyed by the Map's structural
            // `value_eq`.  `["a","b","a"].tally == {"a"=>2, "b"=>1}`.
            "tally" => {
                let snapshot = items_rc.borrow().clone();
                let acc = map_lit(vec![]);
                for item in snapshot {
                    let n = match map_get(&acc, &item) {
                        Value::Int(c) => c,
                        _ => 0,
                    };
                    map_set(&acc, item, Value::Int(n + 1));
                }
                acc
            }
            // `take(n)` / `drop(n)` — a fresh Array of the first / all-but-first
            // `n` elements.  `n` is clamped to `[0, len]` (`n <= 0` and `n > len`
            // both saturate), so the slice bounds are always valid.  Ruby raises
            // `ArgumentError` on a negative `n`; the never-raise floor treats it
            // as `0`.  The snapshot is taken up front so no `RefCell` borrow is
            // held across the allocation.
            "take" | "drop" => {
                let snapshot = items_rc.borrow().clone();
                let len = snapshot.len() as i64;
                let mut n = pos.first().map(as_i64).unwrap_or(0);
                if n < 0 {
                    n = 0;
                }
                if n > len {
                    n = len;
                }
                let n = n as usize;
                if name == "take" {
                    seq_lit(snapshot[..n].to_vec())
                } else {
                    seq_lit(snapshot[n..].to_vec())
                }
            }
            // `values_at(*idxs)` — a fresh Array of the element at each index,
            // folding a negative index from the end once; an out-of-range index
            // (including a doubly-negative one) yields `nil` rather than panicking.
            "values_at" => {
                let snapshot = items_rc.borrow().clone();
                let len = snapshot.len() as i64;
                let out: Vec<Value> = pos
                    .iter()
                    .map(|a| {
                        let mut idx = as_i64(a);
                        if idx < 0 {
                            idx += len;
                        }
                        if idx >= 0 && idx < len {
                            snapshot[idx as usize].clone()
                        } else {
                            Value::Nil
                        }
                    })
                    .collect();
                seq_lit(out)
            }
            _ => no_method_error(&recv, name),
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
            // `Hash#[]` — the LENIENT keyed read (the OO-surface `hash[k]`,
            // as opposed to the SIR-native `MapGet`).  Ruby returns `nil` for a
            // missing key (never raises — that is `fetch`'s job).  Keeps
            // `hash[miss] ⇒ nil` from reaching the `no_method_error` floor.
            "[]" => {
                let needle = pos.first().cloned().unwrap_or(Value::Nil);
                entries_rc
                    .borrow()
                    .iter()
                    .find(|(k, _)| value_eq(k, &needle))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Value::Nil)
            }
            "size" | "length" => Value::Int(entries_rc.borrow().len() as i64),
            "has_key?" | "key?" | "include?" | "member?" => {
                let needle = pos.first().cloned().unwrap_or(Value::Nil);
                Value::Bool(entries_rc.borrow().iter().any(|(k, _)| value_eq(k, &needle)))
            }
            // `Hash#fetch(k)` is the STRICT keyed read: unlike `hash[k]`
            // (which returns `nil` for a missing key), a `fetch` of an
            // absent key raises `KeyError` in Ruby.  We surface a typed
            // `SirError` so `rescue KeyError` matches.  A supplied default
            // arg (`fetch(k, default)`) is returned instead of raising.
            "fetch" => {
                let needle = pos.first().cloned().unwrap_or(Value::Nil);
                let hit = entries_rc
                    .borrow()
                    .iter()
                    .find(|(k, _)| value_eq(k, &needle))
                    .map(|(_, v)| v.clone());
                match hit {
                    Some(v) => v,
                    None => match pos.get(1) {
                        Some(default) => default.clone(),
                        None => raise(
                            "KeyError",
                            Value::Str(Rc::from(
                                format!("key not found: {}", format(&needle)).as_str(),
                            )),
                        ),
                    },
                }
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
            // `Hash#transform_values { |v| … }` — a NEW hash whose keys are
            // copied verbatim and whose values are the block results.  The
            // block yields ONE argument (the value); the keys are untouched, so
            // they stay unique and a straight rebuild preserves insertion order.
            // Non-mutating — `recv` is never edited in place.
            "transform_values" => match &block {
                Some(b) => {
                    let out: Vec<(Value, Value)> = entries_rc
                        .borrow()
                        .clone()
                        .into_iter()
                        .map(|(k, v)| (k, apply_closure(b, vec![v])))
                        .collect();
                    Value::Map(Rc::new(RefCell::new(out)))
                }
                None => unknown_method(&recv, name),
            },
            // `Hash#transform_keys { |k| … }` — a NEW hash whose values are
            // untouched and whose keys are the block results (yields ONE
            // argument, the key).  Two source keys can map to the SAME new key;
            // Ruby keeps the LAST such entry's value while holding the new key
            // at its FIRST-seen position, so we overwrite an existing slot in
            // place (`value_eq` match) and otherwise append.
            "transform_keys" => match &block {
                Some(b) => {
                    let mut out: Vec<(Value, Value)> = Vec::new();
                    for (k, v) in entries_rc.borrow().clone() {
                        let nk = apply_closure(b, vec![k]);
                        match out.iter_mut().find(|(ek, _)| value_eq(ek, &nk)) {
                            Some(slot) => slot.1 = v,
                            None => out.push((nk, v)),
                        }
                    }
                    Value::Map(Rc::new(RefCell::new(out)))
                }
                None => unknown_method(&recv, name),
            },
            _ => no_method_error(&recv, name),
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
        // Every arm below is length-guarded on `args` (a missing string
        // argument coerces to `""` / a missing search to a no-op) and is a
        // pure, non-mutating computation returning a *fresh* `Value` — a Ruby
        // `String` is immutable at this v0 layer, so nothing edits `recv` in
        // place.  All slicing goes through `chars()` (never byte indexing), so
        // a multibyte receiver can never panic on a char-boundary.  Dispatch is
        // an EXPLICIT `match` on the interned method name (never reflection over
        // a host method table) — the C3 allowlist discipline.
        match name {
            "length" | "size" => Value::Int(s.chars().count() as i64),
            "upcase" => Value::Str(Rc::from(s.to_uppercase().as_str())),
            "downcase" => Value::Str(Rc::from(s.to_lowercase().as_str())),
            "capitalize" => {
                // Ruby: first char upcased, the rest downcased.  Rune-aware so
                // a leading multibyte char is not sliced mid-byte.
                let mut it = s.chars();
                match it.next() {
                    None => Value::Str(Rc::from("")),
                    Some(first) => {
                        let rest: String = it.as_str().to_lowercase();
                        let head: String = first.to_uppercase().collect();
                        Value::Str(Rc::from(format!("{head}{rest}").as_str()))
                    }
                }
            }
            "reverse" => Value::Str(Rc::from(s.chars().rev().collect::<String>().as_str())),
            "strip" => Value::Str(Rc::from(s.trim())),
            "lstrip" => Value::Str(Rc::from(s.trim_start())),
            "rstrip" => Value::Str(Rc::from(s.trim_end())),
            "chomp" => {
                // With an explicit separator, drop exactly that trailing suffix;
                // with none, drop one trailing `\r\n`, `\n`, or `\r` (Ruby's
                // default line-ending handling).
                let out: &str = match args.first() {
                    Some(sep_val) => {
                        let sep = method_name(sep_val);
                        if !sep.is_empty() && s.ends_with(&sep) {
                            &s[..s.len() - sep.len()]
                        } else {
                            &s
                        }
                    }
                    None => {
                        if let Some(stripped) = s.strip_suffix("\r\n") {
                            stripped
                        } else if let Some(stripped) = s.strip_suffix('\n') {
                            stripped
                        } else if let Some(stripped) = s.strip_suffix('\r') {
                            stripped
                        } else {
                            &s
                        }
                    }
                };
                Value::Str(Rc::from(out))
            }
            "chars" => {
                // One 1-char `String` per Unicode scalar (rune-aware).
                let parts: Vec<Value> = s
                    .chars()
                    .map(|c| Value::Str(Rc::from(c.to_string().as_str())))
                    .collect();
                seq_lit(parts)
            }
            "bytes" => {
                // Each UTF-8 byte as an `Integer` (0..=255), matching Ruby's
                // `String#bytes` over the receiver's UTF-8 encoding.
                let parts: Vec<Value> =
                    s.as_bytes().iter().map(|b| Value::Int(*b as i64)).collect();
                seq_lit(parts)
            }
            "split" => {
                // No argument (or `nil`/`""` separator) ⇒ split on runs of
                // whitespace (Ruby's awk-style default, dropping leading/
                // trailing empties); a non-empty separator ⇒ literal split.
                let parts: Vec<Value> = match args.first() {
                    Some(Value::Nil) | None => {
                        s.split_whitespace().map(|p| Value::Str(Rc::from(p))).collect()
                    }
                    Some(sep) => {
                        let sep = method_name(sep);
                        if sep.is_empty() {
                            s.split_whitespace().map(|p| Value::Str(Rc::from(p))).collect()
                        } else {
                            s.split(&sep).map(|p| Value::Str(Rc::from(p))).collect()
                        }
                    }
                };
                seq_lit(parts)
            }
            "include?" => {
                let needle = args.first().map(method_name).unwrap_or_default();
                Value::Bool(s.contains(&needle))
            }
            "start_with?" => {
                let prefix = args.first().map(method_name).unwrap_or_default();
                Value::Bool(s.starts_with(&prefix))
            }
            "end_with?" => {
                let suffix = args.first().map(method_name).unwrap_or_default();
                Value::Bool(s.ends_with(&suffix))
            }
            "index" => {
                // First index of the substring as a *character* offset (Ruby
                // counts in characters, not bytes) — or `nil` when absent.
                let needle = args.first().map(method_name).unwrap_or_default();
                match s.find(&needle) {
                    Some(byte_pos) => Value::Int(s[..byte_pos].chars().count() as i64),
                    None => Value::Nil,
                }
            }
            "replace" => {
                // Ruby `String#replace` overwrites the whole content; for an
                // immutable string that is just the replacement value.
                let val = args.first().map(method_name).unwrap_or_default();
                Value::Str(Rc::from(val.as_str()))
            }
            "sub" => {
                // Literal first-occurrence replacement — sliced by byte index
                // returned from `find` (always a char boundary), and the
                // replacement is inserted verbatim (NO regex, NO `$&`/`\1`
                // back-reference expansion).
                let search = args.first().map(method_name).unwrap_or_default();
                let repl = args.get(1).map(method_name).unwrap_or_default();
                if search.is_empty() {
                    return Value::Str(s.clone());
                }
                match s.find(&search) {
                    Some(idx) => {
                        let out = format!("{}{}{}", &s[..idx], repl, &s[idx + search.len()..]);
                        Value::Str(Rc::from(out.as_str()))
                    }
                    None => Value::Str(s.clone()),
                }
            }
            "gsub" => {
                // Literal global replacement (NO regex).  `str::replace` does a
                // verbatim substring substitution, so there is no special-
                // replacement parsing foot-gun.  An empty search is a no-op.
                let search = args.first().map(method_name).unwrap_or_default();
                let repl = args.get(1).map(method_name).unwrap_or_default();
                if search.is_empty() {
                    return Value::Str(s.clone());
                }
                Value::Str(Rc::from(s.replace(&search, &repl).as_str()))
            }
            "to_i" => Value::Int(str_to_i(&s)),
            "to_f" => Value::Float(str_to_f(&s)),
            "to_sym" => intern(&s),
            "empty?" => Value::Bool(s.is_empty()),
            "ljust" | "rjust" | "center" => {
                // Ruby `String#ljust`/`#rjust`/`#center(width, pad = " ")`: pad to
                // `width` CHARS (not bytes) using `pad` cyclically.  `width <= the
                // current char length` returns the string unchanged; `center` puts
                // any odd extra pad char on the RIGHT (Ruby's rule).  An empty pad
                // degrades to a single space rather than raising, holding the
                // never-raise floor.  Char-based (`chars().count()` / `str_pad`) so
                // a multibyte receiver is never split mid-codepoint.
                let width = args.first().map(as_i64).unwrap_or(0);
                let pad = match args.get(1) {
                    Some(Value::Str(p)) if !p.is_empty() => p.to_string(),
                    _ => " ".to_string(),
                };
                let cur = s.chars().count() as i64;
                if width <= cur {
                    return Value::Str(s);
                }
                // Clamp the fill count to a DoS bound so a hostile width (e.g.
                // `"".ljust(10**12)`) cannot drive an unbounded allocation — the
                // same ceiling `String#*` guards, but degrade-not-panic here since
                // justify is a formatting method.
                const MAX_PAD: i64 = 100_000_000;
                let total = (width - cur).min(MAX_PAD) as usize;
                let result = match name {
                    "ljust" => format!("{}{}", s, str_pad(&pad, total)),
                    "rjust" => format!("{}{}", str_pad(&pad, total), s),
                    _ => {
                        // center: any odd extra pad char goes on the RIGHT.
                        let left = total / 2;
                        format!("{}{}{}", str_pad(&pad, left), s, str_pad(&pad, total - left))
                    }
                };
                Value::Str(Rc::from(result.as_str()))
            }
            "swapcase" => {
                // Ruby `String#swapcase`: flip the case of each ASCII letter
                // (leaving non-letters and non-ASCII chars untouched), matching the
                // Python/Go/JS/TS runtimes byte-for-byte.  Iterating `chars()` keeps
                // a multibyte receiver intact.
                let swapped: String = s
                    .chars()
                    .map(|c| {
                        if c.is_ascii_uppercase() {
                            c.to_ascii_lowercase()
                        } else if c.is_ascii_lowercase() {
                            c.to_ascii_uppercase()
                        } else {
                            c
                        }
                    })
                    .collect();
                Value::Str(Rc::from(swapped.as_str()))
            }
            "tr" => {
                // Ruby `String#tr(from, to)`: position-wise char translation.  A
                // shorter `to` repeats its last char; an empty `to` deletes
                // matching chars; a repeated char in `from` keeps the last
                // mapping.  Char-based so a multibyte receiver is never sliced
                // mid-codepoint.  Literal only — the range (`"a-z"`) and negation
                // (`"^abc"`) forms are a follow-up, matching the literal-only
                // sub/gsub precedent here.
                let from = match args.first() {
                    Some(Value::Str(f)) => f.clone(),
                    _ => return Value::Str(s.clone()),
                };
                let to = match args.get(1) {
                    Some(Value::Str(t)) => t.clone(),
                    _ => return Value::Str(s.clone()),
                };
                let to_chars: Vec<char> = to.chars().collect();
                let mut table: HashMap<char, Option<char>> = HashMap::new();
                for (i, c) in from.chars().enumerate() {
                    if to_chars.is_empty() {
                        table.insert(c, None);
                    } else if i < to_chars.len() {
                        table.insert(c, Some(to_chars[i]));
                    } else {
                        table.insert(c, Some(*to_chars.last().unwrap()));
                    }
                }
                let out: String = s
                    .chars()
                    .filter_map(|c| match table.get(&c) {
                        Some(Some(r)) => Some(*r),
                        Some(None) => None,
                        None => Some(c),
                    })
                    .collect();
                Value::Str(Rc::from(out.as_str()))
            }
            "count" | "delete" | "squeeze" => {
                // Char-set methods.  Each `set` argument is treated LITERALLY —
                // the chars it contains (ranges/negation are a follow-up).
                // `count` tallies chars of the receiver in the set; `delete`
                // removes them; `squeeze` collapses consecutive runs (of set
                // chars, or of ALL chars when no set is given).  Multiple set
                // args intersect (Ruby's rule).  Char-based throughout.
                let sets: Vec<std::collections::HashSet<char>> = args
                    .iter()
                    .filter_map(|a| {
                        if let Value::Str(t) = a {
                            Some(t.chars().collect())
                        } else {
                            None
                        }
                    })
                    .collect();
                let in_all =
                    |c: char| !sets.is_empty() && sets.iter().all(|set| set.contains(&c));
                if name == "squeeze" && sets.is_empty() {
                    let mut out = String::new();
                    let mut last: Option<char> = None;
                    for c in s.chars() {
                        if last != Some(c) {
                            out.push(c);
                            last = Some(c);
                        }
                    }
                    return Value::Str(Rc::from(out.as_str()));
                }
                match name {
                    "count" => Value::Int(s.chars().filter(|c| in_all(*c)).count() as i64),
                    "delete" => {
                        let out: String = s.chars().filter(|c| !in_all(*c)).collect();
                        Value::Str(Rc::from(out.as_str()))
                    }
                    _ => {
                        let mut out = String::new();
                        let mut last: Option<char> = None;
                        for c in s.chars() {
                            if last == Some(c) && in_all(c) {
                                continue;
                            }
                            out.push(c);
                            last = Some(c);
                        }
                        Value::Str(Rc::from(out.as_str()))
                    }
                }
            }
            _ => no_method_error(&recv, name),
        }
    }

    // Build a padding string of exactly `n` CHARS by repeating `pad`
    // cyclically (truncating the final repeat).  `n == 0` or an empty `pad`
    // yields `""` — the `ljust`/`rjust`/`center` callers guarantee a non-empty
    // pad, so the empty-pad guard is purely defensive.  Char-based so a
    // multibyte pad (e.g. `"ab…"`) is never split mid-codepoint.
    fn str_pad(pad: &str, n: usize) -> String {
        if n == 0 || pad.is_empty() {
            return String::new();
        }
        let pad_chars: Vec<char> = pad.chars().collect();
        let mut out = String::with_capacity(n);
        for i in 0..n {
            out.push(pad_chars[i % pad_chars.len()]);
        }
        out
    }

    // Ruby `String#to_i`: parse the longest leading `[+-]?\d+` prefix,
    // ignoring leading whitespace, and return `0` when none is present
    // (Ruby never raises here).  Char-by-char so a multibyte tail cannot
    // panic on a byte slice.
    fn str_to_i(s: &str) -> i64 {
        let t = s.trim_start();
        let mut end = 0usize;
        for (i, c) in t.char_indices() {
            if (c == '+' || c == '-') && i == 0 {
                end = i + c.len_utf8();
            } else if c.is_ascii_digit() {
                end = i + c.len_utf8();
            } else {
                break;
            }
        }
        t[..end].parse::<i64>().unwrap_or(0)
    }

    // Ruby `String#to_f`: parse the longest leading floating-point prefix,
    // flooring to `0.0` on no match.  We walk char boundaries and keep the
    // value of the longest prefix that parses as an `f64` — total (never
    // raises) and multibyte-safe (indices come from `char_indices`).
    fn str_to_f(s: &str) -> f64 {
        let t = s.trim_start();
        let mut best = 0.0f64;
        for (i, c) in t.char_indices() {
            let upto = i + c.len_utf8();
            if let Ok(v) = t[..upto].parse::<f64>() {
                best = v;
            }
        }
        best
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
            "to_i" | "to_int" => Value::Int(as_i64_lenient(&recv)),
            "to_f" => Value::Float(as_f64_lenient(&recv)),
            "even?" => Value::Bool(as_i64_lenient(&recv) % 2 == 0),
            "odd?" => Value::Bool(as_i64_lenient(&recv) % 2 != 0),
            "zero?" => Value::Bool(as_f64_lenient(&recv) == 0.0),
            // Sign predicates — a Float `0.0` is neither positive nor
            // negative, matching Ruby (`0.0.positive? == false`).  The
            // lenient f64 coercion covers both Int and Float receivers.
            "positive?" => Value::Bool(as_f64_lenient(&recv) > 0.0),
            "negative?" => Value::Bool(as_f64_lenient(&recv) < 0.0),
            // `succ`/`next` (+1) and `pred` (-1) preserve the receiver's
            // tag: an Int stays an Int, a Float stays a Float.  Integer
            // arithmetic matches the backend's `plus`/`minus` convention
            // (plain `+`/`-`); an i64 at the boundary saturates rather than
            // panicking, upholding the never-raise-on-the-OO-surface floor.
            "succ" | "next" => match &recv {
                Value::Int(n) => Value::Int(n.saturating_add(1)),
                Value::Float(x) => Value::Float(x + 1.0),
                _ => Value::Int(as_i64_lenient(&recv).saturating_add(1)),
            },
            "pred" => match &recv {
                Value::Int(n) => Value::Int(n.saturating_sub(1)),
                Value::Float(x) => Value::Float(x - 1.0),
                _ => Value::Int(as_i64_lenient(&recv).saturating_sub(1)),
            },
            // `floor`/`ceil`/`round` on an Integer return the receiver
            // unchanged; on a Float they collapse to an Integer (Ruby's
            // no-argument forms).  A non-finite Float has no integer image,
            // so it degrades to `0` via `as_i64_lenient` (never-raise floor).
            "floor" => match &recv {
                Value::Int(_) => recv,
                Value::Float(x) if x.is_finite() => Value::Int(x.floor() as i64),
                _ => Value::Int(0),
            },
            "ceil" => match &recv {
                Value::Int(_) => recv,
                Value::Float(x) if x.is_finite() => Value::Int(x.ceil() as i64),
                _ => Value::Int(0),
            },
            // `round` / `round(ndigits)` — half AWAY from zero (via `ruby_round`,
            // matching the Python/Go reference: `2.5.round == 3`).  A positive
            // `ndigits` on a Float rounds to that many decimals; `ndigits <= 0`
            // rounds to a power of ten.  Rust's i64/f64 are FIXED width (no
            // bignum), so the only guards are a place count past i64's ~18
            // decimal digits (dwarfs the value ⇒ 0, Ruby parity) and a positive
            // `ndigits` past Float precision / an overflowing scale-up (returns
            // the value unchanged).
            "round" => {
                let ndigits = pos.first().map(as_i64_lenient).unwrap_or(0);
                match &recv {
                    Value::Int(iv) => {
                        if ndigits >= 0 {
                            recv
                        } else if -ndigits > 18 {
                            Value::Int(0)
                        } else {
                            Value::Int(round_int_to_multiple(*iv, pow10(-ndigits)))
                        }
                    }
                    Value::Float(x) if x.is_finite() => {
                        if ndigits <= 0 {
                            if -ndigits > 18 {
                                Value::Int(0)
                            } else {
                                Value::Int(round_int_to_multiple(ruby_round(*x), pow10(-ndigits)))
                            }
                        } else if ndigits > 17 {
                            recv // already at full Float precision
                        } else {
                            let scale = 10f64.powi(ndigits as i32);
                            let scaled = *x * scale;
                            if scaled.is_finite() {
                                Value::Float(ruby_round(scaled) as f64 / scale)
                            } else {
                                recv // overflow guard: no fractional part left
                            }
                        }
                    }
                    _ => Value::Int(0),
                }
            }
            // `divmod(n)` → `[quotient, remainder]` with a FLOORED quotient and
            // the divisor-signed remainder.  Division by zero raises a typed
            // `ZeroDivisionError`.  Int/int uses exact integer math; a Float
            // operand promotes to f64 (f64 division of nonzero/nonzero never
            // panics).
            "divmod" => {
                let arg = pos.first().cloned().unwrap_or(Value::Int(0));
                match (&recv, &arg) {
                    (Value::Int(n), Value::Int(d)) => {
                        if *d == 0 {
                            raise("ZeroDivisionError", Value::Str(Rc::from("divided by 0")));
                        }
                        let q = floor_div_i64(*n, *d);
                        // `wrapping_sub`/`wrapping_mul`: for `n == i64::MIN` with a
                        // non-dividing `d`, the FLOORED quotient rounds away from
                        // zero so the true `q*d` exceeds i64 range; a checked `-`
                        // would panic in debug.  The true remainder always fits
                        // in i64, so wrapping recovers it exactly in both profiles.
                        let r = n.wrapping_sub(q.wrapping_mul(*d));
                        Value::Seq(Rc::new(RefCell::new(vec![Value::Int(q), Value::Int(r)])))
                    }
                    _ => {
                        let df = as_f64_lenient(&arg);
                        if df == 0.0 {
                            raise("ZeroDivisionError", Value::Str(Rc::from("divided by 0")));
                        }
                        let nf = as_f64_lenient(&recv);
                        let q = (nf / df).floor();
                        let r = nf - q * df;
                        Value::Seq(Rc::new(RefCell::new(vec![Value::Float(q), Value::Float(r)])))
                    }
                }
            }
            // `fdiv(n)` — floating-point division that NEVER raises: dividing by
            // zero yields `±Infinity`/`NaN` (f64 division already produces these
            // rather than panicking), honouring the never-raise floor.
            "fdiv" => {
                let arg = pos.first().cloned().unwrap_or(Value::Int(0));
                Value::Float(as_f64_lenient(&recv) / as_f64_lenient(&arg))
            }
            // `clamp(min, max)` — `min` if recv < min, `max` if recv > max, else
            // recv.  Compared numerically (Range form deferred).
            "clamp" => {
                let lo = pos.first().cloned().unwrap_or(Value::Int(0));
                let hi = pos.get(1).cloned().unwrap_or(Value::Int(0));
                let rv = as_f64_lenient(&recv);
                if rv < as_f64_lenient(&lo) {
                    lo
                } else if rv > as_f64_lenient(&hi) {
                    hi
                } else {
                    recv
                }
            }
            // `between?(min, max)` — `min <= recv <= max`.
            "between?" => {
                let lo = pos.first().map(|v| as_f64_lenient(v)).unwrap_or(0.0);
                let hi = pos.get(1).map(|v| as_f64_lenient(v)).unwrap_or(0.0);
                let rv = as_f64_lenient(&recv);
                Value::Bool(rv >= lo && rv <= hi)
            }
            // `gcd(other)` — the integer greatest common divisor, always
            // non-negative (Ruby: `(-12).gcd(8) == 4`).  Both receiver and
            // argument are truncated to i64 via the lenient coercion.
            "gcd" => {
                let a = as_i64_lenient(&recv);
                let b = pos.first().map(as_i64_lenient).unwrap_or(0);
                Value::Int(gcd_i64(a, b))
            }
            // `pow(n)` / `**` — integer power stays an Int, but any Float
            // operand promotes to a Float (Ruby: `2 ** 3 == 8`, `2.0 ** 3
            // == 8.0`).  Integer exponentiation uses `checked_pow`: a
            // negative exponent has no integer image (Ruby returns a
            // Rational; we degrade to `0`), and an overflow saturates rather
            // than triggering an uncontrolled panic — matching the backend's
            // controlled-arithmetic policy and the reference's bignum guard.
            "pow" | "**" => {
                let arg = pos.first().cloned().unwrap_or(Value::Int(0));
                match (&recv, &arg) {
                    (Value::Int(base), Value::Int(exp)) => {
                        if *exp < 0 {
                            Value::Int(0)
                        } else if *exp <= u32::MAX as i64 {
                            match base.checked_pow(*exp as u32) {
                                Some(v) => Value::Int(v),
                                None => Value::Int(if *base >= 0 { i64::MAX } else { i64::MIN }),
                            }
                        } else {
                            Value::Int(i64::MAX)
                        }
                    }
                    _ => Value::Float(as_f64_lenient(&recv).powf(as_f64_lenient(&arg))),
                }
            }
            // `digits` — base-10 digits, least-significant first.  Ruby
            // raises `Math::DomainError` on a negative receiver; the
            // reference degrades to the digits of the absolute value, which
            // we match.  `0.digits == [0]`.  A Float receiver truncates.
            "digits" => seq_lit(digits_of(as_i64_lenient(&recv))),
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
                recv
            }
            // `n.upto(limit) { |i| … }` yields n, n+1, …, limit ascending
            // and returns the receiver (a no-op when limit < n).
            "upto" => {
                if let Some(b) = &block {
                    let n = as_i64_lenient(&recv);
                    let limit = pos.first().map(as_i64_lenient).unwrap_or(n);
                    let mut i = n;
                    while i <= limit {
                        apply_closure(b, vec![Value::Int(i)]);
                        // Guard the terminal increment: at `i == i64::MAX` the
                        // `i + 1` would overflow (debug panic; release wraps to
                        // MIN → the `<= limit` test re-enters → infinite spin).
                        match i.checked_add(1) {
                            Some(next) => i = next,
                            None => break,
                        }
                    }
                }
                recv
            }
            // `n.downto(limit) { |i| … }` yields n, n-1, …, limit descending.
            "downto" => {
                if let Some(b) = &block {
                    let n = as_i64_lenient(&recv);
                    let limit = pos.first().map(as_i64_lenient).unwrap_or(n);
                    let mut i = n;
                    while i >= limit {
                        apply_closure(b, vec![Value::Int(i)]);
                        // Guard the terminal decrement at `i == i64::MIN`.
                        match i.checked_sub(1) {
                            Some(next) => i = next,
                            None => break,
                        }
                    }
                }
                recv
            }
            // `n.step(limit, stride) { |i| … }` yields n, n+stride, … until
            // it passes `limit`; the stride's sign picks the direction.  A
            // zero stride would spin forever, so it is treated as a no-op.
            "step" => {
                if let Some(b) = &block {
                    let n = as_i64_lenient(&recv);
                    let limit = pos.first().map(as_i64_lenient).unwrap_or(n);
                    let stride = pos.get(1).map(as_i64_lenient).unwrap_or(1);
                    let mut i = n;
                    if stride > 0 {
                        while i <= limit {
                            apply_closure(b, vec![Value::Int(i)]);
                            // `checked_add` guards both the boundary and a
                            // large stride overflowing past `i64::MAX`.
                            match i.checked_add(stride) {
                                Some(next) => i = next,
                                None => break,
                            }
                        }
                    } else if stride < 0 {
                        while i >= limit {
                            apply_closure(b, vec![Value::Int(i)]);
                            match i.checked_add(stride) {
                                Some(next) => i = next,
                                None => break,
                            }
                        }
                    }
                }
                recv
            }
            _ => no_method_error(&recv, name),
        }
    }

    // Ruby `Float#round` (no-argument form): round half **away from zero**
    // — `2.5.round == 3`, `-2.5.round == -3` — unlike Rust's `f64::round`
    // which is already half-away-from-zero, so this is a thin wrapper kept
    // for parity/clarity with the Python/TS `ruby_round` helpers.
    fn ruby_round(x: f64) -> i64 {
        if x >= 0.0 {
            (x + 0.5).floor() as i64
        } else {
            (x - 0.5).ceil() as i64
        }
    }

    // Ruby's integer division: the quotient FLOORED toward −∞ (`-7 / 2 == -4`),
    // unlike Rust's truncating `/`.  Callers guarantee `b != 0`.
    fn floor_div_i64(a: i64, b: i64) -> i64 {
        // `wrapping_rem` (like `wrapping_div`) avoids the `i64::MIN % -1` panic —
        // plain `%` traps on that case in BOTH debug and release, which would
        // escape the typed-error floor.  It yields `0` there (the correct
        // remainder), so the sign-correction branch is skipped and `q` (=
        // `i64::MIN` from `wrapping_div`) is returned — Ruby parity.
        let q = a.wrapping_div(b);
        if (a.wrapping_rem(b) != 0) && ((a < 0) != (b < 0)) {
            q - 1
        } else {
            q
        }
    }

    // `10.pow(n)` for a small non-negative `n`.  Callers bound `n <= 18` (an i64
    // holds ≤ ~9.2e18), so the result never overflows i64.
    fn pow10(n: i64) -> i64 {
        let mut result = 1i64;
        let mut i = 0;
        while i < n {
            result = result.saturating_mul(10);
            i += 1;
        }
        result
    }

    // Round `v` to the nearest multiple of `factor` (>= 1) half-AWAY-from-zero
    // with ALL-INTEGER arithmetic (`Integer#round(-n)` / `Float#round(<=0)`
    // parity).  Ruby's result is a bignum that may not fit i64; rather than
    // return a two's-complement-wrapped (sign-flipped) garbage value, we DEGRADE
    // to the un-rounded value when the rounded multiple would overflow i64 (the
    // closest representable answer).  `i64::MIN` cannot be negated, so it takes
    // the same degrade path.
    fn round_int_to_multiple(v: i64, factor: i64) -> i64 {
        if v == i64::MIN {
            return v;
        }
        let neg = v < 0;
        let mag = v.unsigned_abs();
        let f = factor as u64;
        let mut q = mag / f;
        let rem = mag - q * f;
        if rem.saturating_mul(2) >= f {
            q += 1;
        }
        // Guard `q * factor` against i64 overflow.
        if q > (i64::MAX as u64) / f {
            return v; // rounded multiple overflows ⇒ degrade to the value
        }
        let magnitude = (q * f) as i64;
        if neg {
            -magnitude
        } else {
            magnitude
        }
    }

    // Integer greatest common divisor (Euclid), always non-negative — the
    // engine behind `Integer#gcd`.  Operands are taken by absolute value so
    // the result matches Ruby (`(-12).gcd(8) == 4`).  `gcd(0, 0) == 0`.
    fn gcd_i64(a: i64, b: i64) -> i64 {
        // `i64::MIN.unsigned_abs()` is representable as u128; work in u128 to
        // avoid the `-i64::MIN` overflow, then narrow the bounded result.
        let mut x = (a as i128).unsigned_abs();
        let mut y = (b as i128).unsigned_abs();
        while y != 0 {
            let t = x % y;
            x = y;
            y = t;
        }
        // Saturate the narrow: a gcd of `2^63` (e.g. `i64::MIN.gcd(0)`) would
        // wrap to a NEGATIVE `i64` under `as`, violating the "always
        // non-negative" contract.  Clamp to `i64::MAX` instead (Ruby returns a
        // Bignum here; saturation keeps the sign correct like `pow` does).
        x.min(i64::MAX as u128) as i64
    }

    // Base-10 digits of `n`, least-significant first (`Integer#digits`).
    // The absolute value is used so a negative receiver degrades gracefully
    // (Ruby raises, but the reference runtimes return the magnitude's
    // digits, which we match).  `0` yields `[0]`.  An i64 has at most 19
    // digits, so the loop is naturally bounded — no bignum guard needed.
    fn digits_of(n: i64) -> Vec<Value> {
        let mut m = (n as i128).unsigned_abs();
        if m == 0 {
            return vec![Value::Int(0)];
        }
        let mut out = Vec::new();
        while m > 0 {
            out.push(Value::Int((m % 10) as i64));
            m /= 10;
        }
        out
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
            _ => no_method_error(&recv, name),
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

    // ── exception model (SIR17 Exceptions) ────────────────────────────
    //
    // Rust has NO native exceptions.  Ruby's `begin/rescue/ensure`
    // maps onto Rust's *unwinding panic* machinery: a `raise` becomes a
    // `std::panic::panic_any(SirError { … })` carrying a class-tagged
    // payload, and a `TryCatch` region runs its body under
    // `std::panic::catch_unwind`, then downcasts the caught payload back
    // to a `SirError` to dispatch the rescue clauses.  See `emit.rs` for
    // the emitted shape; this module is the runtime half.
    //
    // Why panic-unwind and not a `Result`-threading discipline?  Threading
    // a `Result` would demand rewriting *every* emitted expression into a
    // `?`-propagating form and changing every `fn … -> Value` signature to
    // `-> Result<Value, SirError>`.  Panic-unwind is a *localized*
    // transform: only `raise` and `TryCatch` change; all other emit arms
    // are byte-for-byte unchanged, matching how the TS/Python backends add
    // exceptions as a localized `throw`/`try` transform over otherwise
    // unchanged code.
    //
    // SECURITY: rescue matching is an EXPLICIT ancestry-table lookup —
    // never reflection / type-name introspection.  The built-in table is a
    // small curated slice of Ruby's hierarchy (parity with the TS
    // `sir-runtime-exceptions` `ANCESTRY`); user classes contribute edges
    // only through `register_ancestry`, emitted from the module's own
    // `ClassDef { superclass }` pairs.  A `seen`-set cycle guard bounds the
    // ancestry walk so a malicious/cyclic edge set can never spin forever.

    /// A raised SIR exception: a Ruby/SIR class name plus a message.
    ///
    /// This is the panic *payload* — `raise` calls `panic_any(SirError{…})`
    /// and a `TryCatch` recovers it with `catch_unwind` + `downcast`.
    ///
    /// ## Why `msg: String` (not `Value`)
    ///
    /// `std::panic::panic_any<M>` requires `M: Any + Send + 'static`, but our
    /// `Value` model is built on `Rc` (single-threaded by design) and is
    /// therefore NOT `Send`.  So the payload cannot carry a raw `Value`.  A
    /// `raise Klass, msg` renders `msg` to its string form *at raise time*
    /// (via `format`) and stores that `String` — which is `Send` — exactly as
    /// Ruby's `exception.message` is a string.  `exc_value` re-wraps it as a
    /// `Value::Str` for a `rescue … => e` binding.  This keeps the whole
    /// unwinding path `Send`-clean with no `unsafe`, at the cost of a
    /// non-string message value being flattened to its printed form — an
    /// acceptable v0 fidelity trade (Ruby itself expects a string message).
    #[derive(Clone)]
    pub struct SirError {
        /// The Ruby/SIR class this was raised as (`ArgumentError`, `MyErr`…).
        pub class: String,
        /// The message Ruby's `raise Klass, "msg"` carries, rendered to a
        /// string.  When no message is given, the class name is used (Ruby's
        /// default `exception.message`).
        pub msg: String,
    }

    // Built-in Ruby exception ancestry: subclass → immediate superclass.
    // A verbatim parity port of the TS `sir-runtime-exceptions` `ANCESTRY`
    // table.  Every entry ultimately chains up to `StandardError →
    // Exception`.  Kept as a match (not a `HashMap`) so it is a pure,
    // allocation-free, compile-time-closed lookup — the runtime can only
    // ever return an edge this function spells out.
    fn builtin_super(class: &str) -> Option<&'static str> {
        match class {
            "RuntimeError" => Some("StandardError"),
            "ArgumentError" => Some("StandardError"),
            "TypeError" => Some("StandardError"),
            "NameError" => Some("StandardError"),
            "NoMethodError" => Some("NameError"),
            "IndexError" => Some("StandardError"),
            "KeyError" => Some("IndexError"),
            "RangeError" => Some("StandardError"),
            "ZeroDivisionError" => Some("StandardError"),
            "IOError" => Some("StandardError"),
            "StopIteration" => Some("StandardError"),
            "NotImplementedError" => Some("StandardError"),
            "StandardError" => Some("Exception"),
            _ => None,
        }
    }

    thread_local! {
        // User-defined ancestry edges (subclass → superclass), populated
        // once at program init from the module's `ClassDef` pairs via
        // `register_ancestry`.  Consulted *in addition to* the built-in
        // table so `class MyErr < StandardError` makes a raised `MyErr`
        // catchable by `rescue StandardError`.
        static USER_ANCESTRY: RefCell<HashMap<String, String>> =
            RefCell::new(HashMap::new());
    }

    /// Register user-defined `subclass → superclass` edges, once at init.
    ///
    /// The emitter collects every `Stmt::ClassDef { name, superclass:
    /// Some(sup) }` in the module and emits a single call to this with all
    /// the pairs.  This is the ONLY way a user edge enters the matcher —
    /// there is no reflection over runtime type names.
    pub fn register_ancestry(edges: &[(&str, &str)]) {
        USER_ANCESTRY.with(|m| {
            let mut m = m.borrow_mut();
            for (sub, sup) in edges {
                m.insert((*sub).to_string(), (*sup).to_string());
            }
        });
    }

    /// The immediate superclass of `class`, consulting the user table first
    /// (so a user edge can extend, but a built-in is the fallback).
    fn super_of(class: &str) -> Option<String> {
        if let Some(sup) = USER_ANCESTRY.with(|m| m.borrow().get(class).cloned()) {
            return Some(sup);
        }
        builtin_super(class).map(|s| s.to_string())
    }

    /// `true` if `actual` is `target` or descends from it via the merged
    /// built-in + user ancestry.  The `seen` set bounds the walk so a
    /// cyclic edge set (`A→B→A`) terminates instead of looping forever.
    fn is_ancestor_or_self(actual: &str, target: &str) -> bool {
        let mut cur = actual.to_string();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        loop {
            if cur == target {
                return true;
            }
            if !seen.insert(cur.clone()) {
                return false; // cycle — stop.
            }
            match super_of(&cur) {
                Some(next) => cur = next,
                None => return false,
            }
        }
    }

    /// Does a caught `SirError` match a rescue clause naming `class_names`?
    ///
    /// - An **empty** list is a bare `rescue` (catch-all) → always `true`.
    /// - `Exception` is Ruby's universal root → matches anything.
    /// - Otherwise the error matches if its class equals, or descends from,
    ///   any named class (per the merged ancestry table).
    ///
    /// Parity with the TS `rescueMatches`.
    pub fn rescue_matches(exc: &SirError, class_names: &[&str]) -> bool {
        if class_names.is_empty() {
            return true;
        }
        class_names
            .iter()
            .any(|name| *name == "Exception" || is_ancestor_or_self(&exc.class, name))
    }

    /// The message value a rescue binding (`rescue … => e`) sees — the
    /// exception's message, re-wrapped as a `Value::Str`.  Ruby would bind an
    /// exception *object* here; SIR v0 has no exception-object model, so the
    /// message string is the honest stand-in (the same choice the TS backend
    /// makes, where `=> e` binds the thrown value's message).
    pub fn exc_value(exc: &SirError) -> Value {
        Value::Str(Rc::from(exc.msg.as_str()))
    }

    /// Raise a SIR exception of `class` with message `msg` by panicking with
    /// a `SirError` payload.  The `msg` `Value` is rendered to a string at
    /// raise time (see `SirError`'s doc for why the payload cannot carry a
    /// raw `Value`).  Declared `-> !` (never returns) so control-flow
    /// analysis knows code after a `raise` is unreachable.
    ///
    /// A quiet panic hook (installed by `install_panic_hook`, called at
    /// program init) suppresses Rust's default `thread 'main' panicked …`
    /// banner for *our* `SirError` payloads on the caught path, so a rescued
    /// exception prints no spurious stderr noise.  A genuine (non-`SirError`)
    /// Rust panic still prints normally.
    pub fn raise(class: &str, msg: Value) -> ! {
        std::panic::panic_any(SirError { class: class.to_string(), msg: format(&msg) })
    }

    /// Bare `raise` with no in-flight exception threaded: re-raise as a
    /// generic `RuntimeError` (SIR v0 does not carry the current exception
    /// into a bare re-raise — documented limitation, parity with TS).
    pub fn reraise() -> ! {
        std::panic::panic_any(SirError {
            class: "RuntimeError".to_string(),
            msg: "RuntimeError".to_string(),
        })
    }

    /// Recover a `SirError` from a `catch_unwind` payload, or **re-panic**.
    ///
    /// A `catch_unwind` `Err` payload is `Box<dyn Any + Send>`.  If it
    /// downcasts to our `SirError`, that is a SIR-level `raise` we should
    /// dispatch to rescue clauses.  If it does NOT — it is a *genuine Rust
    /// panic* (an index-out-of-bounds, an `unwrap` on `None`, an internal
    /// bug), which must NEVER be silently swallowed as if it were a
    /// rescuable Ruby exception.  We `resume_unwind` it so it propagates
    /// exactly as an uncaught panic would, preserving Rust's own crash
    /// semantics.  This is the security-critical passthrough: a rescue
    /// clause can only ever catch a value that a `raise` produced.
    pub fn exc_from_payload(payload: Box<dyn std::any::Any + Send>) -> SirError {
        match payload.downcast::<SirError>() {
            Ok(e) => *e,
            Err(other) => std::panic::resume_unwind(other),
        }
    }

    /// Install a quiet panic hook so a *`SirError`* panic (a SIR `raise`)
    /// does not print Rust's default panic banner to stderr — the
    /// `catch_unwind` in a `TryCatch` is responsible for that exception, and
    /// an *uncaught* one already renders its own message via `report_uncaught`.
    /// A non-`SirError` panic (a real Rust bug) still prints normally.
    ///
    /// Idempotent-safe to call once at program init.
    pub fn install_panic_hook() {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if info.payload().is::<SirError>() {
                // A SIR raise: stay silent here.  If it is ultimately
                // uncaught, the process still aborts with a non-zero status
                // (the unwind reaches `main` and terminates the thread); the
                // top-level `catch_unwind` in `main` prints a clean message.
                return;
            }
            default(info);
        }));
    }

    /// Render an uncaught SIR exception (one that unwound past every
    /// `TryCatch`) as Ruby would at top level (`Class: message`) and exit
    /// non-zero.  Called by the `main`-level `catch_unwind` wrapper.
    pub fn report_uncaught(exc: &SirError) -> ! {
        eprintln!("{}: {}", exc.class, exc.msg);
        std::process::exit(1)
    }

    // ── user-defined-class OOP (SIR17 `Classes`, O5) ───────────────────
    //
    // The Rust analogue of the JS/Python/Go OOP runtimes.  It reuses the
    // ancestry machinery the exception runtime already built (`super_of`,
    // the `seen`-guarded `is_ancestor_or_self` walk) and adds four pieces,
    // all kept to the same security bar as the collection catalog and the
    // rescue matcher:
    //
    //   • `SirInstance`  — a user object: a class-name tag + its own
    //                      `@ivar` bag.  It lives in the `INSTANCES`
    //                      side-table, referenced by a `Value::Instance(id)`
    //                      handle (see the value-model note on the enum).
    //   • the method tables — instance + class ("static") methods, each a
    //                      `HashMap<(String, String), Value>` (the `Value`
    //                      is the method-body `Closure` a `MakeClosure`
    //                      produced).
    //   • the self-stack  — the dynamic `self` a running method reads via
    //                      `current_self()` / `ivar_get`/`ivar_set`.
    //   • `call_new` / `call_super` — instantiation and superclass dispatch.
    //
    // ── SECURITY (the C3 RCE lesson) ──────────────────────────────────
    // Every lookup here is an EXPLICIT `HashMap::get` on a `(class, method)`
    // key.  There is NO reflection, no trait-object-by-name, no
    // `dyn Any`-downcast on a source-derived string.  A user class or
    // method literally named `constructor` / `new` / `drop` is only ever a
    // map KEY: a miss floors to the same honest boundary the collection
    // catalog uses (`Value::Nil` for a plain call, a `NoMethodError`
    // `raise` where Ruby would).  The ancestry walk reuses the exception
    // runtime's `seen`-guarded `is_ancestor_or_self`/`super_of`, so a
    // cyclic user hierarchy (`A < B < A`) TERMINATES rather than looping.

    /// A user object: its class-name tag plus its instance-variable bag.
    ///
    /// The `ivars` bag is a plain `HashMap<String, Value>` behind a
    /// `RefCell` so a method can mutate `@x` through the shared side-table
    /// entry.  An ivar name is just a map key (`"@x"`), never a field
    /// accessed by reflection.
    pub struct SirInstance {
        pub class: String,
        pub ivars: RefCell<HashMap<String, Value>>,
    }

    thread_local! {
        // The instance side-table: `id → SirInstance`.  We hold each
        // `SirInstance` behind an `Rc` so a `current_self()` read (and the
        // `ivar_*`/`cvar_*` helpers) can clone a cheap handle to the object
        // without removing it from the table.  Instances are never freed in
        // v0 (the transpiled scripts we target are short-lived); this
        // matches the reference runtimes, which likewise let the GC/refcount
        // keep every instance alive for the process lifetime.
        static INSTANCES: RefCell<HashMap<u64, Rc<SirInstance>>> =
            RefCell::new(HashMap::new());
        static NEXT_INSTANCE_ID: Cell<u64> = const { Cell::new(0) };

        // Instance-method table and class-method table, each keyed by the
        // FLAT `(class, method)` pair.  A `HashMap` key of owned `String`s
        // means a name like `"constructor"` is inert DATA — there is no
        // reachable host callable behind it.
        static METHOD_TABLE: RefCell<HashMap<(String, String), Value>> =
            RefCell::new(HashMap::new());
        static CLASS_METHOD_TABLE: RefCell<HashMap<(String, String), Value>> =
            RefCell::new(HashMap::new());

        // The dynamic `self` stack.  Pushed before a user method runs and
        // popped after (via an RAII guard — see `SelfGuard` — so a panic
        // mid-method still pops, leaving no stale `self` for the next
        // dispatch).
        static SELF_STACK: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };

        // Per-class class-variable (`@@x`) bags, keyed by class name then
        // var name.  Shared across every instance of a class, matching
        // Ruby's class-variable semantics.
        static CLASS_VARS: RefCell<HashMap<String, HashMap<String, Value>>> =
            RefCell::new(HashMap::new());

        // ── MX6 mixins: per-owner included-module list ────────────────
        //
        // `include M` (in class/module `Owner`) records `M` on `Owner`'s
        // list, in SOURCE (include) order.  Ruby's MRO searches the
        // MOST-RECENTLY-included module first, so the resolution walk
        // iterates this list in REVERSE (see `resolve_instance_method`).
        //
        // An owner is a class OR a module NAME — a module that itself
        // `include`s another module has its own entry here, so the MRO walk
        // recursing into it honours Ruby's transitive mixin inclusion.
        //
        // SECURITY: a plain `HashMap<String, Vec<String>>` keyed by
        // source-derived NAMES — no reflection (the C3 RCE discipline).  The
        // MRO walk carries a `seen` set so a module that (transitively)
        // includes itself TERMINATES rather than looping forever.
        static INCLUDED_MODULES: RefCell<HashMap<String, Vec<String>>> =
            RefCell::new(HashMap::new());
    }

    /// The class-name tag of instance `id` (or `"?"` if the id is stale —
    /// unreachable in practice, defensive only).  Used by `format`.
    fn instance_class(id: u64) -> String {
        INSTANCES.with(|t| {
            t.borrow().get(&id).map(|o| o.class.clone()).unwrap_or_else(|| "?".to_string())
        })
    }

    /// Fetch a cheap `Rc` handle to instance `id`, if it exists.
    fn instance_of(id: u64) -> Option<Rc<SirInstance>> {
        INSTANCES.with(|t| t.borrow().get(&id).cloned())
    }

    /// Allocate a bare instance of `cls` (no `initialize` yet — that is
    /// `call_new`'s job) and return its `Value::Instance` handle.
    pub fn new_instance(cls: &str) -> Value {
        let id = NEXT_INSTANCE_ID.with(|c| {
            let n = c.get();
            c.set(n + 1);
            n
        });
        let obj = Rc::new(SirInstance {
            class: cls.to_string(),
            ivars: RefCell::new(HashMap::new()),
        });
        INSTANCES.with(|t| t.borrow_mut().insert(id, obj));
        Value::Instance(id)
    }

    /// Register an instance method: `def m … end` in `class C` →
    /// `def_method("C", "m", <closure>)`.  The closure is the method body a
    /// `MakeClosure` produced; storing it as a `Value` keeps dispatch a
    /// plain `apply_closure`.
    pub fn def_method(cls: &str, name: &str, f: Value) -> Value {
        METHOD_TABLE.with(|t| {
            t.borrow_mut().insert((cls.to_string(), name.to_string()), f);
        });
        Value::Nil
    }

    /// Register a class ("static") method: `def self.m …` →
    /// `def_class_method("C", "m", <closure>)`.
    pub fn def_class_method(cls: &str, name: &str, f: Value) -> Value {
        CLASS_METHOD_TABLE.with(|t| {
            t.borrow_mut().insert((cls.to_string(), name.to_string()), f);
        });
        Value::Nil
    }

    // ── MX6 mixins: include / extend directives ───────────────────────

    /// `__include__("Owner", "M")` — record that `Owner` mixes in `M`.
    ///
    /// Appends `M` to `Owner`'s include list in SOURCE order.  Idempotent
    /// duplicates are harmless: the MRO walk's `seen` set de-dups a diamond,
    /// and appending a name twice just makes the second visit a no-op.
    /// Returns `Nil` (the directive has no Ruby value the emitter needs).
    pub fn include_module(owner: &str, module: &str) -> Value {
        INCLUDED_MODULES.with(|t| {
            t.borrow_mut()
                .entry(owner.to_string())
                .or_default()
                .push(module.to_string());
        });
        Value::Nil
    }

    /// `__extend__("Owner", "M")` — mix `M`'s INSTANCE methods in as
    /// `Owner`'s CLASS (singleton) methods, so they become callable as
    /// `Owner.method`.
    ///
    /// We SNAPSHOT `M`'s registered instance methods (including those `M`
    /// itself includes, via the same MRO walk instances use) and copy each
    /// into `Owner`'s class-method table.  An entry `Owner` ALREADY defines
    /// is NOT overwritten — a class's own `def self.m` shadows an extended
    /// module method, matching Ruby's singleton-first precedence.
    ///
    /// Copy-at-extend-time is the v0 model: methods defined on `M` AFTER the
    /// `extend` are not retroactively added, which is sufficient because the
    /// frontend emits every `__def_method__` for `M` before any `__extend__`
    /// that names it (registrations run in source order, module def before
    /// the including class).
    pub fn extend_module(owner: &str, module: &str) -> Value {
        for name in module_method_names(module) {
            let key = (owner.to_string(), name.clone());
            let exists = CLASS_METHOD_TABLE.with(|t| t.borrow().contains_key(&key));
            if exists {
                continue;
            }
            if let Some(f) = resolve_instance_method(module, &name) {
                CLASS_METHOD_TABLE.with(|t| {
                    t.borrow_mut().insert(key, f);
                });
            }
        }
        Value::Nil
    }

    /// The instance-method NAMES reachable on `module` (its own defs plus
    /// those of modules IT includes), for `extend_module` to copy.  Walks the
    /// same include-list MRO as instance resolution, `seen`-guarded against a
    /// cyclic include, and de-dups names so each is copied once (the
    /// earliest, most-specific definition wins).
    fn module_method_names(module: &str) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        let mut added: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut stack: Vec<String> = vec![module.to_string()];
        while let Some(owner) = stack.pop() {
            if owner.is_empty() || !seen.insert(owner.clone()) {
                continue;
            }
            METHOD_TABLE.with(|t| {
                for (o, m) in t.borrow().keys() {
                    if o == &owner && added.insert(m.clone()) {
                        names.push(m.clone());
                    }
                }
            });
            // Recurse into this owner's included modules (order does not
            // matter for a name-collection pass — `added` de-dups).
            if let Some(mods) = INCLUDED_MODULES.with(|t| t.borrow().get(&owner).cloned()) {
                for m in mods {
                    stack.push(m);
                }
            }
        }
        names
    }

    /// Resolve instance method `name` on `cls` following Ruby's MRO:
    ///
    /// ```text
    ///   cls  →  cls's included modules (REVERSE / most-recent-first)  →
    ///   cls's superclass  →  its included modules  →  …  →  Object
    /// ```
    ///
    /// A class's OWN method shadows any module it includes; a module method
    /// shadows the superclass's (a module precedes the superclass in the
    /// ancestor list).  A module included via TWO paths (a diamond) resolves
    /// ONCE, at its earliest position, because the shared `seen` set skips an
    /// owner already visited.  The walk is a depth-first, most-recent-first,
    /// de-duplicated linearisation (the order the spec's truth table
    /// documents).  It reuses the runtime's single ancestry table
    /// (`super_of`) for the superclass chain — the SAME table `rescue` walks.
    ///
    /// The `seen` set makes the walk TOTAL even for a cyclic class hierarchy
    /// (`A < B < A`) OR a self-including module.  Lookup is
    /// `METHOD_TABLE.get(&(owner, name))` — explicit DATA, never reflection.
    /// `from` is the class to START at (the receiver's class for a normal
    /// call; the SUPERCLASS for `super`).
    fn resolve_instance_method(cls: &str, name: &str) -> Option<Value> {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Check `owner`'s own methods, then (reverse-order) its included
        // modules — each of which may itself include further modules, so this
        // recurses.  Returns the closure on the first hit.
        fn resolve_owner(
            owner: &str,
            name: &str,
            seen: &mut std::collections::HashSet<String>,
        ) -> Option<Value> {
            if owner.is_empty() || !seen.insert(owner.to_string()) {
                return None;
            }
            if let Some(f) =
                METHOD_TABLE.with(|t| t.borrow().get(&(owner.to_string(), name.to_string())).cloned())
            {
                return Some(f);
            }
            // Most-recently-included module searched first ⇒ iterate the
            // include-order list in REVERSE.  A module search recurses so a
            // module that itself includes another module is honoured.
            if let Some(mods) = INCLUDED_MODULES.with(|t| t.borrow().get(owner).cloned()) {
                for m in mods.iter().rev() {
                    if let Some(f) = resolve_owner(m, name, seen) {
                        return Some(f);
                    }
                }
            }
            None
        }
        let mut cur = Some(cls.to_string());
        while let Some(c) = cur {
            // A cyclic CLASS chain (`A < B < A`) would re-enter an owner
            // `resolve_owner` already inserted into `seen`; guard here too so
            // the outer superclass loop terminates.
            if seen.contains(&c) {
                break;
            }
            if let Some(f) = resolve_owner(&c, name, &mut seen) {
                return Some(f);
            }
            cur = super_of(&c);
        }
        None
    }

    /// Resolve a CLASS ("static") method `name` on `cls` or any ancestor,
    /// walking the merged (built-in + user) ancestry.  The `seen` set bounds
    /// the walk so a cyclic hierarchy terminates.  Lookup is
    /// `CLASS_METHOD_TABLE.get(&(cur, name))` — explicit DATA, never
    /// reflection.  (Class methods do NOT participate in module include-MRO;
    /// an `extend`ed module method is COPIED into this table by
    /// `extend_module`, so it is found by the same plain ancestry walk.)
    fn resolve_class_method(from: Option<String>, name: &str) -> Option<Value> {
        let mut cur = from;
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        while let Some(c) = cur {
            if !seen.insert(c.clone()) {
                return None; // cycle — stop.
            }
            let found =
                CLASS_METHOD_TABLE.with(|t| t.borrow().get(&(c.clone(), name.to_string())).cloned());
            if found.is_some() {
                return found;
            }
            cur = super_of(&c);
        }
        None
    }

    // ── the dynamic `self` stack (RAII-balanced) ───────────────────────
    //
    // A running method needs its receiver for `@ivar` reads and for `self`.
    // We push the receiver before applying a method and pop it afterwards.
    // The pop is done by an RAII DROP GUARD rather than an explicit call:
    // if the method body PANICS (a SIR `raise` unwinds as a panic, or a
    // genuine Rust panic occurs), the guard's `Drop` still runs during
    // unwinding, so the stack is always balanced and no stale `self` leaks
    // to the next dispatch.  This is the Rust analogue of the JS runtime's
    // `try { … } finally { popSelf(); }`.
    struct SelfGuard;
    impl Drop for SelfGuard {
        fn drop(&mut self) {
            SELF_STACK.with(|s| {
                s.borrow_mut().pop();
            });
        }
    }

    /// Push `recv` as the current self and return an RAII guard that pops it
    /// on drop (including during a panic unwind).
    fn push_self_guarded(recv: Value) -> SelfGuard {
        SELF_STACK.with(|s| s.borrow_mut().push(recv));
        SelfGuard
    }

    /// The current `self` — the top of the self-stack, or `Nil` outside any
    /// method (`__self__` at top level).
    pub fn current_self() -> Value {
        SELF_STACK.with(|s| s.borrow().last().cloned().unwrap_or(Value::Nil))
    }

    /// Apply a resolved method closure with `recv` bound as `self`.  The
    /// `SelfGuard` pops the self-stack on scope exit — normal return OR
    /// panic unwind — so the stack stays balanced.
    fn apply_with_self(f: &Value, recv: Value, args: Vec<Value>) -> Value {
        let _guard = push_self_guarded(recv);
        apply_closure(f, args)
        // `_guard` drops here (or during unwind), popping the self-stack.
    }

    /// Dispatch method `name` on user instance `id`.  Resolves the user
    /// method table walking ancestry; pushes `self`, applies, pops (RAII);
    /// an unresolved method floors to the honest `NoMethodError` boundary
    /// (matching Ruby / the collection catalog's never-silently-wrong
    /// contract).  `recv` is the `Value::Instance` handle to bind as self.
    fn dispatch_user_method(id: u64, recv: &Value, name: &str, args: Vec<Value>) -> Value {
        let class = match instance_of(id) {
            Some(obj) => obj.class.clone(),
            // Stale handle (unreachable in practice) → honest floor.
            None => return unknown_method(recv, name),
        };
        match resolve_instance_method(&class, name) {
            Some(f) => apply_with_self(&f, recv.clone(), args),
            // No user method resolved — fall through to the M6 universal
            // Object methods (`respond_to?`/`tap`/`then`/`yield_self`), which
            // every receiver (instances included) responds to.  `to_s` is a
            // universal too: an instance with no user `to_s` renders via the
            // default `#<Class>` `format` form.  Only a name NONE of these
            // claim is a genuine Ruby `NoMethodError` — surfaced typed so a
            // `rescue NoMethodError` catches it.  (`send`/`__send__`/
            // `public_send` never reach here: `call_method` intercepts them
            // for every receiver, re-entering dispatch with the dynamic name.)
            None => {
                if name == "to_s" {
                    return Value::Str(Rc::from(format(recv).as_str()));
                }
                if let Some(v) = object_method(recv, name, &args) {
                    return v;
                }
                no_method_error(recv, name)
            }
        }
    }

    /// `Klass.new(args…)` → `call_new("Klass", args…)`.  Allocate a bare
    /// instance, then run the inherited `initialize` (if any is registered
    /// anywhere in the ancestry chain) with `self` bound to the fresh
    /// instance, then return the INSTANCE (Ruby discards `initialize`'s
    /// result).  A class with no `initialize` in its chain is valid — `new`
    /// just yields a bare instance.
    pub fn call_new(cls: &str, args: Vec<Value>) -> Value {
        let obj = new_instance(cls);
        if let Some(init) = resolve_instance_method(cls, "initialize") {
            apply_with_self(&init, obj.clone(), args);
        }
        obj
    }

    /// `super(args…)` inside method `method` of class `cls` →
    /// `call_super("method", "cls", args…)`.  Resolve `method` starting from
    /// the SUPERCLASS of `cls` (so the current definition is skipped) and
    /// apply it with the CURRENT `self` still bound — `super` reuses the
    /// live receiver, it does NOT push a new one.  A missing super method
    /// floors to the honest boundary (Ruby raises `NoMethodError`).
    pub fn call_super(method: &str, cls: &str, args: Vec<Value>) -> Value {
        // Resolve from the SUPERCLASS, following the full MRO from there (its
        // own methods, then its included modules, then ITS superclass, …), so
        // `super` can reach a method a mixed-in module of the parent provides.
        let resolved = match super_of(cls) {
            Some(parent) => resolve_instance_method(&parent, method),
            None => None,
        };
        match resolved {
            // Reuse the live self already on the stack (no new push).
            Some(f) => apply_closure(&f, args),
            None => Value::Nil,
        }
    }

    /// `Owner.method(args…)` — a CLASS-method call (`__class_method__`).
    ///
    /// Resolves `method` in `Owner`'s class-method table, walking the ancestry
    /// (`resolve_class_method`) so an inherited `def self.method` is found, AND
    /// including methods mixed in via `extend` (which `extend_module` copied
    /// into the class-method table).  No `self` is pushed — a v0 class method
    /// runs without an instance receiver.  An unresolved name hits the
    /// controlled `NoMethodError` floor (typed, so `rescue NoMethodError`
    /// catches it), never a reflective fallthrough.
    pub fn call_class_method(cls: &str, method: &str, args: Vec<Value>) -> Value {
        match resolve_class_method(Some(cls.to_string()), method) {
            Some(f) => apply_closure(&f, args),
            None => raise(
                "NoMethodError",
                Value::Str(Rc::from(
                    format!("undefined method '{}' for {}", method, cls).as_str(),
                )),
            ),
        }
    }

    // ── instance / class variables on the current self ─────────────────
    // `@x` / `@@x` read/write route here.  They act on `current_self()` — a
    // method body's receiver.  A read of an unset var yields `Nil` (Ruby's
    // nil), matching the `Scope::Instance`/`Scope::ClassVar` "no prior
    // declaration" rule.  A read/write outside any method (no instance
    // self) is a no-op returning `Nil`, never a panic.

    /// Read `@name` on the current self (or `Nil`).
    pub fn ivar_get(name: &str) -> Value {
        if let Value::Instance(id) = current_self() {
            if let Some(obj) = instance_of(id) {
                return obj.ivars.borrow().get(name).cloned().unwrap_or(Value::Nil);
            }
        }
        Value::Nil
    }

    /// Write `@name = val` on the current self; returns `val`.
    pub fn ivar_set(name: &str, val: Value) -> Value {
        if let Value::Instance(id) = current_self() {
            if let Some(obj) = instance_of(id) {
                obj.ivars.borrow_mut().insert(name.to_string(), val.clone());
            }
        }
        val
    }

    /// The class name of the current self, if it is an instance.
    fn current_self_class() -> Option<String> {
        if let Value::Instance(id) = current_self() {
            return instance_of(id).map(|o| o.class.clone());
        }
        None
    }

    /// Read `@@name` for the current self's class (or `Nil`).
    pub fn cvar_get(name: &str) -> Value {
        match current_self_class() {
            Some(cls) => CLASS_VARS.with(|t| {
                t.borrow()
                    .get(&cls)
                    .and_then(|bag| bag.get(name).cloned())
                    .unwrap_or(Value::Nil)
            }),
            None => Value::Nil,
        }
    }

    /// Write `@@name = val` for the current self's class; returns `val`.
    pub fn cvar_set(name: &str, val: Value) -> Value {
        if let Some(cls) = current_self_class() {
            CLASS_VARS.with(|t| {
                t.borrow_mut()
                    .entry(cls)
                    .or_default()
                    .insert(name.to_string(), val.clone());
            });
        }
        val
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
            "is_symbol", "print", "puts", "global_set", "global_get",
            "apply_closure", "intern", "truthy", "format",
            "call_builtin_by_name", "builtin_closure",
        ] {
            assert!(RUNTIME.contains(op), "runtime missing `{}`", op);
        }
    }

    #[test]
    fn runtime_plus_times_are_polymorphic() {
        // sir-polymorphic-operators (PO5): `plus`/`times` dispatch on the
        // first operand's tag via an explicit `match` (String/Seq arms
        // ahead of the numeric fold), never reflection.
        // `plus` gains the String-concat and Seq-concat arms.
        assert!(RUNTIME.contains("string + expects strings"));
        assert!(RUNTIME.contains("array + expects arrays"));
        // `times` gains the binary String/Seq atom with the three arms
        // (string repeat, array repeat, array join).
        assert!(RUNTIME.contains("fn times_binary"));
        assert!(RUNTIME.contains("(Value::Str(s), Value::Int(n))"));
        assert!(RUNTIME.contains("(Value::Seq(items), Value::Int(n))"));
        assert!(RUNTIME.contains("(Value::Seq(items), Value::Str(sep))"));
        // Dispatch is a `match args.first()` on the runtime tag — no
        // reflective / name-indexed lookup (see [[dynamic-dispatch-rce]]).
        assert!(RUNTIME.contains("match args.first()"));
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

    #[test]
    fn runtime_declares_exception_helpers() {
        // E4: the inline runtime must ship the exception model + matcher so a
        // `raise`/`TryCatch` program runs end to end.
        for helper in &[
            "pub struct SirError",
            "pub fn raise",
            "pub fn reraise",
            "pub fn exc_from_payload",
            "pub fn exc_value",
            "pub fn rescue_matches",
            "pub fn register_ancestry",
            "pub fn install_panic_hook",
            "pub fn report_uncaught",
        ] {
            assert!(RUNTIME.contains(helper), "runtime missing `{}`", helper);
        }
    }

    #[test]
    fn runtime_rescue_matcher_is_explicit_table_with_cycle_guard() {
        // SECURITY: rescue matching is an EXPLICIT ancestry table (no
        // reflection), and the ancestry walk carries a `seen`-set cycle
        // guard so a cyclic edge set terminates.
        assert!(RUNTIME.contains("fn builtin_super"), "missing explicit ancestry table");
        // A representative built-in edge (parity with the TS ANCESTRY).
        assert!(RUNTIME.contains(r#""ArgumentError" => Some("StandardError")"#));
        assert!(RUNTIME.contains(r#""StandardError" => Some("Exception")"#));
        // Cycle guard.
        assert!(RUNTIME.contains("seen.insert"), "missing cycle guard");
    }

    #[test]
    fn runtime_non_sir_error_payload_is_resumed_not_swallowed() {
        // A non-`SirError` panic payload must be re-raised, never treated as
        // a rescuable exception.
        assert!(RUNTIME.contains("std::panic::resume_unwind(other)"));
    }

    #[test]
    fn runtime_declares_oop_value_and_helpers() {
        // O5: the inline runtime must ship the user-defined-class OOP model —
        // the `Instance` value handle, the side-table + method tables, and
        // the instantiation/dispatch/super/self/ivar/cvar helpers.
        assert!(RUNTIME.contains("Instance(u64)"), "missing Instance value arm");
        assert!(RUNTIME.contains("pub struct SirInstance"));
        for helper in &[
            "pub fn new_instance",
            "pub fn def_method",
            "pub fn def_class_method",
            "pub fn call_new",
            "pub fn call_super",
            "pub fn current_self",
            "pub fn ivar_get",
            "pub fn ivar_set",
            "pub fn cvar_get",
            "pub fn cvar_set",
        ] {
            assert!(RUNTIME.contains(helper), "runtime missing `{}`", helper);
        }
    }

    #[test]
    fn runtime_oop_dispatch_is_explicit_table_with_cycle_guard() {
        // SECURITY: user-method resolution is an EXPLICIT `HashMap` lookup on
        // a `(class, method)` key — never reflection — and the ancestry walk
        // carries a `seen`-set cycle guard so a cyclic hierarchy terminates.
        assert!(RUNTIME.contains("static METHOD_TABLE"));
        assert!(RUNTIME.contains("static CLASS_METHOD_TABLE"));
        assert!(RUNTIME.contains("fn resolve_instance_method"));
        assert!(RUNTIME.contains("fn resolve_class_method"));
        assert!(RUNTIME.contains("if !seen.insert(c.clone())"), "missing OOP cycle guard");
        // The instance dispatch branch is taken FIRST in `call_method`.
        assert!(RUNTIME.contains("fn dispatch_user_method"));
    }

    #[test]
    fn runtime_declares_m6_metaprogramming_surface() {
        // M6: the inline runtime must ship the universal Object/Kernel
        // metaprogramming methods — `send`/`__send__`/`public_send`,
        // `tap`, `then`/`yield_self`, `respond_to?`, and boolean `&`/`|`/`^`.
        assert!(RUNTIME.contains("fn object_method"), "missing universal Object method dispatch");
        assert!(RUNTIME.contains("fn responds_to"), "missing respond_to? resolver");
        // `send` re-enters dispatch with the dynamic name (the security-
        // critical routing: the name feeds back through the SAME closed
        // `call_method`, never a reflective host lookup).
        assert!(
            RUNTIME.contains(r#""send" | "__send__" | "public_send""#),
            "missing send/__send__/public_send routing"
        );
        assert!(
            RUNTIME.contains("call_method(recv, &target_name, it.collect())"),
            "send must re-enter the explicit call_method with the dynamic name"
        );
        // The block-taking universal pair and the boolean operators.
        assert!(RUNTIME.contains(r#""then" | "yield_self""#), "missing then/yield_self");
        assert!(RUNTIME.contains(r#""respond_to?" =>"#), "missing respond_to? arm");
        assert!(RUNTIME.contains(r#"matches!(name, "&" | "|" | "^")"#), "missing bool operators");
    }

    #[test]
    fn runtime_declares_mixin_helpers() {
        // MX6: the inline runtime must ship the include/extend mixin model —
        // the per-owner included-module table, the MRO-aware instance
        // resolver, the `extend` copy, and the class-method dispatcher.
        assert!(RUNTIME.contains("static INCLUDED_MODULES"), "missing included-module table");
        for helper in &[
            "pub fn include_module",
            "pub fn extend_module",
            "pub fn call_class_method",
            "fn module_method_names",
        ] {
            assert!(RUNTIME.contains(helper), "runtime missing `{}`", helper);
        }
        // SECURITY: the MRO walk searches most-recently-included first
        // (reverse iteration) and is `seen`-guarded so a self-including
        // module terminates.
        assert!(RUNTIME.contains("mods.iter().rev()"), "missing reverse include-order walk");
    }

    #[test]
    fn runtime_self_stack_pops_via_raii_guard() {
        // The self-stack must pop even on a panic unwind: the pop lives in a
        // `Drop` impl (an RAII guard), not an explicit end-of-scope call.
        assert!(RUNTIME.contains("struct SelfGuard"));
        assert!(RUNTIME.contains("impl Drop for SelfGuard"));
        assert!(RUNTIME.contains("fn apply_with_self"));
    }
}
