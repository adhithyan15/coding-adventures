//! R5 **reference classes** — `setRefClass`, generators, and instances.
//!
//! This is the payoff of the R-21/22/23 environment work. The central idea is
//! deceptively small:
//!
//! > **An R5 object *is* an environment**, holding its fields as bindings, and a
//! > **method *is* a closure** whose enclosing environment is that instance
//! > environment — so the method body sees the fields directly and writes them
//! > back by reference.
//!
//! Everything else falls out of reusing the shared [`SValue::Environment`] value
//! (a live, mutate-by-reference scope) plus R-21's `<<-` super-assignment.
//!
//! ## The object graph
//!
//! There are two kinds of object, both encoded as [`SValue::Environment`] but
//! distinguished by the *private bindings* they carry (names beginning with a
//! dot, which ordinary `obj$field` access never collides with because a user
//! field of the same dotted name would simply shadow it):
//!
//! ```text
//!   setRefClass("Acc", fields = …, methods = …)
//!        │
//!        ▼
//!   GENERATOR  (an Environment)
//!     .refClassName = "Acc"                 # character
//!     .refFields    = c("total")            # field names, declaration order
//!     .refMethods   = list(add=<closure>,   # methods AS WRITTEN — each closes
//!                          get=<closure>)    #   over the GENERATOR's scope
//!        │
//!        │  generator$new(total = 0)
//!        ▼
//!   INSTANCE  (a fresh child Environment of the generator's defining scope)
//!     total       = 0                       # the field bindings (mutable)
//!     .self       = <Environment → itself>  # so a method can call .self$other()
//!     .refMethods = list(add=…, get=…)      # carried so methods rebuild on access
//! ```
//!
//! ## Why methods are rebuilt lazily (the instance⇄method Rc-cycle)
//!
//! The *obvious* encoding — bind each method as a closure **in the instance's own
//! bindings**, with that closure's `env` being the instance — forms a **strong
//! reference cycle**:
//!
//! ```text
//!   instance.vars["add"]  ──▶  Closure { env: <strong Rc to instance> }
//!         ▲                                   │
//!         └───────────────────────────────────┘   (uncollectable by Rc alone)
//! ```
//!
//! `Rc` never reclaims a cycle, so every such instance would leak its whole scope
//! the moment it became unreachable. **We break the cycle by never storing the
//! instance-bound closures at all.** The instance holds only its field bindings,
//! `.self`, and `.refMethods` — and the closures inside `.refMethods` close over
//! the **generator's** scope, *not* the instance's, so they form no edge back to
//! the instance. The instance-bound method closure (whose `env` *is* the instance)
//! is materialised **lazily, on each `obj$method` access**, lives only for the
//! duration of that one call, and is dropped immediately after. Because it is
//! never stored anywhere reachable from the instance, the
//! instance → method → instance edge never exists *at rest* — there is nothing for
//! `Rc` to fail to collect.
//!
//! The one remaining strong self-reference is `.self` (an `Environment` to the
//! instance, stored inside the instance). That is exactly the *documented,
//! pre-existing* R-22 value-binding self-cycle — `assign("self", e, envir = e)` —
//! which we do not claim to collect but which is *bounded* (not unbounded) by the
//! `MAX_ENVIRONMENTS` session cap. We inherit that boundary verbatim rather than
//! widening the ownership model.
//!
//! ## Reference (alias) semantics
//!
//! Because an instance *is* an [`SValue::Environment`] — a strong `Rc` to a live
//! scope — `b <- a` copies the **handle**, not the scope's contents. `a` and `b`
//! then name the *same* instance: `b$add(1)` mutates the shared scope and
//! `a$total` sees it. This is the deliberate exception to R's otherwise
//! copy-on-modify value semantics, and it is the headline behaviour of R5.

use crate::env::{self, Env};
use crate::error::{SError, SResult};
use crate::value::SValue;

/// Private binding: the reference class's **name** (a length-1 character),
/// carried on the generator. Dotted so it never collides with a user field.
pub const KEY_CLASS_NAME: &str = ".refClassName";

/// Private binding: the **field names** (a character vector, declaration order),
/// carried on the generator. Used by `$new` to know which fields to bind.
pub const KEY_FIELDS: &str = ".refFields";

/// Private binding: the **methods** (an `SValue::List` of closures *as written*,
/// each closing over the generator's defining scope), carried on **both** the
/// generator and every instance. The instance copy is what `obj$method` consults
/// to rebuild a fresh instance-bound closure on access — see the module note on
/// the instance⇄method cycle.
pub const KEY_METHODS: &str = ".refMethods";

