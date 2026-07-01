//! Presentation-MathML reader: an XML-subset event lexer + a recursive element-tree builder
//! that produces the neutral [`MathExpr`](math_frontend::MathExpr).
//!
//! MathML is XML, but we do **not** need a general XML engine — Presentation MathML is a small,
//! regular tree of known elements. So the lexer emits just three event kinds (start tag, end tag,
//! character data), ignoring attributes, namespace prefixes (`m:math` ≡ `math`), the XML
//! declaration, comments, and DOCTYPE. The builder then walks those events into `MathExpr`.
//!
//! Contract (shared by every frontend): **total and panic-free** — every input is `Ok(MathExpr)`
//! or a spanned [`FrontendError`]. Recursion is bounded by `MAX_DEPTH` so deeply-nested input
//! yields a clean error rather than a stack overflow, and the neutral `MathExpr` it returns drops
//! iteratively (math-frontend ≥ 0.3.0), so teardown is panic-free at any depth too.

use math_frontend::{BinOp, Func, FrontendError, MathExpr, Number, RelOp, UnaryOp};

/// Maximum element/fence nesting depth. Presentation MathML for real formulae is shallow (a few
/// dozen levels at most); a pathologically deep `<mrow><mrow>…` or `(((…` is rejected with a spanned
/// error instead of overflowing the parse stack. Kept conservatively low because each nesting level
/// here costs several recursive frames (parse_one_into → build_element → parse_row_children, or the
/// fold_row → fence → fold_row fence loop), and `#[test]` threads run on a ~2 MB stack.
const MAX_DEPTH: usize = 64;

const FRONTEND: &str = "mathml";

fn err<T>(message: impl Into<String>, span: (usize, usize)) -> Result<T, FrontendError> {
    Err(FrontendError::new(FRONTEND, message, span))
}

// ---- XML-subset event lexer -------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    /// `<name …>` — attributes discarded, namespace prefix stripped. `self_closing` for `<name/>`.
    Start { name: String, self_closing: bool, span: (usize, usize) },
    /// `</name>`.
    End { name: String, span: (usize, usize) },
    /// Character data between tags (entity-decoded, never empty after trim unless whitespace-only
    /// is meaningful — we keep it and let the builder decide).
    Text { text: String, span: (usize, usize) },
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0 }
    }

    fn events(mut self) -> Result<Vec<Event>, FrontendError> {
        let mut out = Vec::new();
        let n = self.src.len();
        while self.pos < n {
            if self.src[self.pos] == b'<' {
                // A markup construct. Distinguish comment / declaration / doctype / end / start.
                if self.starts_with("<!--") {
                    self.skip_until("-->", "unterminated comment")?;
                } else if self.starts_with("<?") {
                    self.skip_until("?>", "unterminated processing instruction")?;
                } else if self.starts_with("<!") {
                    // DOCTYPE or CDATA-ish; skip to the next '>'. (CDATA with '>' inside is out of
                    // scope for PR-1 presentation MathML.)
                    self.skip_until(">", "unterminated declaration")?;
                } else if self.starts_with("</") {
                    out.push(self.read_end_tag()?);
                } else {
                    out.push(self.read_start_tag()?);
                }
            } else {
                if let Some(ev) = self.read_text()? {
                    out.push(ev);
                }
            }
        }
        Ok(out)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s.as_bytes())
    }

    fn skip_until(&mut self, close: &str, unterminated: &str) -> Result<(), FrontendError> {
        let start = self.pos;
        let bytes = close.as_bytes();
        while self.pos < self.src.len() {
            if self.src[self.pos..].starts_with(bytes) {
                self.pos += bytes.len();
                return Ok(());
            }
            self.pos += 1;
        }
        err(unterminated, (start, self.src.len()))
    }

    fn read_end_tag(&mut self) -> Result<Event, FrontendError> {
        let start = self.pos;
        self.pos += 2; // consume "</"
        let name = self.read_name();
        if name.is_empty() {
            return err("malformed end tag", (start, self.pos));
        }
        self.skip_spaces();
        if self.pos >= self.src.len() || self.src[self.pos] != b'>' {
            return err(format!("expected '>' to close </{name}"), (start, self.pos));
        }
        self.pos += 1; // '>'
        Ok(Event::End { name, span: (start, self.pos) })
    }

    fn read_start_tag(&mut self) -> Result<Event, FrontendError> {
        let start = self.pos;
        self.pos += 1; // consume '<'
        let name = self.read_name();
        if name.is_empty() {
            return err("malformed start tag", (start, self.pos));
        }
        // Skip attributes: scan to the matching '>' honouring quoted attribute values.
        let mut self_closing = false;
        loop {
            if self.pos >= self.src.len() {
                return err(format!("unterminated <{name}"), (start, self.src.len()));
            }
            match self.src[self.pos] {
                b'>' => {
                    self.pos += 1;
                    break;
                }
                b'/' if self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'>' => {
                    self_closing = true;
                    self.pos += 2;
                    break;
                }
                b'"' | b'\'' => {
                    let quote = self.src[self.pos];
                    self.pos += 1;
                    while self.pos < self.src.len() && self.src[self.pos] != quote {
                        self.pos += 1;
                    }
                    if self.pos >= self.src.len() {
                        return err(format!("unterminated attribute in <{name}"), (start, self.src.len()));
                    }
                    self.pos += 1; // closing quote
                }
                _ => self.pos += 1,
            }
        }
        Ok(Event::Start { name, self_closing, span: (start, self.pos) })
    }

    /// Read an element name and strip any namespace prefix (`m:math` → `math`).
    fn read_name(&mut self) -> String {
        let begin = self.pos;
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b':' || c == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = std::str::from_utf8(&self.src[begin..self.pos]).unwrap_or("");
        match raw.rsplit_once(':') {
            Some((_prefix, local)) => local.to_string(),
            None => raw.to_string(),
        }
    }

    fn skip_spaces(&mut self) {
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// Read character data up to the next `<`, decoding entities. Returns `None` if the run is
    /// empty or pure inter-element whitespace (which carries no math meaning).
    fn read_text(&mut self) -> Result<Option<Event>, FrontendError> {
        let start = self.pos;
        let mut s = String::new();
        while self.pos < self.src.len() && self.src[self.pos] != b'<' {
            if self.src[self.pos] == b'&' {
                s.push_str(&self.read_entity()?);
            } else {
                // Copy one UTF-8 scalar (the source is valid UTF-8, so advance by the lead byte's width).
                let w = utf8_width(self.src[self.pos]);
                let end = (self.pos + w).min(self.src.len());
                if let Ok(chunk) = std::str::from_utf8(&self.src[self.pos..end]) {
                    s.push_str(chunk);
                }
                self.pos = end;
            }
        }
        if s.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(Event::Text { text: s, span: (start, self.pos) }))
        }
    }

    fn read_entity(&mut self) -> Result<String, FrontendError> {
        let start = self.pos;
        self.pos += 1; // '&'
        let begin = self.pos;
        while self.pos < self.src.len() && self.src[self.pos] != b';' && self.src[self.pos] != b'<' {
            self.pos += 1;
        }
        if self.pos >= self.src.len() || self.src[self.pos] != b';' {
            return err("unterminated entity reference", (start, self.pos));
        }
        let name = std::str::from_utf8(&self.src[begin..self.pos]).unwrap_or("");
        self.pos += 1; // ';'
        Ok(decode_entity(name))
    }
}

