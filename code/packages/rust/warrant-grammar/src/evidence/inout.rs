//! # WX-G in the evidence generator: `inout` adapters and postcondition monitors
//!
//! (VER04k sections 4.6 to 4.8 and 4.13.)  A document stays functional: an
//! operation with an `inout` parameter *returns the post-state* as part of its
//! result, and parameter names always denote the pre-state.  The adapter is what
//! turns a host `&mut` call into that pure function:
//!
//! ```text
//!   op write_varint(value: I64, out: inout Bytes) -> (Nat, Bytes)        S3
//!
//!   fn op_write_varint(_w, args: &[V], fault) -> V {
//!       let p0 = host::<i64>(args[0].int());                  // plain argument
//!       let mut h1: Vec<u8> = args[1].bytes().to_vec();       // a COPY: pre-state isolation
//!       let r = krate::varint::write(p0, &mut h1);            // the subject mutates the copy
//!       V::Tup(vec![ V::Int(r as i128),                       // the host's own return
//!                    V::Bytes((h1).to_vec()) ])               // read back from the SAME buffer
//!   }
//! ```
//!
//! Two rules carry the soundness of the design (4.6):
//!
//! 1. **Pre-state isolation.**  The subject gets a fresh copy.  The value the
//!    claims and the `ensures` rows use as the pre-state is `args[i]`, which the
//!    adapter never hands to the subject, so a subject cannot corrupt the
//!    oracle's view of the input.
//! 2. **Read-back from the same buffer.**  The post-state is read from the very
//!    buffer passed to the subject, after the call.  The `stale-post` fault is
//!    exactly the violation of this rule, so the rule is checked by a control,
//!    not trusted.
//!
//! The *monitor* is the other half.  An operation with an `ensures` block is
//! wrapped: every call any generated test makes (examples, claims, table rows,
//! the synthesized `<op>/post` suites, the negative controls) is followed by the
//! evaluation of its rows on the call's arguments and result.

use super::*;

/// One `inout` parameter, resolved against its host form.
struct Carrier {
    /// Position among the operation's parameters (the argument index).
    idx: usize,
    ty: Ty,
    form: InoutForm,
    /// The host type the adapter builds and reads back (`to_host`/`from_host`).
    host: HostTy,
}

fn w0704(op: &str, span: (usize, usize), msg: impl Into<String>) -> CodegenError {
    CodegenError {
        code: DiagnosticCode::W0704,
        kind: CodegenErrorKind::MissingBinding(format!("operation `{op}`: {}", msg.into())),
        span,
    }
}

fn w0501(op: &str, span: (usize, usize), msg: impl Into<String>) -> CodegenError {
    CodegenError {
        code: DiagnosticCode::W0501,
        kind: CodegenErrorKind::MissingBinding(format!("operation `{op}`: {}", msg.into())),
        span,
    }
}

/// The post-state slot of an operation, over the generator's own types.
fn slot_of(posts: &[Ty], full: &Ty) -> Option<PostSlot> {
    let shape = match full {
        Ty::Res(t, _) => match &**t {
            Ty::Tuple(cs) => SlotShape::ResultTuple(cs.clone()),
            other => SlotShape::ResultPlain(other.clone()),
        },
        Ty::Tuple(cs) => SlotShape::Tuple(cs.clone()),
        other => SlotShape::Plain(other.clone()),
    };
    resolve_post_slot(posts, &shape)
}

