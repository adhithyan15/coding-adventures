//! A PIF file as a person reads it: the declarations with their terms as trees with the
//! file's own binder names, and the printed form of that (VER32 3.6, N5, R3).
//!
//! # Three jobs
//!
//! 1. **Decode** ([`decode`]): read PIF *text* into a [`FileModel`]. This is a second reader
//!    of the format, written independently of the trusted reader (`lean_kernel::pif`) and
//!    **not through the kernel store**: the kernel's store keeps whichever binder name was
//!    interned first, so reading names back from it would show a reviewer names the file does
//!    not contain. The decoder keeps the file's own names.
//! 2. **Print** ([`print_file`]) a model as lines of the syntax of [`crate::printer`].
//! 3. **Parse** ([`parse_file`]) that text back, with [`crate::ppparse`].
//!
//! [`crate::printcheck`] then checks `parse(print(F)) = F` and that the decoded file agrees
//! with what the kernel accepted, declaration by declaration.
//!
//! # The printed file
//!
//! ```text
//!   pifpp 1
//!   profile native "pif-prelude 0.1"
//!   features
//!   import sha256:<64 hex>
//!   axiom Name.{u, v} : <type>
//!   def Name.{u} : <type> := <value> @regular 1
//!   theorem Name : <type> := <value>
//!   opaque Name : <type> := <value>
//!   inductive.{u} <params> <types>
//!     type Name : <type>
//!       ctor Name.c : <type>
//!   goal "claim.id" Name sha256:<64 hex>
//! ```
//!
//! The kernel-derived recursor of an inductive block is not printed (the file does not
//! declare it). The trailer count `end k` is a property of the bytes, not of the model.

use crate::cte::{Dterm, Lv, Node};

use crate::ppparse::PpError;
use crate::printer::PrintError;
use lean_kernel::{BinderInfo, Name};

/// Why a file could not be decoded, printed or parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    /// The PIF text is malformed (why, 1-based line).
    Decode(&'static str, usize),
    /// A construct the decoder does not support yet (literals, projections, quotients, prims).
    Unsupported(&'static str),
    /// A tree larger than the size cap.
    TooLarge,
    /// The printer refused.
    Print(PrintError),
    /// The printer refused while printing the named declaration.
    PrintIn(String, PrintError),
    /// The printed text did not parse (why, 1-based line of the printed text).
    Parse(PpError, usize),
}

/// How a definition is unfolded (`def` hints).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintModel {
    Abbrev,
    Opaque,
    Regular(u32),
}

/// One type of an inductive block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndModel {
    pub name: Name,
    pub ty: Dterm,
    pub ctors: Vec<(Name, Dterm)>,
}

/// One declaration of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclModel {
    Axiom {
        name: Name,
        params: Vec<Name>,
        ty: Dterm,
    },
    Def {
        name: Name,
        params: Vec<Name>,
        ty: Dterm,
        value: Dterm,
        hint: HintModel,
    },
    Theorem {
        name: Name,
        params: Vec<Name>,
        ty: Dterm,
        value: Dterm,
    },
    Opaque {
        name: Name,
        params: Vec<Name>,
        ty: Dterm,
        value: Dterm,
    },
    Inductive {
        params: Vec<Name>,
        num_params: u32,
        types: Vec<IndModel>,
    },
    Goal {
        claim: String,
        theorem: Name,
        digest: String,
    },
}

/// A decoded file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileModel {
    pub profile: (String, String),
    pub features: Vec<String>,
    /// Import digests as 64 lowercase hex digits.
    pub imports: Vec<String>,
    pub decls: Vec<DeclModel>,
}

// -----------------------------------------------------------------------------------------
// Decoding PIF text.
// -----------------------------------------------------------------------------------------

/// The size cap shared with the CTE: nodes plus level nodes of one unfolded term.
const MAX_NODES: usize = 1 << 20;

enum LRec {
    Zero,
    Succ(u32),
    Max(u32, u32),
    IMax(u32, u32),
    Param(u32),
}

enum ERec {
    Var(u32),
    Sort(u32),
    Const(u32, Vec<u32>),
    App(u32, u32),
    Lam(BinderInfo, u32, u32, u32),
    Pi(BinderInfo, u32, u32, u32),
    Let(u32, u32, u32, u32),
}

struct Tables {
    names: Vec<Name>,
    levels: Vec<LRec>,
    exprs: Vec<ERec>,
}

