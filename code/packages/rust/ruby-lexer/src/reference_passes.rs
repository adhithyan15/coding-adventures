//! The token-fusion passes as they were before #18814, kept verbatim as the
//! oracle for the differential tests in `differential_tests`.
//!
//! Each body below is the pre-#18814 pass, copied unchanged except for its
//! name (`ref_` prefix) and `self.is_integer_lexeme(…)`, which became the
//! free function [`is_integer_lexeme`].  They fold a pair of tokens by
//! calling `Vec::remove` once per fusion, which shifts the whole tail of the
//! token vector each time, so they are quadratic: about 400 KB of `a&.b`
//! took roughly 150 s.  The production passes in `lib.rs` produce the same
//! stream in one linear compaction; the tests here prove "the same".
//!
//! Test-only: this module is compiled under `#[cfg(test)]` and nothing in
//! the shipped crate calls it.
//!
//! The #18814 review added `ref_finalize_heredoc`, the old heredoc
//! finalizer, which also removed one token per heredoc with `Vec::remove`.

use lexer::token::{Token, TokenType};

use super::{
    is_integer_lexeme, is_radix_integer_body, is_unsigned_exponent_lexeme, strip_common_leading_whitespace,
    HeredocVariant, PendingHeredoc, RubyLexer,
};

impl RubyLexer {
    /// The pre-#18814 `apply_era_token_fusions`, step for step, calling the
    /// reference passes where a pass was rewritten and the production pass
    /// where it was not.  `after_each` sees the lexer after every step, so
    /// a differential test can compare the two pipelines pass by pass.
    pub(crate) fn ref_apply_era_token_fusions(&mut self, after_each: &mut dyn FnMut(&str, &RubyLexer)) {
        self.ref_fuse_range_ops();
        after_each("fuse_range_ops", self);
        self.ref_fuse_radix_integers();
        after_each("fuse_radix_integers", self);
        self.ref_fuse_float_literals();
        after_each("fuse_float_literals", self);
        self.ref_fuse_right_shifts();
        after_each("fuse_right_shifts", self);
        self.ref_fuse_compound_assigns();
        after_each("fuse_compound_assigns", self);
        self.tag_modifier_keywords();
        after_each("tag_modifier_keywords", self);
        if super::era_at_least(&self.era, "1.9.1") {
            self.ref_fuse_lambda_arrow();
            after_each("fuse_lambda_arrow", self);
        }
        if super::era_at_least(&self.era, "2.1") {
            self.ref_fuse_numeric_suffixes();
            after_each("fuse_numeric_suffixes", self);
        }
        if super::era_at_least(&self.era, "2.3") {
            self.ref_fuse_safe_nav();
            after_each("fuse_safe_nav", self);
        }
        if super::era_at_least(&self.era, "2.6") {
            self.mark_endless_ranges();
            after_each("mark_endless_ranges", self);
        }
        if super::era_at_least(&self.era, "2.7") {
            self.mark_numbered_block_params();
            after_each("mark_numbered_block_params", self);
        }
        self.mark_statement_ends();
        after_each("mark_statement_ends", self);
    }

