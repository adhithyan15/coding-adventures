//! MacroNib's assembler-style directive syntax.

use coding_adventures_macronib_lexer::try_tokenize_macronib;
use coding_adventures_source_preprocessor::macros::MacroTable;
use coding_adventures_source_preprocessor::{
    Bounds, Dialect, Directive, FileId, IncludeRequest, PpError,
};
use lexer::token::{Token, TokenType};

/// Stateless MacroNib adapter for the shared preprocessor.
#[derive(Debug, Default, Clone, Copy)]
pub struct MacroNibDialect;

pub(crate) fn strip_eof(tokens: &mut Vec<Token>) {
    while tokens
        .last()
        .is_some_and(|token| token.type_ == TokenType::Eof)
    {
        tokens.pop();
    }
}

fn without_eof(mut tokens: &[Token]) -> &[Token] {
    while tokens
        .last()
        .is_some_and(|token| token.type_ == TokenType::Eof)
    {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

fn identifier(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn operand_error(directive: &str, detail: &str) -> PpError {
    PpError::new(format!("`{directive}` {detail}"))
}

impl Dialect for MacroNibDialect {
    fn classify(&self, line: &[Token]) -> Option<Result<Directive, PpError>> {
        let line = without_eof(line);
        let (head, rest) = line.split_first()?;
        let spelling = head.value.as_str();
        if !matches!(
            spelling,
            ".include" | ".set" | ".ifdef" | ".else" | ".endif"
        ) {
            return None;
        }
        if let Some(next) = rest.first() {
            if next.line == head.line
                && next.column == head.column + spelling.len()
                && next
                    .value
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
            {
                return Some(Err(PpError::new(format!(
                    "unknown MacroNib directive {}",
                    PpError::quote(
                        &format!("{spelling}{}", next.value),
                        Bounds::default().diagnostic_quote_bytes,
                    )
                ))));
            }
        }
        Some(match spelling {
            ".include" => {
                if rest.len() != 1 {
                    Err(operand_error(spelling, "requires one quoted path"))
                } else {
                    let path = &rest[0].value;
                    if path.len() < 2 || !path.starts_with('"') || !path.ends_with('"') {
                        Err(operand_error(spelling, "requires one quoted path"))
                    } else {
                        Ok(Directive::Include(IncludeRequest {
                            spelling: path[1..path.len() - 1].to_string(),
                            from: None,
                            system: false,
                        }))
                    }
                }
            }
            ".set" => {
                if rest.first().is_none_or(|name| !identifier(&name.value)) {
                    Err(operand_error(spelling, "requires an identifier name"))
                } else {
                    Ok(Directive::Define {
                        name: rest[0].value.clone(),
                        params: None,
                        body: rest[1..].to_vec(),
                    })
                }
            }
            ".ifdef" => {
                if rest.len() != 1 || !identifier(&rest[0].value) {
                    Err(operand_error(spelling, "requires exactly one identifier"))
                } else {
                    Ok(Directive::If(rest.to_vec()))
                }
            }
            ".else" => {
                if rest.is_empty() {
                    Ok(Directive::Else)
                } else {
                    Err(operand_error(spelling, "takes no operands"))
                }
            }
            ".endif" => {
                if rest.is_empty() {
                    Ok(Directive::EndIf)
                } else {
                    Err(operand_error(spelling, "takes no operands"))
                }
            }
            _ => unreachable!(),
        })
    }

    fn prepare_condition(
        &self,
        mut tokens: Vec<Token>,
        macros: &MacroTable,
    ) -> Result<Vec<Token>, PpError> {
        if tokens.len() != 1 || !identifier(&tokens[0].value) {
            return Err(PpError::new("`.ifdef` requires exactly one identifier"));
        }
        let value = if macros.is_defined(&tokens[0].value) {
            "1"
        } else {
            "0"
        };
        tokens[0].value = value.to_string();
        tokens[0].type_name = Some("INT_LIT".to_string());
        Ok(tokens)
    }

    fn eval_condition(&self, tokens: &[Token]) -> Result<bool, PpError> {
        match tokens {
            [token] if token.value == "1" => Ok(true),
            [token] if token.value == "0" => Ok(false),
            _ => Err(PpError::new(
                "`.ifdef` condition must resolve to zero or one",
            )),
        }
    }

    fn lex(&self, text: &str, _file: FileId) -> Result<Vec<Token>, PpError> {
        let mut tokens = try_tokenize_macronib(text).map_err(PpError::new)?;
        strip_eof(&mut tokens);
        Ok(tokens)
    }
}
