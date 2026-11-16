//! # SH5: no recursion, by the lexical call graph
//!
//! Why a component must not recurse: the image proof bounds the stack as "the
//! sum of the frames along the deepest call chain", and that sum is finite
//! only if no chain loops.  `rustc` warns about *direct* recursion only; a
//! calls b calls a slips through.
//!
//! ## The graph
//!
//! Nodes are `fn` items, keyed by the type of the enclosing `impl` or `trait`
//! (or none for a free function) and the name.  An edge runs from the
//! innermost enclosing `fn` to whatever a call expression may name:
//!
//! ```text
//!   f(..)                  the free function f
//!   Self::f(..)            f of the current impl type
//!   Q::f(..)  <Q as T>::f  f of Q if the crate defines it; the methods named
//!                          f if Q is a trait of the crate; otherwise nothing
//!   x.f(..)                every method named f (the receiver type is unknown)
//!   m::f(..)  (m lower)    the free function f
//! ```
//!
//! ## Honest limits
//!
//! The graph is by *name*, so two unrelated methods called `f` can look like a
//! cycle (the finding says so; rename one).  A call into another crate, through
//! a generic parameter (`<P::Ready as ReadySet>::insert` has the qualifier `P`,
//! which the crate does not define) or through a macro is invisible: cross-crate
//! recursion is the image-level static stack bound (VOS-028).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::lexer::{Kind, Token};
use crate::report::Rule;
use crate::rules::Ctx;
use crate::syntax::{
    angle_close, is_keyword, matching_lt, scan_to_body, FileCtx, MAX_HEADER_TOKENS,
};

/// The synthetic type key of an `impl` whose self type has no name.
const ANON_TYPE: &str = "_anon";

/// Most edges the graph will hold; more is VT03 (fail closed).
pub const MAX_EDGES: usize = 200_000;

/// Most `fn` items the graph will hold; more is VT03 (fail closed).
pub const MAX_FNS: usize = 4_096;

#[derive(Clone, Debug)]
enum Call {
    Free(String),
    Typed(String, String),
    Method(String),
}

struct Node {
    /// The header scan gave up before a `{` or `;` (fail closed, VT03).
    overlong: bool,
    ty: Option<String>,
    name: String,
    file: usize,
    line: u32,
    calls: Vec<Call>,
}

/// The type an `impl` header names: `impl<T> Tr for a::B<T> {` gives `B`.
fn impl_type(t: &[Token], pair: &[usize], i: usize) -> Option<(String, usize)> {
    let mut k = i + 1;
    if t.get(k)?.is_punct("<") {
        k = angle_close(t, pair, k)? + 1;
    }
    let start = k;
    let mut for_at = None;
    let limit = (i + MAX_HEADER_TOKENS).min(t.len());
    while k < limit && !t[k].is_punct("{") && !t[k].is_punct(";") && !t[k].is_ident("where") {
        if t[k].kind == Kind::Punct && matches!(t[k].text.as_str(), "(" | "[") {
            k = pair[k];
        } else if t[k].is_ident("for") && !t.get(k + 1).is_some_and(|x| x.is_punct("<")) {
            for_at = Some(k);
        }
        k += 1;
    }
    // The body brace may come after a `where` clause.
    let b = scan_to_body(t, pair, k);
    if b >= t.len() || !t[b].is_punct("{") {
        return None;
    }
    let from = for_at.map_or(start, |f| f + 1);
    let mut j = from;
    while j < k {
        let x = &t[j];
        let skippable =
            x.kind == Kind::Lifetime || x.is_punct("&") || x.is_ident("mut") || x.is_ident("dyn");
        if skippable {
            j += 1;
        } else if x.kind == Kind::Ident && !is_keyword(&x.text) {
            let mut name = x.text.clone();
            while t.get(j + 1).is_some_and(|c| c.is_punct("::"))
                && t.get(j + 2).is_some_and(|c| c.kind == Kind::Ident)
            {
                name = t[j + 2].text.clone();
                j += 2;
            }
            return Some((name, b));
        } else {
            // `impl Tr for [u8; 4]`, `(u8, u8)`, `*const T`, `()`: no nameable
            // type; use one synthetic key so `x.f()` method edges still exist.
            return Some((ANON_TYPE.to_string(), b));
        }
    }
    Some((ANON_TYPE.to_string(), b))
}

/// For a call `.. > :: name (`, the qualifier type:
///
/// * `Foo::<u8>::go(..)` (turbofish): the segment BEFORE the `::<..>` group;
/// * `Foo<u8>::go(..)`: the identifier before the `<`;
/// * `<A as B>::go(..)`: the first identifier after the `<`.
fn qself_type(t: &[Token], gt: usize) -> Option<String> {
    let k = matching_lt(t, gt)?;
    if k >= 2 && t[k - 1].is_punct("::") && t[k - 2].kind == Kind::Ident {
        return Some(t[k - 2].text.clone());
    }
    if k >= 1 && t[k - 1].kind == Kind::Ident && !is_keyword(&t[k - 1].text) {
        return Some(t[k - 1].text.clone());
    }
    t.get(k + 1)
        .filter(|x| x.kind == Kind::Ident)
        .map(|x| x.text.clone())
}

