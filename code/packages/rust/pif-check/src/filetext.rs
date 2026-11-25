//! Printing a [`FileModel`] as text and reading that text back (VER32 3.6, N5, R3).
//!
//! [`print_file`] is the printer `pif-pp` for whole files (the prelude, the ADL preludes,
//! the model): one line per declaration in the syntax of [`crate::printer`], deterministic,
//! and refusing shadowed binders and unprintable names. [`parse_file`] is its independent
//! inverse, built on [`crate::ppparse`] and sharing no code with the printer.
//! `parse_file(print_file(f)) == f` is what [`crate::printcheck`] verifies for every file a
//! person reviews, so a printer that mapped two different files to one text would be caught.
//!
//! The format is documented in [`crate::filemodel`].

use crate::filemodel::{DeclModel, FileModel, HintModel, IndModel, ModelError};
use crate::ppparse::{tokenize, Parser, PpError, Tok};
use crate::printer::{print_name, print_term};
use lean_kernel::Name;
use std::fmt::Write as _;

/// A PIF/printed STRING: printable ASCII as itself except `"` and `\`, everything else as
/// `\u{h}` (so a space is `\u{20}`).
fn quote(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '!'..='~' => o.push(c),
            _ => {
                let _ = write!(o, "\\u{{{:x}}}", c as u32);
            }
        }
    }
    o.push('"');
    o
}

fn p<T>(r: Result<T, crate::printer::PrintError>) -> Result<T, ModelError> {
    r.map_err(ModelError::Print)
}

/// `Name.{u, v}` (the `.{..}` only when there are parameters).
fn head(name: &Name, params: &[Name]) -> Result<String, ModelError> {
    let mut s = p(print_name(name))?;
    if !params.is_empty() {
        let ps: Result<Vec<String>, _> = params.iter().map(print_name).collect();
        let _ = write!(s, ".{{{}}}", p(ps)?.join(", "));
    }
    Ok(s)
}

/// Print a file model as text. Every line ends in LF.
pub fn print_file(m: &FileModel) -> Result<String, ModelError> {
    let mut o = String::from("pifpp 1\n");
    let _ = writeln!(o, "profile {} {}", m.profile.0, quote(&m.profile.1));
    let _ = writeln!(
        o,
        "features{}",
        m.features
            .iter()
            .map(|f| format!(" {f}"))
            .collect::<String>()
    );
    for i in &m.imports {
        let _ = writeln!(o, "import sha256:{i}");
    }
    for d in &m.decls {
        print_decl(d, &mut o).map_err(|e| match e {
            ModelError::Print(pe) => ModelError::PrintIn(decl_name(d), pe),
            other => other,
        })?;
    }
    Ok(o)
}

/// The name of a declaration, for error messages.
fn decl_name(d: &DeclModel) -> String {
    match d {
        DeclModel::Axiom { name, .. }
        | DeclModel::Def { name, .. }
        | DeclModel::Theorem { name, .. }
        | DeclModel::Opaque { name, .. } => crate::report::dotted(name),
        DeclModel::Inductive { types, .. } => types
            .first()
            .map(|t| crate::report::dotted(&t.name))
            .unwrap_or_default(),
        DeclModel::Goal { claim, .. } => format!("goal {claim}"),
    }
}