fn utf8_width(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead >> 5 == 0b110 {
        2
    } else if lead >> 4 == 0b1110 {
        3
    } else if lead >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// Decode an entity body (the part between `&` and `;`). Unknown names are returned verbatim
/// wrapped back as `&name;` so nothing is silently dropped — an unknown entity simply becomes part
/// of a symbol's text rather than an error.
fn decode_entity(name: &str) -> String {
    // Numeric character references: &#NN; and &#xHH;.
    if let Some(rest) = name.strip_prefix('#') {
        let code = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            rest.parse::<u32>().ok()
        };
        if let Some(c) = code.and_then(char::from_u32) {
            return c.to_string();
        }
        return format!("&{name};");
    }
    let mapped = match name {
        "lt" => "<",
        "gt" => ">",
        "amp" => "&",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => " ",
        // Common MathML operator entities so `<mo>` content maps to a known operator.
        "times" => "×",
        "div" => "÷",
        "minus" => "−",
        "plusmn" | "PlusMinus" => "±",
        "MinusPlus" => "∓",
        "middot" | "CenterDot" | "sdot" => "·",
        "le" | "leq" => "≤",
        "ge" | "geq" => "≥",
        "ne" | "neq" => "≠",
        "equiv" => "≡",
        "approx" => "≈",
        "InvisibleTimes" | "af" | "ApplyFunction" => "", // invisible operators carry no glyph
        "pi" => "π",
        _ => return format!("&{name};"),
    };
    mapped.to_string()
}

