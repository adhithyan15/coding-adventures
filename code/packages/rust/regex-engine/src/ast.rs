//! # Parser — regular-expression text into an abstract syntax tree (AST)
//!
//! A regex like `a(b|c)*d` is first parsed into a tree describing its structure:
//! a literal `a`, followed by a *starred group* of the alternation `b|c`,
//! followed by a literal `d`. The [`compile`](crate::program) step then turns
//! that tree into bytecode for the matching VM.
//!
//! This parser handles the structural core of the syntax: literals, `.`, escapes
//! (`\d \w \s` and their negations, plus escaped metacharacters), character
//! classes `[...]`/`[^...]` with ranges, groups `(...)`/`(?:...)`, alternation
//! `|`, the quantifiers `* + ?` and `{m}`/`{m,}`/`{m,n}` (greedy or lazy), the
//! anchors `^ $`, and word boundaries `\b \B`. In ASCII mode the character
//! classes are the ASCII sets; Unicode-aware classes are a later addition.

use std::fmt;

/// A parse error with a human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "regex parse error: {}", self.0)
    }
}

/// One inclusive range of characters, e.g. `a-z` is `('a', 'z')`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassRange {
    pub start: char,
    pub end: char,
}

/// A character class: a set of ranges, possibly negated (`[^...]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub negated: bool,
    pub ranges: Vec<ClassRange>,
}

/// An assertion is a zero-width condition on a position in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assertion {
    /// `^` — start of the input (this engine is not multi-line).
    StartText,
    /// `$` — end of the input.
    EndText,
    /// `\b` — an ASCII word boundary.
    WordBoundary,
    /// `\B` — a non-word-boundary.
    NotWordBoundary,
}

/// The parsed regular-expression tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    /// Matches the empty string.
    Empty,
    /// A single literal character.
    Literal(char),
    /// `.` — any character (whether it matches `\n` depends on the dot-all flag,
    /// applied at compile time).
    AnyChar,
    /// A character class (`[...]`, or an escape like `\d`).
    Class(Class),
    /// A zero-width assertion (`^ $ \b \B`).
    Assert(Assertion),
    /// A sequence: match each child in turn.
    Concat(Vec<Ast>),
    /// An alternation `a|b|c`: match any one child (tried left-to-right).
    Alternate(Vec<Ast>),
    /// A quantified sub-expression. `min`/`max` bound the repetition (`max =
    /// None` means unbounded); `greedy` chooses whether to prefer matching more
    /// (`*`) or fewer (`*?`) repetitions.
    Repeat {
        inner: Box<Ast>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
    /// A group. `capture` is `Some(n)` for a capturing group (the nth pair of
    /// parentheses, 1-based) or `None` for `(?:...)`.
    Group {
        inner: Box<Ast>,
        capture: Option<usize>,
    },
}

/// Flags that can be toggled inline at the very start of a pattern (`(?is)`),
/// mirroring the small subset of the `regex` crate's flags this engine needs.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub case_insensitive: bool,
    pub dot_matches_new_line: bool,
}

/// Maximum group/alternation nesting depth. Past this the parser errors instead
/// of recursing, so a pathological pattern like `(((…(a)…)))` cannot overflow the
/// call stack. Matches the spirit of the `regex` crate's default `nest_limit`.
const NEST_LIMIT: u32 = 250;

/// Maximum repetition bound accepted in `{m}`/`{m,n}`. A pattern requesting more
/// (e.g. `a{4000000000}`) is rejected at parse time, so the compiler never tries
/// to expand billions of copies (a memory / CPU DoS).
const REPEAT_LIMIT: u32 = 100_000;