impl Tables {
    fn name(&self, i: u32, line: usize) -> Result<Name, ModelError> {
        self.names
            .get(i as usize)
            .cloned()
            .ok_or(ModelError::Decode("name index", line))
    }

    /// Unfold level `i` into `levels` (an explicit stack); returns the arena id.
    fn unfold_level(
        &self,
        i: u32,
        out: &mut Vec<Lv>,
        nodes: usize,
        line: usize,
    ) -> Result<u32, ModelError> {
        enum W {
            Visit(u32),
            Build(u32),
        }
        let bad = ModelError::Decode("level index", line);
        let (mut work, mut vals) = (vec![W::Visit(i)], Vec::<u32>::new());
        while let Some(w) = work.pop() {
            if nodes + out.len() >= MAX_NODES {
                return Err(ModelError::TooLarge);
            }
            match w {
                W::Visit(x) => match self.levels.get(x as usize).ok_or_else(|| bad.clone())? {
                    LRec::Zero => {
                        out.push(Lv::Zero);
                        vals.push(out.len() as u32 - 1);
                    }
                    LRec::Param(n) => {
                        out.push(Lv::Param(self.name(*n, line)?));
                        vals.push(out.len() as u32 - 1);
                    }
                    LRec::Succ(a) => work.extend([W::Build(x), W::Visit(*a)]),
                    LRec::Max(a, b) | LRec::IMax(a, b) => {
                        work.extend([W::Build(x), W::Visit(*b), W::Visit(*a)])
                    }
                },
                W::Build(x) => {
                    let node = match self.levels.get(x as usize).ok_or_else(|| bad.clone())? {
                        LRec::Succ(_) => Lv::Succ(vals.pop().ok_or_else(|| bad.clone())?),
                        r => {
                            let (Some(b), Some(a)) = (vals.pop(), vals.pop()) else {
                                return Err(bad);
                            };
                            if matches!(r, LRec::Max(..)) {
                                Lv::Max(a, b)
                            } else {
                                Lv::IMax(a, b)
                            }
                        }
                    };
                    out.push(node);
                    vals.push(out.len() as u32 - 1);
                }
            }
        }
        vals.pop().ok_or(bad)
    }

    /// Unfold expression `i` (1-based table index) into a tree.
    fn unfold(&self, i: u32, line: usize) -> Result<Dterm, ModelError> {
        enum W {
            Visit(u32),
            Build(u32),
        }
        let bad = ModelError::Decode("expression index", line);
        let (mut nodes, mut levels): (Vec<Node>, Vec<Lv>) = (vec![], vec![]);
        let (mut work, mut vals) = (vec![W::Visit(i)], Vec::<u32>::new());
        let get = |x: u32| -> Result<&ERec, ModelError> {
            (x as usize)
                .checked_sub(1)
                .and_then(|k| self.exprs.get(k))
                .ok_or_else(|| bad.clone())
        };
        while let Some(w) = work.pop() {
            if nodes.len() + levels.len() >= MAX_NODES {
                return Err(ModelError::TooLarge);
            }
            match w {
                W::Visit(x) => match get(x)? {
                    ERec::Var(k) => {
                        nodes.push(Node::Var(*k));
                        vals.push(nodes.len() as u32 - 1);
                    }
                    ERec::Sort(l) => {
                        let id = self.unfold_level(*l, &mut levels, nodes.len(), line)?;
                        nodes.push(Node::Sort(id));
                        vals.push(nodes.len() as u32 - 1);
                    }
                    ERec::Const(n, ls) => {
                        let mut ids = Vec::new();
                        for l in ls {
                            ids.push(self.unfold_level(*l, &mut levels, nodes.len(), line)?);
                        }
                        nodes.push(Node::Const(self.name(*n, line)?, ids));
                        vals.push(nodes.len() as u32 - 1);
                    }
                    ERec::App(f, a) => work.extend([W::Build(x), W::Visit(*a), W::Visit(*f)]),
                    ERec::Lam(_, _, t, b) | ERec::Pi(_, _, t, b) => {
                        work.extend([W::Build(x), W::Visit(*b), W::Visit(*t)])
                    }
                    ERec::Let(_, t, v, b) => {
                        work.extend([W::Build(x), W::Visit(*b), W::Visit(*v), W::Visit(*t)])
                    }
                },
                W::Build(x) => {
                    let node = match get(x)? {
                        ERec::App(..) => {
                            let (Some(a), Some(f)) = (vals.pop(), vals.pop()) else {
                                return Err(bad);
                            };
                            Node::App(f, a)
                        }
                        ERec::Lam(info, n, ..) | ERec::Pi(info, n, ..) => {
                            let (Some(b), Some(t)) = (vals.pop(), vals.pop()) else {
                                return Err(bad);
                            };
                            let name = self.name(*n, line)?;
                            if matches!(get(x)?, ERec::Lam(..)) {
                                Node::Lam(*info, name, t, b)
                            } else {
                                Node::Pi(*info, name, t, b)
                            }
                        }
                        ERec::Let(n, ..) => {
                            let (Some(b), Some(v), Some(t)) = (vals.pop(), vals.pop(), vals.pop())
                            else {
                                return Err(bad);
                            };
                            Node::Let(self.name(*n, line)?, t, v, b)
                        }
                        _ => return Err(bad),
                    };
                    nodes.push(node);
                    vals.push(nodes.len() as u32 - 1);
                }
            }
        }
        let root = vals.pop().ok_or(bad)?;
        Ok(Dterm {
            nodes,
            levels,
            root,
        })
    }
}