/// Read an attribute's value out of an already-lexed start-tag byte span
/// (`src[span.0..span.1]` = `<name … attr="value" …>`). The lexer drops attributes as it scans
/// (they are presentation), so a handler that needs a *meaning-bearing* one — the `open`/`close`
/// delimiters on `<mfenced>` — re-reads it from the source slice here. `attr` matches only as a
/// whole word (the byte before it must be whitespace, so `open` never matches inside another
/// attribute name); the value may be single- or double-quoted; entity references in the value are
/// decoded (`&lang;` → ⟨, `&#x2016;` → ‖). Returns `None` when the attribute is absent.
fn tag_attr(src: &[u8], span: (usize, usize), attr: &str) -> Option<String> {
    let end = span.1.min(src.len());
    let start = span.0.min(end);
    let tag = &src[start..end];
    let key = attr.as_bytes();
    // Start at 1: the byte at index 0 is `<`, and every real attribute is preceded by whitespace.
    let mut i = 1;
    while i + key.len() <= tag.len() {
        if tag[i - 1].is_ascii_whitespace() && tag[i..].starts_with(key) {
            let mut j = i + key.len();
            while j < tag.len() && tag[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < tag.len() && tag[j] == b'=' {
                j += 1;
                while j < tag.len() && tag[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < tag.len() && (tag[j] == b'"' || tag[j] == b'\'') {
                    let quote = tag[j];
                    j += 1;
                    let val_start = j;
                    while j < tag.len() && tag[j] != quote {
                        j += 1;
                    }
                    return Some(decode_attr_value(&tag[val_start..j]));
                }
            }
        }
        i += 1;
    }
    None
}

/// Decode entity references (`&name;`, `&#NN;`) inside an attribute value, reusing the same entity
/// table as character data. Non-entity bytes pass through (lossy UTF-8, matching the lexer).
fn decode_attr_value(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    let mut out = String::new();
    let mut rest: &str = &s;
    loop {
        match rest.find('&') {
            None => {
                out.push_str(rest);
                break;
            }
            Some(amp) => {
                out.push_str(&rest[..amp]);
                match rest[amp + 1..].find(';') {
                    Some(semi) => {
                        out.push_str(&decode_entity(&rest[amp + 1..amp + 1 + semi]));
                        rest = &rest[amp + 1 + semi + 1..];
                    }
                    None => {
                        // A bare `&` with no terminator: keep it verbatim, stop.
                        out.push_str(&rest[amp..]);
                        break;
                    }
                }
            }
        }
    }
    out
}

// ---- element-tree builder ---------------------------------------------------------------------

/// One processed child of a row: either a finished operand expression, or a raw `<mo>` operator
/// string (interpreted by the row folder), or a fenced sub-row already wrapped as a `Group`.
enum Child {
    Expr(MathExpr),
    Op(String),
}

struct Parser {
    events: Vec<Event>,
    pos: usize,
    src_len: usize,
    /// The original source bytes. The lexer discards attributes as it scans (they are
    /// presentation), but a handler that needs a *meaning-bearing* attribute — the `open`/`close`
    /// delimiters on `<mfenced>` — re-reads it here from the start-tag's byte span (`tag_attr`).
    src: Vec<u8>,
}

impl Parser {
    fn new(events: Vec<Event>, src: &[u8]) -> Self {
        Parser { events, pos: 0, src_len: src.len(), src: src.to_vec() }
    }

    fn peek(&self) -> Option<&Event> {
        self.events.get(self.pos)
    }

    /// Parse the whole document: an optional single `<math>` wrapper around a row, or a bare row of
    /// top-level presentation elements.
    fn parse_document(&mut self) -> Result<MathExpr, FrontendError> {
        // Collect every top-level element/text into children, then fold as one row. A `<math>`
        // wrapper is transparent: its children become this row.
        let children = self.parse_children_until_eof(0)?;
        if children.is_empty() {
            return err("empty MathML: no elements found", (0, self.src_len));
        }
        fold_row(children, (0, self.src_len), 0)
    }

    fn parse_children_until_eof(&mut self, depth: usize) -> Result<Vec<Child>, FrontendError> {
        let mut children = Vec::new();
        while self.peek().is_some() {
            match self.peek().unwrap() {
                Event::End { name, span } => {
                    return err(format!("unexpected </{name}> with no open element"), *span);
                }
                _ => self.parse_one_into(&mut children, depth)?,
            }
        }
        Ok(children)
    }

    /// Parse a single element (and its subtree) appending the resulting `Child`(ren). A `<math>`
    /// wrapper is spliced transparently (its children join the current row).
    fn parse_one_into(&mut self, out: &mut Vec<Child>, depth: usize) -> Result<(), FrontendError> {
        if depth > MAX_DEPTH {
            let span = self.current_span();
            return err("MathML nested too deeply", span);
        }
        let ev = self.events[self.pos].clone();
        match ev {
            Event::Text { text, .. } => {
                self.pos += 1;
                // Bare text outside a leaf element is unusual; treat as a symbol token.
                out.push(Child::Expr(MathExpr::Symbol(text.trim().to_string())));
                Ok(())
            }
            Event::Start { name, self_closing, span } => {
                self.pos += 1;
                if self_closing {
                    out.push(self.build_leaf(&name, "", span)?);
                    return Ok(());
                }
                // `math` and `mstyle`/`mpadded` are transparent containers in PR-1: their children
                // join the surrounding row.
                if name == "math" || name == "mstyle" || name == "mpadded" {
                    let kids = self.parse_row_children(&name, depth + 1)?;
                    let folded = fold_row(kids, span, depth + 1)?;
                    out.push(Child::Expr(folded));
                    return Ok(());
                }
                let child = self.build_element(&name, span, depth + 1)?;
                out.push(child);
                Ok(())
            }
            Event::End { name, span } => err(format!("unexpected </{name}>"), span),
        }
    }

    /// Parse children up to the matching `</name>`, returning them as a row of `Child`s.
    fn parse_row_children(&mut self, name: &str, depth: usize) -> Result<Vec<Child>, FrontendError> {
        let mut children = Vec::new();
        loop {
            match self.peek() {
                None => {
                    return err(format!("unclosed <{name}> (expected </{name}>)"), (0, self.src_len));
                }
                Some(Event::End { name: end, .. }) => {
                    if end == name {
                        self.pos += 1; // consume the matching end tag
                        return Ok(children);
                    }
                    let span = self.current_span();
                    let end = end.clone();
                    return err(format!("mismatched </{end}>, expected </{name}>"), span);
                }
                _ => self.parse_one_into(&mut children, depth)?,
            }
        }
    }

    /// Build a known container/script element (children already to be read up to its end tag).
    fn build_element(&mut self, name: &str, span: (usize, usize), depth: usize) -> Result<Child, FrontendError> {
        match name {
            // Leaf token elements: exactly one text child.
            "mn" | "mi" | "mo" | "mtext" => {
                let text = self.read_leaf_text(name)?;
                self.build_leaf(name, &text, span)
            }
            // `mrow` and friends: a sub-row, folded.
            "mrow" => {
                let kids = self.parse_row_children(name, depth)?;
                Ok(Child::Expr(fold_row(kids, span, depth)?))
            }
            "msqrt" => {
                let kids = self.parse_row_children(name, depth)?;
                let radicand = fold_row(kids, span, depth)?;
                Ok(Child::Expr(MathExpr::Root { degree: None, radicand: Box::new(radicand) }))
            }
            "mfrac" => {
                let args = self.read_n_args(name, 2, depth)?;
                let mut it = args.into_iter();
                let num = it.next().unwrap();
                let den = it.next().unwrap();
                Ok(Child::Expr(MathExpr::Frac(Box::new(num), Box::new(den))))
            }
            "mroot" => {
                let args = self.read_n_args(name, 2, depth)?;
                let mut it = args.into_iter();
                let radicand = it.next().unwrap();
                let degree = it.next().unwrap();
                Ok(Child::Expr(MathExpr::Root {
                    degree: Some(Box::new(degree)),
                    radicand: Box::new(radicand),
                }))
            }
            "msup" => {
                let args = self.read_n_args(name, 2, depth)?;
                let mut it = args.into_iter();
                let base = it.next().unwrap();
                let sup = it.next().unwrap();
                Ok(Child::Expr(MathExpr::Bin(BinOp::Pow, Box::new(base), Box::new(sup))))
            }
            "msub" => {
                let args = self.read_n_args(name, 2, depth)?;
                let mut it = args.into_iter();
                let base = it.next().unwrap();
                let sub = it.next().unwrap();
                Ok(Child::Expr(MathExpr::Subscript(Box::new(base), Box::new(sub))))
            }
            "msubsup" => {
                let args = self.read_n_args(name, 3, depth)?;
                let mut it = args.into_iter();
                let base = it.next().unwrap();
                let sub = it.next().unwrap();
                let sup = it.next().unwrap();
                let subbed = MathExpr::Subscript(Box::new(base), Box::new(sub));
                Ok(Child::Expr(MathExpr::Bin(BinOp::Pow, Box::new(subbed), Box::new(sup))))
            }
            // `<mover>base over</mover>` → annotation stacked over the base (drops the `accent`
            // attribute, which we ignore — a generic Overset). `<munder>base under</munder>` →
            // Underset. `<munderover>base under over</munderover>` → both, under-most outside.
            "mover" => {
                let args = self.read_n_script_args(name, 2, depth)?;
                let mut it = args.into_iter();
                let base = it.next().unwrap();
                let over = it.next().unwrap();
                Ok(Child::Expr(MathExpr::Overset { over: Box::new(over), base: Box::new(base) }))
            }
            "munder" => {
                let args = self.read_n_script_args(name, 2, depth)?;
                let mut it = args.into_iter();
                let base = it.next().unwrap();
                let under = it.next().unwrap();
                Ok(Child::Expr(MathExpr::Underset { under: Box::new(under), base: Box::new(base) }))
            }
            "munderover" => {
                let args = self.read_n_script_args(name, 3, depth)?;
                let mut it = args.into_iter();
                let base = it.next().unwrap();
                let under = it.next().unwrap();
                let over = it.next().unwrap();
                let overset = MathExpr::Overset { over: Box::new(over), base: Box::new(base) };
                Ok(Child::Expr(MathExpr::Underset { under: Box::new(under), base: Box::new(overset) }))
            }
            // `<mfenced>…</mfenced>` — a fence. Three shapes, decided by the top-level separators:
            //
            //   * NO separator            → a single delimited group → `Fenced { open, body, close }`,
            //                               carrying WHICH delimiters bracketed it (the `open`/`close`
            //                               attributes, default `(`/`)`) so `|x|` (absolute value) is
            //                               kept distinct from `(x)`. This is the mathml frontend
            //                               adopting the neutral `Fenced` node (latex already does).
            //   * `<mo>,</mo>` only       → a flat LIST `(a, b, c)` → `Sequence([a, b, c])`.
            //   * any `<mo>;</mo>`        → ROWS. Semicolons are the row separator and commas the
            //                               within-row (column) separator — the classic fenced-matrix
            //                               reading `(a, b; c, d)` → `Sequence([Sequence([a, b]),
            //                               Sequence([c, d])])`. A row with no comma is a single
            //                               expression, so a semicolon-only fence `(a; b; c)` — a
            //                               column vector — collapses to the same flat
            //                               `Sequence([a, b, c])` as a comma list (no row has a
            //                               second column). A ragged fence `(a; b, c)` is faithful:
            //                               `Sequence([a, Sequence([b, c])])`.
            //
            // The `separators` attribute is presentation and dropped — only *literal* `<mo>,</mo>`/
            // `<mo>;</mo>` children are read as separators, matching how the comma list already
            // worked. The `open`/`close` delimiter attributes ARE meaning-bearing in ALL three shapes
            // and re-read from the tag span into a wrapping `Fenced` — so a comma list `(a, b)` is kept
            // distinct from `[a, b]`, and the row structure is preserved as the `Fenced` body. This
            // mirrors the latex frontend, which likewise wraps its list fences in `Fenced` of a
            // `Sequence`. The adj-lang adapter unwraps `Fenced`, so a `Fenced`-of-`Sequence` lowers
            // exactly as the bare `Sequence` did — no downstream behaviour changes, only the delimiters
            // are now carried rather than silently dropped.
            "mfenced" => {
                let kids = self.parse_row_children(name, depth)?;
                let has_comma = kids.iter().any(|c| matches!(c, Child::Op(s) if s == ","));
                let has_semicolon = kids.iter().any(|c| matches!(c, Child::Op(s) if s == ";"));
                // MathML's own defaults for a bare `<mfenced>` are `(` and `)`. Read once — every
                // shape below carries the same open/close on its wrapping `Fenced`.
                let open = tag_attr(&self.src, span, "open").unwrap_or_else(|| "(".to_string());
                let close = tag_attr(&self.src, span, "close").unwrap_or_else(|| ")".to_string());
                let body = if !has_comma && !has_semicolon {
                    // A single delimited group — the body is just the folded row.
                    fold_row(kids, span, depth)?
                } else if !has_semicolon {
                    // Comma-only: a flat list. Each comma-delimited segment folds to one expression.
                    let items = split_fence_children(kids, ",")
                        .into_iter()
                        .map(|seg| fold_row(seg, span, depth))
                        .collect::<Result<Vec<_>, _>>()?;
                    MathExpr::Sequence(items)
                } else {
                    // Semicolon present: split into rows first, then columns within each row.
                    let mut rows: Vec<MathExpr> = Vec::new();
                    for row in split_fence_children(kids, ";") {
                        if row.iter().any(|c| matches!(c, Child::Op(s) if s == ",")) {
                            let cols = split_fence_children(row, ",")
                                .into_iter()
                                .map(|seg| fold_row(seg, span, depth))
                                .collect::<Result<Vec<_>, _>>()?;
                            rows.push(MathExpr::Sequence(cols));
                        } else {
                            rows.push(fold_row(row, span, depth)?);
                        }
                    }
                    MathExpr::Sequence(rows)
                };
                Ok(Child::Expr(MathExpr::Fenced {
                    open,
                    body: Box::new(body),
                    close,
                }))
            }
            // `<mtable>` of `<mtr>` rows of `<mtd>` cells → MathExpr::Matrix (delimiter style is
            // not part of MathML's mtable, so nothing to drop). Parsed structurally below.
            "mtable" => self.build_mtable(depth),
            other => {
                // Unknown element: consume its subtree so the lexer stays balanced, then report
                // it honestly rather than silently dropping content.
                let _ = self.parse_row_children(name, depth);
                err(format!("unsupported MathML element <{other}>"), span)
            }
        }
    }

    /// Parse a `<mtable>` body: a sequence of `<mtr>` rows, each a sequence of `<mtd>` cells, into
    /// `MathExpr::Matrix`. Stray non-`mtr` content (or non-`mtd` inside a row) is a spanned error.
    fn build_mtable(&mut self, depth: usize) -> Result<Child, FrontendError> {
        if depth > MAX_DEPTH {
            return err("MathML nested too deeply", self.current_span());
        }
        let mut rows: Vec<Vec<MathExpr>> = Vec::new();
        loop {
            match self.peek() {
                None => return err("unclosed <mtable> (expected </mtable>)", (0, self.src_len)),
                Some(Event::End { name, .. }) if name == "mtable" => {
                    self.pos += 1;
                    break;
                }
                Some(Event::Start { name, self_closing, .. }) if name == "mtr" => {
                    let self_closing = *self_closing;
                    self.pos += 1;
                    let cells = if self_closing { Vec::new() } else { self.build_mtr_cells(depth + 1)? };
                    rows.push(cells);
                }
                _ => {
                    let span = self.current_span();
                    return err("<mtable> may contain only <mtr> rows", span);
                }
            }
        }
        Ok(Child::Expr(MathExpr::Matrix(rows)))
    }

    /// Parse one `<mtr>` row: its `<mtd>` cells (each a folded row), up to `</mtr>`.
    fn build_mtr_cells(&mut self, depth: usize) -> Result<Vec<MathExpr>, FrontendError> {
        let mut cells: Vec<MathExpr> = Vec::new();
        loop {
            match self.peek() {
                None => return err("unclosed <mtr> (expected </mtr>)", (0, self.src_len)),
                Some(Event::End { name, .. }) if name == "mtr" => {
                    self.pos += 1;
                    return Ok(cells);
                }
                Some(Event::Start { name, self_closing, span }) if name == "mtd" => {
                    let self_closing = *self_closing;
                    let span = *span;
                    self.pos += 1;
                    if self_closing {
                        cells.push(MathExpr::Symbol(String::new())); // an empty cell
                    } else {
                        let kids = self.parse_row_children("mtd", depth + 1)?;
                        if kids.is_empty() {
                            cells.push(MathExpr::Symbol(String::new()));
                        } else {
                            cells.push(fold_row(kids, span, depth + 1)?);
                        }
                    }
                }
                _ => {
                    let span = self.current_span();
                    return err("<mtr> may contain only <mtd> cells", span);
                }
            }
        }
    }

    /// Read exactly `n` child *element* arguments (each folded to one expression) up to `</name>`.
    fn read_n_args(&mut self, name: &str, n: usize, depth: usize) -> Result<Vec<MathExpr>, FrontendError> {
        let kids = self.parse_row_children(name, depth)?;
        // Each child must be an operand expression (script/fraction args are single elements, not
        // bare operators).
        let mut args = Vec::with_capacity(kids.len());
        for c in kids {
            match c {
                Child::Expr(e) => args.push(e),
                Child::Op(s) => {
                    return err(
                        format!("<{name}> argument cannot be a bare operator {s:?}"),
                        (0, self.src_len),
                    )
                }
            }
        }
        if args.len() != n {
            return err(
                format!("<{name}> expects {n} arguments, got {}", args.len()),
                (0, self.src_len),
            );
        }
        Ok(args)
    }

    /// Like [`read_n_args`](Self::read_n_args), but an operator-glyph child (`<mo>^</mo>`,
    /// `<mo>‾</mo>`, `<mo>→</mo>`, `<mo>⏞</mo>`) is accepted as an annotation *symbol* rather than
    /// rejected. In over/under-script position a glyph is a mark stacked on the base, not an infix
    /// operator — so `<mover><mi>x</mi><mo>^</mo></mover>` is a legitimate "x with a hat". Used by
    /// `<mover>`/`<munder>`/`<munderover>`.
    fn read_n_script_args(&mut self, name: &str, n: usize, depth: usize) -> Result<Vec<MathExpr>, FrontendError> {
        let kids = self.parse_row_children(name, depth)?;
        let mut args = Vec::with_capacity(kids.len());
        for c in kids {
            match c {
                Child::Expr(e) => args.push(e),
                Child::Op(s) => args.push(MathExpr::Symbol(s)),
            }
        }
        if args.len() != n {
            return err(
                format!("<{name}> expects {n} arguments, got {}", args.len()),
                (0, self.src_len),
            );
        }
        Ok(args)
    }

    /// A leaf token element (`mn`/`mi`/`mo`/`mtext`) holds exactly one text child (possibly empty).
    fn read_leaf_text(&mut self, name: &str) -> Result<String, FrontendError> {
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return err(format!("unclosed <{name}>"), (0, self.src_len)),
                Some(Event::Text { text: t, .. }) => {
                    text.push_str(t);
                    self.pos += 1;
                }
                Some(Event::End { name: end, span }) => {
                    if end == name {
                        self.pos += 1;
                        return Ok(text);
                    }
                    return err(format!("mismatched </{end}>, expected </{name}>"), *span);
                }
                Some(Event::Start { name: inner, span, .. }) => {
                    // A token element must not contain child elements in PR-1.
                    return err(format!("<{name}> may not contain <{inner}>"), *span);
                }
            }
        }
    }

    fn build_leaf(&self, name: &str, text: &str, span: (usize, usize)) -> Result<Child, FrontendError> {
        let trimmed = text.trim();
        match name {
            "mn" => {
                let normalized = trimmed.replace(',', "");
                match Number::parse(&normalized) {
                    Some(n) => Ok(Child::Expr(MathExpr::Number(n))),
                    None => err(format!("<mn> is not a number: {trimmed:?}"), span),
                }
            }
            "mi" => {
                if trimmed.is_empty() {
                    // An empty/invisible <mi/> contributes nothing.
                    Ok(Child::Expr(MathExpr::Symbol(String::new())))
                } else {
                    Ok(Child::Expr(MathExpr::Symbol(trimmed.to_string())))
                }
            }
            "mtext" => Ok(Child::Expr(MathExpr::Text(text.to_string()))),
            "mo" => Ok(Child::Op(trimmed.to_string())),
            _ => err(format!("<{name}> is not a leaf token element"), span),
        }
    }

    fn current_span(&self) -> (usize, usize) {
        match self.events.get(self.pos) {
            Some(Event::Start { span, .. })
            | Some(Event::End { span, .. })
            | Some(Event::Text { span, .. }) => *span,
            None => (self.src_len, self.src_len),
        }
    }
}