/// Resolve one parameter's host form, or refuse it (W0704).
fn carrier_of(
    op: &OpDecl<'_>,
    idx: usize,
    ty: &Ty,
    form: &InoutForm,
) -> Result<Carrier, CodegenError> {
    let name = op.params[idx].name.to_string();
    let no = |why: String| {
        w0704(
            op.name,
            op.params[idx].span,
            format!("host form `{}` cannot carry the type of inout parameter `{name}`: {why}", form.text()),
        )
    };
    let host = match (form, ty) {
        (InoutForm::Slice, Ty::Bytes | Ty::BytesN(_)) => HostTy::Bytes,
        (InoutForm::Vec, Ty::Bytes) => HostTy::Bytes,
        (InoutForm::Vec, Ty::List(c)) => {
            HostTy::List(Box::new(default_host(c).map_err(&no)?))
        }
        (InoutForm::Array, Ty::BytesN(n)) => HostTy::BytesN(*n),
        (InoutForm::Int(ht), Ty::Fx(f)) if f.suffix() == ht => HostTy::Int(ht.clone()),
        (InoutForm::Int(ht), Ty::Nat | Ty::NatCount) if ht.starts_with('u') => {
            HostTy::Int(ht.clone())
        }
        (InoutForm::Bool, Ty::Bool) => HostTy::Bool,
        (InoutForm::Struct(n), Ty::Data { name: tn, ctors }) if n == tn && ctors.len() == 1 => {
            HostTy::Named(n.clone())
        }
        (
            InoutForm::Returned
            | InoutForm::SliceOfStruct(_)
            | InoutForm::VecOfStruct(_)
            | InoutForm::Codec(_),
            _,
        ) => {
            return Err(w0704(
                op.name,
                op.params[idx].span,
                format!(
                    "host form `{}` of inout parameter `{name}` is not yet implemented (VER04k section 4.13 item 3)",
                    form.text()
                ),
            ))
        }
        _ => return Err(no(format!("the parameter is {ty:?}"))),
    };
    Ok(Carrier {
        idx,
        ty: ty.clone(),
        form: form.clone(),
        host,
    })
}

impl Carrier {
    /// The expression that passes the host copy `h<idx>` to the subject.
    fn call_arg(&self) -> String {
        match self.form {
            // The slice of the copy, not the `Vec` that holds it, so a trait
            // method (`Zeroize::zeroize`) resolves to the `[u8]` implementation.
            InoutForm::Slice => format!("&mut h{}[..]", self.idx),
            _ => format!("&mut h{}", self.idx),
        }
    }

    /// Does `fault` rewrite this parameter's post-state (VER04k section 4.8)?
    fn faultable_by(&self, fault: &str) -> bool {
        match fault {
            "stale-post" => true,
            "skip-last-write" => matches!(self.ty, Ty::Bytes | Ty::BytesN(_) | Ty::List(_)),
            "drop-last-push" => self.form == InoutForm::Vec,
            "clobber-prefix" => self.form == InoutForm::Vec && self.ty == Ty::Bytes,
            "cursor-off-by-one" => matches!(self.form, InoutForm::Int(_)),
            _ => false,
        }
    }
}

