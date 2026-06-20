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
pub fn make_generator(
    class_name: &str,
    fields: &SValue,
    methods: &SValue,
    defining_env: &Env,
) -> SResult<SValue> {
    // The field NAMES come from the `fields = list(x = "numeric", …)` argument's
    // names attribute. R also accepts a bare character vector (`fields =
    // c("x", "y")`); we support both shapes.
    let field_vec = extract_field_names(fields)?;

    // Validate the methods list: it must be a (possibly empty) list of *named*
    // closures. Anything else is a clean error.
    let method_list = validate_methods(methods)?;

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
    Ok(SValue::Environment(scope))
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
    let fields = field_names(generator);

    // The generator's *defining scope* is the instance's lexical parent, so a
    // method's free variables resolve up to where the class was defined. The
    // generator scope's own parent is that defining scope (see `make_generator`),
    // so we parent the instance on the generator scope itself — a method then sees
    // the generator's private bindings shadowed only by the instance's fields,
    // which is harmless (a user field never starts with a dot).
    let scope = env::Scope::child(generator);

    // Bind every declared field, defaulting to NULL when `new` omitted it.
    for field in &fields {
        let value = init_args
            .iter()
            .find(|(name, _)| name.as_deref() == Some(field.as_str()))
            .map(|(_, v)| v.clone())
            .unwrap_or(SValue::Null);
        env::define(&scope, field, value);
    }

    // Reject any `new(arg = …)` whose name is not a declared field — a typo
    // should fail loudly rather than silently create a stray binding.
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

    // Carry the methods list (closures over the generator scope) onto the
    // instance so `obj$method` can rebuild an instance-bound closure on demand.
    if let Some(methods) = env::lookup(generator, KEY_METHODS) {
        env::define(&scope, KEY_METHODS, methods);
    }

    // `.self` — a strong Environment value to the instance itself, so a method
    // can reach a sibling via `.self$other(...)`. This is the one (documented,
    // bounded) R-22 value-binding self-cycle; see the module note.
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
        // `generator$new` → the instantiation marker. Any other name on a
        // generator reads its (private or user) binding via the ordinary path.
        if name == "new" {
            return Some(new_marker(obj));
        }
        return None;
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