/// One line's fields, with strict numerals.
struct Fields<'a> {
    it: std::str::Split<'a, char>,
    line: usize,
}

impl<'a> Fields<'a> {
    fn tok(&mut self) -> Result<&'a str, ModelError> {
        self.it
            .next()
            .ok_or(ModelError::Decode("missing field", self.line))
    }
    /// No fields may remain.
    fn done(&mut self) -> Result<(), ModelError> {
        if self.it.next().is_none() {
            Ok(())
        } else {
            Err(ModelError::Decode("extra field", self.line))
        }
    }
    fn u32(&mut self) -> Result<u32, ModelError> {
        let t = self.tok()?;
        let ok = !t.is_empty()
            && t.bytes().all(|b| b.is_ascii_digit())
            && (t == "0" || !t.starts_with('0'));
        if !ok {
            return Err(ModelError::Decode("bad numeral", self.line));
        }
        t.parse()
            .map_err(|_| ModelError::Decode("number out of range", self.line))
    }
    /// A string token with PIF's escapes.
    fn string(&mut self) -> Result<String, ModelError> {
        let t = self.tok()?;
        let line = self.line;
        let inner = t
            .strip_prefix('"')
            .and_then(|t| t.strip_suffix('"'))
            .ok_or(ModelError::Decode("bad string", line))?;
        let (mut out, mut it) = (String::new(), inner.chars().peekable());
        while let Some(c) = it.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match it.next() {
                Some(q @ ('"' | '\\')) => out.push(q),
                Some('u') if it.next() == Some('{') => {
                    let mut hex = String::new();
                    loop {
                        match it.next() {
                            Some('}') => break,
                            Some(h) if h.is_ascii_hexdigit() && hex.len() < 6 => hex.push(h),
                            _ => return Err(ModelError::Decode("bad escape", line)),
                        }
                    }
                    let v = u32::from_str_radix(&hex, 16)
                        .map_err(|_| ModelError::Decode("bad escape", line))?;
                    out.push(char::from_u32(v).ok_or(ModelError::Decode("bad escape", line))?);
                }
                _ => return Err(ModelError::Decode("bad escape", line)),
            }
        }
        Ok(out)
    }
    fn digest(&mut self) -> Result<String, ModelError> {
        let t = self
            .tok()?
            .strip_prefix("sha256:")
            .ok_or(ModelError::Decode("bad digest", self.line))?;
        if t.len() == 64
            && t.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(t.to_string())
        } else {
            Err(ModelError::Decode("bad digest", self.line))
        }
    }
    /// `[]`, `[1]`, `[1 2]`.
    fn list(&mut self) -> Result<Vec<u32>, ModelError> {
        let first = self.tok()?;
        let bad = ModelError::Decode("bad list", self.line);
        let mut parts = vec![first];
        while !parts.last().is_some_and(|p| p.ends_with(']')) {
            parts.push(self.it.next().ok_or_else(|| bad.clone())?);
        }
        let joined = parts.join(" ");
        let inner = joined
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| bad.clone())?;
        if inner.is_empty() {
            return Ok(vec![]);
        }
        inner
            .split(' ')
            .map(|x| x.parse::<u32>().map_err(|_| bad.clone()))
            .collect()
    }
    fn binfo(&mut self) -> Result<BinderInfo, ModelError> {
        Ok(match self.tok()? {
            "default" => BinderInfo::Default,
            "implicit" => BinderInfo::Implicit,
            "strict" => BinderInfo::StrictImplicit,
            "inst" => BinderInfo::InstImplicit,
            _ => return Err(ModelError::Decode("bad binder info", self.line)),
        })
    }
}