/// Private binding: `.self`, an [`SValue::Environment`] pointing at the instance
/// itself, bound inside the instance so a method body can reach a sibling method
/// as `.self$other(...)`.
pub const KEY_SELF: &str = ".self";

/// Private binding (R-25): the **parent generator** for a subclass defined with
/// `contains = "Base"`, stored on the subclass generator as an
/// [`SValue::Environment`] pointing at the parent generator. Absent on a root
/// (non-inheriting) class. This single link is what makes the class an inheritance
/// **chain**: walking `.refParent` from a generator yields its ancestors in order.
/// The edge is a strict child → parent **DAG** edge — a cyclic `contains =` is
/// rejected at [`make_generator`] time (see [`would_form_cycle`]) — so it can never
/// create an `Rc` cycle through the generators.
pub const KEY_PARENT: &str = ".refParent";

/// Private binding (R-25): the **generator** an instance was built from, stored on
/// the instance as a strong [`SValue::Environment`]. R-24 instances had no need to
/// name their generator; R-25's `is()`/`inherits()` and `$copy()` do — the former
/// to read the class chain, the latter to re-instantiate a sibling. This is an
/// instance → generator edge; the generator never points back at its instances, so
/// it forms no cycle. (Relying on the instance's *parent* scope link instead is
/// unsafe: that link is a `Weak` that fails to upgrade once the generator variable
/// goes out of scope.)
pub const KEY_GENERATOR: &str = ".refGenerator";

/// Hard cap on the inheritance-chain depth walked by [`class_chain`],
/// [`effective_fields`], [`effective_methods`], and the cycle check. A
/// well-formed `contains =` chain is a DAG so this is never hit in practice; it is
/// a belt-and-braces termination bound that makes every chain walk provably
/// finite even if a future change (or a corrupted environment) reintroduced a
/// loop. Generous: no real R5 hierarchy is hundreds deep.
pub const MAX_CHAIN_DEPTH: usize = 256;

/// Is `env` a **reference-class generator** (carries the class-name marker)?
/// Distinguishes a generator from an ordinary `new.env()` environment so the `$`
/// dispatch can route `generator$new`. A plain environment has none of the
/// private bindings, so this is `false` for it.
pub fn is_generator(env: &Env) -> bool {
    // **Frame-local** markers only: an instance is a *child* of its generator, so
    // a chain-walking lookup of the generator's `.refClassName` would find it
    // through the parent and misclassify the instance. A generator binds
    // `.refClassName` directly and never binds `.self`.
    env::lookup_local(env, KEY_CLASS_NAME).is_some()
        && env::lookup_local(env, KEY_FIELDS).is_some()
        && env::lookup_local(env, KEY_SELF).is_none()
}

/// Is `env` a reference-class **instance** (carries `.self` and `.refMethods` in
/// its own frame)? An instance always binds `.self` to itself; a generator never
/// does. Frame-local for the same reason as [`is_generator`].
pub fn is_instance(env: &Env) -> bool {
    env::lookup_local(env, KEY_SELF).is_some() && env::lookup_local(env, KEY_METHODS).is_some()
}

