use crate::{
    parse_netlist, AnalysisExecutionError, AnalysisExecutionResult, AnalysisKind,
    NetlistParseError, ParsedNetlist,
};

pub const BERKELEY_SPICE_GRAMMAR_NAME: &str = "berkeley-spice-logical-card";
pub const BERKELEY_SPICE_GRAMMAR_VERSION: u32 = 1;
pub const BERKELEY_SPICE_TOKEN_GRAMMAR: &str =
    include_str!("../../../../grammars/spice/berkeley.tokens");
pub const BERKELEY_SPICE_PARSER_GRAMMAR: &str =
    include_str!("../../../../grammars/spice/berkeley.grammar");

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl SourceSpan {
    fn point(line: usize, column: usize) -> Self {
        Self {
            start_line: line,
            start_column: column,
            end_line: line,
            end_column: column,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct SourcePosition {
    line: usize,
    column: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BerkeleyDiagnosticSeverity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleySyntaxDiagnostic {
    pub code: String,
    pub severity: BerkeleyDiagnosticSeverity,
    pub message: String,
    pub span: Option<SourceSpan>,
}

impl BerkeleySyntaxDiagnostic {
    fn new(
        code: &str,
        severity: BerkeleyDiagnosticSeverity,
        message: impl Into<String>,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            code: code.to_string(),
            severity,
            message: message.into(),
            span,
        }
    }

    fn error(code: &str, message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self::new(code, BerkeleyDiagnosticSeverity::Error, message, span)
    }

    pub fn is_error(&self) -> bool {
        self.severity == BerkeleyDiagnosticSeverity::Error
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BerkeleyCardKind {
    Element,
    Model,
    SubcktStart,
    SubcktEnd,
    End,
    Param,
    Func,
    Options,
    Condition,
    Analysis,
    Output,
    Source,
    ControlStart,
    ControlEnd,
    UnknownDirective,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleySyntaxToken {
    pub kind: String,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyLogicalCard {
    pub kind: BerkeleyCardKind,
    pub head: String,
    pub text: String,
    pub span: SourceSpan,
    pub physical_lines: Vec<usize>,
    pub tokens: Vec<BerkeleySyntaxToken>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BerkeleyGrammarMetadata {
    pub name: &'static str,
    pub version: u32,
    pub token_grammar: &'static str,
    pub parser_grammar: &'static str,
}

impl BerkeleyGrammarMetadata {
    pub fn current() -> Self {
        Self {
            name: BERKELEY_SPICE_GRAMMAR_NAME,
            version: BERKELEY_SPICE_GRAMMAR_VERSION,
            token_grammar: BERKELEY_SPICE_TOKEN_GRAMMAR,
            parser_grammar: BERKELEY_SPICE_PARSER_GRAMMAR,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAnalysisInventoryEntry {
    pub index: usize,
    pub directive: String,
    pub analysis: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleySyntaxDeck {
    pub grammar: BerkeleyGrammarMetadata,
    pub title: Option<String>,
    pub cards: Vec<BerkeleyLogicalCard>,
    pub diagnostics: Vec<BerkeleySyntaxDiagnostic>,
}

impl BerkeleySyntaxDeck {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(BerkeleySyntaxDiagnostic::is_error)
    }

    pub fn analysis_inventory(&self) -> Vec<BerkeleyAnalysisInventoryEntry> {
        self.cards
            .iter()
            .enumerate()
            .filter_map(|(index, card)| {
                if card.kind != BerkeleyCardKind::Analysis {
                    return None;
                }
                let analysis = card.head.trim_start_matches('.').to_ascii_lowercase();
                Some(BerkeleyAnalysisInventoryEntry {
                    index,
                    directive: card.head.clone(),
                    analysis,
                    span: card.span,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BerkeleyAppDeck {
    pub syntax: BerkeleySyntaxDeck,
    pub parsed: Option<ParsedNetlist>,
    pub diagnostics: Vec<BerkeleySyntaxDiagnostic>,
}

impl BerkeleyAppDeck {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(BerkeleySyntaxDiagnostic::is_error)
    }

    pub fn analysis_inventory(&self) -> Vec<BerkeleyAnalysisInventoryEntry> {
        self.syntax.analysis_inventory()
    }

    pub fn run_source_order(&self) -> Result<Vec<AnalysisExecutionResult>, AnalysisExecutionError> {
        let parsed = self.parsed.as_ref().ok_or_else(|| {
            AnalysisExecutionError::Netlist(NetlistParseError::new(self.blocking_message()))
        })?;
        parsed.run_analysis_plan()
    }

    pub fn run_selected_analysis(
        &self,
        syntax_card_index: usize,
    ) -> Result<Option<AnalysisExecutionResult>, AnalysisExecutionError> {
        let Some(entry) = self
            .analysis_inventory()
            .into_iter()
            .find(|entry| entry.index == syntax_card_index)
        else {
            return Ok(None);
        };
        if runnable_analysis_kind(&entry.directive).is_none() {
            return Ok(None);
        }
        let result_ordinal = self
            .syntax
            .cards
            .iter()
            .take(entry.index + 1)
            .filter(|card| runnable_analysis_kind(&card.head).is_some())
            .count()
            - 1;
        Ok(self.run_source_order()?.into_iter().nth(result_ordinal))
    }

    fn blocking_message(&self) -> String {
        self.diagnostics
            .iter()
            .find(|diagnostic| diagnostic.is_error())
            .map(|diagnostic| format!("Berkeley SPICE app deck: {}", diagnostic.message))
            .unwrap_or_else(|| {
                "Berkeley SPICE app deck did not produce a parsed netlist".to_string()
            })
    }
}

fn runnable_analysis_kind(directive: &str) -> Option<AnalysisKind> {
    match directive.to_ascii_lowercase().as_str() {
        ".op" => Some(AnalysisKind::Op),
        ".dc" => Some(AnalysisKind::Dc),
        ".ac" => Some(AnalysisKind::Ac),
        ".tran" => Some(AnalysisKind::Tran),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalCardBuilder {
    text: String,
    positions: Vec<SourcePosition>,
    physical_lines: Vec<usize>,
}

impl LogicalCardBuilder {
    fn new(line_number: usize, text: &str, start_column: usize) -> Self {
        let positions = text
            .chars()
            .enumerate()
            .map(|(offset, _)| SourcePosition {
                line: line_number,
                column: start_column + offset,
            })
            .collect();
        Self {
            text: text.to_string(),
            positions,
            physical_lines: vec![line_number],
        }
    }

    fn append_continuation(&mut self, line_number: usize, text: &str, start_column: usize) {
        if text.is_empty() {
            return;
        }
        let join_position = self.positions.last().copied().unwrap_or(SourcePosition {
            line: line_number,
            column: start_column,
        });
        self.text.push(' ');
        self.positions.push(join_position);
        for (offset, ch) in text.chars().enumerate() {
            self.text.push(ch);
            self.positions.push(SourcePosition {
                line: line_number,
                column: start_column + offset,
            });
        }
        self.physical_lines.push(line_number);
    }

    fn span(&self) -> SourceSpan {
        let Some(start) = self.positions.first() else {
            return SourceSpan::point(1, 1);
        };
        let Some(end) = self.positions.last() else {
            return SourceSpan::point(start.line, start.column);
        };
        SourceSpan {
            start_line: start.line,
            start_column: start.column,
            end_line: end.line,
            end_column: end.column + 1,
        }
    }
}

pub fn parse_berkeley_syntax(text: &str) -> BerkeleySyntaxDeck {
    let mut builders = Vec::new();
    let mut diagnostics = Vec::new();
    let mut pending: Option<LogicalCardBuilder> = None;
    let mut title = None;
    let mut saw_content = false;

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let without_comment = strip_inline_comment(raw_line);
        let Some((trimmed, start_column)) = trimmed_with_column(without_comment) else {
            continue;
        };
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('*') {
            if !saw_content && title.is_none() {
                let candidate = trimmed[1..].trim();
                if !candidate.is_empty() {
                    title = Some(candidate.to_string());
                }
            }
            continue;
        }
        if let Some(after_plus) = trimmed.strip_prefix('+') {
            let continuation = after_plus.trim();
            let continuation_start_column = start_column
                + 1
                + after_plus
                    .len()
                    .saturating_sub(after_plus.trim_start().len());
            if let Some(card) = pending.as_mut() {
                card.append_continuation(line_number, continuation, continuation_start_column);
            } else {
                diagnostics.push(BerkeleySyntaxDiagnostic::error(
                    "SPICE_SYNTAX_CONTINUATION_WITHOUT_CARD",
                    "continuation line appears before any logical SPICE card",
                    Some(SourceSpan::point(line_number, start_column)),
                ));
            }
            continue;
        }
        saw_content = true;
        if let Some(card) = pending.take() {
            builders.push(card);
        }
        pending = Some(LogicalCardBuilder::new(line_number, trimmed, start_column));
    }

    if let Some(card) = pending {
        builders.push(card);
    }

    let cards = builders
        .into_iter()
        .map(|builder| logical_card(builder, &mut diagnostics))
        .collect();

    BerkeleySyntaxDeck {
        grammar: BerkeleyGrammarMetadata::current(),
        title,
        cards,
        diagnostics,
    }
}

pub fn parse_berkeley_app_deck(text: &str) -> BerkeleyAppDeck {
    let syntax = parse_berkeley_syntax(text);
    let mut diagnostics = syntax.diagnostics.clone();
    let parsed = if syntax.has_errors() {
        None
    } else {
        match parse_netlist(text) {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                let message = error.to_string();
                diagnostics.push(BerkeleySyntaxDiagnostic::error(
                    "SPICE_BERKELEY_LOWERING_ERROR",
                    message.clone(),
                    parse_line_error_span(&message),
                ));
                None
            }
        }
    };

    BerkeleyAppDeck {
        syntax,
        parsed,
        diagnostics,
    }
}

fn logical_card(
    builder: LogicalCardBuilder,
    diagnostics: &mut Vec<BerkeleySyntaxDiagnostic>,
) -> BerkeleyLogicalCard {
    let tokens = tokenize_card(&builder, diagnostics);
    let head = card_head(&tokens);
    let kind = classify_card(&head);
    let span = builder.span();
    BerkeleyLogicalCard {
        kind,
        head,
        text: builder.text,
        span,
        physical_lines: builder.physical_lines,
        tokens,
    }
}

fn tokenize_card(
    builder: &LogicalCardBuilder,
    diagnostics: &mut Vec<BerkeleySyntaxDiagnostic>,
) -> Vec<BerkeleySyntaxToken> {
    let chars = builder.text.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut paren_depth = 0_i32;

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }

        if ch == '"' {
            let start = index;
            index += 1;
            let mut escaped = false;
            let mut closed = false;
            while index < chars.len() {
                let current = chars[index];
                if escaped {
                    escaped = false;
                } else if current == '\\' {
                    escaped = true;
                } else if current == '"' {
                    index += 1;
                    closed = true;
                    break;
                }
                index += 1;
            }
            if !closed {
                diagnostics.push(BerkeleySyntaxDiagnostic::error(
                    "SPICE_SYNTAX_UNCLOSED_QUOTE",
                    "quoted string is missing its closing quote",
                    Some(span_for_range(&builder.positions, start, index)),
                ));
            }
            tokens.push(token(
                "QUOTED_STRING",
                &chars,
                &builder.positions,
                start,
                index,
            ));
            continue;
        }

        if ch == '{' {
            let start = index;
            index += 1;
            let mut closed = false;
            while index < chars.len() {
                if chars[index] == '}' {
                    index += 1;
                    closed = true;
                    break;
                }
                index += 1;
            }
            if !closed {
                diagnostics.push(BerkeleySyntaxDiagnostic::error(
                    "SPICE_SYNTAX_UNCLOSED_BRACED_EXPR",
                    "braced expression is missing its closing brace",
                    Some(span_for_range(&builder.positions, start, index)),
                ));
            }
            tokens.push(token(
                "BRACED_EXPR",
                &chars,
                &builder.positions,
                start,
                index,
            ));
            continue;
        }

        match ch {
            '(' => {
                paren_depth += 1;
                tokens.push(token(
                    "LPAREN",
                    &chars,
                    &builder.positions,
                    index,
                    index + 1,
                ));
                index += 1;
                continue;
            }
            ')' => {
                if paren_depth == 0 {
                    diagnostics.push(BerkeleySyntaxDiagnostic::error(
                        "SPICE_SYNTAX_UNMATCHED_RPAREN",
                        "closing parenthesis has no matching opening parenthesis",
                        Some(span_for_range(&builder.positions, index, index + 1)),
                    ));
                } else {
                    paren_depth -= 1;
                }
                tokens.push(token(
                    "RPAREN",
                    &chars,
                    &builder.positions,
                    index,
                    index + 1,
                ));
                index += 1;
                continue;
            }
            ',' => {
                tokens.push(token("COMMA", &chars, &builder.positions, index, index + 1));
                index += 1;
                continue;
            }
            '=' => {
                tokens.push(token(
                    "EQUALS",
                    &chars,
                    &builder.positions,
                    index,
                    index + 1,
                ));
                index += 1;
                continue;
            }
            _ => {}
        }

        if ch == '.' {
            let atom_end = read_atom_end(&chars, index);
            let raw = chars[index..atom_end].iter().collect::<String>();
            if let Some(kind) = known_dot_token(&raw) {
                tokens.push(token(kind, &chars, &builder.positions, index, atom_end));
                index = atom_end;
            } else {
                tokens.push(token("DOT", &chars, &builder.positions, index, index + 1));
                index += 1;
            }
            continue;
        }

        let start = index;
        index = read_atom_end(&chars, index);
        let raw = chars[start..index].iter().collect::<String>();
        let kind = if is_number_token(&raw) {
            "NUMBER"
        } else {
            "ATOM"
        };
        tokens.push(token(kind, &chars, &builder.positions, start, index));
    }

    if paren_depth > 0 {
        diagnostics.push(BerkeleySyntaxDiagnostic::error(
            "SPICE_SYNTAX_UNCLOSED_PAREN",
            "opening parenthesis is missing its closing parenthesis",
            Some(builder.span()),
        ));
    }

    tokens
}

fn token(
    kind: &str,
    chars: &[char],
    positions: &[SourcePosition],
    start: usize,
    end: usize,
) -> BerkeleySyntaxToken {
    BerkeleySyntaxToken {
        kind: kind.to_string(),
        text: chars[start..end].iter().collect(),
        span: span_for_range(positions, start, end),
    }
}

fn span_for_range(positions: &[SourcePosition], start: usize, end: usize) -> SourceSpan {
    let first = positions
        .get(start)
        .copied()
        .or_else(|| positions.first().copied())
        .unwrap_or(SourcePosition { line: 1, column: 1 });
    let last = positions
        .get(end.saturating_sub(1))
        .copied()
        .unwrap_or(first);
    SourceSpan {
        start_line: first.line,
        start_column: first.column,
        end_line: last.line,
        end_column: last.column + 1,
    }
}

fn card_head(tokens: &[BerkeleySyntaxToken]) -> String {
    let Some(first) = tokens.first() else {
        return String::new();
    };
    if first.kind == "DOT" {
        if let Some(second) = tokens.get(1) {
            if second.kind == "ATOM" {
                return format!(".{}", second.text);
            }
        }
    }
    first.text.clone()
}

fn classify_card(head: &str) -> BerkeleyCardKind {
    match head.to_ascii_lowercase().as_str() {
        ".model" => BerkeleyCardKind::Model,
        ".subckt" => BerkeleyCardKind::SubcktStart,
        ".ends" => BerkeleyCardKind::SubcktEnd,
        ".end" => BerkeleyCardKind::End,
        ".param" => BerkeleyCardKind::Param,
        ".func" => BerkeleyCardKind::Func,
        ".options" => BerkeleyCardKind::Options,
        ".temp" | ".ic" | ".nodeset" => BerkeleyCardKind::Condition,
        ".op" | ".dc" | ".ac" | ".tran" | ".tf" | ".sens" | ".noise" | ".disto" | ".pz" => {
            BerkeleyCardKind::Analysis
        }
        ".print" | ".plot" | ".save" | ".probe" | ".measure" | ".meas" | ".four" => {
            BerkeleyCardKind::Output
        }
        ".include" | ".lib" => BerkeleyCardKind::Source,
        ".control" => BerkeleyCardKind::ControlStart,
        ".endc" => BerkeleyCardKind::ControlEnd,
        value if value.starts_with('.') => BerkeleyCardKind::UnknownDirective,
        _ => BerkeleyCardKind::Element,
    }
}

fn read_atom_end(chars: &[char], mut index: usize) -> usize {
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() || matches!(ch, '(' | ')' | ',' | '=' | '"' | '{' | '}') {
            break;
        }
        index += 1;
    }
    index
}

fn known_dot_token(raw: &str) -> Option<&'static str> {
    match raw.to_ascii_lowercase().as_str() {
        ".end" => Some("DOT_END"),
        ".ends" => Some("DOT_ENDS"),
        ".subckt" => Some("DOT_SUBCKT"),
        ".model" => Some("DOT_MODEL"),
        ".param" => Some("DOT_PARAM"),
        ".func" => Some("DOT_FUNC"),
        ".options" => Some("DOT_OPTIONS"),
        ".temp" => Some("DOT_TEMP"),
        ".ic" => Some("DOT_IC"),
        ".nodeset" => Some("DOT_NODESET"),
        ".op" => Some("DOT_OP"),
        ".dc" => Some("DOT_DC"),
        ".ac" => Some("DOT_AC"),
        ".tran" => Some("DOT_TRAN"),
        ".tf" => Some("DOT_TF"),
        ".sens" => Some("DOT_SENS"),
        ".noise" => Some("DOT_NOISE"),
        ".disto" => Some("DOT_DISTO"),
        ".pz" => Some("DOT_PZ"),
        ".print" => Some("DOT_PRINT"),
        ".plot" => Some("DOT_PLOT"),
        ".save" => Some("DOT_SAVE"),
        ".probe" => Some("DOT_PROBE"),
        ".measure" => Some("DOT_MEASURE"),
        ".meas" => Some("DOT_MEAS"),
        ".four" => Some("DOT_FOUR"),
        ".include" => Some("DOT_INCLUDE"),
        ".lib" => Some("DOT_LIB"),
        ".control" => Some("DOT_CONTROL"),
        ".endc" => Some("DOT_ENDC"),
        _ => None,
    }
}

fn is_number_token(raw: &str) -> bool {
    let mut chars = raw.chars().peekable();
    if matches!(chars.peek(), Some('+') | Some('-')) {
        chars.next();
    }

    let mut digits_before_dot = 0;
    while matches!(chars.peek(), Some(ch) if ch.is_ascii_digit()) {
        digits_before_dot += 1;
        chars.next();
    }

    let mut digits_after_dot = 0;
    if matches!(chars.peek(), Some('.')) {
        chars.next();
        while matches!(chars.peek(), Some(ch) if ch.is_ascii_digit()) {
            digits_after_dot += 1;
            chars.next();
        }
    }

    if digits_before_dot == 0 && digits_after_dot == 0 {
        return false;
    }

    if matches!(chars.peek(), Some('e') | Some('E')) {
        let mut probe = chars.clone();
        probe.next();
        if matches!(probe.peek(), Some('+') | Some('-')) {
            probe.next();
        }
        let mut exponent_digits = 0;
        while matches!(probe.peek(), Some(ch) if ch.is_ascii_digit()) {
            exponent_digits += 1;
            probe.next();
        }
        if exponent_digits > 0 {
            chars = probe;
        }
    }

    chars.all(|ch| ch.is_ascii_alphabetic())
}

fn strip_inline_comment(line: &str) -> &str {
    line.split_once(';').map_or(line, |(before, _)| before)
}

fn trimmed_with_column(line: &str) -> Option<(&str, usize)> {
    let trimmed_end = line.trim_end();
    if trimmed_end.is_empty() {
        return None;
    }
    let trimmed = trimmed_end.trim_start();
    let leading_columns = trimmed_end.len() - trimmed.len();
    Some((trimmed, leading_columns + 1))
}

fn parse_line_error_span(message: &str) -> Option<SourceSpan> {
    let after_line = message.strip_prefix("line ")?;
    let line_text = after_line.split_once(':')?.0;
    let line = line_text.parse::<usize>().ok()?;
    Some(SourceSpan::point(line, 1))
}
