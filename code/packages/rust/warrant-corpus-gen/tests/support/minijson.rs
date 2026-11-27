//! A tiny strict JSON reader for the tests: objects (key order kept), arrays, strings,
//! non-negative integers, booleans and `null` (only the v1 corpus has one; a test asserts the
//! v2 text contains none: VER04r 1.4, conditional keys are omitted, never null).

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum J {
    B(bool),
    Null,
    N(u64),
    S(String),
    A(Vec<J>),
    O(Vec<(String, J)>),
}

impl J {
    pub fn get(&self, k: &str) -> &J {
        match self {
            J::O(p) => &p.iter().find(|(n, _)| n == k).unwrap_or_else(|| panic!("no key {k}")).1,
            _ => panic!("not an object"),
        }
    }
    pub fn get_opt(&self, k: &str) -> Option<&J> {
        match self {
            J::O(p) => p.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn has(&self, k: &str) -> bool {
        matches!(self, J::O(p) if p.iter().any(|(n, _)| n == k))
    }
    pub fn keys(&self) -> Vec<&str> {
        match self {
            J::O(p) => p.iter().map(|(n, _)| n.as_str()).collect(),
            _ => panic!("not an object"),
        }
    }
    pub fn s(&self) -> &str {
        match self {
            J::S(s) => s,
            _ => panic!("not a string: {self:?}"),
        }
    }
    pub fn n(&self) -> u64 {
        match self {
            J::N(n) => *n,
            _ => panic!("not a number"),
        }
    }
    pub fn a(&self) -> &[J] {
        match self {
            J::A(v) => v,
            _ => panic!("not an array"),
        }
    }
    pub fn strs(&self) -> Vec<&str> {
        self.a().iter().map(J::s).collect()
    }
}

pub fn parse(text: &str) -> Result<J, String> {
    let b = text.as_bytes();
    let mut i = 0;
    let v = value(b, &mut i)?;
    ws(b, &mut i);
    if i != b.len() {
        return Err("trailing bytes".into());
    }
    Ok(v)
}

fn ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && matches!(b[*i], b' ' | b'\n' | b'\t' | b'\r') {
        *i += 1;
    }
}

fn value(b: &[u8], i: &mut usize) -> Result<J, String> {
    ws(b, i);
    match b.get(*i) {
        Some(b'{') => {
            *i += 1;
            let mut out = Vec::new();
            loop {
                ws(b, i);
                if b.get(*i) == Some(&b'}') {
                    *i += 1;
                    return Ok(J::O(out));
                }
                if !out.is_empty() {
                    if b.get(*i) != Some(&b',') {
                        return Err("expected ,".into());
                    }
                    *i += 1;
                    ws(b, i);
                }
                let k = string(b, i)?;
                ws(b, i);
                if b.get(*i) != Some(&b':') {
                    return Err("expected :".into());
                }
                *i += 1;
                out.push((k, value(b, i)?));
            }
        }
        Some(b'[') => {
            *i += 1;
            let mut out = Vec::new();
            loop {
                ws(b, i);
                if b.get(*i) == Some(&b']') {
                    *i += 1;
                    return Ok(J::A(out));
                }
                if !out.is_empty() {
                    if b.get(*i) != Some(&b',') {
                        return Err("expected ,".into());
                    }
                    *i += 1;
                }
                out.push(value(b, i)?);
            }
        }
        Some(b'"') => Ok(J::S(string(b, i)?)),
        Some(b't') if b[*i..].starts_with(b"true") => {
            *i += 4;
            Ok(J::B(true))
        }
        Some(b'n') if b[*i..].starts_with(b"null") => {
            *i += 4;
            Ok(J::Null)
        }
        Some(b'f') if b[*i..].starts_with(b"false") => {
            *i += 5;
            Ok(J::B(false))
        }
        Some(c) if c.is_ascii_digit() => {
            let s = *i;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            std::str::from_utf8(&b[s..*i]).unwrap().parse().map(J::N).map_err(|e| format!("{e}"))
        }
        other => Err(format!("unexpected {other:?}")),
    }
}

fn string(b: &[u8], i: &mut usize) -> Result<String, String> {
    if b.get(*i) != Some(&b'"') {
        return Err("expected string".into());
    }
    *i += 1;
    let mut out = String::new();
    while let Some(&c) = b.get(*i) {
        *i += 1;
        match c {
            b'"' => return Ok(out),
            b'\\' => {
                let e = *b.get(*i).ok_or("eof")?;
                *i += 1;
                match e {
                    b'n' => out.push('\n'),
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'u' => {
                        let h = std::str::from_utf8(&b[*i..*i + 4]).map_err(|e| e.to_string())?;
                        *i += 4;
                        out.push(char::from_u32(u32::from_str_radix(h, 16).map_err(|e| e.to_string())?).ok_or("bad")?);
                    }
                    _ => return Err("bad escape".into()),
                }
            }
            _ => out.push(c as char),
        }
    }
    Err("unterminated".into())
}
