//! C's directive syntax for the shared PREP01 engine.
//!
//! This is the classification handoff. The C frontend will compose it with
//! `preprocess` once conditional expressions and C's `#`/`##` macro operators
//! are implemented. Until then the existing source compiler is unchanged.

use coding_adventures_c_lexer::try_tokenize_c;
use coding_adventures_source_preprocessor::{
    macros::MacroTable, Bounds, Dialect, Directive, FileId, IncludeRequest, PpError,
};
use lexer::token::{Token, TokenType};
use std::cell::Cell;
use std::collections::HashSet;

/// C directive adapter with a finite pre-lex byte budget.
#[derive(Debug)]
pub struct CDialect {
    max_file_bytes: u64,
    max_total_bytes: u64,
    lexed_bytes: Cell<u64>,
}

impl CDialect {
    /// Construct a dialect whose bounds can only tighten the engine defaults.
    #[must_use]
    pub fn new(bounds: Bounds) -> Self {
        let bounds = bounds.tighten(Bounds::default());
        let max_total_bytes = bounds
            .total_source_bytes
            .min(bounds.tokens_produced.saturating_sub(1));
        Self {
            max_file_bytes: bounds.bytes_per_file.min(max_total_bytes),
            max_total_bytes,
            lexed_bytes: Cell::new(0),
        }
    }

    /// Charge source bytes before `GrammarLexer` allocates a token vector.
    pub fn reserve_source(&self, source: &str) -> Result<(), PpError> {
        let bytes = u64::try_from(source.len()).unwrap_or(u64::MAX);
        if bytes > self.max_file_bytes {
            return Err(PpError::new("C source exceeds the pre-lex file budget"));
        }
        let total = self.lexed_bytes.get().saturating_add(bytes);
        if total > self.max_total_bytes {
            return Err(PpError::new(
                "C source exceeds the aggregate pre-lex budget",
            ));
        }
        self.lexed_bytes.set(total);
        Ok(())
    }
}

impl Default for CDialect {
    fn default() -> Self {
        Self::new(Bounds::default())
    }
}

fn without_eof(mut line: &[Token]) -> &[Token] {
    while line
        .last()
        .is_some_and(|token| token.type_ == TokenType::Eof)
    {
        line = &line[..line.len() - 1];
    }
    line
}

