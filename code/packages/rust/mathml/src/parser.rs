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

use math_frontend::{BinOp, FrontendError, MathExpr, Number, RelOp, UnaryOp};

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
}

impl Parser {
    fn new(events: Vec<Event>, src_len: usize) -> Self {
        Parser { events, pos: 0, src_len }
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
            // `<mfenced>…</mfenced>` — a parenthesised group. We model the fence as a `Group` over
            // the folded contents (its `open`/`close`/`separators` attributes are presentation,
            // dropped like all attributes); a comma-separated list folds as one row (PR-2 limit).
            "mfenced" => {
                let kids = self.parse_row_children(name, depth)?;
                let inner = fold_row(kids, span, depth)?;
                Ok(Child::Expr(MathExpr::Group(Box::new(inner))))
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

/// The crate entry point: parse a Presentation-MathML string into the neutral [`MathExpr`].
pub fn parse(src: &str) -> Result<MathExpr, FrontendError> {
    let events = Lexer::new(src).events()?;
    let mut parser = Parser::new(events, src.len());
    parser.parse_document()
}