/// The adapter of an operation with `inout` parameters (VER04k section 4.6),
/// emitted in the shape every other adapter has: `fn op_<name>(_w, args, fault) -> V`.
#[allow(clippy::too_many_lines)]
pub(super) fn inout_adapter<'a>(
    op: &OpDecl<'a>,
    env: &TyEnv<'a>,
    export: &str,
    err_case: Option<&str>,
    faults: &[String],
    options: &EvidenceOptions,
) -> Result<String, CodegenError> {
    let n = op.name;
    let bad = |m: String| unsupported(format!("operation `{n}`: {m}"), op.span);
    if !op.static_params.is_empty() {
        return Err(bad("static widths are not supported on an inout operation".into()));
    }
    let host = options
        .hosts
        .ops
        .get(n)
        .ok_or_else(|| {
            let names: Vec<&str> = op.params.iter().filter(|p| p.inout).map(|p| p.name).collect();
            w0501(
                n,
                op.span,
                format!(
                    "has inout parameter(s) {} and no `inout=` entry in its binding",
                    names.join(", ")
                ),
            )
        })?;

    // The bindings name exactly the inout parameters: a stray entry is refused
    // first (it is usually a typo for a parameter that then looks "missing").
    for (p, _) in &host.inout {
        if !op.params.iter().any(|q| q.inout && q.name == p) {
            return Err(w0501(
                n,
                op.span,
                format!("`inout=` names `{p}`, which is not an inout parameter"),
            ));
        }
    }
    let ptys: Vec<Ty> = op
        .params
        .iter()
        .map(|p| ty_of_op_param(p, true, env))
        .collect::<Result<_, _>>()?;
    let mut carriers: Vec<Carrier> = Vec::new();
    for (i, p) in op.params.iter().enumerate() {
        if !p.inout {
            continue;
        }
        let Some((_, form)) = host.inout.iter().find(|(q, _)| q == p.name) else {
            return Err(w0501(
                n,
                p.span,
                format!("inout parameter `{}` has no host form (write `inout={}:<form>`)", p.name, p.name),
            ));
        };
        carriers.push(carrier_of(op, i, &ptys[i], form)?);
    }

    // Plain (non-inout) parameters: host types from `args=` (one per plain
    // parameter, in order) or the defaults of the Warrant types.
    let plain: Vec<usize> = (0..ptys.len()).filter(|i| !op.params[*i].inout).collect();
    let hargs: Vec<HostTy> = match &host.args {
        Some(a) if a.len() == plain.len() => a.clone(),
        Some(a) => {
            return Err(bad(format!(
                "args= has {} entries for {} non-inout parameters",
                a.len(),
                plain.len()
            )))
        }
        None => plain
            .iter()
            .map(|i| default_host(&ptys[*i]))
            .collect::<Result<_, _>>()
            .map_err(bad)?,
    };
    if options.arg_types.contains_key(n) && host.args.is_none() {
        return Err(bad("an args= entry is not a host type".into()));
    }

    // The result type and the slot of the post-states.
    let full = ty_of_type(&op.return_type, env, true, op.span, 0)?;
    let posts: Vec<Ty> = carriers.iter().map(|c| c.ty.clone()).collect();
    let slot = slot_of(&posts, &full).ok_or_else(|| {
        bad("the result type has no post-state slot for its inout parameters (S1 to S4)".into())
    })?;
    let (payload, cases, is_result) = match &full {
        Ty::Res(t, cases) => ((**t).clone(), cases.clone(), true),
        other => (other.clone(), Vec::new(), false),
    };
    // The components the host returns itself (the first `h` of a tuple).
    let rem: Vec<Ty> = match (&payload, slot) {
        (Ty::Tuple(cs), PostSlot::S3 { h } | PostSlot::S4 { h }) => cs[..h].to_vec(),
        _ => Vec::new(),
    };

    // Every fault must apply (4.8, "Applies when"), never be silently inert.
    for f in faults {
        let applies = match f.as_str() {
            "missing-error" => is_result,
            other if WXG_FAULTS.iter().any(|(k, _)| *k == other) => {
                carriers.iter().any(|c| c.faultable_by(other))
            }
            other => {
                return Err(bad(format!("fault `{other}` is not an inout-operation fault")));
            }
        };
        if !applies {
            return Err(bad(format!(
                "fault `{f}` does not apply to the inout parameters or the result of the operation (VER04k section 4.8)"
            )));
        }
    }

    let mut hc = HostCx {
        op: n.to_string(),
        spec: &options.hosts,
        fields: &env.fields,
        ctr: 0,
    };
    let mut s = String::new();
    let _ = writeln!(s, "#[allow(unused_variables, unused_mut)]");
    let _ = writeln!(s, "fn op_{n}(_w: &[u64], args: &[V], fault: Fault) -> V {{");
    let _ = writeln!(s, "    let fault = fault_for(\"{n}\", fault);");
    for (k, i) in plain.iter().enumerate() {
        let v = to_host(&ptys[*i], &hargs[k], &format!("args[{i}]"), true, &mut hc).map_err(bad)?;
        let _ = writeln!(s, "    let p{i} = {v};");
    }
    for c in &carriers {
        // Pre-state isolation: the subject's buffer is a copy of `args[idx]`.
        let v = to_host(&c.ty, &c.host, &format!("args[{}]", c.idx), false, &mut hc).map_err(bad)?;
        if matches!(c.host, HostTy::Bytes) {
            let _ = writeln!(s, "    let mut h{}: Vec<u8> = {v};", c.idx);
        } else {
            let _ = writeln!(s, "    let mut h{} = {v};", c.idx);
        }
    }
    let arglist: Vec<String> = (0..ptys.len())
        .map(|i| match carriers.iter().find(|c| c.idx == i) {
            Some(c) => c.call_arg(),
            None => format!("p{i}"),
        })
        .collect();
    let call = format!("{}::{export}({})", options.package_crate, arglist.join(", "));

    // The post-state of one parameter, read back from the buffer the subject
    // was given and then rewritten by the faults that attack it.
    let post_of = |c: &Carrier, hc: &mut HostCx<'_>| -> Result<String, CodegenError> {
        let raw = from_host(&c.ty, &c.host, &format!("h{}", c.idx), hc).map_err(&bad)?;
        let mut arms = String::new();
        for f in faults {
            if !c.faultable_by(f) {
                continue;
            }
            let arm = match f.as_str() {
                "stale-post" => format!("Fault::StalePost => args[{}].clone(), ", c.idx),
                "skip-last-write" => {
                    format!("Fault::SkipLastWrite => fault_skip_last_write(raw, &args[{}]), ", c.idx)
                }
                "drop-last-push" => {
                    format!("Fault::DropLastPush => fault_drop_last_push(raw, &args[{}]), ", c.idx)
                }
                "clobber-prefix" => {
                    format!("Fault::ClobberPrefix => fault_clobber_prefix(raw, &args[{}]), ", c.idx)
                }
                _ => "Fault::CursorOffByOne => fault_cursor_off_by_one(raw), ".to_string(),
            };
            arms.push_str(&arm);
        }
        Ok(if arms.is_empty() {
            raw
        } else {
            format!("{{ let raw: V = {raw}; match fault {{ {arms}_ => raw }} }}")
        })
    };
    let mut post_srcs = Vec::new();
    for c in &carriers {
        post_srcs.push(post_of(c, &mut hc)?);
    }

    // The value built from the host's return `r` and the post-states.
    let mut parts: Vec<String> = Vec::new();
    match rem.len() {
        0 => {}
        1 => {
            let h = match host.ret.clone() {
                Some(h) => h,
                None => default_host(&rem[0]).map_err(&bad)?,
            };
            parts.push(from_host(&rem[0], &h, "r", &mut hc).map_err(&bad)?);
        }
        k => {
            let hs = match host.ret.clone() {
                Some(HostTy::Tuple(hs)) if hs.len() == k => hs,
                Some(_) => return Err(bad(format!("ret= must be a {k}-tuple host type"))),
                None => rem
                    .iter()
                    .map(default_host)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(&bad)?,
            };
            for (j, (t, h)) in rem.iter().zip(&hs).enumerate() {
                parts.push(from_host(t, h, &format!("r{j}"), &mut hc).map_err(&bad)?);
            }
        }
    }
    // The exact pattern the host's return is bound with (VER04k 4.13 items 12, 13):
    // `()` for no component, `r` for one, `(r0, r1, ..)` for several, so a host that
    // returns more, fewer or other components is a type error, not a silent drop.
    let r_pat = match rem.len() {
        0 => "()".to_string(),
        1 => "r".to_string(),
        k => format!("({})", (0..k).map(|j| format!("r{j}")).collect::<Vec<_>>().join(", ")),
    };
    parts.extend(post_srcs.iter().cloned());
    let built = match slot {
        PostSlot::S1 | PostSlot::S2 => post_srcs[0].clone(),
        PostSlot::S3 { .. } | PostSlot::S4 { .. } => format!("V::Tup(vec![{}])", parts.join(", ")),
    };
    let ok_value = if is_result {
        format!("V::Ok(Box::new({built}))")
    } else {
        built
    };
    let minimal = format!("domain_take({}, 1).remove(0)", payload.rust());
    let want_missing = faults.iter().any(|f| f == "missing-error");

    if !is_result {
        // `let ()` for a host that returns nothing makes one that does return
        // something fail to compile instead of being silently ignored.
        let _ = writeln!(s, "    let {r_pat} = {call};");
        let _ = writeln!(s, "    let v: V = {ok_value};");
    } else {
        // No returned component: the host's `Ok` must be exactly `()`, so a host
        // that returns data (a count) fails to compile instead of being dropped.
        let ok_pat = format!("Ok({r_pat})");
        if let Some(case) = host.err_any.as_deref() {
            // WX-N `err_any=CASE`: every `Err(_)` is the one Warrant case.
            let declared = env_cases_of(&op.return_type, env);
            let n_cases = if declared.is_empty() { cases.len() } else { declared.len() };
            if n_cases != 1 || !declared.contains(&case.to_string()) {
                return Err(CodegenError {
                    code: DiagnosticCode::W0704,
                    kind: CodegenErrorKind::MissingBinding(format!(
                        "err_any= is incompatible with `{n}`: its error type must have exactly one case, and `{case}` must be it"
                    )),
                    span: op.span,
                });
            }
            let _ = writeln!(s, "    let v: V = match {call} {{");
            let _ = writeln!(s, "        {ok_pat} => {ok_value},");
            if want_missing {
                let _ = writeln!(
                    s,
                    "        Err(_) if fault == Fault::MissingError => V::Ok(Box::new({minimal})),"
                );
            }
            let _ = writeln!(s, "        Err(_) => V::Err(\"{case}\".into()),\n    }};");
        } else if let Some(he) = host.errors.as_ref() {
            let decl_cases = env_cases_of(&op.return_type, env);
            for (_, case) in &he.map {
                if !cases.contains(case) && !decl_cases.contains(case) {
                    return Err(bad(format!("errors= names `{case}`, which is not an error case")));
                }
            }
            let _ = writeln!(s, "    let v: V = match {call} {{");
            let _ = writeln!(s, "        {ok_pat} => {ok_value},");
            if want_missing {
                let _ = writeln!(
                    s,
                    "        Err(_) if fault == Fault::MissingError => V::Ok(Box::new({minimal})),"
                );
            }
            let _ = writeln!(s, "        Err(e) => match e {{");
            for (variant, case) in &he.map {
                let _ = writeln!(
                    s,
                    "            {}::{variant} {{ .. }} => V::Err(\"{case}\".into()),",
                    he.path
                );
            }
            let _ = writeln!(
                s,
                "            #[allow(unreachable_patterns)]\n            _ => panic!(\"unmapped host error\"),\n        }},\n    }};"
            );
        } else {
            // `error=CASE` (VER04k section 4.6, amended 2026-11-14): the host
            // returns the remaining components directly; a panic is `Err` and
            // carries no post-state.
            let case = err_case.ok_or_else(|| {
                w0704(n, op.span, "binding has neither error normalization nor errors= nor err_any=")
            })?;
            let some_pat = format!("Some({r_pat})");
            let _ = writeln!(s, "    let v: V = match contained(|| {call}) {{");
            let _ = writeln!(s, "        {some_pat} => {ok_value},");
            if want_missing {
                let _ = writeln!(
                    s,
                    "        None if fault == Fault::MissingError => V::Ok(Box::new({minimal})),"
                );
            }
            let _ = writeln!(s, "        None => V::Err(\"{case}\".into()),\n    }};");
        }
    }
    s.push_str("    v\n}\n\n");
    Ok(s)
}

