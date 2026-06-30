use crate::{
    parse_netlist, AnalysisExecutionError, AnalysisExecutionResult, AnalysisKind,
    NetlistParseError, ParsedNetlist,
};
use spice_engine::{
    run_deck, DeckOutputPlanArtifact, DeckRawfileArtifact, DeckRunArtifact, DeckTableArtifact,
    DeckWrdataArtifact,
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
    pub canonical_source: String,
    pub parsed: Option<ParsedNetlist>,
    pub diagnostics: Vec<BerkeleySyntaxDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BerkeleyAppAnalysisArtifact {
    pub syntax_card_index: Option<usize>,
    pub directive: String,
    pub analysis: String,
    pub span: Option<SourceSpan>,
    pub table: String,
    pub table_columns: Vec<String>,
    pub table_row_count: usize,
    pub waveform_series_count: usize,
    pub waveform_series: Vec<BerkeleyAppWaveformSeries>,
    pub table_artifacts: Vec<DeckTableArtifact>,
    pub output_plan_artifacts: Vec<DeckOutputPlanArtifact>,
    pub run_artifacts: Vec<DeckRunArtifact>,
    pub rawfile_artifacts: Vec<DeckRawfileArtifact>,
    pub wrdata_artifacts: Vec<DeckWrdataArtifact>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BerkeleyAppWaveformPoint {
    pub row_index: usize,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BerkeleyAppWaveformSeries {
    pub syntax_card_index: Option<usize>,
    pub directive: String,
    pub analysis: String,
    pub table_name: String,
    pub name: String,
    pub x_column: String,
    pub y_column: String,
    pub group_column: Option<String>,
    pub group_value: Option<String>,
    pub point_count: usize,
    pub points: Vec<BerkeleyAppWaveformPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BerkeleyAppExecution {
    pub canonical_source: String,
    pub analyses: Vec<BerkeleyAppAnalysisArtifact>,
    pub waveform_series_count: usize,
    pub waveform_series: Vec<BerkeleyAppWaveformSeries>,
    pub run_artifacts: Vec<DeckRunArtifact>,
    pub run_artifact_table: String,
    pub run_artifact_csv: String,
    pub run_artifact_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppSessionAnalysis {
    pub syntax_card_index: usize,
    pub directive: String,
    pub analysis: String,
    pub span: SourceSpan,
    pub runnable: bool,
    pub artifact_supported: bool,
    pub selected: bool,
    pub execution_available: bool,
    pub table_row_count: Option<usize>,
    pub table_columns: Vec<String>,
    pub waveform_series_count: Option<usize>,
    pub output_probes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppSessionState {
    pub canonical_source: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub card_count: usize,
    pub analysis_count: usize,
    pub diagnostic_count: usize,
    pub has_errors: bool,
    pub parsed: bool,
    pub execution_available: bool,
    pub selected_syntax_card_index: Option<usize>,
    pub selected_analysis: Option<BerkeleyAppSessionAnalysis>,
    pub selected_table_columns: Vec<String>,
    pub selected_output_probes: Vec<String>,
    pub selected_waveform_series_count: Option<usize>,
    pub blocking_message: Option<String>,
    pub diagnostics: Vec<BerkeleySyntaxDiagnostic>,
    pub analyses: Vec<BerkeleyAppSessionAnalysis>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BerkeleyAppEditorActionKind {
    SelectAnalysis,
    RunAnalysis,
    InspectTable,
    InspectWaveform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppEditorAction {
    pub kind: BerkeleyAppEditorActionKind,
    pub label: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppAnalysisControl {
    pub syntax_card_index: usize,
    pub directive: String,
    pub analysis: String,
    pub span: SourceSpan,
    pub selected: bool,
    pub runnable: bool,
    pub artifact_supported: bool,
    pub execution_available: bool,
    pub table_available: bool,
    pub waveform_available: bool,
    pub action_count: usize,
    pub actions: Vec<BerkeleyAppEditorAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppEditorControls {
    pub canonical_source: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub parsed: bool,
    pub execution_available: bool,
    pub selected_syntax_card_index: Option<usize>,
    pub selected_control: Option<BerkeleyAppAnalysisControl>,
    pub control_count: usize,
    pub controls: Vec<BerkeleyAppAnalysisControl>,
    pub blocking_message: Option<String>,
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

    pub fn session_state(
        &self,
        selected_syntax_card_index: Option<usize>,
    ) -> BerkeleyAppSessionState {
        let has_errors = self.has_errors();
        let parsed = self.parsed.is_some();
        let mut analyses = self
            .analysis_inventory()
            .into_iter()
            .map(|entry| BerkeleyAppSessionAnalysis {
                syntax_card_index: entry.index,
                directive: entry.directive.clone(),
                analysis: entry.analysis,
                span: entry.span,
                runnable: runnable_analysis_kind(&entry.directive).is_some(),
                artifact_supported: deck_artifact_analysis_directive(&entry.directive),
                selected: selected_syntax_card_index == Some(entry.index),
                execution_available: false,
                table_row_count: None,
                table_columns: Vec::new(),
                waveform_series_count: None,
                output_probes: Vec::new(),
            })
            .collect::<Vec<_>>();
        let selected_analysis = analyses.iter().find(|analysis| analysis.selected).cloned();

        BerkeleyAppSessionState {
            canonical_source: self.canonical_source.clone(),
            source_fingerprint: stable_source_fingerprint(&self.canonical_source),
            title: self.syntax.title.clone(),
            card_count: self.syntax.cards.len(),
            analysis_count: analyses.len(),
            diagnostic_count: self.diagnostics.len(),
            has_errors,
            parsed,
            execution_available: false,
            selected_syntax_card_index,
            selected_analysis,
            selected_table_columns: Vec::new(),
            selected_output_probes: Vec::new(),
            selected_waveform_series_count: None,
            blocking_message: if parsed {
                None
            } else {
                Some(self.blocking_message())
            },
            diagnostics: self.diagnostics.clone(),
            analyses: std::mem::take(&mut analyses),
        }
    }

    pub fn run_session_state(
        &self,
        selected_syntax_card_index: Option<usize>,
    ) -> Result<BerkeleyAppSessionState, AnalysisExecutionError> {
        let mut state = self.session_state(selected_syntax_card_index);
        if self.parsed.is_none() {
            return Ok(state);
        }

        let execution = self.run_artifacts()?;
        state.execution_available = true;
        for artifact in &execution.analyses {
            let Some(syntax_card_index) = artifact.syntax_card_index else {
                continue;
            };
            let Some(analysis) = state
                .analyses
                .iter_mut()
                .find(|analysis| analysis.syntax_card_index == syntax_card_index)
            else {
                continue;
            };
            analysis.execution_available = true;
            analysis.table_row_count = Some(artifact.table_row_count);
            analysis.table_columns = artifact.table_columns.clone();
            analysis.waveform_series_count = Some(artifact.waveform_series_count);
            analysis.output_probes = output_plan_probes(&artifact.output_plan_artifacts);
        }
        refresh_selected_session_analysis(&mut state);
        Ok(state)
    }

    pub fn editor_controls(
        &self,
        selected_syntax_card_index: Option<usize>,
    ) -> BerkeleyAppEditorControls {
        editor_controls_from_session_state(&self.session_state(selected_syntax_card_index))
    }

    pub fn run_editor_controls(
        &self,
        selected_syntax_card_index: Option<usize>,
    ) -> Result<BerkeleyAppEditorControls, AnalysisExecutionError> {
        Ok(editor_controls_from_session_state(
            &self.run_session_state(selected_syntax_card_index)?,
        ))
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

    pub fn run_artifacts(&self) -> Result<BerkeleyAppExecution, AnalysisExecutionError> {
        let parsed = self.parsed.as_ref().ok_or_else(|| {
            AnalysisExecutionError::Netlist(NetlistParseError::new(self.blocking_message()))
        })?;
        let deck_execution = run_deck(&parsed.circuit, &self.canonical_source)?;
        let analysis_entries = self
            .analysis_inventory()
            .into_iter()
            .filter(|entry| deck_artifact_analysis_directive(&entry.directive))
            .collect::<Vec<_>>();
        let mut analysis_entries = analysis_entries.into_iter();
        let mut execution_waveform_series = Vec::new();
        let analyses = deck_execution
            .executions
            .into_iter()
            .map(|execution| {
                let entry = if execution.plan.line_number == 0 {
                    None
                } else {
                    analysis_entries.next()
                };
                let syntax_card_index = entry.as_ref().map(|entry| entry.index);
                let directive = entry
                    .as_ref()
                    .map(|entry| entry.directive.clone())
                    .unwrap_or_else(|| execution.plan.directive.clone());
                let analysis = execution.plan.analysis.clone();
                let waveform_series = deck_waveform_series(
                    syntax_card_index,
                    &directive,
                    &analysis,
                    &execution.table,
                );
                execution_waveform_series.extend(waveform_series.iter().cloned());
                BerkeleyAppAnalysisArtifact {
                    syntax_card_index,
                    directive,
                    analysis,
                    span: entry.as_ref().map(|entry| entry.span),
                    table_columns: deck_table_columns(&execution.table),
                    table_row_count: deck_table_row_count(&execution.table),
                    waveform_series_count: waveform_series.len(),
                    waveform_series,
                    table: execution.table,
                    table_artifacts: execution.table_artifacts,
                    output_plan_artifacts: execution.output_plan_artifacts,
                    run_artifacts: execution.run_artifacts,
                    rawfile_artifacts: execution.rawfile_artifacts,
                    wrdata_artifacts: execution.wrdata_artifacts,
                }
            })
            .collect();

        Ok(BerkeleyAppExecution {
            canonical_source: self.canonical_source.clone(),
            analyses,
            waveform_series_count: execution_waveform_series.len(),
            waveform_series: execution_waveform_series,
            run_artifacts: deck_execution.run_artifacts,
            run_artifact_table: deck_execution.run_artifact_table,
            run_artifact_csv: deck_execution.run_artifact_csv,
            run_artifact_json: deck_execution.run_artifact_json,
        })
    }

    pub fn run_selected_artifact(
        &self,
        syntax_card_index: usize,
    ) -> Result<Option<BerkeleyAppAnalysisArtifact>, AnalysisExecutionError> {
        Ok(self
            .run_artifacts()?
            .analyses
            .into_iter()
            .find(|artifact| artifact.syntax_card_index == Some(syntax_card_index)))
    }

    pub fn run_selected_waveform_series(
        &self,
        syntax_card_index: usize,
    ) -> Result<Option<Vec<BerkeleyAppWaveformSeries>>, AnalysisExecutionError> {
        Ok(self
            .run_selected_artifact(syntax_card_index)?
            .map(|artifact| artifact.waveform_series))
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

fn editor_controls_from_session_state(
    state: &BerkeleyAppSessionState,
) -> BerkeleyAppEditorControls {
    let controls = state
        .analyses
        .iter()
        .map(|analysis| analysis_control_from_session_state(state, analysis))
        .collect::<Vec<_>>();
    let selected_control = controls.iter().find(|control| control.selected).cloned();

    BerkeleyAppEditorControls {
        canonical_source: state.canonical_source.clone(),
        source_fingerprint: state.source_fingerprint.clone(),
        title: state.title.clone(),
        parsed: state.parsed,
        execution_available: state.execution_available,
        selected_syntax_card_index: state.selected_syntax_card_index,
        selected_control,
        control_count: controls.len(),
        controls,
        blocking_message: state.blocking_message.clone(),
        diagnostics: state.diagnostics.clone(),
    }
}

fn analysis_control_from_session_state(
    state: &BerkeleyAppSessionState,
    analysis: &BerkeleyAppSessionAnalysis,
) -> BerkeleyAppAnalysisControl {
    let table_available = analysis.execution_available && analysis.table_row_count.is_some();
    let waveform_available =
        analysis.execution_available && analysis.waveform_series_count.unwrap_or(0) > 0;
    let actions = vec![
        BerkeleyAppEditorAction {
            kind: BerkeleyAppEditorActionKind::SelectAnalysis,
            label: format!("Select {}", analysis.directive),
            enabled: true,
            disabled_reason: None,
        },
        editor_action(
            BerkeleyAppEditorActionKind::RunAnalysis,
            format!("Run {}", analysis.directive),
            state.parsed && analysis.runnable,
            run_action_disabled_reason(state, analysis),
        ),
        editor_action(
            BerkeleyAppEditorActionKind::InspectTable,
            format!("Inspect {} table", analysis.directive),
            table_available,
            table_action_disabled_reason(state, analysis),
        ),
        editor_action(
            BerkeleyAppEditorActionKind::InspectWaveform,
            format!("Inspect {} waveform", analysis.directive),
            waveform_available,
            waveform_action_disabled_reason(state, analysis),
        ),
    ];

    BerkeleyAppAnalysisControl {
        syntax_card_index: analysis.syntax_card_index,
        directive: analysis.directive.clone(),
        analysis: analysis.analysis.clone(),
        span: analysis.span,
        selected: analysis.selected,
        runnable: analysis.runnable,
        artifact_supported: analysis.artifact_supported,
        execution_available: analysis.execution_available,
        table_available,
        waveform_available,
        action_count: actions.len(),
        actions,
    }
}

fn editor_action(
    kind: BerkeleyAppEditorActionKind,
    label: String,
    enabled: bool,
    disabled_reason: Option<String>,
) -> BerkeleyAppEditorAction {
    BerkeleyAppEditorAction {
        kind,
        label,
        enabled,
        disabled_reason: if enabled { None } else { disabled_reason },
    }
}

fn run_action_disabled_reason(
    state: &BerkeleyAppSessionState,
    analysis: &BerkeleyAppSessionAnalysis,
) -> Option<String> {
    if !state.parsed {
        state.blocking_message.clone()
    } else if !analysis.runnable {
        Some("analysis is not runnable from the app facade".to_string())
    } else {
        None
    }
}

fn table_action_disabled_reason(
    state: &BerkeleyAppSessionState,
    analysis: &BerkeleyAppSessionAnalysis,
) -> Option<String> {
    if !state.parsed {
        state.blocking_message.clone()
    } else if !analysis.artifact_supported {
        Some("analysis artifacts are not supported".to_string())
    } else if !analysis.execution_available {
        Some("run deck artifacts to populate analysis table".to_string())
    } else {
        Some("analysis did not produce a result table".to_string())
    }
}

fn waveform_action_disabled_reason(
    state: &BerkeleyAppSessionState,
    analysis: &BerkeleyAppSessionAnalysis,
) -> Option<String> {
    if !state.parsed {
        state.blocking_message.clone()
    } else if !analysis.artifact_supported {
        Some("analysis artifacts are not supported".to_string())
    } else if !analysis.execution_available {
        Some("run deck artifacts to populate waveform series".to_string())
    } else {
        Some("analysis has no waveform series".to_string())
    }
}

fn refresh_selected_session_analysis(state: &mut BerkeleyAppSessionState) {
    state.selected_analysis = state
        .analyses
        .iter()
        .find(|analysis| Some(analysis.syntax_card_index) == state.selected_syntax_card_index)
        .cloned();
    if let Some(selected) = &state.selected_analysis {
        state.selected_table_columns = selected.table_columns.clone();
        state.selected_output_probes = selected.output_probes.clone();
        state.selected_waveform_series_count = selected.waveform_series_count;
    } else {
        state.selected_table_columns.clear();
        state.selected_output_probes.clear();
        state.selected_waveform_series_count = None;
    }
}

fn output_plan_probes(artifacts: &[DeckOutputPlanArtifact]) -> Vec<String> {
    let mut probes = Vec::new();
    for artifact in artifacts {
        for probe in &artifact.output_probes {
            if !probes.iter().any(|existing| existing == probe) {
                probes.push(probe.clone());
            }
        }
    }
    probes
}

fn stable_source_fingerprint(source: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn deck_artifact_analysis_directive(directive: &str) -> bool {
    matches!(
        directive.to_ascii_lowercase().as_str(),
        ".op" | ".dc" | ".ac" | ".tran" | ".tf" | ".sens" | ".noise"
    )
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
    let canonical_source = canonical_source(&syntax);
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
        canonical_source,
        parsed,
        diagnostics,
    }
}

fn canonical_source(syntax: &BerkeleySyntaxDeck) -> String {
    let mut lines = Vec::new();
    if let Some(title) = &syntax.title {
        lines.push(format!("* {title}"));
    }
    lines.extend(syntax.cards.iter().map(|card| card.text.clone()));
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn deck_table_columns(table: &str) -> Vec<String> {
    table
        .lines()
        .next()
        .map(|header| header.split('\t').map(str::to_string).collect())
        .unwrap_or_default()
}

fn deck_table_row_count(table: &str) -> usize {
    let mut lines = table.lines();
    if lines.next().is_none() {
        0
    } else {
        lines.count()
    }
}

fn deck_waveform_series(
    syntax_card_index: Option<usize>,
    directive: &str,
    analysis: &str,
    table: &str,
) -> Vec<BerkeleyAppWaveformSeries> {
    let mut lines = table.lines();
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let columns = header.split('\t').map(str::to_string).collect::<Vec<_>>();
    let Some(x_index) = deck_waveform_x_column(&columns) else {
        return Vec::new();
    };
    let rows = lines
        .map(|row| row.split('\t').map(str::to_string).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let group_index = deck_waveform_group_column(&columns, x_index);
    let group_values = deck_waveform_group_values(&rows, group_index);
    let mut series = Vec::new();

    for (y_index, y_column) in columns.iter().enumerate() {
        if y_index == x_index || Some(y_index) == group_index || !deck_waveform_y_column(y_column) {
            continue;
        }

        if group_index.is_some() {
            for group_value in &group_values {
                let points =
                    deck_waveform_points(&rows, x_index, y_index, group_index, Some(group_value));
                if points.is_empty() {
                    continue;
                }
                series.push(BerkeleyAppWaveformSeries {
                    syntax_card_index,
                    directive: directive.to_string(),
                    analysis: analysis.to_string(),
                    table_name: "result".to_string(),
                    name: format!("{group_value}:{y_column}"),
                    x_column: columns[x_index].clone(),
                    y_column: y_column.clone(),
                    group_column: group_index.map(|index| columns[index].clone()),
                    group_value: Some(group_value.clone()),
                    point_count: points.len(),
                    points,
                });
            }
        } else {
            let points = deck_waveform_points(&rows, x_index, y_index, None, None);
            if points.is_empty() {
                continue;
            }
            series.push(BerkeleyAppWaveformSeries {
                syntax_card_index,
                directive: directive.to_string(),
                analysis: analysis.to_string(),
                table_name: "result".to_string(),
                name: y_column.clone(),
                x_column: columns[x_index].clone(),
                y_column: y_column.clone(),
                group_column: None,
                group_value: None,
                point_count: points.len(),
                points,
            });
        }
    }

    series
}

fn deck_waveform_x_column(columns: &[String]) -> Option<usize> {
    ["Time", "Frequency", "Value", "TemperatureKelvin"]
        .into_iter()
        .find_map(|preferred| {
            columns
                .iter()
                .position(|column| column.eq_ignore_ascii_case(preferred))
        })
        .or_else(|| {
            columns
                .iter()
                .position(|column| column.eq_ignore_ascii_case("Index"))
        })
}

fn deck_waveform_group_column(columns: &[String], x_index: usize) -> Option<usize> {
    ["Probe", "Source", "Corner", "Method"]
        .into_iter()
        .find_map(|preferred| {
            columns.iter().enumerate().find_map(|(index, column)| {
                if index != x_index && column.eq_ignore_ascii_case(preferred) {
                    Some(index)
                } else {
                    None
                }
            })
        })
}

fn deck_waveform_y_column(column: &str) -> bool {
    !matches!(
        column.to_ascii_lowercase().as_str(),
        "index" | "source" | "probe" | "corner" | "method" | "converged" | "stepsrejected"
    )
}

fn deck_waveform_group_values(rows: &[Vec<String>], group_index: Option<usize>) -> Vec<String> {
    let Some(group_index) = group_index else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for row in rows {
        let value = row.get(group_index).cloned().unwrap_or_default();
        if !values.iter().any(|existing| existing == &value) {
            values.push(value);
        }
    }
    values
}

fn deck_waveform_points(
    rows: &[Vec<String>],
    x_index: usize,
    y_index: usize,
    group_index: Option<usize>,
    group_value: Option<&String>,
) -> Vec<BerkeleyAppWaveformPoint> {
    rows.iter()
        .enumerate()
        .filter_map(|(row_index, row)| {
            if let (Some(group_index), Some(group_value)) = (group_index, group_value) {
                if row.get(group_index) != Some(group_value) {
                    return None;
                }
            }
            let x = parse_finite_cell(row, x_index)?;
            let y = parse_finite_cell(row, y_index)?;
            Some(BerkeleyAppWaveformPoint { row_index, x, y })
        })
        .collect()
}

fn parse_finite_cell(row: &[String], index: usize) -> Option<f64> {
    row.get(index)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
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
            "unclosed parenthesis: opening parenthesis is missing its closing parenthesis",
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