    pub(crate) fn ref_fuse_range_ops(&mut self) {
        // First pass: combine pairs of adjacent Dots into `..`.
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let both_dots =
                    a.type_ == TokenType::Dot && b.type_ == TokenType::Dot;
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                both_dots && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "..".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
        // Second pass: `..` (now a Name token from the first pass)
        // followed by a Dot fuses into `...`.
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_dotdot = a.type_ == TokenType::Name && a.value == "..";
                let b_is_dot = b.type_ == TokenType::Dot;
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_dotdot && b_is_dot && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "...".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    pub(crate) fn ref_fuse_radix_integers(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_zero = a.type_ == TokenType::Number && a.value == "0";
                let b_is_radix_body = b.type_ == TokenType::Name
                    && is_radix_integer_body(&b.value);
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_zero && b_is_radix_body && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "0{}",
                    self.tokens[i + 1].value
                );
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    pub(crate) fn ref_fuse_float_literals(&mut self) {
        // Step 1: `Int Dot Int` (no whitespace between any of the
        // three) fuses to a single `Number("X.Y")` token.
        let mut i = 0;
        while i + 2 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let c = &self.tokens[i + 2];
                let a_is_int = a.type_ == TokenType::Number
                    && is_integer_lexeme(&a.value);
                let b_is_dot = b.type_ == TokenType::Dot;
                let c_is_int = c.type_ == TokenType::Number
                    && is_integer_lexeme(&c.value);
                let same_line = a.line == b.line && b.line == c.line;
                let no_ws_b = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                let no_ws_c = !self
                    .whitespace_before_token
                    .get(i + 2)
                    .copied()
                    .unwrap_or(false);
                a_is_int && b_is_dot && c_is_int && same_line && no_ws_b && no_ws_c
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "{}.{}",
                    self.tokens[i].value,
                    self.tokens[i + 2].value
                );
                self.tokens.remove(i + 2);
                self.whitespace_before_token.remove(i + 2);
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
                // Don't advance i — the scientific-notation suffix
                // (if any) may follow.
            } else {
                i += 1;
            }
        }
        // Step 2: `Number Name(e<digits>)` (or `Int Name(e<digits>)`)
        // fuses the trailing scientific notation when the name's
        // lexeme is exactly `[eE]<digit_or_underscore>+`.  No sign
        // case (handled in step 3).
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_num = a.type_ == TokenType::Number;
                let b_is_unsigned_exp = b.type_ == TokenType::Name
                    && is_unsigned_exponent_lexeme(&b.value);
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_num && b_is_unsigned_exp && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "{}{}",
                    self.tokens[i].value,
                    self.tokens[i + 1].value
                );
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
                // Don't advance — the signed-exponent step below
                // might still match (unlikely but harmless).
            } else {
                i += 1;
            }
        }
        // Step 3: `Number Name("e"|"E") (Plus|Minus) Int` fuses the
        // trailing signed exponent.  Four tokens collapse to one.
        let mut i = 0;
        while i + 3 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let c = &self.tokens[i + 2];
                let d = &self.tokens[i + 3];
                let a_is_num = a.type_ == TokenType::Number;
                let b_is_e = b.type_ == TokenType::Name
                    && (b.value == "e" || b.value == "E");
                let c_is_sign = (c.type_ == TokenType::Plus && c.value == "+")
                    || (c.type_ == TokenType::Minus && c.value == "-");
                let d_is_int = d.type_ == TokenType::Number
                    && is_integer_lexeme(&d.value);
                let same_line =
                    a.line == b.line && b.line == c.line && c.line == d.line;
                let no_ws_b = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                let no_ws_c = !self
                    .whitespace_before_token
                    .get(i + 2)
                    .copied()
                    .unwrap_or(false);
                let no_ws_d = !self
                    .whitespace_before_token
                    .get(i + 3)
                    .copied()
                    .unwrap_or(false);
                a_is_num
                    && b_is_e
                    && c_is_sign
                    && d_is_int
                    && same_line
                    && no_ws_b
                    && no_ws_c
                    && no_ws_d
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!(
                    "{}{}{}{}",
                    self.tokens[i].value,
                    self.tokens[i + 1].value,
                    self.tokens[i + 2].value,
                    self.tokens[i + 3].value
                );
                for _ in 0..3 {
                    self.tokens.remove(i + 1);
                    self.whitespace_before_token.remove(i + 1);
                }
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            } else {
                i += 1;
            }
        }
    }

    pub(crate) fn ref_fuse_right_shifts(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            // Left token must be `Name(">")`.
            let is_gt = {
                let a = &self.tokens[i];
                a.type_ == TokenType::Name && a.value == ">"
            };
            if !is_gt {
                i += 1;
                continue;
            }
            // Right token must be `Name(">")` or `Name(">=")`, on the
            // same line, with no whitespace gap.
            let (new_value_opt, same_line, no_ws) = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                let nv: Option<&'static str> = if b.type_ == TokenType::Name {
                    match b.value.as_str() {
                        ">" => Some(">>"),
                        ">=" => Some(">>="),
                        _ => None,
                    }
                } else {
                    None
                };
                (nv, same_line, no_ws)
            };
            if let Some(nv) = new_value_opt {
                if same_line && no_ws {
                    let span_line = self.tokens[i].line;
                    let span_col = self.tokens[i].column;
                    self.tokens.remove(i + 1);
                    self.whitespace_before_token.remove(i + 1);
                    self.tokens[i] = Token {
                        type_: TokenType::Name,
                        value: nv.to_string(),
                        line: span_line,
                        column: span_col,
                        type_name: None,
                        flags: None, cv: None,
                    };
                    // Don't advance — three-`>` sequences (e.g. `a>>>b`,
                    // which is illegal in Ruby anyway) get correctly
                    // partial-folded on the next iteration.
                    continue;
                }
            }
            i += 1;
        }
    }

    pub(crate) fn ref_fuse_compound_assigns(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let (left_lexeme, is_arith): (&str, bool) = {
                let a = &self.tokens[i];
                match a.type_ {
                    TokenType::Plus => ("+", true),
                    TokenType::Minus => ("-", true),
                    TokenType::Star => ("*", true),
                    TokenType::Slash => ("/", true),
                    // `||`, `&&`, `%`, `**`, `<<`, `&`, `|`, `^`
                    // all come through as Name (catch-all in
                    // classify_op_token).  Filter by value so we
                    // don't accidentally fuse `foo =` into something.
                    TokenType::Name
                        if matches!(
                            a.value.as_str(),
                            "||" | "&&" | "%" | "**" | "<<" | "&" | "|" | "^"
                        ) =>
                    {
                        let lex: &'static str = match a.value.as_str() {
                            "||" => "||",
                            "&&" => "&&",
                            "%" => "%",
                            "**" => "**",
                            "<<" => "<<",
                            "&" => "&",
                            "|" => "|",
                            "^" => "^",
                            _ => unreachable!(),
                        };
                        (lex, false)
                    }
                    _ => {
                        i += 1;
                        continue;
                    }
                }
            };
            let merge = {
                let b = &self.tokens[i + 1];
                let b_is_eq = b.type_ == TokenType::Equals;
                let same_line = self.tokens[i].line == b.line;
                // No whitespace gap between op and `=`.
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                b_is_eq && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!("{left_lexeme}=");
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
                // Don't advance — chained compounds are illegal but
                // the next iteration's guard will handle it cleanly.
            } else {
                // Reference `is_arith` once so the compiler doesn't
                // warn — the variable is documentary for now.
                let _ = is_arith;
                i += 1;
            }
        }
    }

    pub(crate) fn ref_fuse_lambda_arrow(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_minus =
                    a.type_ == TokenType::Minus && a.value == "-";
                let b_is_gt = b.value == ">"
                    && matches!(b.type_, TokenType::Name);
                let same_line = a.line == b.line;
                // Phase 4c: require that no whitespace was consumed
                // between the two tokens — the parallel array tracks
                // this explicitly so we don't have to interpret
                // peek-state column quirks.
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_minus && b_is_gt && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "->".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    pub(crate) fn ref_fuse_numeric_suffixes(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_num = a.type_ == TokenType::Number;
                let b_is_suffix = b.type_ == TokenType::Name
                    && (b.value == "r" || b.value == "i");
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_num && b_is_suffix && same_line && no_ws
            };
            if merge {
                let suffix = self.tokens[i + 1].value.clone();
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                let new_value = format!("{}{}", self.tokens[i].value, suffix);
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Number,
                    value: new_value,
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    pub(crate) fn ref_fuse_safe_nav(&mut self) {
        let mut i = 0;
        while i + 1 < self.tokens.len() {
            let merge = {
                let a = &self.tokens[i];
                let b = &self.tokens[i + 1];
                let a_is_amp = a.value == "&" && matches!(a.type_, TokenType::Name);
                let b_is_dot = b.type_ == TokenType::Dot && b.value == ".";
                let same_line = a.line == b.line;
                let no_ws = !self
                    .whitespace_before_token
                    .get(i + 1)
                    .copied()
                    .unwrap_or(false);
                a_is_amp && b_is_dot && same_line && no_ws
            };
            if merge {
                let span_line = self.tokens[i].line;
                let span_col = self.tokens[i].column;
                self.tokens.remove(i + 1);
                self.whitespace_before_token.remove(i + 1);
                self.tokens[i] = Token {
                    type_: TokenType::Name,
                    value: "&.".to_string(),
                    line: span_line,
                    column: span_col,
                    type_name: None,
                    flags: None, cv: None,
                };
            }
            i += 1;
        }
    }

    /// The pre-#18814 `finalize_heredoc`, verbatim: it removed the tag
    /// `Name` at once with `Vec::remove`, shifting the tail once per
    /// heredoc.  Used when `reference_heredocs` is set.
    pub(crate) fn ref_finalize_heredoc(&mut self, h: PendingHeredoc) {
        let body = match h.variant {
            HeredocVariant::Plain | HeredocVariant::DashIndent => h.body.clone(),
            HeredocVariant::TildeIndent => strip_common_leading_whitespace(&h.body),
        };
        let prefix = match h.variant {
            HeredocVariant::Plain => "<<",
            HeredocVariant::DashIndent => "<<-",
            HeredocVariant::TildeIndent => "<<~",
        };
        let value = format!("{prefix}{}\n{body}{}", h.tag, h.tag);
        // Remove the tag Name first (higher index) so op_idx stays
        // valid.  Both indices were captured at emit time and the
        // tokens between them have not moved (body capture inserts
        // no tokens).
        if h.tag_idx < self.tokens.len() {
            self.tokens.remove(h.tag_idx);
        }
        if h.op_idx < self.tokens.len() {
            let line = self.tokens[h.op_idx].line;
            let column = self.tokens[h.op_idx].column;
            self.tokens[h.op_idx] = Token {
                type_: TokenType::String,
                value,
                line,
                column,
                type_name: None,
                flags: None, cv: None,
            };
        }
    }
}
