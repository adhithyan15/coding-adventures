//! The parser of the printed syntax: the independent half of `parse(print(x)) = x`.
//!
//! # Why a second parser, and why it is hand-written
//!
//! The gate does not trust [`crate::printer`] to be injective; it *checks* that parsing the
//! printed text gives the original back (VER32 3.6, N5). A check is only worth something if
//! the parser **shares no code with the printer**. This module reads the printed syntax
//! (documented in [`crate::printer`]) and shares only the [`Dterm`] data type with it.
//!
//! **Why not a grammar-driven parser from `grammar-tools`?** The repository's rule is that
//! lexers and parsers are generated from `.tokens`/`.grammar` files. For a *trusted* gate
//! that is the same conflict VER32 Q1 discusses for the reader: a generated parser puts a
//! lexer engine, a parser engine and a grammar compiler into the trusted base, and the part
//! that turns a parse tree into a [`Dterm`] (de Bruijn indices, binder info, level
//! arithmetic) would still be hand-written code of this size. The syntax here is small and
//! unambiguous, the parser is a stack machine of about 300 lines, and the differential is the
//! round trip itself. The grammar in [`crate::printer`] is written so that a `.grammar` file
//! can be derived from it if the repository later audits `grammar-tools` to this level.
//!
//! # No recursion on input-controlled depth
//!
//! Terms nest as deeply as an attacker likes. Tokenising is a loop, and parsing is a loop
//! over an explicit stack of tasks, so no input can overflow the call stack. Sizes are capped
//! (tokens, nodes) before anything is allocated.

use crate::cte::{Dterm, Lv, Node};
use lean_kernel::{BinderInfo, Name};

/// The cap on nodes plus level nodes in one parsed term (the CTE's own cap).
const MAX_NODES: usize = 1 << 20;

/// Why parsing failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PpError {
    /// The text is not in the printed syntax (why, token position).
    Syntax(&'static str, usize),
    /// The text or the tree is over a size cap.
    TooLarge,
}

/// A token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    Hash(u32),
    Name(String),
    Nat(u64),
    Str(String),
    Digest(String),
    At(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LStrict,
    RStrict,
    LBrack,
    RBrack,
    Comma,
    Colon,
    Assign,
    Semi,
    Arrow,
    FatArrow,
    Plus,
    DotBrace,
}

fn start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}
fn cont(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'\''
}

/// Read a numeral `0 | [1-9][0-9]*` at `i`; returns the value and the next index.
fn numeral(s: &[u8], i: usize) -> Result<(u64, usize), PpError> {
    let mut j = i;
    while s.get(j).is_some_and(u8::is_ascii_digit) {
        j += 1;
    }
    let d = &s[i..j];
    if d.is_empty() || (d[0] == b'0' && d.len() > 1) {
        return Err(PpError::Syntax("bad numeral", i));
    }
    let mut v: u64 = 0;
    for &b in d {
        v = v
            .checked_mul(10)
            .and_then(|v| v.checked_add(u64::from(b - b'0')))
            .ok_or(PpError::Syntax("number too large", i))?;
    }
    Ok((v, j))
}