fn identifier(value: &str) -> bool {
    value.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn directive_error(name: &str, detail: &str) -> PpError {
    PpError::new(format!("`#{name}` {detail}"))
}

fn condition_operand(token: &Token) -> Result<i64, PpError> {
    if token.effective_type_name() == "INT_LIT" {
        if token.value.len() > 1 && token.value.starts_with('0') {
            return Err(PpError::new(
                "C octal or hexadecimal condition literal is outside the decimal handoff",
            ));
        }
        return token
            .value
            .parse::<i64>()
            .map_err(|_| PpError::new("C condition integer is out of range"));
    }
    if identifier(&token.value) {
        // An identifier left after macro expansion reads as zero in #if.
        return Ok(0);
    }
    Err(PpError::new(
        "C conditional operand is not supported by this handoff yet",
    ))
}

fn condition_clause(tokens: &[Token]) -> Result<bool, PpError> {
    match tokens {
        [token] => Ok(condition_operand(token)? != 0),
        [not, token] if not.value == "!" => Ok(condition_operand(token)? == 0),
        [left, op, right] => {
            let left = condition_operand(left)?;
            let right = condition_operand(right)?;
            match op.value.as_str() {
                "==" => Ok(left == right),
                "!=" => Ok(left != right),
                "<" => Ok(left < right),
                "<=" => Ok(left <= right),
                ">" => Ok(left > right),
                ">=" => Ok(left >= right),
                _ => Err(PpError::new(
                    "C conditional expression is not supported by this handoff yet",
                )),
            }
        }
        _ => Err(PpError::new(
            "C conditional expression is not supported by this handoff yet",
        )),
    }
}

fn include(rest: &[Token]) -> Result<Directive, PpError> {
    let [path] = rest else {
        return Err(directive_error("include", "requires one quoted local path"));
    };
    let spelling = &path.value;
    if spelling.len() < 2 || !spelling.starts_with('"') || !spelling.ends_with('"') {
        return Err(directive_error("include", "requires one quoted local path"));
    }
    Ok(Directive::Include(IncludeRequest {
        spelling: spelling[1..spelling.len() - 1].to_string(),
        from: None,
        system: false,
    }))
}

fn define(rest: &[Token]) -> Result<Directive, PpError> {
    let Some((name, remaining)) = rest.split_first() else {
        return Err(directive_error("define", "requires a macro name"));
    };
    if !identifier(&name.value) {
        return Err(directive_error("define", "requires an identifier name"));
    }
    let touching_paren = remaining.first().is_some_and(|next| {
        next.value == "("
            && next.line == name.line
            && name
                .column
                .checked_add(name.value.len())
                .is_some_and(|column| next.column == column)
    });
    if !touching_paren {
        reject_unsupported_macro_operators(remaining)?;
        return Ok(Directive::Define {
            name: name.value.clone(),
            params: None,
            body: remaining.to_vec(),
        });
    }

    // C's adjacency rule makes `F(x)` function-like and `F (x)` object-like.
    // Parse the parameter list iteratively; its length is bounded by the
    // pre-lex source limit and no native recursion is needed.
    let mut params = Vec::new();
    let mut seen = HashSet::new();
    let mut cursor = 1;
    if remaining
        .get(cursor)
        .is_some_and(|token| token.value == ")")
    {
        cursor += 1;
    } else {
        loop {
            let Some(param) = remaining.get(cursor) else {
                return Err(directive_error("define", "has an unclosed parameter list"));
            };
            if !identifier(&param.value) || !seen.insert(param.value.as_str()) {
                return Err(directive_error(
                    "define",
                    "has an invalid or duplicate parameter",
                ));
            }
            params.push(param.value.clone());
            cursor += 1;
            match remaining.get(cursor).map(|token| token.value.as_str()) {
                Some(")") => {
                    cursor += 1;
                    break;
                }
                Some(",") => {
                    cursor += 1;
                }
                _ => {
                    return Err(directive_error(
                        "define",
                        "requires comma-separated parameters",
                    ))
                }
            }
        }
    }
    reject_unsupported_macro_operators(&remaining[cursor..])?;
    Ok(Directive::Define {
        name: name.value.clone(),
        params: Some(params),
        body: remaining[cursor..].to_vec(),
    })
}

fn reject_unsupported_macro_operators(body: &[Token]) -> Result<(), PpError> {
    if body
        .iter()
        .any(|token| matches!(token.value.as_str(), "#" | "##"))
    {
        return Err(directive_error(
            "define",
            "stringize and paste operators are not supported by this handoff yet",
        ));
    }
    Ok(())
}

impl Dialect for CDialect {
    fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>> {
        let line = without_eof(line);
        let (marker, rest) = line.split_first()?;
        if marker.value != "#" {
            return None;
        }
        let Some((name, operands)) = rest.split_first() else {
            return Some(Err(PpError::new("C directive is missing its name")));
        };
        let result = match name.value.as_str() {
            "include" => include(operands),
            "define" => define(operands),
            "if" if !operands.is_empty() => Ok(Directive::If(operands.to_vec())),
            "if" => Err(directive_error("if", "requires an expression")),
            "ifdef" | "ifndef" => {
                if operands.len() != 1 || !identifier(&operands[0].value) {
                    Err(directive_error(
                        &name.value,
                        "requires exactly one identifier",
                    ))
                } else {
                    let mut token = operands[0].clone();
                    token.type_name = Some(
                        if name.value == "ifdef" {
                            "PP_IFDEF"
                        } else {
                            "PP_IFNDEF"
                        }
                        .to_string(),
                    );
                    Ok(Directive::If(vec![token]))
                }
            }
            "else" if operands.is_empty() => Ok(Directive::Else),
            "endif" if operands.is_empty() => Ok(Directive::EndIf),
            "else" | "endif" => Err(directive_error(&name.value, "takes no operands")),
            _ => Err(PpError::new(format!(
                "unknown C directive {}",
                PpError::quote(&name.value, Bounds::default().diagnostic_quote_bytes)
            ))),
        };
        Some(result)
    }

    fn prepare_condition(
        &self,
        mut tokens: Vec<Token>,
        macros: &MacroTable,
    ) -> Result<Vec<Token>, PpError> {
        if let [token] = tokens.as_mut_slice() {
            match token.effective_type_name() {
                "PP_IFDEF" | "PP_IFNDEF" => {
                    let defined = macros.is_defined(&token.value);
                    let true_branch = if token.effective_type_name() == "PP_IFDEF" {
                        defined
                    } else {
                        !defined
                    };
                    token.value = if true_branch { "1" } else { "0" }.to_string();
                    token.type_name = Some("INT_LIT".to_string());
                }
                _ => {}
            }
        }
        // Resolve C's exceptional operator before the generic engine expands
        // macros. In `defined(NAME)`, NAME is inspected as written, even when
        // it is itself a macro. The rewrite only shrinks token count and text.
        let mut prepared = Vec::with_capacity(tokens.len());
        let mut input = tokens.into_iter();
        while let Some(mut token) = input.next() {
            if token.value != "defined" {
                prepared.push(token);
                continue;
            }
            let Some(next) = input.next() else {
                return Err(PpError::new("C `defined` requires an identifier"));
            };
            let name = if next.value == "(" {
                let Some(name) = input.next() else {
                    return Err(PpError::new("C `defined` requires an identifier"));
                };
                if input.next().is_none_or(|close| close.value != ")") {
                    return Err(PpError::new("C `defined` requires a closing `)`"));
                }
                name
            } else {
                next
            };
            if !identifier(&name.value) {
                return Err(PpError::new("C `defined` requires an identifier"));
            }
            token.value = if macros.is_defined(&name.value) {
                "1"
            } else {
                "0"
            }
            .to_string();
            token.type_name = Some("INT_LIT".to_string());
            prepared.push(token);
        }
        Ok(prepared)
    }

    fn eval_condition(&self, tokens: &[Token]) -> Result<bool, PpError> {
        let mut any = false;
        for disjunct in tokens.split(|token| token.value == "||") {
            if disjunct.is_empty() {
                return Err(PpError::new(
                    "C conditional expression has an empty OR operand",
                ));
            }
            let mut all = true;
            for conjunct in disjunct.split(|token| token.value == "&&") {
                if conjunct.is_empty() {
                    return Err(PpError::new(
                        "C conditional expression has an empty AND operand",
                    ));
                }
                // Validate every operand even when a previous one determines
                // the result: this partial grammar must reject unsupported syntax.
                all &= condition_clause(conjunct)?;
            }
            any |= all;
        }
        Ok(any)
    }

    fn lex(&self, text: &str, _file: FileId) -> Result<Vec<Token>, PpError> {
        self.reserve_source(text)?;
        let mut tokens = try_tokenize_c(text).map_err(PpError::new)?;
        while tokens
            .last()
            .is_some_and(|token| token.type_ == TokenType::Eof)
        {
            tokens.pop();
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_source_preprocessor::{preprocess, MemoryFs};

    fn directive(source: &str) -> Directive {
        let tokens = try_tokenize_c(source).unwrap();
        CDialect::default().classify(&tokens).unwrap().unwrap()
    }

    #[test]
    fn quoted_include_and_function_adjacency_are_classified() {
        match directive("#include \"local.h\"") {
            Directive::Include(request) => {
                assert_eq!(request.spelling, "local.h");
                assert!(!request.system);
            }
            other => panic!("unexpected directive: {other:?}"),
        }
        match directive("#define F(x,y) x + y") {
            Directive::Define { params, body, .. } => {
                assert_eq!(params, Some(vec!["x".to_string(), "y".to_string()]));
                assert_eq!(body[1].value, "+");
            }
            other => panic!("unexpected directive: {other:?}"),
        }
        match directive("#define F (x)") {
            Directive::Define { params, body, .. } => {
                assert_eq!(params, None);
                assert_eq!(body[0].value, "(");
            }
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn direct_tokens_with_overflowing_column_do_not_panic() {
        let mut tokens = try_tokenize_c("#define F(x) x").unwrap();
        let name = tokens.iter_mut().find(|token| token.value == "F").unwrap();
        name.column = usize::MAX;
        match CDialect::default().classify(&tokens).unwrap().unwrap() {
            Directive::Define { params, .. } => assert!(params.is_none()),
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn malformed_directives_fail_without_panicking() {
        for source in [
            "#",
            "#include x",
            "#include <stdio.h>",
            "#define F(x,x) x",
            "#define F(x,)",
            "#ifdef",
            "#else extra",
            "#unknown",
        ] {
            let tokens = try_tokenize_c(source).unwrap();
            assert!(
                CDialect::default().classify(&tokens).unwrap().is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn unsupported_stringize_and_paste_definitions_fail() {
        for source in [
            "#define STR(x) #x\nint x;\n",
            "#define JOIN(a,b) a ## b\nint x;\n",
        ] {
            let mut fs = MemoryFs::new();
            let file = fs.insert("<main>", source);
            let dialect = CDialect::default();
            let tokens = dialect.lex(source, file).unwrap();
            assert!(
                preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn object_macro_ifdef_and_local_include_run_through_shared_engine() {
        let source =
            "#define ANSWER 7\n#ifdef ANSWER\n#include \"part.h\"\n#else\nint x = 0;\n#endif\n";
        let mut fs = MemoryFs::new();
        fs.insert("part.h", "int x = ANSWER;\n");
        let file = fs.insert("<main>", source);
        let dialect = CDialect::default();
        let tokens = dialect.lex(source, file).unwrap();
        let result = preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).unwrap();
        let values: Vec<_> = result
            .tokens
            .iter()
            .map(|token| token.value.as_str())
            .collect();
        assert_eq!(values, ["int", "x", "=", "7", ";"]);
        assert_eq!(result.map.len(), result.tokens.len());
    }

    #[test]
    fn source_limit_is_checked_before_token_allocation() {
        let dialect = CDialect::new(Bounds {
            bytes_per_file: 3,
            ..Bounds::default()
        });
        let mut fs = MemoryFs::new();
        let file = fs.insert("<main>", "abcd");
        assert!(dialect.lex("abcd", file).is_err());
    }

    #[test]
    fn defined_operands_are_not_macro_expanded() {
        for condition in ["defined ANSWER", "defined(ANSWER)"] {
            let source = format!(
                "#define ANSWER 7\n#if {condition}\nint x = ANSWER;\n#else\nint x = 0;\n#endif\n"
            );
            let mut fs = MemoryFs::new();
            let file = fs.insert("<main>", &source);
            let dialect = CDialect::default();
            let tokens = dialect.lex(&source, file).unwrap();
            let result = preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).unwrap();
            let values: Vec<_> = result
                .tokens
                .iter()
                .map(|token| token.value.as_str())
                .collect();
            assert_eq!(values, ["int", "x", "=", "7", ";"], "{condition}");
        }
    }

    #[test]
    fn undefined_and_malformed_defined_conditions_are_handled() {
        let source = "#if defined(MISSING)\nint x = 0;\n#else\nint x = 9;\n#endif\n";
        let mut fs = MemoryFs::new();
        let file = fs.insert("<main>", source);
        let dialect = CDialect::default();
        let tokens = dialect.lex(source, file).unwrap();
        let result = preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).unwrap();
        let values: Vec<_> = result
            .tokens
            .iter()
            .map(|token| token.value.as_str())
            .collect();
        assert_eq!(values, ["int", "x", "=", "9", ";"]);

        for expression in ["defined", "defined()", "defined(1)", "defined(X"] {
            let source = format!("#if {expression}\nint x;\n#endif\n");
            let mut fs = MemoryFs::new();
            let file = fs.insert("<main>", &source);
            let dialect = CDialect::default();
            let tokens = dialect.lex(&source, file).unwrap();
            assert!(
                preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).is_err(),
                "{expression}"
            );
        }
    }

    #[test]
    fn expanded_macro_values_support_bounded_comparisons() {
        for (condition, expected) in [
            ("LED_PORT == 7", "1"),
            ("LED_PORT != 7", "0"),
            ("MISSING == 0", "1"),
            ("LED_PORT > 2", "1"),
            ("LED_PORT >= 7", "1"),
            ("LED_PORT < 2", "0"),
            ("LED_PORT <= 7", "1"),
        ] {
            let source = format!(
                "#define LED_PORT 7\n#if {condition}\nint x = 1;\n#else\nint x = 0;\n#endif\n"
            );
            let mut fs = MemoryFs::new();
            let file = fs.insert("<main>", &source);
            let dialect = CDialect::default();
            let tokens = dialect.lex(&source, file).unwrap();
            let result = preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).unwrap();
            let values: Vec<_> = result
                .tokens
                .iter()
                .map(|token| token.value.as_str())
                .collect();
            assert_eq!(values, ["int", "x", "=", expected, ";"], "{condition}");
        }
    }

    #[test]
    fn octal_condition_is_not_evaluated_as_decimal() {
        let source = "#if 010 == 10\nint x = 1;\n#endif\n";
        let mut fs = MemoryFs::new();
        let file = fs.insert("<main>", source);
        let dialect = CDialect::default();
        let tokens = dialect.lex(source, file).unwrap();
        assert!(preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).is_err());
    }

    #[test]
    fn logical_conditions_use_and_before_or() {
        for (condition, expected) in [
            ("1 || 0 && 0", "1"),
            ("0 || 1 && 0", "0"),
            ("!0 && 7 > 2", "1"),
            ("!1 || 2 == 2", "1"),
            ("defined(ANSWER) && ANSWER == 7", "1"),
        ] {
            let source = format!(
                "#define ANSWER 7\n#if {condition}\nint x = 1;\n#else\nint x = 0;\n#endif\n"
            );
            let mut fs = MemoryFs::new();
            let file = fs.insert("<main>", &source);
            let dialect = CDialect::default();
            let tokens = dialect.lex(&source, file).unwrap();
            let result = preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).unwrap();
            let values: Vec<_> = result
                .tokens
                .iter()
                .map(|token| token.value.as_str())
                .collect();
            assert_eq!(values, ["int", "x", "=", expected, ";"], "{condition}");
        }
    }

    #[test]
    fn unsupported_or_malformed_logical_conditions_fail() {
        for condition in ["1 &&", "|| 1", "1 || || 0", "(1)", "!1 == 0", "1 + 2"] {
            let source = format!("#if {condition}\nint x = 1;\n#endif\n");
            let mut fs = MemoryFs::new();
            let file = fs.insert("<main>", &source);
            let dialect = CDialect::default();
            let tokens = dialect.lex(&source, file).unwrap();
            assert!(
                preprocess(tokens, file, &dialect, &mut fs, Bounds::default()).is_err(),
                "{condition}"
            );
        }
    }
}