/// Parse `pattern` into an [`Ast`], returning the tree, the number of capturing
/// groups, and any leading inline flags.
pub fn parse(pattern: &str) -> Result<(Ast, usize, Flags), ParseError> {
    let mut p = Parser {
        chars: pattern.chars().collect(),
        pos: 0,
        group_count: 0,
        flags: Flags::default(),
    };
    p.parse_leading_flags();
    let ast = p.parse_alternation(0)?;
    if p.pos != p.chars.len() {
        return Err(ParseError(format!(
            "unexpected `{}` at position {}",
            p.chars[p.pos], p.pos
        )));
    }
    Ok((ast, p.group_count, p.flags))
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    group_count: usize,
    flags: Flags,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }
    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consume a leading `(?flags)` group, e.g. `(?is)`, setting [`Flags`].
    fn parse_leading_flags(&mut self) {
        // Only a `(?<letters>)` at position 0 is treated as a global flag set.
        if self.chars.get(self.pos) != Some(&'(') || self.chars.get(self.pos + 1) != Some(&'?') {
            return;
        }
        // Scan the flag letters up to the closing `)`; bail if it looks like a
        // non-flag construct such as `(?:`.
        let mut j = self.pos + 2;
        let mut f = Flags::default();
        while let Some(&c) = self.chars.get(j) {
            match c {
                'i' => f.case_insensitive = true,
                's' => f.dot_matches_new_line = true,
                'u' | 'U' | 'm' | 'x' => {} // accepted, no effect in this engine
                ')' => {
                    self.flags = f;
                    self.pos = j + 1;
                    return;
                }
                _ => return, // not a pure flag group (e.g. `(?:`)
            }
            j += 1;
        }
    }

    /// `alternation := concat ('|' concat)*`
    fn parse_alternation(&mut self, depth: u32) -> Result<Ast, ParseError> {
        let mut branches = vec![self.parse_concat(depth)?];
        while self.eat('|') {
            branches.push(self.parse_concat(depth)?);
        }
        if branches.len() == 1 {
            Ok(branches.pop().unwrap())
        } else {
            Ok(Ast::Alternate(branches))
        }
    }

    /// `concat := repeat*`
    fn parse_concat(&mut self, depth: u32) -> Result<Ast, ParseError> {
        let mut items = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            items.push(self.parse_repeat(depth)?);
        }
        Ok(match items.len() {
            0 => Ast::Empty,
            1 => items.pop().unwrap(),
            _ => Ast::Concat(items),
        })
    }

    /// `repeat := atom quantifier?`
    fn parse_repeat(&mut self, depth: u32) -> Result<Ast, ParseError> {
        let atom = self.parse_atom(depth)?;
        let (min, max) = match self.peek() {
            Some('*') => {
                self.bump();
                (0, None)
            }
            Some('+') => {
                self.bump();
                (1, None)
            }
            Some('?') => {
                self.bump();
                (0, Some(1))
            }
            Some('{') => match self.try_parse_bound()? {
                Some(bound) => bound,
                None => return Ok(atom), // a literal `{`
            },
            _ => return Ok(atom),
        };
        // A trailing `?` after a quantifier makes it lazy.
        let greedy = !self.eat('?');
        Ok(Ast::Repeat {
            inner: Box::new(atom),
            min,
            max,
            greedy,
        })
    }

    /// Parse `{m}`, `{m,}`, or `{m,n}`. Returns `None` (and does not consume) if
    /// the `{` does not begin a valid bound, so it can be treated as a literal.
    fn try_parse_bound(&mut self) -> Result<Option<(u32, Option<u32>)>, ParseError> {
        let save = self.pos;
        self.bump(); // consume '{'
        let min = self.parse_number();
        let bound = match self.peek() {
            Some('}') if min.is_some() => {
                self.bump();
                Some((min.unwrap(), Some(min.unwrap())))
            }
            Some(',') if min.is_some() => {
                self.bump();
                let max = self.parse_number();
                if self.eat('}') {
                    Some((min.unwrap(), max))
                } else {
                    None
                }
            }
            _ => None,
        };
        if bound.is_none() {
            self.pos = save; // not a bound — restore for literal handling
        }
        if let Some((lo, hi)) = bound {
            if let Some(hi) = hi {
                if lo > hi {
                    return Err(ParseError(format!("invalid bound {{{lo},{hi}}}")));
                }
                if hi > REPEAT_LIMIT {
                    return Err(ParseError(format!("repetition bound {hi} too large")));
                }
            }
            if lo > REPEAT_LIMIT {
                return Err(ParseError(format!("repetition bound {lo} too large")));
            }
        }
        Ok(bound)
    }

    fn parse_number(&mut self) -> Option<u32> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse()
            .ok()
    }

    /// `atom := group | class | '.' | '^' | '$' | escape | literal`
    fn parse_atom(&mut self, depth: u32) -> Result<Ast, ParseError> {
        match self.peek() {
            Some('(') => self.parse_group(depth),
            Some('[') => self.parse_class(),
            Some('.') => {
                self.bump();
                Ok(Ast::AnyChar)
            }
            Some('^') => {
                self.bump();
                Ok(Ast::Assert(Assertion::StartText))
            }
            Some('$') => {
                self.bump();
                Ok(Ast::Assert(Assertion::EndText))
            }
            Some('\\') => self.parse_escape(),
            Some('*' | '+' | '?') => {
                Err(ParseError("quantifier with nothing to repeat".to_string()))
            }
            Some(c) => {
                self.bump();
                Ok(Ast::Literal(c))
            }
            None => Ok(Ast::Empty),
        }
    }

    fn parse_group(&mut self, depth: u32) -> Result<Ast, ParseError> {
        if depth + 1 > NEST_LIMIT {
            return Err(ParseError(format!(
                "pattern nested too deeply (> {NEST_LIMIT})"
            )));
        }
        self.bump(); // '('
        let capture = if self.peek() == Some('?') {
            self.bump(); // '?'
            match self.bump() {
                Some(':') => None,
                other => {
                    return Err(ParseError(format!(
                        "unsupported group flag `(?{}`",
                        other.map(String::from).unwrap_or_default()
                    )))
                }
            }
        } else {
            self.group_count += 1;
            Some(self.group_count)
        };
        let inner = self.parse_alternation(depth + 1)?;
        if !self.eat(')') {
            return Err(ParseError("unclosed group `(`".to_string()));
        }
        Ok(Ast::Group {
            inner: Box::new(inner),
            capture,
        })
    }

    fn parse_escape(&mut self) -> Result<Ast, ParseError> {
        self.bump(); // '\\'
        let c = self
            .bump()
            .ok_or_else(|| ParseError("trailing backslash".to_string()))?;
        Ok(match c {
            'd' => Ast::Class(digit_class(false)),
            'D' => Ast::Class(digit_class(true)),
            'w' => Ast::Class(word_class(false)),
            'W' => Ast::Class(word_class(true)),
            's' => Ast::Class(space_class(false)),
            'S' => Ast::Class(space_class(true)),
            'b' => Ast::Assert(Assertion::WordBoundary),
            'B' => Ast::Assert(Assertion::NotWordBoundary),
            'n' => Ast::Literal('\n'),
            'r' => Ast::Literal('\r'),
            't' => Ast::Literal('\t'),
            'f' => Ast::Literal('\u{0C}'),
            'v' => Ast::Literal('\u{0B}'),
            '0' => Ast::Literal('\0'),
            // Any other escaped character is that literal character (covers the
            // metacharacters `. * + ? ( ) [ ] { } | ^ $ \` and ordinary escapes).
            other => Ast::Literal(other),
        })
    }

    fn parse_class(&mut self) -> Result<Ast, ParseError> {
        self.bump(); // '['
        let negated = self.eat('^');
        let mut ranges: Vec<ClassRange> = Vec::new();
        // A `]` immediately after `[` or `[^` is a literal `]`.
        let mut first = true;
        loop {
            match self.peek() {
                None => return Err(ParseError("unclosed character class `[`".to_string())),
                Some(']') if !first => {
                    self.bump();
                    break;
                }
                _ => {}
            }
            first = false;
            let lo = self.class_char()?;
            match lo {
                ClassItem::Range(r) => ranges.extend(r), // `\d` etc. inside a class
                ClassItem::Char(c) => {
                    // Possible `c-d` range (but `-` right before `]` is literal).
                    if self.peek() == Some('-') && self.chars.get(self.pos + 1) != Some(&']') {
                        self.bump(); // '-'
                        match self.class_char()? {
                            ClassItem::Char(hi) => {
                                if (c as u32) > (hi as u32) {
                                    return Err(ParseError(format!(
                                        "invalid class range {c}-{hi}"
                                    )));
                                }
                                ranges.push(ClassRange { start: c, end: hi });
                            }
                            ClassItem::Range(_) => {
                                return Err(ParseError(
                                    "class escape cannot be a range endpoint".to_string(),
                                ))
                            }
                        }
                    } else {
                        ranges.push(ClassRange { start: c, end: c });
                    }
                }
            }
        }
        Ok(Ast::Class(Class { negated, ranges }))
    }

    /// One element inside `[...]`: a literal char or an embedded class escape.
    fn class_char(&mut self) -> Result<ClassItem, ParseError> {
        match self.bump() {
            Some('\\') => {
                let c = self
                    .bump()
                    .ok_or_else(|| ParseError("trailing backslash in class".to_string()))?;
                Ok(match c {
                    'd' => ClassItem::from(digit_class(false)),
                    'w' => ClassItem::from(word_class(false)),
                    's' => ClassItem::from(space_class(false)),
                    'n' => ClassItem::Char('\n'),
                    'r' => ClassItem::Char('\r'),
                    't' => ClassItem::Char('\t'),
                    'f' => ClassItem::Char('\u{0C}'),
                    'v' => ClassItem::Char('\u{0B}'),
                    other => ClassItem::Char(other),
                })
            }
            Some(c) => Ok(ClassItem::Char(c)),
            None => Err(ParseError("unexpected end of character class".to_string())),
        }
    }
}