/// Read a string token starting at the opening quote; returns the value and the next index.
fn string(s: &[u8], i: usize) -> Result<(String, usize), PpError> {
    let bad = |why| PpError::Syntax(why, i);
    let (mut out, mut j) = (String::new(), i + 1);
    loop {
        match *s.get(j).ok_or_else(|| bad("unterminated string"))? {
            b'"' => return Ok((out, j + 1)),
            b'\\' => match s.get(j + 1) {
                Some(&c @ (b'"' | b'\\')) => {
                    out.push(char::from(c));
                    j += 2;
                }
                Some(b'u') if s.get(j + 2) == Some(&b'{') => {
                    let mut k = j + 3;
                    while s
                        .get(k)
                        .is_some_and(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
                    {
                        k += 1;
                    }
                    let h = &s[j + 3..k];
                    if h.is_empty()
                        || h.len() > 6
                        || (h[0] == b'0' && h.len() > 1)
                        || s.get(k) != Some(&b'}')
                    {
                        return Err(bad("bad escape"));
                    }
                    let v = h.iter().fold(0u32, |a, &b| {
                        a * 16 + char::from(b).to_digit(16).unwrap_or(0)
                    });
                    out.push(
                        char::from_u32(v)
                            .filter(|c| !('!'..='~').contains(c))
                            .ok_or_else(|| bad("bad escape"))?,
                    );
                    j = k + 1;
                }
                _ => return Err(bad("bad escape")),
            },
            b @ b'!'..=b'~' => {
                out.push(char::from(b));
                j += 1;
            }
            _ => return Err(bad("bad string byte")),
        }
        if out.len() > MAX_NODES {
            return Err(PpError::TooLarge);
        }
    }
}

/// Split one line of printed text into tokens. Spaces separate tokens; any other whitespace
/// or non-ASCII byte is an error.
pub fn tokenize(text: &str) -> Result<Vec<Tok>, PpError> {
    let s = text.as_bytes();
    if s.len() > crate::printer::MAX_PRINT_BYTES {
        return Err(PpError::TooLarge);
    }
    let (mut toks, mut i) = (Vec::new(), 0usize);
    while i < s.len() {
        let c = s[i];
        let next = s.get(i + 1).copied();
        let (tok, len) = match c {
            b' ' => {
                i += 1;
                continue;
            }
            b'#' => {
                let (v, j) = numeral(s, i + 1)?;
                (
                    Tok::Hash(u32::try_from(v).map_err(|_| PpError::Syntax("index too large", i))?),
                    j - i,
                )
            }
            b'0'..=b'9' => {
                let (v, j) = numeral(s, i)?;
                (Tok::Nat(v), j - i)
            }
            b'"' => {
                let (v, j) = string(s, i)?;
                (Tok::Str(v), j - i)
            }
            b'@' => {
                let mut j = i + 1;
                while s.get(j).is_some_and(u8::is_ascii_lowercase) {
                    j += 1;
                }
                (Tok::At(text[i + 1..j].to_string()), j - i)
            }
            _ if start(c) => {
                if let Some(rest) = text[i..].strip_prefix("sha256:") {
                    let hex = rest
                        .as_bytes()
                        .iter()
                        .take_while(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
                        .count();
                    if hex == 64 {
                        toks.push(Tok::Digest(rest[..64].to_string()));
                        i += 7 + 64;
                        continue;
                    }
                }
                // name = comp { '.' comp }, a '.' being part of the name only before a comp start
                let mut j = i;
                loop {
                    while s.get(j).copied().is_some_and(cont) {
                        j += 1;
                    }
                    if s.get(j) == Some(&b'.') && s.get(j + 1).copied().is_some_and(start) {
                        j += 1;
                    } else {
                        break;
                    }
                }
                (Tok::Name(text[i..j].to_string()), j - i)
            }
            b'(' => (Tok::LParen, 1),
            b')' => (Tok::RParen, 1),
            b'{' if next == Some(b'|') => (Tok::LStrict, 2),
            b'|' if next == Some(b'}') => (Tok::RStrict, 2),
            b'{' => (Tok::LBrace, 1),
            b'}' => (Tok::RBrace, 1),
            b'[' => (Tok::LBrack, 1),
            b']' => (Tok::RBrack, 1),
            b',' => (Tok::Comma, 1),
            b';' => (Tok::Semi, 1),
            b'+' => (Tok::Plus, 1),
            b'-' if next == Some(b'>') => (Tok::Arrow, 2),
            b'=' if next == Some(b'>') => (Tok::FatArrow, 2),
            b':' if next == Some(b'=') => (Tok::Assign, 2),
            b':' => (Tok::Colon, 1),
            b'.' if next == Some(b'{') => (Tok::DotBrace, 2),
            _ => return Err(PpError::Syntax("unexpected character", i)),
        };
        toks.push(tok);
        i += len;
        if toks.len() > MAX_NODES {
            return Err(PpError::TooLarge);
        }
    }
    Ok(toks)
}

/// The keywords of the term syntax: never a constant, binder or parameter name.
fn keyword(s: &str) -> bool {
    matches!(s, "forall" | "fun" | "let" | "Sort")
}

/// A dotted name token as a kernel name (each component is a string component).
fn name_of(s: &str) -> Name {
    Name::from_dotted(s)
}

enum Task {
    Term,
    Atom,
    AppLoop,
    ArrowTail,
    Expect(Tok),
    BuildApp,
    BuildArrow,
    BuildBinder(bool, BinderInfo, Name),
    BuildLet(Name),
}

/// A cursor over the tokens of one line, with the ability to parse terms from it.
pub struct Parser {
    pub toks: Vec<Tok>,
    pub pos: usize,
}

impl Parser {
    pub fn new(toks: Vec<Tok>) -> Parser {
        Parser { toks, pos: 0 }
    }
    pub fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    pub fn bump(&mut self) -> Result<Tok, PpError> {
        let t = self
            .toks
            .get(self.pos)
            .cloned()
            .ok_or(PpError::Syntax("unexpected end", self.pos))?;
        self.pos += 1;
        Ok(t)
    }
    pub fn err<T>(&self, why: &'static str) -> Result<T, PpError> {
        Err(PpError::Syntax(why, self.pos))
    }
    pub fn expect(&mut self, t: &Tok) -> Result<(), PpError> {
        if &self.bump()? == t {
            Ok(())
        } else {
            self.pos -= 1;
            self.err("unexpected token")
        }
    }
    pub fn at_end(&self) -> bool {
        self.pos == self.toks.len()
    }
    /// A (possibly multi-component) name that is not a keyword.
    pub fn name(&mut self) -> Result<Name, PpError> {
        match self.bump()? {
            Tok::Name(s) if !keyword(&s) => Ok(name_of(&s)),
            _ => {
                self.pos -= 1;
                self.err("expected a name")
            }
        }
    }
    /// A string token.
    pub fn string(&mut self) -> Result<String, PpError> {
        match self.bump()? {
            Tok::Str(s) => Ok(s),
            _ => {
                self.pos -= 1;
                self.err("expected a string")
            }
        }
    }
    /// `.{name, name}` after a declared name, if present.
    pub fn params(&mut self) -> Result<Vec<Name>, PpError> {
        let mut out = Vec::new();
        if self.peek() == Some(&Tok::DotBrace) {
            self.pos += 1;
            loop {
                out.push(self.name()?);
                match self.bump()? {
                    Tok::Comma => {}
                    Tok::RBrace => return Ok(out),
                    _ => {
                        self.pos -= 1;
                        return self.err("expected , or }");
                    }
                }
            }
        }
        Ok(out)
    }

    fn atom_starts(&self) -> bool {
        match self.peek() {
            Some(Tok::Hash(_) | Tok::LParen) => true,
            Some(Tok::Name(s)) => !matches!(s.as_str(), "forall" | "fun" | "let"),
            _ => false,
        }
    }

    /// Parse a level: `base { '+' NAT }`, with `max(l, l)` and `imax(l, l)` as bases.
    fn level(&mut self, levels: &mut Vec<Lv>, nodes: usize) -> Result<u32, PpError> {
        enum L {
            Level,
            Suffix,
            Expect(Tok),
            Max(bool),
        }
        let push = |levels: &mut Vec<Lv>, v: Lv| -> Result<u32, PpError> {
            if nodes + levels.len() >= MAX_NODES {
                return Err(PpError::TooLarge);
            }
            levels.push(v);
            Ok(levels.len() as u32 - 1)
        };
        let (mut tasks, mut vals) = (vec![L::Level], Vec::<u32>::new());
        while let Some(t) = tasks.pop() {
            match t {
                L::Expect(tok) => self.expect(&tok)?,
                L::Level => match self.bump()? {
                    Tok::Nat(n) => {
                        let mut id = push(levels, Lv::Zero)?;
                        for _ in 0..n {
                            id = push(levels, Lv::Succ(id))?;
                        }
                        vals.push(id);
                        tasks.push(L::Suffix);
                    }
                    Tok::Name(s)
                        if (s == "max" || s == "imax") && self.peek() == Some(&Tok::LParen) =>
                    {
                        self.pos += 1;
                        tasks.extend([
                            L::Suffix,
                            L::Max(s == "imax"),
                            L::Expect(Tok::RParen),
                            L::Level,
                            L::Expect(Tok::Comma),
                            L::Level,
                        ]);
                    }
                    Tok::Name(s) if !keyword(&s) => {
                        vals.push(push(levels, Lv::Param(name_of(&s)))?);
                        tasks.push(L::Suffix);
                    }
                    _ => {
                        self.pos -= 1;
                        return self.err("expected a level");
                    }
                },
                L::Max(imax) => {
                    let (Some(b), Some(a)) = (vals.pop(), vals.pop()) else {
                        return self.err("level stack");
                    };
                    vals.push(push(
                        levels,
                        if imax { Lv::IMax(a, b) } else { Lv::Max(a, b) },
                    )?);
                }
                L::Suffix => {
                    while self.peek() == Some(&Tok::Plus) {
                        self.pos += 1;
                        let Tok::Nat(k) = self.bump()? else {
                            self.pos -= 1;
                            return self.err("expected a number after +");
                        };
                        let mut id = vals.pop().ok_or(PpError::Syntax("level stack", self.pos))?;
                        for _ in 0..k {
                            id = push(levels, Lv::Succ(id))?;
                        }
                        vals.push(id);
                    }
                }
            }
        }
        vals.pop().ok_or(PpError::Syntax("level stack", self.pos))
    }

    /// Parse one term starting at the cursor, stopping where the term grammar cannot
    /// continue (the caller checks what follows).
    pub fn term(&mut self) -> Result<Dterm, PpError> {
        let (mut nodes, mut levels): (Vec<Node>, Vec<Lv>) = (vec![], vec![]);
        let mut vals: Vec<u32> = Vec::new();
        let mut tasks = vec![Task::Term];
        macro_rules! add {
            ($n:expr) => {{
                if nodes.len() + levels.len() >= MAX_NODES {
                    return Err(PpError::TooLarge);
                }
                nodes.push($n);
                nodes.len() as u32 - 1
            }};
        }
        while let Some(t) = tasks.pop() {
            match t {
                Task::Expect(tok) => self.expect(&tok)?,
                Task::Term => match self.peek().cloned() {
                    Some(Tok::Name(k)) if k == "forall" || k == "fun" => {
                        self.pos += 1;
                        let (info, close) = match self.bump()? {
                            Tok::LParen => (BinderInfo::Default, Tok::RParen),
                            Tok::LBrace => (BinderInfo::Implicit, Tok::RBrace),
                            Tok::LStrict => (BinderInfo::StrictImplicit, Tok::RStrict),
                            Tok::LBrack => (BinderInfo::InstImplicit, Tok::RBrack),
                            _ => {
                                self.pos -= 1;
                                return self.err("expected a binder");
                            }
                        };
                        let name = self.name()?;
                        self.expect(&Tok::Colon)?;
                        let sep = if k == "fun" {
                            Tok::FatArrow
                        } else {
                            Tok::Comma
                        };
                        tasks.extend([
                            Task::BuildBinder(k == "fun", info, name),
                            Task::Term,
                            Task::Expect(sep),
                            Task::Expect(close),
                            Task::Term,
                        ]);
                    }
                    Some(Tok::Name(k)) if k == "let" => {
                        self.pos += 1;
                        let name = self.name()?;
                        self.expect(&Tok::Colon)?;
                        tasks.extend([
                            Task::BuildLet(name),
                            Task::Term,
                            Task::Expect(Tok::Semi),
                            Task::Term,
                            Task::Expect(Tok::Assign),
                            Task::Term,
                        ]);
                    }
                    _ => tasks.extend([Task::ArrowTail, Task::AppLoop, Task::Atom]),
                },
                Task::Atom => match self.bump()? {
                    Tok::Hash(k) => {
                        if k >= lean_kernel::MAX_BVAR_INDEX {
                            return Err(PpError::Syntax("index too large", self.pos));
                        }
                        vals.push(add!(Node::Var(k)));
                    }
                    Tok::LParen => tasks.extend([Task::Expect(Tok::RParen), Task::Term]),
                    Tok::Name(s) if s == "Sort" => {
                        let l = self.level(&mut levels, nodes.len())?;
                        vals.push(add!(Node::Sort(l)));
                    }
                    Tok::Name(s) if !keyword(&s) => {
                        let mut ls = Vec::new();
                        if self.peek() == Some(&Tok::DotBrace) {
                            self.pos += 1;
                            loop {
                                ls.push(self.level(&mut levels, nodes.len())?);
                                match self.bump()? {
                                    Tok::Comma => {}
                                    Tok::RBrace => break,
                                    _ => {
                                        self.pos -= 1;
                                        return self.err("expected , or }");
                                    }
                                }
                            }
                        }
                        vals.push(add!(Node::Const(name_of(&s), ls)));
                    }
                    _ => {
                        self.pos -= 1;
                        return self.err("expected a term");
                    }
                },
                Task::AppLoop => {
                    if self.atom_starts() {
                        tasks.extend([Task::AppLoop, Task::BuildApp, Task::Atom]);
                    }
                }
                Task::ArrowTail => {
                    if self.peek() == Some(&Tok::Arrow) {
                        self.pos += 1;
                        tasks.extend([Task::BuildArrow, Task::Term]);
                    }
                }
                Task::BuildApp => {
                    let (Some(a), Some(f)) = (vals.pop(), vals.pop()) else {
                        return self.err("term stack");
                    };
                    vals.push(add!(Node::App(f, a)));
                }
                Task::BuildArrow => {
                    let (Some(b), Some(ty)) = (vals.pop(), vals.pop()) else {
                        return self.err("term stack");
                    };
                    vals.push(add!(Node::Pi(BinderInfo::Default, name_of("_"), ty, b)));
                }
                Task::BuildBinder(lam, info, name) => {
                    let (Some(b), Some(ty)) = (vals.pop(), vals.pop()) else {
                        return self.err("term stack");
                    };
                    vals.push(add!(if lam {
                        Node::Lam(info, name, ty, b)
                    } else {
                        Node::Pi(info, name, ty, b)
                    }));
                }
                Task::BuildLet(name) => {
                    let (Some(b), Some(v), Some(ty)) = (vals.pop(), vals.pop(), vals.pop()) else {
                        return self.err("term stack");
                    };
                    vals.push(add!(Node::Let(name, ty, v, b)));
                }
            }
        }
        match vals.as_slice() {
            [root] => Ok(Dterm {
                nodes,
                levels,
                root: *root,
            }),
            _ => self.err("term stack"),
        }
    }
}

/// Parse a whole text as one term (nothing may follow it).
pub fn parse_term(text: &str) -> Result<Dterm, PpError> {
    let mut p = Parser::new(tokenize(text)?);
    let t = p.term()?;
    if !p.at_end() {
        return p.err("trailing tokens");
    }
    Ok(t)
}