/// The helper functions the used WX-G faults call, emitted only when a binding
/// names them, so a document that has none generates what it always did.
pub(super) fn fault_helpers(used: &BTreeSet<String>) -> String {
    let mut s = String::new();
    if used.contains("skip-last-write") {
        s.push_str(
            "/// skip-last-write (CWE-193, an off-by-one loop bound): the last element of the\n\
             /// post-state is the pre-state's at the same index, when the pre-state has one.\n\
             fn fault_skip_last_write(post: V, pre: &V) -> V {\n\
             \x20   match (post, pre) {\n\
             \x20       (V::Bytes(mut p), V::Bytes(q)) => {\n\
             \x20           if let Some(i) = p.len().checked_sub(1) {\n\
             \x20               if i < q.len() {\n\
             \x20                   p[i] = q[i];\n\
             \x20               }\n\
             \x20           }\n\
             \x20           V::Bytes(p)\n\
             \x20       }\n\
             \x20       (V::List(mut p), V::List(q)) => {\n\
             \x20           if let Some(i) = p.len().checked_sub(1) {\n\
             \x20               if i < q.len() {\n\
             \x20                   p[i] = q[i].clone();\n\
             \x20               }\n\
             \x20           }\n\
             \x20           V::List(p)\n\
             \x20       }\n\
             \x20       (other, _) => other,\n\
             \x20   }\n\
             }\n\n",
        );
    }
    if used.contains("drop-last-push") {
        s.push_str(
            "/// drop-last-push (a lost append): a post-state longer than the pre-state loses\n\
             /// its last element.\n\
             fn fault_drop_last_push(post: V, pre: &V) -> V {\n\
             \x20   match (post, pre) {\n\
             \x20       (V::Bytes(mut p), V::Bytes(q)) => {\n\
             \x20           if p.len() > q.len() {\n\
             \x20               p.pop();\n\
             \x20           }\n\
             \x20           V::Bytes(p)\n\
             \x20       }\n\
             \x20       (V::List(mut p), V::List(q)) => {\n\
             \x20           if p.len() > q.len() {\n\
             \x20               p.pop();\n\
             \x20           }\n\
             \x20           V::List(p)\n\
             \x20       }\n\
             \x20       (other, _) => other,\n\
             \x20   }\n\
             }\n\n",
        );
    }
    if used.contains("clobber-prefix") {
        s.push_str(
            "/// clobber-prefix (an append that overwrote existing data): when the pre-state is\n\
             /// non-empty, the first byte of the post-state has its low bit flipped.\n\
             fn fault_clobber_prefix(post: V, pre: &V) -> V {\n\
             \x20   match (post, pre) {\n\
             \x20       (V::Bytes(mut p), V::Bytes(q)) => {\n\
             \x20           if !q.is_empty() && !p.is_empty() {\n\
             \x20               p[0] ^= 1;\n\
             \x20           }\n\
             \x20           V::Bytes(p)\n\
             \x20       }\n\
             \x20       (other, _) => other,\n\
             \x20   }\n\
             }\n\n",
        );
    }
    if used.contains("cursor-off-by-one") {
        s.push_str(
            "/// cursor-off-by-one (a cursor advanced by the wrong amount): an integer post-state\n\
             /// gets +1 when even and -1 when odd, wrapping in its type.\n\
             fn fault_cursor_off_by_one(post: V) -> V {\n\
             \x20   match post {\n\
             \x20       V::Fixed(f, v) => {\n\
             \x20           let w = if v.rem_euclid(2) == 0 { v + 1 } else { v - 1 };\n\
             \x20           V::Fixed(f, f.canon(f.pattern(w)))\n\
             \x20       }\n\
             \x20       V::Int(v) => V::Int(if v.rem_euclid(2) == 0 { v + 1 } else { v - 1 }),\n\
             \x20       other => other,\n\
             \x20   }\n\
             }\n\n",
        );
    }
    s
}

