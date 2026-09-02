//! The PDF object model.
//!
//! A PDF file is, underneath the page-description language, a small graph of
//! eight object types. Everything else — pages, fonts, images, annotations — is
//! built out of dictionaries and streams whose keys happen to mean something to
//! a reader. Getting these eight right is therefore the whole foundation, and
//! they are simple enough to state completely:
//!
//! ```text
//!   null            null
//!   boolean         true | false
//!   numeric         42        3.14
//!   string          (literal)  <68657820>
//!   name            /Type
//!   array           [1 2 /Three]
//!   dictionary      << /Key /Value >>
//!   stream          << /Length 5 >> stream\n....\nendstream
//! ```
//!
//! plus the **indirect reference** — `3 0 R` — which is what turns the list
//! into a graph.
//!
//! ## Why strings are the fiddly one
//!
//! A PDF literal string is delimited by parentheses, and parentheses *nest*. So
//! `(a(b)c)` is a single valid string, while `(a(b)` is malformed. Rather than
//! track nesting when writing, this module escapes `(`, `)` and `\` always,
//! which is always correct and never ambiguous. Binary data goes out as a hex
//! string instead, because escaping arbitrary bytes into a literal is both
//! larger and easier to get wrong.

use std::fmt::Write as _;

/// One PDF object.
///
/// `Stream` carries already-encoded bytes plus the dictionary describing them.
/// The writer fills in `/Length` from the byte slice rather than trusting a
/// caller-supplied value — a `/Length` that disagrees with the actual stream is
/// one of the few ways to produce a file that some readers accept and others
/// reject, which is the worst kind of wrong.
#[derive(Clone, Debug, PartialEq)]
pub enum Object {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    /// A literal string, written `(…)` with `(`, `)` and `\` escaped.
    Str(Vec<u8>),
    /// A hex string, written `<…>`. Preferred for binary.
    HexStr(Vec<u8>),
    Name(String),
    Array(Vec<Object>),
    Dict(Dict),
    Stream { dict: Dict, data: Vec<u8> },
    /// `n g R`. The generation is almost always 0 for a freshly written file.
    Ref(ObjId),
}

/// An indirect object's identity: number and generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjId {
    pub number: u32,
    pub generation: u16,
}

impl ObjId {
    pub fn new(number: u32) -> Self {
        Self {
            number,
            generation: 0,
        }
    }
}

/// A PDF dictionary.
///
/// Insertion-ordered rather than sorted. PDF itself does not care about key
/// order, but a stable order makes output diffable, which matters a great deal
/// when the thing you are debugging is a byte offset.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dict {
    entries: Vec<(String, Object)>,
}

impl Dict {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `key`, replacing any existing value while keeping its position.
    pub fn set(&mut self, key: impl Into<String>, value: Object) -> &mut Self {
        let key = key.into();
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.entries.push((key, value)),
        }
        self
    }

    pub fn get(&self, key: &str) -> Option<&Object> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Object)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Build a dictionary from pairs, for readability at call sites.
#[macro_export]
macro_rules! dict {
    ($($key:expr => $value:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut d = $crate::Dict::new();
        $( d.set($key, $value); )*
        d
    }};
}

impl Object {
    /// A convenience for the very common `Object::Name("Foo".into())`.
    pub fn name(value: impl Into<String>) -> Self {
        Object::Name(value.into())
    }

    /// Serialise this object into `out`.
    ///
    /// Streams are **not** written here — they are only legal as indirect
    /// objects, so the writer handles them. Encountering one in a nested
    /// position writes its dictionary alone, which is the closest thing to
    /// correct available without silently dropping information.
    pub fn write(&self, out: &mut Vec<u8>) {
        match self {
            Object::Null => out.extend_from_slice(b"null"),
            Object::Bool(true) => out.extend_from_slice(b"true"),
            Object::Bool(false) => out.extend_from_slice(b"false"),
            Object::Int(value) => {
                let mut buf = String::new();
                let _ = write!(buf, "{value}");
                out.extend_from_slice(buf.as_bytes());
            }
            Object::Real(value) => out.extend_from_slice(format_real(*value).as_bytes()),
            Object::Str(bytes) => write_literal_string(bytes, out),
            Object::HexStr(bytes) => write_hex_string(bytes, out),
            Object::Name(name) => write_name(name, out),
            Object::Array(items) => {
                out.push(b'[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(b' ');
                    }
                    item.write(out);
                }
                out.push(b']');
            }
            Object::Dict(dict) => write_dict(dict, out),
            Object::Stream { dict, .. } => write_dict(dict, out),
            Object::Ref(id) => {
                let mut buf = String::new();
                let _ = write!(buf, "{} {} R", id.number, id.generation);
                out.extend_from_slice(buf.as_bytes());
            }
        }
    }
}

fn write_dict(dict: &Dict, out: &mut Vec<u8>) {
    out.extend_from_slice(b"<<");
    for (key, value) in dict.iter() {
        out.push(b' ');
        write_name(key, out);
        out.push(b' ');
        value.write(out);
    }
    out.extend_from_slice(b" >>");
}

/// Write a PDF name, escaping anything that is not a regular character.
///
/// `#` introduces a two-digit hex escape. The delimiters `()<>[]{}/%`, the
/// whitespace characters, and `#` itself all need it — a raw `/` inside a name
/// would start a *second* name, so this is not cosmetic.
fn write_name(name: &str, out: &mut Vec<u8>) {
    out.push(b'/');
    for &byte in name.as_bytes() {
        let regular = byte > 0x20
            && byte < 0x7f
            && !matches!(
                byte,
                b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' | b'#'
            );
        if regular {
            out.push(byte);
        } else {
            out.push(b'#');
            out.extend_from_slice(format!("{byte:02X}").as_bytes());
        }
    }
}

/// Write a literal string with `(`, `)` and `\` escaped.
///
/// Escaping the parentheses unconditionally, rather than tracking whether they
/// happen to balance, is deliberate: balanced parentheses are legal unescaped,
/// but checking that is work whose only payoff is a slightly shorter file, and
/// getting it wrong produces a string that silently swallows the rest of the
/// object.
fn write_literal_string(bytes: &[u8], out: &mut Vec<u8>) {
    out.push(b'(');
    for &byte in bytes {
        match byte {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(byte);
            }
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            _ => out.push(byte),
        }
    }
    out.push(b')');
}

fn write_hex_string(bytes: &[u8], out: &mut Vec<u8>) {
    out.push(b'<');
    for &byte in bytes {
        out.extend_from_slice(format!("{byte:02X}").as_bytes());
    }
    out.push(b'>');
}

/// Format a real number the way PDF wants it.
///
/// PDF reals have no exponent notation — `1e-5` is not a number, it is a syntax
/// error — so Rust's default float formatting cannot be used directly for small
/// or large magnitudes. Trailing zeros are trimmed because coordinate-heavy
/// content streams are mostly numbers, and the saving is real.
pub fn format_real(value: f64) -> String {
    if !value.is_finite() {
        // Neither infinity nor NaN is expressible. Zero is the least surprising
        // substitute for a coordinate, and silently emitting `inf` would
        // produce a file no reader accepts.
        return "0".to_string();
    }
    if value == value.trunc() && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let mut text = format!("{value:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}
