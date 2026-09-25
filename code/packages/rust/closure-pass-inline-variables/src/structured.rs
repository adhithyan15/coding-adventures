//! Resolving a member-access chain against an object or array literal.
//!
//! Per [CLOC28](../../../specs/CLOC28-literal-property-propagation.md).
//! The scalar half of this crate answers "what literal is `X` bound to?".
//! This half answers a narrower question one level down:
//!
//! ```js
//! var o = { a: { b: { c: 1 } } };
//! console.log(o.a.b.c);          // ← what does this whole chain read?
//! ```
//!
//! `o.a.b.c` is three nested [`MemberExpression`]s. Peeling them from the
//! outside gives the root name `o` and the key path `a`, `b`, `c`; walking
//! that path into the literal lands on `1`, and the entire chain can be
//! replaced by that `1`.
//!
//! # Why a chain and not a variable
//!
//! Substituting the *binding* (`o` → `{a:{b:{c:1}}}`) would be wrong at
//! more than one use site: each substitution constructs a **new** object,
//! so `o.a === o.a` would flip from `true` to `false`. Upstream shows the
//! same care — it keeps one binding rather than duplicating the literal:
//!
//! ```text
//! var o={a:{b:1}};console.log(o.a===o.a);  =>  var a={};console.log(a===a);
//! ```
//!
//! Replacing the *chain* has no such hazard, because what lands at the use
//! site is a scalar: a number, string, boolean, `null` or bigint. Scalars
//! have no identity to preserve, so duplicating one is free. That is why
//! [`resolve`] refuses to return anything else.

use coding_adventures_javascript_ast::{Expression, ObjectMember, PropertyKind};

/// One step of a key path: `o.a` and `o["a"]` are both `Name("a")`;
/// `a[0]` is `Index(0)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Name(String),
    Index(usize),
}


/// Peel a member chain to its root identifier and the key path read from
/// it, outermost expression in, returning the path in *access* order.
///
/// Accepts a chain that mixes `.` and `?.` steps and sits under a
/// [`Expression::ChainExpression`] (CLOC30).
///
/// Returns `None` the moment a step is something this pass does not model
/// — a computed key that is not a literal (`o[i]`), a call in the middle
/// of the chain (`o.f().b`), anything rooted in something other than a
/// plain name. Declining here is always safe: the caller treats an
/// unrecognised occurrence as a reason to leave the whole binding alone.
///
/// `o?.a?.b`, `o.a?.b` and `o?.a.b` all parse as an interleaving of
/// [`Expression::MemberExpression`] and
/// [`Expression::OptionalMemberExpression`] beneath one `ChainExpression`,
/// so one walker has to accept both at every step.
///
/// # Why `?.` needs no extra guard
///
/// `?.` short-circuits to `undefined` when its object is `null` or
/// `undefined`. It cannot do that here. The root is a candidate whose
/// initializer [`is_structured_literal`] — an object or array literal is
/// never nullish — and every intermediate step is resolved by [`resolve`]
/// against that literal, which only continues through a nested object or
/// array literal and otherwise returns `None`. So wherever this walker
/// produces a path that `resolve` accepts, no `?.` on it could have
/// short-circuited, and `?.` reads exactly as `.` would.
///
/// The cases where a `?.` *would* short-circuit therefore decline rather
/// than fold: `var o={a:null}; o?.a?.b` stops at the `NullLiteral`,
/// because a null is not a structure to step into. Upstream folds that one
/// to `void 0`; we leave it, which is a gap and not a divergence in the
/// dangerous direction.
pub fn chain_of_expr(e: &Expression) -> Option<(String, Vec<Key>)> {
    // A chain expression is a transparent wrapper around its spine.
    let mut cur = match e {
        Expression::ChainExpression(c) => c.expression.as_ref(),
        other => other,
    };
    let mut keys = Vec::new();
    loop {
        let (computed, property, object) = match cur {
            Expression::MemberExpression(m) => (m.computed, m.property.as_ref(), m.object.as_ref()),
            Expression::OptionalMemberExpression(m) => {
                (m.computed, m.property.as_ref(), m.object.as_ref())
            }
            _ => return None,
        };
        keys.push(key_of_parts(computed, property)?);
        match object {
            Expression::Identifier(id) => {
                keys.reverse();
                return Some((id.name.clone(), keys));
            }
            // A nested `ChainExpression` cannot appear mid-spine in
            // well-formed input, but unwrapping one costs nothing and
            // keeps the walker total.
            Expression::ChainExpression(c) => cur = c.expression.as_ref(),
            inner => cur = inner,
        }
    }
}