// ---- row folding (operators + implicit multiplication + fences) -------------------------------

/// A token in the flat row passed to the precedence parser.
enum RowTok {
    Operand(MathExpr),
    Bin(BinOp),
    Rel(RelOp),
}

/// Split a fence's children into the maximal segments separated by a top-level `sep` operator
/// (e.g. `,` or `;`). A leading, trailing, or doubled separator yields an *empty* segment, which
/// `fold_row` later rejects as "empty MathML group" — so a malformed list is a clean spanned error,
/// never a silently-dropped item. Only *literal* `<mo>sep</mo>` children are separators; nested
/// elements keep their own inner separators (this looks at the top level only). A single bounded
/// pass over `children`, no recursion.
fn split_fence_children(children: Vec<Child>, sep: &str) -> Vec<Vec<Child>> {
    let mut segments: Vec<Vec<Child>> = Vec::new();
    let mut current: Vec<Child> = Vec::new();
    for child in children {
        match &child {
            Child::Op(s) if s == sep => segments.push(std::mem::take(&mut current)),
            _ => current.push(child),
        }
    }
    segments.push(current);
    segments
}

/// Fold a row of `Child`s into one `MathExpr`, applying parenthesis fences, operator precedence
/// (relations < add/sub < mul/div), implicit multiplication of adjacent operands, and unary signs.
/// `depth` bounds the fence recursion (`(`…`)` → `Group`) so adversarially nested fences error
/// rather than overflow.
fn fold_row(children: Vec<Child>, span: (usize, usize), depth: usize) -> Result<MathExpr, FrontendError> {
    if depth > MAX_DEPTH {
        return err("MathML nested too deeply", span);
    }
    let toks = lower_children_to_tokens(children, span, depth)?;
    if toks.is_empty() {
        return err("empty MathML group", span);
    }
    let mut rp = RowParser { toks, pos: 0, span };
    let e = rp.parse_rel()?;
    if rp.pos != rp.toks.len() {
        return err("trailing tokens in MathML row", span);
    }
    Ok(e)
}