/// Print one declaration (one or more lines) onto `o`.
fn print_decl(d: &DeclModel, o: &mut String) -> Result<(), ModelError> {
    {
        match d {
            DeclModel::Axiom { name, params, ty } => {
                let _ = writeln!(o, "axiom {} : {}", head(name, params)?, p(print_term(ty))?);
            }
            DeclModel::Theorem {
                name,
                params,
                ty,
                value,
            }
            | DeclModel::Opaque {
                name,
                params,
                ty,
                value,
            } => {
                let kw = if matches!(d, DeclModel::Theorem { .. }) {
                    "theorem"
                } else {
                    "opaque"
                };
                let _ = writeln!(
                    o,
                    "{kw} {} : {} := {}",
                    head(name, params)?,
                    p(print_term(ty))?,
                    p(print_term(value))?
                );
            }
            DeclModel::Def {
                name,
                params,
                ty,
                value,
                hint,
            } => {
                let h = match hint {
                    HintModel::Abbrev => "@abbrev".to_string(),
                    HintModel::Opaque => "@opaque".to_string(),
                    HintModel::Regular(k) => format!("@regular {k}"),
                };
                let _ = writeln!(
                    o,
                    "def {} : {} := {} {h}",
                    head(name, params)?,
                    p(print_term(ty))?,
                    p(print_term(value))?
                );
            }
            DeclModel::Inductive {
                params,
                num_params,
                types,
            } => {
                let ps = if params.is_empty() {
                    String::new()
                } else {
                    let v: Result<Vec<String>, _> = params.iter().map(print_name).collect();
                    format!(".{{{}}}", p(v)?.join(", "))
                };
                let _ = writeln!(o, "inductive{ps} {num_params} {}", types.len());
                for t in types {
                    let _ = writeln!(
                        o,
                        "  type {} : {}",
                        p(print_name(&t.name))?,
                        p(print_term(&t.ty))?
                    );
                    for (c, ty) in &t.ctors {
                        let _ =
                            writeln!(o, "    ctor {} : {}", p(print_name(c))?, p(print_term(ty))?);
                    }
                }
            }
            DeclModel::Goal {
                claim,
                theorem,
                digest,
            } => {
                let _ = writeln!(
                    o,
                    "goal {} {} sha256:{digest}",
                    quote(claim),
                    p(print_name(theorem))?
                );
            }
        }
    }
    Ok(())
}