/// The key one member step reads, or `None` if it is not a literal key.
///
/// Takes the two fields rather than a `MemberExpression`, because
/// `OptionalMemberExpression` carries the same pair and reads the same way
/// (CLOC30) — see [`chain_of_expr`].
fn key_of_parts(computed: bool, property: &Expression) -> Option<Key> {
    match (computed, property) {
        // `o.a` — the property is a name, not a value to evaluate.
        (false, Expression::Identifier(id)) => Some(Key::Name(id.name.clone())),
        // `o["a-b"]` — a string subscript is the same read as `o.a` would
        // be if the key were an identifier. This is the `quoted_key` rung.
        (true, Expression::StringLiteral(s)) => Some(Key::Name(s.value.clone())),
        // `a[0]` — only a non-negative integer index, and only one that
        // round-trips exactly. `a[1.5]` and `a[-1]` are property reads on
        // an array object, not element reads, so they are not modelled.
        (true, Expression::NumericLiteral(n)) => {
            let v = n.value;
            if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= usize::MAX as f64 {
                Some(Key::Index(v as usize))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// True for the initializer shapes a chain can be resolved against.
pub fn is_structured_literal(e: &Expression) -> bool {
    matches!(
        e,
        Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
    )
}

/// Walk `keys` into `root`, returning the expression the chain reads.
///
/// Returns `None` unless every step lands on an **own data property**
/// that is present in the literal. A missing key is emphatically not
/// `undefined`: `o.b` on `{a:1}` may resolve up the prototype chain, and
/// `o.toString` certainly does. Upstream keeps the access rather than
/// folding it, and so do we, by declining here:
///
/// ```text
/// var o={a:1};console.log(o.b);               =>  console.log({}.b);
/// var o={a:1};console.log(typeof o.toString); =>  console.log(typeof{}.toString);
/// ```
pub fn resolve<'a>(root: &'a Expression, keys: &[Key]) -> Option<&'a Expression> {
    let mut cur = root;
    for k in keys {
        cur = step(cur, k)?;
    }
    Some(cur)
}

/// One resolution step: read `k` out of `cur`, or decline.
fn step<'a>(cur: &'a Expression, k: &Key) -> Option<&'a Expression> {
    match (cur, k) {
        (Expression::ObjectExpression(obj), Key::Name(want)) => {
            let mut found = None;
            for member in &obj.properties {
                match member {
                    // A spread can contribute keys we cannot see, so any
                    // spread anywhere in the literal makes every read of
                    // it unresolvable — the key we want might come from
                    // the spread and shadow, or be shadowed by, a literal
                    // one. Upstream resolves this; we decline (CLOC28,
                    // "where v1 stays behind upstream").
                    ObjectMember::Spread(_) => return None,
                    ObjectMember::Property(p) => {
                        // A getter or setter turns the read into a call.
                        if p.kind != PropertyKind::Init {
                            return None;
                        }
                        // A computed key is not known without evaluating it.
                        if p.computed {
                            return None;
                        }
                        if property_key_name(p)? == *want {
                            // Keep scanning: a later duplicate key wins in
                            // JS, and upstream refuses duplicates outright
                            // (JSC_DUPLICATE_MEMBER), so taking the last
                            // match keeps us correct on input it accepts.
                            found = Some(p.value.as_ref());
                        }
                    }
                }
            }
            found
        }
        (Expression::ArrayExpression(arr), Key::Index(i)) => {
            // A spread AT OR BEFORE the index makes positional reading
            // meaningless: it contributes an unknown number of elements, so
            // everything after it shifts by an amount we cannot know.
            //
            //     var a = [..."xy", 5];
            //     a[1]   // "y" — the spread supplied elements 0 and 1
            //
            // Reading the literal positionally would answer `5`. A spread
            // *after* the index is harmless, because the elements before it
            // are still where they appear (`[1, 2, ...x][0]` is `1`), and
            // upstream folds that case too — so this declines on position
            // rather than on the mere presence of a spread.
            if arr
                .elements
                .iter()
                .take(*i + 1)
                .any(|e| matches!(e, Some(Expression::SpreadElement(_))))
            {
                return None;
            }
            // An out-of-range index reads `undefined`, and so does a hole.
            // Both are folded by upstream and declined here (CLOC28).
            match arr.elements.get(*i)? {
                None => None,
                Some(e) => Some(e),
            }
        }
        _ => None,
    }
}