/// Turn the row's children into `RowTok`s, resolving `(`…`)` fences into `Group` operands first.
fn lower_children_to_tokens(
    children: Vec<Child>,
    span: (usize, usize),
    depth: usize,
) -> Result<Vec<RowTok>, FrontendError> {
    let mut toks = Vec::new();
    let mut iter = children.into_iter().peekable();
    while let Some(child) = iter.next() {
        match child {
            Child::Expr(e) => toks.push(RowTok::Operand(e)),
            Child::Op(s) => {
                if s == "(" || s == "[" {
                    // Collect up to the matching close fence (depth-counted), recurse, wrap Group.
                    let mut inner: Vec<Child> = Vec::new();
                    let mut fence_depth = 1usize;
                    for next in iter.by_ref() {
                        if let Child::Op(o) = &next {
                            if o == "(" || o == "[" {
                                fence_depth += 1;
                            } else if o == ")" || o == "]" {
                                fence_depth -= 1;
                                if fence_depth == 0 {
                                    break;
                                }
                            }
                        }
                        inner.push(next);
                    }
                    if fence_depth != 0 {
                        return err("unbalanced fence in MathML row", span);
                    }
                    let grouped = fold_row(inner, span, depth + 1)?;
                    toks.push(RowTok::Operand(MathExpr::Group(Box::new(grouped))));
                } else if s == ")" || s == "]" {
                    return err("unmatched closing fence in MathML row", span);
                } else if let Some(op) = operator_token(&s) {
                    toks.push(op);
                } else if s.is_empty() {
                    // An invisible operator (InvisibleTimes / ApplyFunction): contributes nothing;
                    // adjacency will become implicit multiplication.
                } else {
                    // An unrecognised operator glyph is preserved as a symbol operand so meaning is
                    // not silently lost (e.g. a stray punctuation mark).
                    toks.push(RowTok::Operand(MathExpr::Symbol(s)));
                }
            }
        }
    }
    Ok(toks)
}