/// Pull the method **closures** carried by a generator/instance as `(name,
/// closure)` pairs. Returns an empty list when the `.refMethods` binding is
/// absent or is not a list (a defensive default — never a panic). Only entries
/// that are actually callable closures are surfaced; a non-closure method entry
/// (which `setRefClass` rejects up front) is silently skipped here as a second
/// line of defence.
fn methods_of(env: &Env) -> Vec<(String, SValue)> {
    match env::lookup_local(env, KEY_METHODS) {
        Some(SValue::List { names, items }) => names
            .into_iter()
            .zip(items)
            .filter_map(|(n, v)| match (n, &v) {
                (Some(name), SValue::Closure { .. }) => Some((name, v)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The declared field names of a generator (the `.refFields` character vector),
/// in declaration order. An absent or non-character marker yields an empty list.
fn field_names(env: &Env) -> Vec<String> {
    match env::lookup_local(env, KEY_FIELDS) {
        Some(SValue::Character(v)) => v.into_iter().flatten().collect(),
        _ => Vec::new(),
    }
}

/// The **effective field names** of a generator: the union of this class's own
/// fields with every ancestor's, in **base-first declaration order** (a field
/// inherited from the base precedes a field newly declared on the subclass), with
/// duplicates removed (a subclass re-declaring a base field does not duplicate it
/// — the base position is kept). Implemented by collecting each generator's *own*
/// `.refFields` from the **root down to this class** (so base fields come first),
/// then de-duplicating while preserving first-seen order. Bounded by
/// [`MAX_CHAIN_DEPTH`].
pub fn effective_fields(generator: &Env) -> Vec<String> {
    // Walk root → self by collecting the chain self → root then reversing.
    let mut chain: Vec<Env> = Vec::new();
    let mut cur = Some(generator.clone());
    let mut depth = 0usize;
    while let Some(g) = cur {
        if depth >= MAX_CHAIN_DEPTH {
            break;
        }
        chain.push(g.clone());
        cur = parent_generator(&g);
        depth += 1;
    }
    chain.reverse(); // root first

    let mut out: Vec<String> = Vec::new();
    for g in &chain {
        for f in field_names(g) {
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

/// The **effective methods** of a generator as `(name, closure)` pairs: the union
/// of this class's own methods with every ancestor's, with a **subclass method
/// overriding** a same-named ancestor method. Implemented by collecting from the
/// **root down to this class** so that when a name recurs, the *most-derived*
/// (latest-seen) closure wins. Returns each method once, in first-declaration
/// order (base methods first, then sub-only methods). Bounded by
/// [`MAX_CHAIN_DEPTH`].
fn effective_methods(generator: &Env) -> Vec<(String, SValue)> {
    let mut chain: Vec<Env> = Vec::new();
    let mut cur = Some(generator.clone());
    let mut depth = 0usize;
    while let Some(g) = cur {
        if depth >= MAX_CHAIN_DEPTH {
            break;
        }
        chain.push(g.clone());
        cur = parent_generator(&g);
        depth += 1;
    }
    chain.reverse(); // root first

    // Preserve first-declaration order for names, but let a more-derived class
    // *replace* the closure bound to an already-seen name (override semantics).
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, SValue> = std::collections::HashMap::new();
    for g in &chain {
        for (name, closure) in methods_of(g) {
            if !map.contains_key(&name) {
                order.push(name.clone());
            }
            map.insert(name, closure);
        }
    }
    order
        .into_iter()
        .map(|n| {
            let v = map.get(&n).cloned().unwrap_or(SValue::Null);
            (n, v)
        })
        .collect()
}

/// The **class chain** of a reference-class object (generator *or* instance), most
/// derived first: e.g. `["Sub", "Base", "envRefClass", "environment"]`. For an
/// instance, the walk starts from its generator (recovered via `.refParent`'s
/// sibling — actually from the generator the instance was built under, reachable
/// because the instance's *parent scope* is the generator). The two synthetic tail
/// classes `"envRefClass"` and `"environment"` mirror R, where every reference
/// object `is()` an `envRefClass` and an `environment`. Bounded by
/// [`MAX_CHAIN_DEPTH`]. This is what `is()`/`inherits()` consult.
pub fn class_chain(generator: &Env) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = Some(generator.clone());
    let mut depth = 0usize;
    while let Some(g) = cur {
        if depth >= MAX_CHAIN_DEPTH {
            break;
        }
        if let Some(name) = generator_name(&g) {
            if !out.contains(&name) {
                out.push(name);
            }
        }
        cur = parent_generator(&g);
        depth += 1;
    }
    out.push("envRefClass".to_string());
    out.push("environment".to_string());
    out
}

/// The class chain of an **instance**: recover the generator the instance was
/// built from (its `.refGenerator` strong link — see [`instantiate`]), then
/// delegate to [`class_chain`]. Returns `None` if `obj` is not an instance or its
/// `.refGenerator` is missing/not a generator (defensive — never a panic).
pub fn instance_class_chain(obj: &Env) -> Option<Vec<String>> {
    if !is_instance(obj) {
        return None;
    }
    let gen = instance_generator(obj)?;
    if is_generator(&gen) {
        Some(class_chain(&gen))
    } else {
        None
    }
}

/// The R5 class vector of a value **iff** it is a reference-class instance:
/// `Some(["Sub", "Base", …, "envRefClass", "environment"])`. Returns `None` for
/// every non-instance value, so `is`/`inherits`/`class` can fall back to the
/// ordinary [`crate::value::class_of`] for those. This is the single entry point
/// the builtins use to make R5 inheritance visible to S3-style class queries.
pub fn instance_class_vector(value: &SValue) -> Option<Vec<String>> {
    if let SValue::Environment(e) = value {
        return instance_class_chain(e);
    }
    None
}

/// The **generator** an instance was built from (its `.refGenerator` link), or
/// `None` if absent/malformed. Used by `is`/`inherits` (class chain) and `$copy()`
/// (re-instantiation).
fn instance_generator(instance: &Env) -> Option<Env> {
    match env::lookup_local(instance, KEY_GENERATOR) {
        Some(SValue::Environment(e)) if is_generator(&e) => Some(e),
        _ => None,
    }
}

/// Build a reference-class **generator** from already-evaluated pieces:
/// `class_name`, the `fields` value (a list whose *names* are the field names —
/// the type strings are recorded but not enforced in this subset), and the
/// `methods` value (a list of closures). `defining_env` becomes the generator
/// scope's parent (so a method's free variables can still resolve up the lexical
/// chain to where `setRefClass` was called).
///
/// Each argument is validated; a malformed one is a clean `BadArgs`/`TypeError`,
/// never a panic. The generator is returned as an [`SValue::Environment`]; the
/// caller is responsible for charging it against `MAX_ENVIRONMENTS`.
///
/// **R-25 inheritance.** When `parent` is `Some(base_generator)`, the new class
/// is a *subclass*: its own (declared) fields and methods are stored exactly as
/// for a root class, plus a `.refParent` link to the base generator. The *effective*
/// field/method union (base ∪ sub) is **not** flattened into the subclass here —
/// it is computed lazily by walking `.refParent` at `$new`/introspection time
/// ([`effective_fields`]/[`effective_methods`]). A `contains =` that would close a
/// cycle (`A contains B contains A`, or self-inheritance by name) is **rejected**
/// up front via [`would_form_cycle`], so the `.refParent` edges always form a DAG.
pub fn make_generator(
    class_name: &str,
    fields: &SValue,
    methods: &SValue,
    parent: Option<&Env>,
    defining_env: &Env,
) -> SResult<SValue> {
    // The field NAMES come from the `fields = list(x = "numeric", …)` argument's
    // names attribute. R also accepts a bare character vector (`fields =
    // c("x", "y")`); we support both shapes.
    let field_vec = extract_field_names(fields)?;

    // Validate the methods list: it must be a (possibly empty) list of *named*
    // closures. Anything else is a clean error.
    let method_list = validate_methods(methods)?;

    // `contains = parent`: the parent must itself be a reference-class generator,
    // and the chain must stay acyclic (no `A contains B contains A`, no class
    // inheriting from one that already bears its name). Reject both up front so the
    // `.refParent` edges form a strict DAG and every later chain walk terminates.
    if let Some(p) = parent {
        if !is_generator(p) {
            return Err(SError::TypeError(
                "setRefClass: `contains =` must name a reference-class generator".into(),
            ));
        }
        if would_form_cycle(class_name, p) {
            return Err(SError::BadArgs(format!(
                "setRefClass: `contains =` would make class '{class_name}' inherit from itself (cyclic hierarchy)"
            )));
        }
    }

    let scope = env::Scope::child(defining_env);
    env::define(
        &scope,
        KEY_CLASS_NAME,
        SValue::Character(vec![Some(class_name.to_string())]),
    );
    env::define(
        &scope,
        KEY_FIELDS,
        SValue::Character(field_vec.into_iter().map(Some).collect()),
    );
    env::define(&scope, KEY_METHODS, method_list);
    if let Some(p) = parent {
        env::define(&scope, KEY_PARENT, SValue::Environment(p.clone()));
    }
    Ok(SValue::Environment(scope))
}

/// Would making `class_name` a subclass of generator `parent` close a cycle? A
/// hierarchy is cyclic if the new class's name already appears anywhere in the
/// prospective parent's own class chain (including the parent itself) — e.g.
/// `A <- setRefClass("A", contains = B)` where `B`'s chain already contains `"A"`.
/// Walks `.refParent` from `parent` up to the root, bounded by [`MAX_CHAIN_DEPTH`]
/// (so even a pre-existing corrupt loop terminates the check). Returns `true` to
/// **reject** the class.
fn would_form_cycle(class_name: &str, parent: &Env) -> bool {
    let mut cur = Some(parent.clone());
    let mut depth = 0usize;
    while let Some(g) = cur {
        if depth >= MAX_CHAIN_DEPTH {
            // A chain this deep is already pathological; treat it as cyclic and
            // refuse rather than risk a non-terminating walk.
            return true;
        }
        if generator_name(&g).as_deref() == Some(class_name) {
            return true;
        }
        cur = parent_generator(&g);
        depth += 1;
    }
    false
}

/// The class name a generator carries (its `.refClassName`), or `None` if the
/// marker is missing/malformed (never a panic).
fn generator_name(generator: &Env) -> Option<String> {
    match env::lookup_local(generator, KEY_CLASS_NAME) {
        Some(SValue::Character(v)) => v.into_iter().next().flatten(),
        _ => None,
    }
}

/// The parent **generator** of `generator` (its `.refParent` link), or `None` for
/// a root class. Only an actual generator environment is returned — a malformed
/// marker yields `None` (defensive, never a panic).
fn parent_generator(generator: &Env) -> Option<Env> {
    match env::lookup_local(generator, KEY_PARENT) {
        Some(SValue::Environment(e)) if is_generator(&e) => Some(e),
        _ => None,
    }
}

/// Extract field names from the `fields =` argument. Two shapes are accepted:
/// a **named list** `list(x = "numeric", y = "character")` (the field names are
/// the list's names; the type strings are ignored in this subset), or a plain
/// **character vector** `c("x", "y")` (the elements *are* the field names). A
/// missing `fields` (R allows a class with no fields) yields an empty list. Any
/// other shape — an unnamed list, a numeric vector — is a clean error.
fn extract_field_names(fields: &SValue) -> SResult<Vec<String>> {
    match fields.strip_attrs() {
        SValue::Null => Ok(Vec::new()),
        SValue::List { names, .. } => {
            let mut out = Vec::with_capacity(names.len());
            for n in names {
                match n {
                    Some(name) => out.push(name.clone()),
                    None => {
                        return Err(SError::BadArgs(
                            "setRefClass: every entry in `fields` must be named".into(),
                        ))
                    }
                }
            }
            Ok(out)
        }
        SValue::Character(v) => v
            .iter()
            .map(|o| {
                o.clone().ok_or_else(|| {
                    SError::BadArgs("setRefClass: a field name may not be NA".into())
                })
            })
            .collect(),
        other => Err(SError::TypeError(format!(
            "setRefClass: `fields` must be a list or character vector, got {}",
            other.type_name()
        ))),
    }
}

/// Validate the `methods =` argument into a normalised `list` of named closures.
/// `NULL` (no methods) becomes the empty list. A non-list, an unnamed entry, or
/// a non-function entry is a clean error — never a panic.
fn validate_methods(methods: &SValue) -> SResult<SValue> {
    match methods.strip_attrs() {
        SValue::Null => Ok(SValue::list(Vec::new())),
        SValue::List { names, items } => {
            for (n, v) in names.iter().zip(items.iter()) {
                match n {
                    None => {
                        return Err(SError::BadArgs(
                            "setRefClass: every entry in `methods` must be named".into(),
                        ))
                    }
                    Some(name) => {
                        // A method must be a user `function(...) ...` (an
                        // `SValue::Closure`) so it can be *re-homed* onto the
                        // instance on access (see `rebuild_method`). A builtin or
                        // `Negate(f)` wrapper is callable but has no re-parentable
                        // body, so it could never see the instance's fields —
                        // reject it up front with a clear error rather than have it
                        // silently read back as `NULL` later.
                        if !matches!(v, SValue::Closure { .. }) {
                            return Err(SError::TypeError(format!(
                                "setRefClass: method '{name}' must be a function defined with `function(...)`"
                            )));
                        }
                    }
                }
            }
            Ok(SValue::List {
                names: names.clone(),
                items: items.clone(),
            })
        }
        other => Err(SError::TypeError(format!(
            "setRefClass: `methods` must be a list, got {}",
            other.type_name()
        ))),
    }
}

/// Instantiate a generator: `generator$new(field = value, …)`. Builds a fresh
/// **instance** environment (a child of the generator's defining scope), binds
/// each declared field — to the matching `new(field = …)` argument value, or
/// `NULL` when omitted — and binds the two private markers `.self` (the instance,
/// for `.self$method()`) and `.refMethods` (carried from the generator so
/// `obj$method` can rebuild a closure on access). An argument naming a field that
/// the class did not declare is a clean error. The instance is returned as an
/// [`SValue::Environment`]; the caller charges it against `MAX_ENVIRONMENTS`.
///
/// `init_args` are `(optional name, value)` pairs as supplied at the call site;
/// `new` matches them **by name** (R5 `$new` is keyword-only for fields).
pub fn instantiate(generator: &Env, init_args: &[(Option<String>, SValue)]) -> SResult<SValue> {
    if !is_generator(generator) {
        return Err(SError::TypeError(
            "$new: the target is not a reference-class generator".into(),
        ));
    }
    // R-25: the *effective* field/method sets are the union over the whole
    // `contains =` chain (base ∪ sub, with sub methods overriding same-named base
    // methods) — see `effective_fields`/`effective_methods`. For a root class with
    // no parent these are exactly the class's own fields/methods, so R-24 behaviour
    // is unchanged.
    let fields = effective_fields(generator);

    // The generator's *defining scope* is the instance's lexical parent, so a
    // method's free variables resolve up to where the class was defined. The
    // generator scope's own parent is that defining scope (see `make_generator`),
    // so we parent the instance on the generator scope itself — a method then sees
    // the generator's private bindings shadowed only by the instance's fields,
    // which is harmless (a user field never starts with a dot).
    let scope = env::Scope::child(generator);

    // Bind every effective field (base-first), defaulting to NULL when `new`
    // omitted it. Inherited base fields are bound here too, so a Sub instance has a
    // flat frame holding *all* fields — a base or sub method then reads/writes any
    // of them identically via `<<-`.
    for field in &fields {
        let value = init_args
            .iter()
            .find(|(name, _)| name.as_deref() == Some(field.as_str()))
            .map(|(_, v)| v.clone())
            .unwrap_or(SValue::Null);
        env::define(&scope, field, value);
    }

    // Reject any `new(arg = …)` whose name is not an effective field (including
    // inherited ones) — a typo should fail loudly rather than silently create a
    // stray binding.
    for (name, _) in init_args {
        match name {
            Some(n) if fields.iter().any(|f| f == n) => {}
            Some(n) => {
                return Err(SError::BadArgs(format!(
                    "$new: '{n}' is not a field of this reference class"
                )))
            }
            None => {
                return Err(SError::BadArgs(
                    "$new: fields must be supplied by name (e.g. new(total = 0))".into(),
                ))
            }
        }
    }

    // Carry the *effective* methods list (base ∪ sub, sub overriding) onto the
    // instance so `obj$method` can rebuild an instance-bound closure on demand —
    // including inherited base methods, which are thereby callable on a Sub.
    env::define(&scope, KEY_METHODS, methods_list_value(generator));

    // Remember the generator we were built from (strong instance → generator edge,
    // no cycle) so `is`/`inherits` can read the class chain and `$copy()` can
    // re-instantiate a sibling.
    env::define(&scope, KEY_GENERATOR, SValue::Environment(generator.clone()));

    // `.self` — a strong Environment value to the instance itself, so a method
    // can reach a sibling via `.self$other(...)`. This is the one (documented,
    // bounded) R-22 value-binding self-cycle; see the module note.
    env::define(&scope, KEY_SELF, SValue::Environment(scope.clone()));

    Ok(SValue::Environment(scope))
}

/// The effective methods of `generator` packaged as an `SValue::List` of named
/// closures (base ∪ sub, sub overriding). This is what is carried onto an instance
/// as `.refMethods` so `obj$method` (via [`methods_of`]/[`rebuild_method`]) sees
/// inherited methods too.
fn methods_list_value(generator: &Env) -> SValue {
    let pairs = effective_methods(generator);
    let names = pairs.iter().map(|(n, _)| Some(n.clone())).collect();
    let items = pairs.into_iter().map(|(_, v)| v).collect();
    SValue::List { names, items }
}

/// `obj$copy()` (R-25) — a **deep** value-copy producing a NEW, independent
/// instance. Contrast `b <- a`, which aliases the *same* scope (R-24 reference
/// semantics): a copy shares no state, so a later `b$x <- …` does not touch `a`.
///
/// The copy is built exactly like `$new` (a fresh child scope of the instance's
/// generator, with `.refMethods`/`.refGenerator`/`.self` re-bound), then each
/// effective field is copied across **by value** from the source instance. A field
/// that itself holds another reference instance is copied as a **handle** (a shallow
/// alias of that nested instance) — matching R5's `copy(shallow = TRUE)` default —
/// rather than recursively deep-copied, which keeps the copy bounded by the field
/// count (no unbounded recursion through a graph of nested instances) and is in any
/// case charged against `MAX_ENVIRONMENTS` by the caller. Errors (a `copy()` on a
/// non-instance, a missing generator) are clean, never panics.
pub fn copy_instance(instance: &Env) -> SResult<SValue> {
    if !is_instance(instance) {
        return Err(SError::TypeError(
            "$copy: the target is not a reference-class instance".into(),
        ));
    }
    let generator = instance_generator(instance).ok_or_else(|| {
        SError::TypeError("$copy: the instance has lost its generator link".into())
    })?;

    // Build the fresh sibling scope and re-bind the private markers, mirroring
    // `instantiate` (but copying field *values* rather than reading `new` args).
    let scope = env::Scope::child(&generator);
    for field in effective_fields(&generator) {
        // `env::lookup_local` reads the source instance's *own* field binding; the
        // value is cloned (a shallow `SValue` clone — a nested instance is aliased,
        // not recursed) and bound into the new scope. An absent field defaults to
        // NULL, matching `$new`.
        let value = env::lookup_local(instance, &field).unwrap_or(SValue::Null);
        env::define(&scope, &field, value);
    }
    env::define(&scope, KEY_METHODS, methods_list_value(&generator));
    env::define(&scope, KEY_GENERATOR, SValue::Environment(generator));
    env::define(&scope, KEY_SELF, SValue::Environment(scope.clone()));
    Ok(SValue::Environment(scope))
}

/// Resolve `obj$name` where `obj` is a reference-class object (generator or
/// instance). Returns `Some(value)` if `name` is handled here, or `None` to let
/// the caller fall through to plain-environment `$` access (an ordinary field /
/// binding lookup). The three reference-class-specific cases are:
///
/// * **`generator$new`** → a callable that the trailing `call_suffix` applies.
///   We hand back a marker the dispatcher recognises (see [`new_marker`]); the
///   actual instantiation happens in [`instantiate`] once the args are evaluated.
/// * **`obj$method`** (a name present in `.refMethods`) → a **fresh** closure
///   whose `env` is the **instance** (so its body sees the fields and updates
///   them with `<<-`). Built lazily here and never stored — see the module note
///   on the instance⇄method cycle.
/// * everything else → `None` (fall through to a field/binding lookup).
///
/// A field that *shadows* a method name is read as a field (the instance's own
/// binding wins), matching the lookup order a user expects.
pub fn dollar_access(obj: &Env, name: &str) -> Option<SValue> {
    if is_instance(obj) {
        // A **field** binding on the instance frame always wins over a method of
        // the same name (matching the lookup order a user expects). Read it
        // frame-locally so `obj$x` never leaks a generator/parent binding.
        if let Some(value) = env::lookup_local(obj, name) {
            return Some(value);
        }
        // R-25: `obj$copy` → a nullary marker the call dispatcher routes to
        // `copy_instance`. A *field* named `copy` would already have been returned
        // above (the field-first rule), and a *user method* named `copy` overrides
        // the builtin (checked first), matching R5 where a user-defined `copy`
        // shadows the inherited one.
        if rebuild_method(obj, name).is_none() && name == "copy" {
            return Some(ref_method_marker(obj, REF_METHOD_COPY));
        }
        // Not a field → a method? Rebuild a fresh instance-bound closure on
        // access (never stored — see the module note on the instance⇄method
        // cycle).
        if let Some(closure) = rebuild_method(obj, name) {
            return Some(closure);
        }
        // Unknown member → NULL, as R5 reads an unset field / missing method.
        return Some(SValue::Null);
    }

    if is_generator(obj) {
        // `generator$new` → the instantiation marker. `generator$fields` /
        // `generator$methods` → nullary introspection markers (R-25). Any other
        // name on a generator reads its (private or user) binding via the ordinary
        // path.
        match name {
            "new" => return Some(new_marker(obj)),
            "fields" => return Some(ref_method_marker(obj, REF_METHOD_FIELDS)),
            "methods" => return Some(ref_method_marker(obj, REF_METHOD_METHODS)),
            _ => return None,
        }
    }

    None
}

/// Rebuild a fresh **instance-bound** closure for method `name`: take the method
/// closure as written (which closes over the *generator* scope) and return a new
/// `Closure` with the same params/body but its `env` swapped to the **instance**.
/// The body then sees the instance's fields as free variables and writes them
/// back with `<<-`. Returns `None` if `name` is not a method. **Never stored** —
/// this lives only for the call that triggered it, so it forms no lasting
/// instance⇄method `Rc` cycle (see the module note).
fn rebuild_method(instance: &Env, name: &str) -> Option<SValue> {
    for (mname, m) in methods_of(instance) {
        if mname == name {
            if let SValue::Closure { params, body, .. } = m {
                return Some(SValue::Closure {
                    params,
                    body,
                    env: instance.clone(),
                });
            }
        }
    }
    None
}

/// The marker value returned for `generator$new`: a `Classed` wrapper carrying
/// the generator environment and the private class `".refGeneratorNew"`, which
/// the call dispatcher in `eval.rs` recognises to route to [`instantiate`].
/// Using a value the existing `apply` path can recognise (rather than a bespoke
/// `SValue` variant) keeps the change localised and the exhaustive matches
/// elsewhere untouched.
pub fn new_marker(generator: &Env) -> SValue {
    SValue::Classed {
        inner: Box::new(SValue::Environment(generator.clone())),
        class: vec![CLASS_NEW_MARKER.to_string()],
    }
}

/// The private S3 class tag the `$new` marker carries; the call dispatcher keys
/// on it to route a `generator$new(...)` application to [`instantiate`].
pub const CLASS_NEW_MARKER: &str = ".refGeneratorNew";

/// If `value` is the `generator$new` marker produced by [`new_marker`], return
/// the generator environment it wraps; otherwise `None`. Lets the call
/// dispatcher detect the marker without matching on the private class string in
/// more than one place.
pub fn as_new_marker(value: &SValue) -> Option<Env> {
    if let SValue::Classed { inner, class } = value {
        if class.iter().any(|c| c == CLASS_NEW_MARKER) {
            if let SValue::Environment(e) = inner.as_ref() {
                return Some(e.clone());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// R-25 nullary built-in reference methods: `obj$copy()`, `generator$fields()`,
// `generator$methods()`.
//
// Each is reached as `obj$name` (returning a marker) immediately followed by a
// `()` application. We reuse the same `Classed`-wrapper marker trick as
// `generator$new`: the marker carries the bound env plus a private class tag that
// names the action, and the call dispatcher (`apply`/`call_value`) routes it to
// `apply_ref_method`. Encoding the action in the class tag keeps a *single* extra
// branch in each dispatch site (rather than one per method) and leaves every
// exhaustive `match` on `SValue` untouched.
// ---------------------------------------------------------------------------

/// Private class tag: `obj$copy()` — deep-copy the bound **instance**.
pub const REF_METHOD_COPY: &str = ".refMethodCopy";
/// Private class tag: `generator$fields()` — the sorted effective field names.
pub const REF_METHOD_FIELDS: &str = ".refMethodFields";
/// Private class tag: `generator$methods()` — the sorted effective method names.
pub const REF_METHOD_METHODS: &str = ".refMethodMethods";

/// Build a nullary reference-method marker for `env` tagged with `action` (one of
/// the `REF_METHOD_*` constants). The dispatcher recognises it via
/// [`as_ref_method_marker`].
fn ref_method_marker(env: &Env, action: &str) -> SValue {
    SValue::Classed {
        inner: Box::new(SValue::Environment(env.clone())),
        class: vec![action.to_string()],
    }
}

/// If `value` is one of the R-25 nullary reference-method markers, return the
/// `(action, env)` pair it wraps; otherwise `None`. `action` is the matched
/// `REF_METHOD_*` tag.
pub fn as_ref_method_marker(value: &SValue) -> Option<(&'static str, Env)> {
    if let SValue::Classed { inner, class } = value {
        if let SValue::Environment(e) = inner.as_ref() {
            for c in class {
                match c.as_str() {
                    REF_METHOD_COPY => return Some((REF_METHOD_COPY, e.clone())),
                    REF_METHOD_FIELDS => return Some((REF_METHOD_FIELDS, e.clone())),
                    REF_METHOD_METHODS => return Some((REF_METHOD_METHODS, e.clone())),
                    _ => {}
                }
            }
        }
    }
    None
}

/// Execute a nullary reference-method marker: `obj$copy()` deep-copies the bound
/// instance ([`copy_instance`]); `generator$fields()` / `generator$methods()`
/// return the **sorted** effective field / method names as a character vector. The
/// caller (`eval.rs`) charges any new environment against `MAX_ENVIRONMENTS` before
/// invoking the copy path. `action` must be a `REF_METHOD_*` tag.
pub fn apply_ref_method(action: &str, env: &Env) -> SResult<SValue> {
    match action {
        REF_METHOD_COPY => copy_instance(env),
        REF_METHOD_FIELDS => {
            let mut names = effective_fields(env);
            names.sort();
            Ok(SValue::Character(names.into_iter().map(Some).collect()))
        }
        REF_METHOD_METHODS => {
            let mut names: Vec<String> =
                effective_methods(env).into_iter().map(|(n, _)| n).collect();
            names.sort();
            Ok(SValue::Character(names.into_iter().map(Some).collect()))
        }
        // Unreachable in practice (only the three tags are constructed), but a
        // clean error beats a panic if a future tag is added without a branch.
        other => Err(SError::TypeError(format!(
            "internal: unknown reference-method marker '{other}'"
        ))),
    }
}