/// The static name of a non-computed property key, or `None` if it is a
/// shape we do not read (a private name, say).
fn property_key_name(p: &coding_adventures_javascript_ast::Property) -> Option<String> {
    use coding_adventures_javascript_ast::PropertyKey;
    match &p.key {
        PropertyKey::Identifier(id) => Some(id.name.clone()),
        PropertyKey::StringLiteral(s) => Some(s.value.clone()),
        // A numeric key `{0:1}` is the string "0" as a property name; an
        // index read `a[0]` on an OBJECT reaches it, but we only model
        // numeric keys on arrays, so decline rather than guess.
        _ => None,
    }
}

/// Count every object in a serialized AST fragment carrying
/// `"name": "<name>"` — declarations, reads, writes, property keys, the
/// lot.
///
/// # Why this does not key on `"type": "Identifier"`
///
/// Because two of the enums that can hold a name are `#[serde(untagged)]`
/// and therefore serialize *without* a type tag:
///
/// * `BindingTarget` — the `id` of a declarator, so `var o = …` contributes
///   an untagged `{"name":"o"}`;
/// * `AssignmentTarget` — so the left of `o = {a:2}` does too.
///
/// A tag-keyed counter misses both. Missing the first is harmless (it just
/// shifts the arithmetic), but missing the second is a **miscompile**:
/// `var o={a:1};o={a:2};console.log(o.a)` then shows one reference, that
/// reference is a resolvable chain, and the fold prints `1` where the
/// program prints `2`. Keying on the `name` field instead sees every
/// mention whatever the enum tagging, and over-counting (a property key
/// that happens to share the name, say) only ever causes a candidate to be
/// declined.
///
/// The earlier, tag-keyed version is kept below for the one call that
/// genuinely wants "identifier nodes only".
///
/// This is deliberately exhaustive rather than a typed walk. The guard it
/// backs is "no occurrence of this name is anything other than a chain we
/// are about to rewrite", and a typed walk gets that guard wrong the first
/// time the AST grows a variant nobody remembered to visit. Counting over
/// the serialized form cannot miss a variant: an occurrence this pass does
/// not understand still lands in the total, the total then fails to match
/// the number of chains rewritten, and the binding is left alone.
///
/// That matters more than it looks. `count_uses_*` in this crate skips a
/// bare identifier assignment target, because a `const` cannot be
/// assigned and the scalar path only ever sees `const`. Extend the same
/// reasoning to `var` and `var o={a:1};o={a:2};console.log(o.a)` reports
/// one use, matches one chain, and folds to `console.log(1)` — printing
/// `1` where the program prints `2`.
pub fn count_name_mentions(v: &serde_json::Value, name: &str) -> usize {
    match v {
        serde_json::Value::Object(map) => {
            let is_hit = map.get("name").and_then(serde_json::Value::as_str) == Some(name);
            let nested: usize = map
                .values()
                .map(|child| count_name_mentions(child, name))
                .sum();
            usize::from(is_hit) + nested
        }
        serde_json::Value::Array(items) => items
            .iter()
            .map(|child| count_name_mentions(child, name))
            .sum(),
        _ => 0,
    }
}

#[allow(dead_code)]
pub fn count_identifier_nodes(v: &serde_json::Value, name: &str) -> usize {
    match v {
        serde_json::Value::Object(map) => {
            let is_hit = map.get("type").and_then(serde_json::Value::as_str) == Some("Identifier")
                && map.get("name").and_then(serde_json::Value::as_str) == Some(name);
            let nested: usize = map
                .values()
                .map(|child| count_identifier_nodes(child, name))
                .sum();
            usize::from(is_hit) + nested
        }
        serde_json::Value::Array(items) => items
            .iter()
            .map(|child| count_identifier_nodes(child, name))
            .sum(),
        _ => 0,
    }
}