fn collect_file(f: &FileCtx, fi: usize, nodes: &mut Vec<Node>, traits: &mut BTreeSet<String>) {
    let t = &f.toks;
    let mut ty_stack: Vec<(usize, Option<String>)> = Vec::new();
    let mut fn_stack: Vec<(usize, usize)> = Vec::new();
    for i in 0..t.len() {
        while ty_stack.last().is_some_and(|&(c, _)| c < i) {
            ty_stack.pop();
        }
        while fn_stack.last().is_some_and(|&(c, _)| c < i) {
            fn_stack.pop();
        }
        if f.in_attr[i] || t[i].kind != Kind::Ident {
            continue;
        }
        let prev = i.checked_sub(1).map(|p| &t[p]);
        let item_start = prev.is_none_or(|p| {
            (p.kind == Kind::Punct && matches!(p.text.as_str(), ";" | "}" | "{" | "]"))
                || p.is_ident("unsafe")
        });
        let cur_ty = ty_stack.last().and_then(|(_, ty)| ty.clone());
        match t[i].text.as_str() {
            "impl" if item_start => {
                if let Some((name, brace)) = impl_type(t, &f.pair, i) {
                    ty_stack.push((f.pair[brace], Some(name)));
                }
            }
            "trait" => {
                let after_vis = prev.is_none_or(|p| {
                    item_start || p.is_ident("pub") || p.is_ident("unsafe") || p.is_punct(")")
                });
                if after_vis {
                    if let Some(n) = t.get(i + 1).filter(|x| x.kind == Kind::Ident) {
                        traits.insert(n.text.clone());
                        let b = scan_to_body(t, &f.pair, i + 2);
                        if b < t.len() && t[b].is_punct("{") {
                            ty_stack.push((f.pair[b], Some(n.text.clone())));
                        }
                    }
                }
            }
            "fn" => {
                if let Some(n) = t.get(i + 1).filter(|x| x.kind == Kind::Ident) {
                    nodes.push(Node {
                        overlong: false,
                        ty: cur_ty,
                        name: n.text.clone(),
                        file: fi,
                        line: n.line,
                        calls: Vec::new(),
                    });
                    let b = scan_to_body(t, &f.pair, i + 2);
                    if let Some(node) = nodes.last_mut() {
                        node.overlong = b < t.len()
                            && !t[b].is_punct("{")
                            && !t[b].is_punct(";")
                            && !matches!(
                                t[b].text.as_str(),
                                "struct" | "enum" | "union" | "trait" | "mod" | "use"
                            );
                    }
                    if b < t.len() && t[b].is_punct("{") {
                        fn_stack.push((f.pair[b], nodes.len() - 1));
                    }
                }
            }
            name => {
                let Some(&(_, owner)) = fn_stack.last() else {
                    continue;
                };
                let is_call = t.get(i + 1).is_some_and(|x| x.is_punct("("))
                    || (t.get(i + 1).is_some_and(|x| x.is_punct("::"))
                        && t.get(i + 2).is_some_and(|x| x.is_punct("<")));
                if !is_call || is_keyword(name) || prev.is_some_and(|p| p.is_ident("fn")) {
                    continue;
                }
                let call = match prev {
                    Some(p) if p.is_punct(".") => Call::Method(name.to_string()),
                    Some(p) if p.is_punct("::") => {
                        let q = i.checked_sub(2).map(|k| &t[k]);
                        let qual = match q {
                            Some(x) if x.kind == Kind::Ident => Some(x.text.clone()),
                            Some(x) if x.is_punct(">") => qself_type(t, i - 2),
                            _ => None,
                        };
                        match qual {
                            Some(q) if q == "Self" => match &cur_ty {
                                Some(ty) => Call::Typed(ty.clone(), name.to_string()),
                                None => Call::Free(name.to_string()),
                            },
                            Some(q) if q.starts_with(|c: char| c.is_ascii_uppercase()) => {
                                Call::Typed(q, name.to_string())
                            }
                            Some(_) => Call::Free(name.to_string()),
                            None => continue,
                        }
                    }
                    _ => Call::Free(name.to_string()),
                };
                nodes[owner].calls.push(call);
            }
        }
    }
}