/// Decode the text of a PIF file. Lenient where the trusted reader is strict (the reader
/// has already accepted the file, or the caller only wants to look at it), strict where a
/// wrong reading would show a reviewer the wrong thing: dense indices, known keywords.
pub fn decode(text: &str) -> Result<FileModel, ModelError> {
    let mut t = Tables {
        names: vec![Name::anonymous()],
        levels: vec![LRec::Zero],
        exprs: vec![],
    };
    let mut m = FileModel {
        profile: Default::default(),
        features: vec![],
        imports: vec![],
        decls: vec![],
    };
    let body = text
        .strip_suffix('\n')
        .ok_or(ModelError::Decode("no final LF", 1))?;
    let all: Vec<(usize, &str)> = body
        .split('\n')
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .collect();
    let mut at = 0usize;
    // The fixed header lines, in order.
    let hdr = |kw: &str, at: &mut usize| -> Result<(usize, &str), ModelError> {
        let &(n, l) = all
            .get(*at)
            .ok_or(ModelError::Decode("truncated header", 1))?;
        *at += 1;
        Ok((
            n,
            l.strip_prefix(kw)
                .ok_or(ModelError::Decode("bad header line", n))?,
        ))
    };
    let (n, v) = hdr("pif", &mut at)?;
    if v != " 1" {
        return Err(ModelError::Decode("unknown pif version", n));
    }
    let (n, v) = hdr("logic", &mut at)?;
    if v != " 1" {
        return Err(ModelError::Decode("unknown logic version", n));
    }
    let (n, v) = hdr("profile", &mut at)?;
    let mut f = Fields {
        it: v
            .strip_prefix(' ')
            .ok_or(ModelError::Decode("bad profile", n))?
            .split(' '),
        line: n,
    };
    m.profile = (f.tok()?.to_string(), f.string()?);
    let (n, v) = hdr("features", &mut at)?;
    if !v.is_empty() {
        let list = v
            .strip_prefix(' ')
            .ok_or(ModelError::Decode("bad features", n))?;
        m.features = list.split(' ').map(str::to_string).collect();
    }
    while let Some(&(n, l)) = all.get(at) {
        let Some(rest) = l.strip_prefix("import ") else {
            break;
        };
        at += 1;
        m.imports.push(
            Fields {
                it: rest.split(' '),
                line: n,
            }
            .digest()?,
        );
    }
    let first = all.get(at).map_or(0, |l| l.0);
    let mut lines = all[at..].iter().copied();
    // records
    while let Some((n, l)) = lines.next() {
        let (kw, rest) = l.split_once(' ').unwrap_or((l, ""));
        let mut f = Fields {
            it: rest.split(' '),
            line: n,
        };
        match kw {
            "end" => {
                // `end k`: k is the number of record lines between the header and here.
                let k = f.u32()?;
                f.done()?;
                if k as usize != n - first {
                    return Err(ModelError::Decode("bad trailer count", n));
                }
                return if lines.next().is_none() {
                    Ok(m)
                } else {
                    Err(ModelError::Decode("bytes after end", n))
                };
            }
            "nstr" | "nnum" => {
                let (id, p) = (f.u32()?, f.u32()?);
                if id as usize != t.names.len() {
                    return Err(ModelError::Decode("name index not dense", n));
                }
                let prefix = t.name(p, n)?;
                let name = if kw == "nstr" {
                    Name::str(&prefix, &f.string()?)
                } else {
                    Name::num(
                        &prefix,
                        f.tok()?
                            .parse::<u64>()
                            .map_err(|_| ModelError::Decode("bad numeral", n))?,
                    )
                };
                f.done()?;
                t.names.push(name);
            }
            "lsucc" | "lmax" | "limax" | "lparam" => {
                let id = f.u32()?;
                if id as usize != t.levels.len() {
                    return Err(ModelError::Decode("level index not dense", n));
                }
                let r = match kw {
                    "lsucc" => LRec::Succ(f.u32()?),
                    "lparam" => LRec::Param(f.u32()?),
                    "lmax" => LRec::Max(f.u32()?, f.u32()?),
                    _ => LRec::IMax(f.u32()?, f.u32()?),
                };
                f.done()?;
                t.levels.push(r);
            }
            "evar" | "esort" | "econst" | "eapp" | "elam" | "epi" | "elet" => {
                let id = f.u32()?;
                if id as usize != t.exprs.len() + 1 {
                    return Err(ModelError::Decode("expression index not dense", n));
                }
                let r = match kw {
                    "evar" => ERec::Var(f.u32()?),
                    "esort" => ERec::Sort(f.u32()?),
                    "econst" => ERec::Const(f.u32()?, f.list()?),
                    "eapp" => ERec::App(f.u32()?, f.u32()?),
                    "elet" => ERec::Let(f.u32()?, f.u32()?, f.u32()?, f.u32()?),
                    _ => {
                        let info = f.binfo()?;
                        let (name, ty, body) = (f.u32()?, f.u32()?, f.u32()?);
                        if kw == "elam" {
                            ERec::Lam(info, name, ty, body)
                        } else {
                            ERec::Pi(info, name, ty, body)
                        }
                    }
                };
                f.done()?;
                t.exprs.push(r);
            }
            "axiom" | "def" | "theorem" | "opaque" => {
                let name = t.name(f.u32()?, n)?;
                let params = f
                    .list()?
                    .into_iter()
                    .map(|i| t.name(i, n))
                    .collect::<Result<Vec<_>, _>>()?;
                let ty = t.unfold(f.u32()?, n)?;
                m.decls.push(match kw {
                    "axiom" => DeclModel::Axiom { name, params, ty },
                    "theorem" => DeclModel::Theorem {
                        name,
                        params,
                        ty,
                        value: t.unfold(f.u32()?, n)?,
                    },
                    "opaque" => DeclModel::Opaque {
                        name,
                        params,
                        ty,
                        value: t.unfold(f.u32()?, n)?,
                    },
                    _ => {
                        let value = t.unfold(f.u32()?, n)?;
                        let hint = match f.tok()? {
                            "@abbrev" => HintModel::Abbrev,
                            "@opaque" => HintModel::Opaque,
                            "@regular" => HintModel::Regular(f.u32()?),
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
                f.done()?;
            }
            "inductive" => {
                let params = f
                    .list()?
                    .into_iter()
                    .map(|i| t.name(i, n))
                    .collect::<Result<Vec<_>, _>>()?;
                let (num_params, nt) = (f.u32()?, f.u32()?);
                f.done()?;
                let mut types = Vec::new();
                for _ in 0..nt {
                    let (n2, l2) = lines
                        .next()
                        .ok_or(ModelError::Decode("truncated block", n))?;
                    let rest = l2
                        .strip_prefix("itype ")
                        .ok_or(ModelError::Decode("expected itype", n2))?;
                    let mut f2 = Fields {
                        it: rest.split(' '),
                        line: n2,
                    };
                    let (name, ty, nc) =
                        (t.name(f2.u32()?, n2)?, t.unfold(f2.u32()?, n2)?, f2.u32()?);
                    f2.done()?;
                    let mut ctors = Vec::new();
                    for _ in 0..nc {
                        let (n3, l3) = lines
                            .next()
                            .ok_or(ModelError::Decode("truncated block", n2))?;
                        let rest = l3
                            .strip_prefix("ictor ")
                            .ok_or(ModelError::Decode("expected ictor", n3))?;
                        let mut f3 = Fields {
                            it: rest.split(' '),
                            line: n3,
                        };
                        ctors.push((t.name(f3.u32()?, n3)?, t.unfold(f3.u32()?, n3)?));
                        f3.done()?;
                    }
                    types.push(IndModel { name, ty, ctors });
                }
                m.decls.push(DeclModel::Inductive {
                    params,
                    num_params,
                    types,
                });
            }
            "goal" => {
                let claim = f.string()?;
                let theorem = t.name(f.u32()?, n)?;
                let digest = f.digest()?;
                f.done()?;
                m.decls.push(DeclModel::Goal {
                    claim,
                    theorem,
                    digest,
                });
            }
            "enat" | "estr" | "eproj" => {
                return Err(ModelError::Unsupported("literal or projection"))
            }
            "quot" | "prim" => return Err(ModelError::Unsupported("quot or prim")),
            _ => return Err(ModelError::Decode("unknown record", n)),
        }
    }
    Err(ModelError::Decode("no end line", 1))
}