/// Map a `<mo>` glyph to a binary/relational operator, or `None` if it is not one we model.
fn operator_token(s: &str) -> Option<RowTok> {
    // One spelling per glyph: ASCII plus the Unicode math glyphs MathML commonly uses. `\u{2062}`
    // is INVISIBLE TIMES (a literal U+2062 in `<mo>` content), distinct from the entity we already
    // decode to "".
    Some(match s {
        "+" => RowTok::Bin(BinOp::Add),
        "-" | "−" => RowTok::Bin(BinOp::Sub), // ASCII hyphen-minus, U+2212 MINUS SIGN
        "*" | "×" | "·" | "⋅" | "\u{2062}" => RowTok::Bin(BinOp::Mul), // *, MULTIPLICATION, MIDDLE DOT, DOT OPERATOR, INVISIBLE TIMES
        "/" | "÷" => RowTok::Bin(BinOp::Div),
        "±" => RowTok::Bin(BinOp::PlusMinus),
        "∓" => RowTok::Bin(BinOp::MinusPlus),
        "=" => RowTok::Rel(RelOp::Eq),
        "≠" => RowTok::Rel(RelOp::Ne),
        "<" => RowTok::Rel(RelOp::Lt),
        ">" => RowTok::Rel(RelOp::Gt),
        "≤" => RowTok::Rel(RelOp::Le),
        "≥" => RowTok::Rel(RelOp::Ge),
        "≈" => RowTok::Rel(RelOp::Approx),
        "≡" => RowTok::Rel(RelOp::Equiv),
        _ => return None,
    })
}