/// An element parsed inside `[...]`.
enum ClassItem {
    Char(char),
    /// The ranges of an embedded positive class escape (`\d`, `\w`, `\s`). Only
    /// the positive forms are usable inside a class here.
    Range(Vec<ClassRange>),
}

impl ClassItem {
    fn from(class: Class) -> Self {
        ClassItem::Range(class.ranges)
    }
}

// --- ASCII class definitions (Unicode-aware variants come in a later PR) ------

fn ascii_class(negated: bool, ranges: Vec<ClassRange>) -> Class {
    Class { negated, ranges }
}

fn digit_class(negated: bool) -> Class {
    ascii_class(
        negated,
        vec![ClassRange {
            start: '0',
            end: '9',
        }],
    )
}

fn word_class(negated: bool) -> Class {
    ascii_class(
        negated,
        vec![
            ClassRange {
                start: '0',
                end: '9',
            },
            ClassRange {
                start: 'A',
                end: 'Z',
            },
            ClassRange {
                start: 'a',
                end: 'z',
            },
            ClassRange {
                start: '_',
                end: '_',
            },
        ],
    )
}

fn space_class(negated: bool) -> Class {
    // Matches the ASCII whitespace set `[\t\n\x0B\x0C\r ]`, i.e. the `regex`
    // crate's `(?-u:\s)`.
    ascii_class(
        negated,
        vec![
            ClassRange {
                start: '\t',
                end: '\r',
            }, // 09..0D: \t \n \v \f \r
            ClassRange {
                start: ' ',
                end: ' ',
            }, // 20
        ],
    )
}