/// Read printed text back into a model.
pub fn parse_file(text: &str) -> Result<FileModel, ModelError> {
    let body = text
        .strip_suffix('\n')
        .ok_or(ModelError::Decode("no final LF", 1))?;
    let lines: Vec<(usize, &str)> = body
        .split('\n')
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .collect();
    let pe = |e: PpError, n: usize| ModelError::Parse(e, n);
    let mut at = 0usize;
    let next = |at: &mut usize| -> Result<(usize, Parser), ModelError> {
        let &(n, l) = lines.get(*at).ok_or(ModelError::Decode("truncated", 1))?;
        *at += 1;
        Ok((n, Parser::new(tokenize(l).map_err(|e| pe(e, n))?)))
    };
    let kw = |q: &mut Parser, want: &str, n: usize| -> Result<(), ModelError> {
        match q.bump().map_err(|e| pe(e, n))? {
            Tok::Name(s) if s == want => Ok(()),
            _ => Err(ModelError::Decode("unexpected line", n)),
        }
    };
    let (n, mut q) = next(&mut at)?;
    kw(&mut q, "pifpp", n)?;
    if q.bump().map_err(|e| pe(e, n))? != Tok::Nat(1) || !q.at_end() {
        return Err(ModelError::Decode("unknown printed version", n));
    }
    let (n, mut q) = next(&mut at)?;
    kw(&mut q, "profile", n)?;
    let id = match q.bump().map_err(|e| pe(e, n))? {
        Tok::Name(s) => s,
        _ => return Err(ModelError::Decode("bad profile", n)),
    };
    let label = q.string().map_err(|e| pe(e, n))?;
    if !q.at_end() {
        return Err(ModelError::Decode("bad profile", n));
    }
    let (n, mut q) = next(&mut at)?;
    kw(&mut q, "features", n)?;
    let mut features = Vec::new();
    while let Some(Tok::Name(s)) = q.peek().cloned() {
        q.pos += 1;
        features.push(s);
    }
    if !q.at_end() {
        return Err(ModelError::Decode("bad features", n));
    }
    let mut m = FileModel {
        profile: (id, label),
        features,
        imports: vec![],
        decls: vec![],
    };
    while let Some(&(n, l)) = lines.get(at) {
        if !l.starts_with("import ") {
            break;
        }
        let (_, mut q) = next(&mut at)?;
        q.pos = 1;
        match (q.bump().map_err(|e| pe(e, n))?, q.at_end()) {
            (Tok::Digest(d), true) => m.imports.push(d),
            _ => return Err(ModelError::Decode("bad import", n)),
        }
    }
    while at < lines.len() {
        let (n, mut q) = next(&mut at)?;
        let Tok::Name(word) = q.bump().map_err(|e| pe(e, n))? else {
            return Err(ModelError::Decode("expected a keyword", n));
        };
        let colon = |q: &mut Parser| q.expect(&Tok::Colon).map_err(|e| pe(e, n));
        match word.as_str() {
            "axiom" | "def" | "theorem" | "opaque" => {
                let name = q.name().map_err(|e| pe(e, n))?;
                let params = q.params().map_err(|e| pe(e, n))?;
                colon(&mut q)?;
                let ty = q.term().map_err(|e| pe(e, n))?;
                if word == "axiom" {
                    m.decls.push(DeclModel::Axiom { name, params, ty });
                } else {
                    q.expect(&Tok::Assign).map_err(|e| pe(e, n))?;
                    let value = q.term().map_err(|e| pe(e, n))?;
                    m.decls.push(match word.as_str() {
                        "theorem" => DeclModel::Theorem {
                            name,
                            params,
                            ty,
                            value,
                        },
                        "opaque" => DeclModel::Opaque {
                            name,
                            params,
                            ty,
                            value,
                        },
                        _ => {
                            let hint = match q.bump().map_err(|e| pe(e, n))? {
                                Tok::At(h) if h == "abbrev" => HintModel::Abbrev,
                                Tok::At(h) if h == "opaque" => HintModel::Opaque,
                                Tok::At(h) if h == "regular" => {
                                    match q.bump().map_err(|e| pe(e, n))? {
                                        Tok::Nat(k) => HintModel::Regular(
                                            u32::try_from(k)
                                                .map_err(|_| ModelError::Decode("bad hint", n))?,
                                        ),
                                        _ => return Err(ModelError::Decode("bad hint", n)),
                                    }
                                }
                                _ => return Err(ModelError::Decode("bad hint", n)),
                            };
                            DeclModel::Def {
                                name,
                                params,
                                ty,
                                value,
                                hint,
                            }
                        }
                    });
                }
                if !q.at_end() {
                    return Err(ModelError::Decode("trailing tokens", n));
                }
            }
            "inductive" => {
                let params = q.params().map_err(|e| pe(e, n))?;
                let (Tok::Nat(np), Tok::Nat(nt)) = (
                    q.bump().map_err(|e| pe(e, n))?,
                    q.bump().map_err(|e| pe(e, n))?,
                ) else {
                    return Err(ModelError::Decode("bad inductive header", n));
                };
                if !q.at_end() || nt > 4096 {
                    return Err(ModelError::Decode("bad inductive header", n));
                }
                let mut types = Vec::new();
                for _ in 0..nt {
                    let (n2, mut q2) = next(&mut at)?;
                    kw(&mut q2, "type", n2)?;
                    let name = q2.name().map_err(|e| pe(e, n2))?;
                    q2.expect(&Tok::Colon).map_err(|e| pe(e, n2))?;
                    let ty = q2.term().map_err(|e| pe(e, n2))?;
                    if !q2.at_end() {
                        return Err(ModelError::Decode("trailing tokens", n2));
                    }
                    let mut ctors = Vec::new();
                    while let Some(&(n3, l3)) = lines.get(at) {
                        if !l3.trim_start().starts_with("ctor ") {
                            break;
                        }
                        let (_, mut q3) = next(&mut at)?;
                        kw(&mut q3, "ctor", n3)?;
                        let cname = q3.name().map_err(|e| pe(e, n3))?;
                        q3.expect(&Tok::Colon).map_err(|e| pe(e, n3))?;
                        let cty = q3.term().map_err(|e| pe(e, n3))?;
                        if !q3.at_end() {
                            return Err(ModelError::Decode("trailing tokens", n3));
                        }
                        ctors.push((cname, cty));
                    }
                    types.push(IndModel { name, ty, ctors });
                }
                m.decls.push(DeclModel::Inductive {
                    params,
                    num_params: u32::try_from(np)
                        .map_err(|_| ModelError::Decode("bad inductive header", n))?,
                    types,
                });
            }
            "goal" => {
                let claim = q.string().map_err(|e| pe(e, n))?;
                let theorem = q.name().map_err(|e| pe(e, n))?;
                match (q.bump().map_err(|e| pe(e, n))?, q.at_end()) {
                    (Tok::Digest(digest), true) => m.decls.push(DeclModel::Goal {
                        claim,
                        theorem,
                        digest,
                    }),
                    _ => return Err(ModelError::Decode("bad goal", n)),
                }
            }
            _ => return Err(ModelError::Decode("unknown declaration", n)),
        }
    }
    Ok(m)
}