struct RowParser {
    toks: Vec<RowTok>,
    pos: usize,
    span: (usize, usize),
}

impl RowParser {
    fn parse_rel(&mut self) -> Result<MathExpr, FrontendError> {
        let mut left = self.parse_add()?;
        while let Some(RowTok::Rel(op)) = self.toks.get(self.pos) {
            let op = *op;
            self.pos += 1;
            let right = self.parse_add()?;
            left = MathExpr::Rel(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<MathExpr, FrontendError> {
        let mut left = self.parse_mul()?;
        while let Some(RowTok::Bin(op @ (BinOp::Add | BinOp::Sub | BinOp::PlusMinus | BinOp::MinusPlus))) =
            self.toks.get(self.pos)
        {
            let op = *op;
            self.pos += 1;
            let right = self.parse_mul()?;
            left = MathExpr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<MathExpr, FrontendError> {
        let mut left = self.parse_unary()?;
        loop {
            match self.toks.get(self.pos) {
                Some(RowTok::Bin(op @ (BinOp::Mul | BinOp::Div))) => {
                    let op = *op;
                    self.pos += 1;
                    let right = self.parse_unary()?;
                    left = MathExpr::Bin(op, Box::new(left), Box::new(right));
                }
                // Implicit multiplication: two operands adjacent with no operator between.
                Some(RowTok::Operand(_)) => {
                    let right = self.parse_unary()?;
                    left = MathExpr::Bin(BinOp::Mul, Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<MathExpr, FrontendError> {
        // Collect a run of leading unary signs ITERATIVELY, then fold them back onto one atom.
        // This must not recurse per sign: the signs of a row live in one flat token list at a
        // single nesting level, so a recursive `parse_unary` would grow the call stack linearly
        // with the operator count and a long run of `<mo>-</mo>` would overflow the stack (an
        // uncatchable abort, defeating the panic-free contract). The loop keeps it O(1) in stack
        // depth; the resulting (possibly deep) `Unary` chain is built and dropped iteratively.
        let mut signs: Vec<UnaryOp> = Vec::new();
        loop {
            match self.toks.get(self.pos) {
                Some(RowTok::Bin(BinOp::Add)) => {
                    self.pos += 1;
                    signs.push(UnaryOp::Pos);
                }
                Some(RowTok::Bin(BinOp::Sub)) => {
                    self.pos += 1;
                    signs.push(UnaryOp::Neg);
                }
                _ => break,
            }
        }
        let mut e = self.parse_atom()?;
        // Fold innermost-first so `+ - x` becomes `Unary(Pos, Unary(Neg, x))` (outermost = the
        // first sign read).
        for op in signs.into_iter().rev() {
            e = MathExpr::Unary(op, Box::new(e));
        }
        Ok(e)
    }

    fn parse_atom(&mut self) -> Result<MathExpr, FrontendError> {
        // Named-function application. A function name arrives as a `<mi>` → `Symbol` operand (e.g.
        // `<mi>sin</mi>`), typically with an invisible `<mo>&ApplyFunction;</mo>` before its argument
        // that lowering already dropped — so the function symbol sits directly adjacent to its
        // argument. We recognise `sin x`, `cos(θ)`, `ln 2`, … → `Call { func, arg }`.
        //
        // The run of leading function names is collected ITERATIVELY, then folded onto the base atom
        // right-to-left, so `sin cos x` → `Call(sin, Call(cos, x))`. This deliberately does NOT
        // recurse per function: a flat run of N `<mi>sin</mi>` lives at one nesting level, and a
        // recursive folder would grow the stack with N and overflow on a long run (the same hazard
        // the iterative unary collector avoids). A function name NOT followed by an operand is a
        // plain symbol (`sin` alone is the variable/symbol `sin`, not an empty application).
        let mut funcs: Vec<Func> = Vec::new();
        while let Some(RowTok::Operand(MathExpr::Symbol(name))) = self.toks.get(self.pos) {
            if func_of(name).is_some() && matches!(self.toks.get(self.pos + 1), Some(RowTok::Operand(_))) {
                let f = func_of(name).unwrap();
                self.pos += 1;
                funcs.push(f);
            } else {
                break;
            }
        }
        let mut atom = self.take_operand()?;
        for f in funcs.into_iter().rev() {
            atom = MathExpr::Call { func: f, arg: Box::new(atom) };
        }
        Ok(atom)
    }

    /// Extract the operand at the cursor (the base of an atom), erroring on an operator or end.
    fn take_operand(&mut self) -> Result<MathExpr, FrontendError> {
        match self.toks.get(self.pos) {
            Some(RowTok::Operand(_)) => {
                // Take ownership of the operand by swapping in a placeholder.
                let RowTok::Operand(e) =
                    std::mem::replace(&mut self.toks[self.pos], RowTok::Operand(MathExpr::Symbol(String::new())))
                else {
                    unreachable!()
                };
                self.pos += 1;
                Ok(e)
            }
            Some(RowTok::Bin(_)) | Some(RowTok::Rel(_)) => {
                err("expected an operand in MathML row", self.span)
            }
            None => err("unexpected end of MathML row", self.span),
        }
    }
}

/// Map a `<mi>` identifier to a known mathematical function, or `None` if it is an ordinary symbol.
/// The recognised set matches the other frontends (`unicode-math`, `asciimath`) so the four
/// notations agree on one neutral `Func`. Only these exact spellings are functions — a one-letter
/// variable like `s` stays a `Symbol`, never a function.
fn func_of(name: &str) -> Option<Func> {
    Some(match name {
        "sin" => Func::Sin,
        "cos" => Func::Cos,
        "tan" => Func::Tan,
        "cot" => Func::Cot,
        "sec" => Func::Sec,
        "csc" => Func::Csc,
        "arcsin" => Func::Asin,
        "arccos" => Func::Acos,
        "arctan" => Func::Atan,
        "sinh" => Func::Sinh,
        "cosh" => Func::Cosh,
        "tanh" => Func::Tanh,
        "ln" => Func::Ln,
        "log" => Func::Log,
        "exp" => Func::Exp,
        "min" => Func::Min,
        "max" => Func::Max,
        "gcd" => Func::Gcd,
        "lcm" => Func::Lcm,
        "det" => Func::Det,
        _ => return None,
    })
}

/// The crate entry point: parse a Presentation-MathML string into the neutral [`MathExpr`].
pub fn parse(src: &str) -> Result<MathExpr, FrontendError> {
    let events = Lexer::new(src).events()?;
    let mut parser = Parser::new(events, src.as_bytes());
    parser.parse_document()
}