/// SH5 over the whole crate.
pub fn recursion(cx: &mut Ctx<'_>) {
    let files = cx.files;
    let mut nodes: Vec<Node> = Vec::new();
    let mut traits: BTreeSet<String> = BTreeSet::new();
    for (fi, f) in files.iter().enumerate() {
        collect_file(f, fi, &mut nodes, &mut traits);
    }
    for node in nodes.iter().filter(|n| n.overlong) {
        let file = files[node.file].path.clone();
        cx.add(
            Rule::Vt03,
            &file,
            node.line,
            format!("header of `fn {}` is over {MAX_HEADER_TOKENS} tokens; its body was not scanned (fail closed)", node.name),
        );
    }
    if nodes.len() > MAX_FNS {
        cx.add(
            Rule::Vt03,
            "src/lib.rs",
            1,
            format!("more than {MAX_FNS} functions; the call graph is not built (fail closed)"),
        );
        return;
    }
    let mut by_key: BTreeMap<(Option<String>, String), Vec<usize>> = BTreeMap::new();
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (n, node) in nodes.iter().enumerate() {
        by_key
            .entry((node.ty.clone(), node.name.clone()))
            .or_default()
            .push(n);
        if node.ty.is_some() {
            by_name.entry(node.name.clone()).or_default().push(n);
        }
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut edges = 0usize;
    for (n, node) in nodes.iter().enumerate() {
        let mut to: BTreeSet<usize> = BTreeSet::new();
        for c in &node.calls {
            let found: Vec<usize> = match c {
                Call::Free(name) => by_key
                    .get(&(None, name.clone()))
                    .cloned()
                    .unwrap_or_default(),
                Call::Method(name) => by_name.get(name).cloned().unwrap_or_default(),
                // A path through a trait of the crate may reach any impl of it.
                Call::Typed(ty, name) if traits.contains(ty) => {
                    by_name.get(name).cloned().unwrap_or_default()
                }
                Call::Typed(ty, name) => by_key
                    .get(&(Some(ty.clone()), name.clone()))
                    .cloned()
                    .unwrap_or_default(),
            };
            to.extend(found);
            // Fail closed before the work, not after it: a crate whose method
            // names collide thousands of times is refused, not ground through.
            if edges + to.len() > MAX_EDGES {
                cx.add(
                    Rule::Vt03,
                    "src/lib.rs",
                    1,
                    format!("more than {MAX_EDGES} call-graph edges; the graph is not built (fail closed)"),
                );
                return;
            }
        }
        edges += to.len();
        adj[n] = to.into_iter().collect();
    }
    let label = |n: usize| match &nodes[n].ty {
        Some(ty) => format!("{ty}::{}", nodes[n].name),
        None => nodes[n].name.clone(),
    };
    // Strongly connected components in O(V + E) (iterative Tarjan): every
    // component of two or more functions, and every self-loop, is a cycle.
    for comp in tarjan(&adj) {
        let cyclic = comp.len() > 1 || adj[comp[0]].contains(&comp[0]);
        if !cyclic {
            continue;
        }
        let members: BTreeSet<usize> = comp.iter().copied().collect();
        for &v in &comp {
            let shown = if comp.len() <= 16 {
                cycle_through(&adj, &members, v)
                    .iter()
                    .map(|&n| label(n))
                    .collect::<Vec<_>>()
                    .join(" -> ")
            } else {
                let names: Vec<String> = comp.iter().take(8).map(|&n| label(n)).collect();
                format!(
                    "one of {} functions in a cycle, e.g. {}",
                    comp.len(),
                    names.join(", ")
                )
            };
            let msg = format!(
                "`{}` is on a call cycle ({shown}); recursion is forbidden (the call graph is by name: rename an unrelated method of the same name)",
                label(v)
            );
            let file = files[nodes[v].file].path.clone();
            cx.add(Rule::Sh5, &file, nodes[v].line, msg);
        }
    }
}

/// A path `v -> .. -> v` inside `members` (breadth-first, small components only).
fn cycle_through(adj: &[Vec<usize>], members: &BTreeSet<usize>, v: usize) -> Vec<usize> {
    let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    for &s in &adj[v] {
        if members.contains(&s) {
            parent.entry(s).or_insert(v);
            queue.push_back(s);
        }
    }
    while let Some(u) = queue.pop_front() {
        if u == v {
            break;
        }
        for &w in &adj[u] {
            if members.contains(&w) {
                if let std::collections::btree_map::Entry::Vacant(e) = parent.entry(w) {
                    e.insert(u);
                    queue.push_back(w);
                }
            }
        }
    }
    let mut chain = vec![v];
    let mut cur = parent[&v];
    while cur != v {
        chain.push(cur);
        cur = parent[&cur];
    }
    chain.push(v);
    chain.reverse();
    chain
}

/// Iterative Tarjan: the strongly connected components of `adj`.
fn tarjan(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut comps: Vec<Vec<usize>> = Vec::new();
    let mut counter = 0usize;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        // (node, next successor to visit)
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = counter;
        low[root] = counter;
        counter += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&(v, i)) = call.last() {
            if i < adj[v].len() {
                call.last_mut().expect("non-empty").1 += 1;
                let w = adj[v][i];
                if index[w] == usize::MAX {
                    index[w] = counter;
                    low[w] = counter;
                    counter += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(p, _)) = call.last() {
                    low[p] = low[p].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut comp = Vec::new();
                    while let Some(w) = stack.pop() {
                        on_stack[w] = false;
                        comp.push(w);
                        if w == v {
                            break;
                        }
                    }
                    comp.sort_unstable();
                    comps.push(comp);
                }
            }
        }
    }
    comps
}