/// The monitor of one operation's `ensures` block: `fn post_<op>(args, result)`.
///
/// It runs only when the arguments satisfy the parameters' refinements (a call
/// outside the admitted domain is governed by the input contract, VER04k section
/// 4.7), evaluates each row with the parameters bound to the arguments (the
/// pre-state) and `result` to the operation's return value, and raises a
/// `PostViolation` for a false row (`postcondition-violated: <op> row <i>`) or a
/// row that cannot be evaluated (`postcondition-unevaluable: <op> row <i>`),
/// numbering rows from 1.  Rows cannot call operations, so monitoring never
/// recurses, and they cannot quantify, so it never multiplies the work.
pub(super) fn post_monitor<'a>(
    op: &OpDecl<'a>,
    ens: &EnsuresDecl<'a>,
    models: &HashSet<String>,
    ops: &HashMap<String, usize>,
    errors: &HashSet<String>,
    ctors: &HashMap<String, (String, usize, usize)>,
    widths: &mut BTreeMap<String, BTreeSet<u64>>,
) -> Result<String, CodegenError> {
    let n = op.name;
    if !op.static_params.is_empty() {
        return Err(unsupported(
            format!("operation `{n}`: an `ensures` block on an operation with static widths is not supported"),
            ens.span,
        ));
    }
    let mut binders: HashSet<String> = op.params.iter().map(|p| p.name.to_string()).collect();
    binders.insert("result".to_string());
    let mut cx = Cx {
        binders,
        models,
        ops,
        errors,
        allow_ops: false,
        widths,
        v1: true,
        ctors,
    };
    let mut admit = String::from("true");
    for p in &op.params {
        if let Some(r) = &p.refinement {
            let _ = write!(admit, " && {}.truthy()", emit(r, &mut cx)?);
        }
    }
    let mut body = String::new();
    let _ = writeln!(body, "#[allow(unused_variables, unused_mut)]");
    let _ = writeln!(body, "fn post_{n}(args: &[V], result: &V) {{");
    for (i, p) in op.params.iter().enumerate() {
        let _ = writeln!(body, "    let b_{} = args[{i}].clone();", p.name);
    }
    let _ = writeln!(body, "    if !({admit}) {{\n        return;\n    }}");
    let _ = writeln!(body, "    let b_result = result.clone();");
    let _ = writeln!(body, "    post_count(\"{n}\");");
    for (i, row) in ens.rows.iter().enumerate() {
        let r = emit(row, &mut cx)?;
        let k = i + 1;
        let _ = writeln!(
            body,
            "    match catch_unwind(AssertUnwindSafe(|| {r}.truthy())) {{\n\
             \x20       Ok(true) => {{}}\n\
             \x20       Ok(false) => std::panic::panic_any(PostViolation(\"postcondition-violated: {n} row {k}\".to_string())),\n\
             \x20       Err(_) => std::panic::panic_any(PostViolation(\"postcondition-unevaluable: {n} row {k}\".to_string())),\n\
             \x20   }}"
        );
    }
    body.push_str("}\n\n");
    Ok(body)
}

/// The wrapper every call of a monitored operation goes through.  The operation's
/// own adapter was renamed `op_<n>_core` by the caller.
pub(super) fn post_wrapper(n: &str) -> String {
    format!(
        "#[allow(unused_variables)]\nfn op_{n}(w: &[u64], args: &[V], fault: Fault) -> V {{\n    let v = op_{n}_core(w, args, fault);\n    post_{n}(args, &v);\n    v\n}}\n\n"
    )
}

/// Example self-consistency (VER04k section 4.7, W0601): for an example of the
/// form `op(<ground args>) == <ground expected>` of an operation with an `ensures`
/// block, evaluate the rows with the parameters bound to the arguments and
/// `result` to the expected value.  A false row means the document contradicts
/// itself, and generation fails before any subject runs.  An example of another
/// form, or a row the ground evaluator cannot evaluate (the monitor will report
/// it at run time), is not this check's business.
pub(super) fn check_examples_against_ensures<'a>(
    file: &'a WarrantFile<'a>,
    ensures_of: &BTreeMap<&str, &'a EnsuresDecl<'a>>,
) -> Result<(), CodegenError> {
    use crate::codegen::{extract_example_case, CodegenLimits, EvalValue, Evaluator};
    if ensures_of.is_empty() {
        return Ok(());
    }
    let mut ev = Evaluator::new(file, CodegenLimits::default());
    for d in &file.declarations {
        let Declaration::Example(ex) = d else { continue };
        let Ok(case) = extract_example_case(ex.name, &ex.body, &mut ev) else {
            continue;
        };
        let Some(en) = ensures_of.get(case.op_name.as_str()) else {
            continue;
        };
        let Some(op) = file.declarations.iter().find_map(|d| match d {
            Declaration::Op(o) if o.name == case.op_name => Some(o),
            _ => None,
        }) else {
            continue;
        };
        if op.params.len() != case.args.len() {
            continue;
        }
        let mut env: HashMap<&'a str, EvalValue> = HashMap::new();
        for (p, a) in op.params.iter().zip(&case.args) {
            env.insert(p.name, a.clone());
        }
        env.insert("result", case.expected.clone());
        // Rows are only required to hold where the refinements admit the call.
        let mut admitted = true;
        for p in &op.params {
            if let Some(r) = &p.refinement {
                match ev.eval(r, &env) {
                    Ok(EvalValue::Bool(true)) => {}
                    _ => admitted = false,
                }
            }
        }
        if !admitted {
            continue;
        }
        for (i, row) in en.rows.iter().enumerate() {
            if let Ok(EvalValue::Bool(false)) = ev.eval(row, &env) {
                return Err(CodegenError {
                    code: DiagnosticCode::W0601,
                    kind: CodegenErrorKind::UnsupportedTarget(format!(
                        "example `{}` contradicts the postcondition of `{}`: ensures row {} is false for its arguments and expected value",
                        ex.name,
                        op.name,
                        i + 1
                    )),
                    span: ex.span,
                });
            }
        }
    }
    Ok(())
}
