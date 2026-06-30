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
pub const BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_PACKAGE_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_BOOTSTRAP_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_STARTUP_SUMMARY_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_PACKAGE_NAME: &str = "berkeley-spice-mosaic-app";
pub const BERKELEY_APP_SOURCE_FINGERPRINT_ALGORITHM: &str = "fnv1a-64";

const BERKELEY_APP_HOST_PANEL_KINDS: &[&str] =
    &["source", "diagnostics", "analysis", "table", "waveform"];
const BERKELEY_APP_EDITOR_ACTION_KINDS: &[&str] = &[
    "select-analysis",
    "run-analysis",
    "inspect-table",
    "inspect-waveform",
];
const BERKELEY_APP_COMMAND_TARGETS: &[&str] = &[
    "analysis-selection",
    "analysis-runner",
    "analysis-table",
    "analysis-waveform",
];
const BERKELEY_APP_RUNNABLE_ANALYSIS_DIRECTIVES: &[&str] = &[".op", ".dc", ".ac", ".tran"];
const BERKELEY_APP_ARTIFACT_ANALYSIS_DIRECTIVES: &[&str] =
    &[".op", ".dc", ".ac", ".tran", ".tf", ".sens", ".noise"];
const BERKELEY_APP_ARTIFACT_CAPABILITIES: &[&str] = &[
    "canonical-source",
    "analysis-inventory",
    "source-fingerprint",
    "session-state",
    "editor-controls",
    "editor-command-plan",
    "persisted-editor-state",
    "host-surface",
    "host-surface-wire-json",
    "app-bootstrap-json",
    "app-startup-summary-json",
    "result-tables",
    "waveform-series",
    "run-artifacts",
    "output-plan-artifacts",
    "rawfile-artifacts",
    "wrdata-artifacts",
];

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
pub struct BerkeleyAppPackageManifest {
    pub schema_version: u32,
    pub package_name: String,
    pub grammar_name: String,
    pub grammar_version: u32,
    pub host_surface_wire_schema_version: u32,
    pub source_fingerprint_algorithm: String,
    pub host_panel_kinds: Vec<String>,
    pub editor_action_kinds: Vec<String>,
    pub command_targets: Vec<String>,
    pub runnable_analysis_directives: Vec<String>,
    pub artifact_analysis_directives: Vec<String>,
    pub artifact_capabilities: Vec<String>,
}

impl BerkeleyAppPackageManifest {
    pub fn to_json(&self) -> String {
        app_package_manifest_json_value(self).to_string()
    }
}

pub fn berkeley_app_package_manifest() -> BerkeleyAppPackageManifest {
    BerkeleyAppPackageManifest {
        schema_version: BERKELEY_APP_PACKAGE_MANIFEST_SCHEMA_VERSION,
        package_name: BERKELEY_APP_PACKAGE_NAME.to_string(),
        grammar_name: BERKELEY_SPICE_GRAMMAR_NAME.to_string(),
        grammar_version: BERKELEY_SPICE_GRAMMAR_VERSION,
        host_surface_wire_schema_version: BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION,
        source_fingerprint_algorithm: BERKELEY_APP_SOURCE_FINGERPRINT_ALGORITHM.to_string(),
        host_panel_kinds: manifest_strings(BERKELEY_APP_HOST_PANEL_KINDS),
        editor_action_kinds: manifest_strings(BERKELEY_APP_EDITOR_ACTION_KINDS),
        command_targets: manifest_strings(BERKELEY_APP_COMMAND_TARGETS),
        runnable_analysis_directives: manifest_strings(BERKELEY_APP_RUNNABLE_ANALYSIS_DIRECTIVES),
        artifact_analysis_directives: manifest_strings(BERKELEY_APP_ARTIFACT_ANALYSIS_DIRECTIVES),
        artifact_capabilities: manifest_strings(BERKELEY_APP_ARTIFACT_CAPABILITIES),
    }
}

pub fn berkeley_app_package_manifest_json() -> String {
    berkeley_app_package_manifest().to_json()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppBootstrapSnapshot {
    pub schema_version: u32,
    pub package_manifest: BerkeleyAppPackageManifest,
    pub host_surface: BerkeleyAppHostSurfaceWire,
}

impl BerkeleyAppBootstrapSnapshot {
    pub fn to_json(&self) -> String {
        app_bootstrap_snapshot_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppStartupSummary {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub parsed: bool,
    pub execution_available: bool,
    pub ready: bool,
    pub requested_selected_syntax_card_index: Option<usize>,
    pub requested_active_command_id: Option<String>,
    pub resolved_selected_syntax_card_index: Option<usize>,
    pub resolved_active_command_id: Option<String>,
    pub selection_stale: bool,
    pub command_stale: bool,
    pub panel_count: usize,
    pub active_panel_id: Option<String>,
    pub diagnostic_count: usize,
    pub blocking_message: Option<String>,
}

impl BerkeleyAppStartupSummary {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        let host_surface = &snapshot.host_surface;
        Self {
            schema_version: BERKELEY_APP_STARTUP_SUMMARY_SCHEMA_VERSION,
            package_name: snapshot.package_manifest.package_name.clone(),
            source_fingerprint: host_surface.source_fingerprint.clone(),
            title: host_surface.title.clone(),
            parsed: host_surface.parsed,
            execution_available: host_surface.execution_available,
            ready: host_surface.parsed && host_surface.execution_available,
            requested_selected_syntax_card_index: host_surface.requested_selected_syntax_card_index,
            requested_active_command_id: host_surface.requested_active_command_id.clone(),
            resolved_selected_syntax_card_index: host_surface.resolved_selected_syntax_card_index,
            resolved_active_command_id: host_surface.resolved_active_command_id.clone(),
            selection_stale: host_surface.selection_stale,
            command_stale: host_surface.command_stale,
            panel_count: host_surface.panel_count,
            active_panel_id: host_surface.active_panel_id.clone(),
            diagnostic_count: host_surface.diagnostics.len(),
            blocking_message: host_surface.blocking_message.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        app_startup_summary_json_value(self).to_string()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppEditorCommand {
    pub id: String,
    pub kind: BerkeleyAppEditorActionKind,
    pub syntax_card_index: usize,
    pub directive: String,
    pub analysis: String,
    pub span: SourceSpan,
    pub target: String,
    pub label: String,
    pub enabled: bool,
    pub selected: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppEditorCommandPlan {
    pub canonical_source: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub parsed: bool,
    pub execution_available: bool,
    pub selected_syntax_card_index: Option<usize>,
    pub command_count: usize,
    pub commands: Vec<BerkeleyAppEditorCommand>,
    pub blocking_message: Option<String>,
    pub diagnostics: Vec<BerkeleySyntaxDiagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BerkeleyAppPersistedEditorState {
    pub selected_syntax_card_index: Option<usize>,
    pub active_command_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppEditorStateSnapshot {
    pub canonical_source: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub parsed: bool,
    pub execution_available: bool,
    pub requested_state: BerkeleyAppPersistedEditorState,
    pub resolved_state: BerkeleyAppPersistedEditorState,
    pub selection_stale: bool,
    pub command_stale: bool,
    pub selected_control: Option<BerkeleyAppAnalysisControl>,
    pub active_command: Option<BerkeleyAppEditorCommand>,
    pub command_plan: BerkeleyAppEditorCommandPlan,
    pub blocking_message: Option<String>,
    pub diagnostics: Vec<BerkeleySyntaxDiagnostic>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BerkeleyAppHostPanelKind {
    Source,
    Diagnostics,
    Analysis,
    Table,
    Waveform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppHostPanel {
    pub id: String,
    pub kind: BerkeleyAppHostPanelKind,
    pub title: String,
    pub target: String,
    pub enabled: bool,
    pub active: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppHostSurface {
    pub canonical_source: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub parsed: bool,
    pub execution_available: bool,
    pub editor_state: BerkeleyAppEditorStateSnapshot,
    pub panel_count: usize,
    pub active_panel: Option<BerkeleyAppHostPanel>,
    pub panels: Vec<BerkeleyAppHostPanel>,
    pub blocking_message: Option<String>,
    pub diagnostics: Vec<BerkeleySyntaxDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppHostSpanWire {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppHostDiagnosticWire {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub span: Option<BerkeleyAppHostSpanWire>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppHostPanelWire {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub target: String,
    pub enabled: bool,
    pub active: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppHostSurfaceWire {
    pub schema_version: u32,
    pub canonical_source: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub parsed: bool,
    pub execution_available: bool,
    pub requested_selected_syntax_card_index: Option<usize>,
    pub requested_active_command_id: Option<String>,
    pub resolved_selected_syntax_card_index: Option<usize>,
    pub resolved_active_command_id: Option<String>,
    pub selection_stale: bool,
    pub command_stale: bool,
    pub panel_count: usize,
    pub active_panel_id: Option<String>,
    pub panels: Vec<BerkeleyAppHostPanelWire>,
    pub blocking_message: Option<String>,
    pub diagnostics: Vec<BerkeleyAppHostDiagnosticWire>,
}

impl BerkeleyAppHostSurfaceWire {
    pub fn to_json(&self) -> String {
        host_surface_wire_json_value(self).to_string()
    }
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

    pub fn editor_command_plan(
        &self,
        selected_syntax_card_index: Option<usize>,
    ) -> BerkeleyAppEditorCommandPlan {
        editor_command_plan_from_controls(&self.editor_controls(selected_syntax_card_index))
    }

    pub fn run_editor_command_plan(
        &self,
        selected_syntax_card_index: Option<usize>,
    ) -> Result<BerkeleyAppEditorCommandPlan, AnalysisExecutionError> {
        Ok(editor_command_plan_from_controls(
            &self.run_editor_controls(selected_syntax_card_index)?,
        ))
    }

    pub fn editor_state_snapshot(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppEditorStateSnapshot {
        editor_state_snapshot_from_session_state(self.session_state(None), persisted_state)
    }

    pub fn run_editor_state_snapshot(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppEditorStateSnapshot, AnalysisExecutionError> {
        Ok(editor_state_snapshot_from_session_state(
            self.run_session_state(None)?,
            persisted_state,
        ))
    }

    pub fn host_surface(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppHostSurface {
        host_surface_from_editor_state(self.editor_state_snapshot(persisted_state))
    }

    pub fn run_host_surface(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppHostSurface, AnalysisExecutionError> {
        Ok(host_surface_from_editor_state(
            self.run_editor_state_snapshot(persisted_state)?,
        ))
    }

    pub fn host_surface_wire(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppHostSurfaceWire {
        BerkeleyAppHostSurfaceWire::from(self.host_surface(persisted_state))
    }

    pub fn run_host_surface_wire(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppHostSurfaceWire, AnalysisExecutionError> {
        Ok(BerkeleyAppHostSurfaceWire::from(
            self.run_host_surface(persisted_state)?,
        ))
    }

    pub fn host_surface_wire_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.host_surface_wire(persisted_state).to_json()
    }

    pub fn run_host_surface_wire_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_host_surface_wire(persisted_state)?.to_json())
    }

    pub fn app_bootstrap_snapshot(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppBootstrapSnapshot {
        BerkeleyAppBootstrapSnapshot {
            schema_version: BERKELEY_APP_BOOTSTRAP_SCHEMA_VERSION,
            package_manifest: berkeley_app_package_manifest(),
            host_surface: self.host_surface_wire(persisted_state),
        }
    }

    pub fn run_app_bootstrap_snapshot(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppBootstrapSnapshot, AnalysisExecutionError> {
        Ok(BerkeleyAppBootstrapSnapshot {
            schema_version: BERKELEY_APP_BOOTSTRAP_SCHEMA_VERSION,
            package_manifest: berkeley_app_package_manifest(),
            host_surface: self.run_host_surface_wire(persisted_state)?,
        })
    }

    pub fn app_bootstrap_json(&self, persisted_state: BerkeleyAppPersistedEditorState) -> String {
        self.app_bootstrap_snapshot(persisted_state).to_json()
    }

    pub fn run_app_bootstrap_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_bootstrap_snapshot(persisted_state)?.to_json())
    }

    pub fn app_startup_summary(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppStartupSummary {
        BerkeleyAppStartupSummary::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_startup_summary(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppStartupSummary, AnalysisExecutionError> {
        Ok(BerkeleyAppStartupSummary::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_startup_summary_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_startup_summary(persisted_state).to_json()
    }

    pub fn run_app_startup_summary_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_startup_summary(persisted_state)?.to_json())
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

fn editor_command_plan_from_controls(
    controls: &BerkeleyAppEditorControls,
) -> BerkeleyAppEditorCommandPlan {
    let commands = controls
        .controls
        .iter()
        .flat_map(|control| {
            control
                .actions
                .iter()
                .map(|action| editor_command_from_control(control, action))
        })
        .collect::<Vec<_>>();

    BerkeleyAppEditorCommandPlan {
        canonical_source: controls.canonical_source.clone(),
        source_fingerprint: controls.source_fingerprint.clone(),
        title: controls.title.clone(),
        parsed: controls.parsed,
        execution_available: controls.execution_available,
        selected_syntax_card_index: controls.selected_syntax_card_index,
        command_count: commands.len(),
        commands,
        blocking_message: controls.blocking_message.clone(),
        diagnostics: controls.diagnostics.clone(),
    }
}

fn editor_state_snapshot_from_session_state(
    mut state: BerkeleyAppSessionState,
    requested_state: BerkeleyAppPersistedEditorState,
) -> BerkeleyAppEditorStateSnapshot {
    let resolved_selection =
        resolve_editor_selection(&state, requested_state.selected_syntax_card_index);
    apply_session_selection(&mut state, resolved_selection);

    let controls = editor_controls_from_session_state(&state);
    let command_plan = editor_command_plan_from_controls(&controls);
    let active_command = resolve_active_editor_command(
        &command_plan,
        resolved_selection,
        requested_state.active_command_id.as_deref(),
    )
    .cloned();
    let resolved_state = BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: resolved_selection,
        active_command_id: active_command.as_ref().map(|command| command.id.clone()),
    };
    let selection_stale = requested_state.selected_syntax_card_index.is_some()
        && requested_state.selected_syntax_card_index != resolved_state.selected_syntax_card_index;
    let command_stale = requested_state.active_command_id.is_some()
        && requested_state.active_command_id != resolved_state.active_command_id;

    BerkeleyAppEditorStateSnapshot {
        canonical_source: command_plan.canonical_source.clone(),
        source_fingerprint: command_plan.source_fingerprint.clone(),
        title: command_plan.title.clone(),
        parsed: command_plan.parsed,
        execution_available: command_plan.execution_available,
        requested_state,
        resolved_state,
        selection_stale,
        command_stale,
        selected_control: controls.selected_control,
        active_command,
        blocking_message: command_plan.blocking_message.clone(),
        diagnostics: command_plan.diagnostics.clone(),
        command_plan,
    }
}

fn resolve_editor_selection(
    state: &BerkeleyAppSessionState,
    requested_selection: Option<usize>,
) -> Option<usize> {
    if let Some(selection) = requested_selection {
        if state
            .analyses
            .iter()
            .any(|analysis| analysis.syntax_card_index == selection)
        {
            return Some(selection);
        }
    }
    state
        .analyses
        .first()
        .map(|analysis| analysis.syntax_card_index)
}

fn apply_session_selection(
    state: &mut BerkeleyAppSessionState,
    selected_syntax_card_index: Option<usize>,
) {
    state.selected_syntax_card_index = selected_syntax_card_index;
    for analysis in &mut state.analyses {
        analysis.selected = Some(analysis.syntax_card_index) == selected_syntax_card_index;
    }
    refresh_selected_session_analysis(state);
}

fn resolve_active_editor_command<'a>(
    command_plan: &'a BerkeleyAppEditorCommandPlan,
    selected_syntax_card_index: Option<usize>,
    requested_command_id: Option<&str>,
) -> Option<&'a BerkeleyAppEditorCommand> {
    if let Some(command_id) = requested_command_id {
        if let Some(command) = command_plan.commands.iter().find(|command| {
            command.id == command_id
                && Some(command.syntax_card_index) == selected_syntax_card_index
        }) {
            return Some(command);
        }
    }

    let selected_syntax_card_index = selected_syntax_card_index?;
    command_plan.commands.iter().find(|command| {
        command.syntax_card_index == selected_syntax_card_index
            && command.kind == BerkeleyAppEditorActionKind::SelectAnalysis
    })
}

fn host_surface_from_editor_state(
    editor_state: BerkeleyAppEditorStateSnapshot,
) -> BerkeleyAppHostSurface {
    let active_kind = active_host_panel_kind(&editor_state);
    let selected_control = editor_state.selected_control.as_ref();
    let table_enabled = selected_control.is_some_and(|control| control.table_available);
    let waveform_enabled = selected_control.is_some_and(|control| control.waveform_available);
    let diagnostics_enabled =
        !editor_state.diagnostics.is_empty() || editor_state.blocking_message.is_some();
    let analysis_enabled = editor_state.command_plan.command_count > 0;

    let panels = vec![
        host_panel(
            BerkeleyAppHostPanelKind::Source,
            "source",
            "Source",
            "source-editor",
            true,
            active_kind == BerkeleyAppHostPanelKind::Source,
            None,
        ),
        host_panel(
            BerkeleyAppHostPanelKind::Diagnostics,
            "diagnostics",
            "Diagnostics",
            "diagnostics",
            diagnostics_enabled,
            active_kind == BerkeleyAppHostPanelKind::Diagnostics,
            if diagnostics_enabled {
                None
            } else {
                Some("deck has no diagnostics".to_string())
            },
        ),
        host_panel(
            BerkeleyAppHostPanelKind::Analysis,
            "analysis",
            "Analysis",
            "analysis-controls",
            analysis_enabled,
            active_kind == BerkeleyAppHostPanelKind::Analysis,
            if analysis_enabled {
                None
            } else {
                Some("deck has no analysis controls".to_string())
            },
        ),
        host_panel(
            BerkeleyAppHostPanelKind::Table,
            "table",
            "Table",
            "analysis-table",
            table_enabled,
            active_kind == BerkeleyAppHostPanelKind::Table,
            if table_enabled {
                None
            } else {
                host_panel_disabled_reason(&editor_state, BerkeleyAppEditorActionKind::InspectTable)
            },
        ),
        host_panel(
            BerkeleyAppHostPanelKind::Waveform,
            "waveform",
            "Waveform",
            "analysis-waveform",
            waveform_enabled,
            active_kind == BerkeleyAppHostPanelKind::Waveform,
            if waveform_enabled {
                None
            } else {
                host_panel_disabled_reason(
                    &editor_state,
                    BerkeleyAppEditorActionKind::InspectWaveform,
                )
            },
        ),
    ];
    let active_panel = panels.iter().find(|panel| panel.active).cloned();

    BerkeleyAppHostSurface {
        canonical_source: editor_state.canonical_source.clone(),
        source_fingerprint: editor_state.source_fingerprint.clone(),
        title: editor_state.title.clone(),
        parsed: editor_state.parsed,
        execution_available: editor_state.execution_available,
        panel_count: panels.len(),
        active_panel,
        blocking_message: editor_state.blocking_message.clone(),
        diagnostics: editor_state.diagnostics.clone(),
        editor_state,
        panels,
    }
}

fn active_host_panel_kind(
    editor_state: &BerkeleyAppEditorStateSnapshot,
) -> BerkeleyAppHostPanelKind {
    if editor_state.blocking_message.is_some() {
        return BerkeleyAppHostPanelKind::Diagnostics;
    }

    match editor_state
        .active_command
        .as_ref()
        .map(|command| command.target.as_str())
    {
        Some("analysis-table") => BerkeleyAppHostPanelKind::Table,
        Some("analysis-waveform") => BerkeleyAppHostPanelKind::Waveform,
        Some("analysis-selection") | Some("analysis-runner") => BerkeleyAppHostPanelKind::Analysis,
        _ => BerkeleyAppHostPanelKind::Source,
    }
}

fn host_panel(
    kind: BerkeleyAppHostPanelKind,
    id: &str,
    title: &str,
    target: &str,
    enabled: bool,
    active: bool,
    disabled_reason: Option<String>,
) -> BerkeleyAppHostPanel {
    BerkeleyAppHostPanel {
        id: id.to_string(),
        kind,
        title: title.to_string(),
        target: target.to_string(),
        enabled,
        active,
        disabled_reason: if enabled { None } else { disabled_reason },
    }
}

fn host_panel_disabled_reason(
    editor_state: &BerkeleyAppEditorStateSnapshot,
    kind: BerkeleyAppEditorActionKind,
) -> Option<String> {
    let selected_syntax_card_index = editor_state.resolved_state.selected_syntax_card_index?;
    editor_state
        .command_plan
        .commands
        .iter()
        .find(|command| {
            command.syntax_card_index == selected_syntax_card_index && command.kind == kind
        })
        .and_then(|command| command.disabled_reason.clone())
        .or_else(|| Some("selected analysis does not expose this panel".to_string()))
}

impl From<BerkeleyAppHostSurface> for BerkeleyAppHostSurfaceWire {
    fn from(surface: BerkeleyAppHostSurface) -> Self {
        let active_panel_id = surface.active_panel.as_ref().map(|panel| panel.id.clone());
        let requested_state = surface.editor_state.requested_state.clone();
        let resolved_state = surface.editor_state.resolved_state.clone();

        Self {
            schema_version: BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION,
            canonical_source: surface.canonical_source,
            source_fingerprint: surface.source_fingerprint,
            title: surface.title,
            parsed: surface.parsed,
            execution_available: surface.execution_available,
            requested_selected_syntax_card_index: requested_state.selected_syntax_card_index,
            requested_active_command_id: requested_state.active_command_id,
            resolved_selected_syntax_card_index: resolved_state.selected_syntax_card_index,
            resolved_active_command_id: resolved_state.active_command_id,
            selection_stale: surface.editor_state.selection_stale,
            command_stale: surface.editor_state.command_stale,
            panel_count: surface.panel_count,
            active_panel_id,
            panels: surface
                .panels
                .into_iter()
                .map(BerkeleyAppHostPanelWire::from)
                .collect(),
            blocking_message: surface.blocking_message,
            diagnostics: surface
                .diagnostics
                .into_iter()
                .map(BerkeleyAppHostDiagnosticWire::from)
                .collect(),
        }
    }
}

impl From<BerkeleyAppHostPanel> for BerkeleyAppHostPanelWire {
    fn from(panel: BerkeleyAppHostPanel) -> Self {
        Self {
            id: panel.id,
            kind: host_panel_kind_wire(panel.kind).to_string(),
            title: panel.title,
            target: panel.target,
            enabled: panel.enabled,
            active: panel.active,
            disabled_reason: panel.disabled_reason,
        }
    }
}

impl From<BerkeleySyntaxDiagnostic> for BerkeleyAppHostDiagnosticWire {
    fn from(diagnostic: BerkeleySyntaxDiagnostic) -> Self {
        Self {
            code: diagnostic.code,
            severity: diagnostic_severity_wire(diagnostic.severity).to_string(),
            message: diagnostic.message,
            span: diagnostic.span.map(BerkeleyAppHostSpanWire::from),
        }
    }
}

impl From<SourceSpan> for BerkeleyAppHostSpanWire {
    fn from(span: SourceSpan) -> Self {
        Self {
            start_line: span.start_line,
            start_column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }
    }
}

fn host_panel_kind_wire(kind: BerkeleyAppHostPanelKind) -> &'static str {
    match kind {
        BerkeleyAppHostPanelKind::Source => "source",
        BerkeleyAppHostPanelKind::Diagnostics => "diagnostics",
        BerkeleyAppHostPanelKind::Analysis => "analysis",
        BerkeleyAppHostPanelKind::Table => "table",
        BerkeleyAppHostPanelKind::Waveform => "waveform",
    }
}

fn diagnostic_severity_wire(severity: BerkeleyDiagnosticSeverity) -> &'static str {
    match severity {
        BerkeleyDiagnosticSeverity::Error => "error",
        BerkeleyDiagnosticSeverity::Warning => "warning",
        BerkeleyDiagnosticSeverity::Note => "note",
    }
}

fn host_surface_wire_json_value(wire: &BerkeleyAppHostSurfaceWire) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": wire.schema_version,
        "canonicalSource": &wire.canonical_source,
        "sourceFingerprint": &wire.source_fingerprint,
        "title": &wire.title,
        "parsed": wire.parsed,
        "executionAvailable": wire.execution_available,
        "requestedSelectedSyntaxCardIndex": wire.requested_selected_syntax_card_index,
        "requestedActiveCommandId": &wire.requested_active_command_id,
        "resolvedSelectedSyntaxCardIndex": wire.resolved_selected_syntax_card_index,
        "resolvedActiveCommandId": &wire.resolved_active_command_id,
        "selectionStale": wire.selection_stale,
        "commandStale": wire.command_stale,
        "panelCount": wire.panel_count,
        "activePanelId": &wire.active_panel_id,
        "panels": wire
            .panels
            .iter()
            .map(host_panel_wire_json_value)
            .collect::<Vec<_>>(),
        "blockingMessage": &wire.blocking_message,
        "diagnostics": wire
            .diagnostics
            .iter()
            .map(host_diagnostic_wire_json_value)
            .collect::<Vec<_>>(),
    })
}

fn host_panel_wire_json_value(panel: &BerkeleyAppHostPanelWire) -> serde_json::Value {
    serde_json::json!({
        "id": &panel.id,
        "kind": &panel.kind,
        "title": &panel.title,
        "target": &panel.target,
        "enabled": panel.enabled,
        "active": panel.active,
        "disabledReason": &panel.disabled_reason,
    })
}

fn host_diagnostic_wire_json_value(
    diagnostic: &BerkeleyAppHostDiagnosticWire,
) -> serde_json::Value {
    serde_json::json!({
        "code": &diagnostic.code,
        "severity": &diagnostic.severity,
        "message": &diagnostic.message,
        "span": diagnostic
            .span
            .as_ref()
            .map(host_span_wire_json_value),
    })
}

fn host_span_wire_json_value(span: &BerkeleyAppHostSpanWire) -> serde_json::Value {
    serde_json::json!({
        "startLine": span.start_line,
        "startColumn": span.start_column,
        "endLine": span.end_line,
        "endColumn": span.end_column,
    })
}

fn app_package_manifest_json_value(manifest: &BerkeleyAppPackageManifest) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": manifest.schema_version,
        "packageName": &manifest.package_name,
        "grammarName": &manifest.grammar_name,
        "grammarVersion": manifest.grammar_version,
        "hostSurfaceWireSchemaVersion": manifest.host_surface_wire_schema_version,
        "sourceFingerprintAlgorithm": &manifest.source_fingerprint_algorithm,
        "hostPanelKinds": &manifest.host_panel_kinds,
        "editorActionKinds": &manifest.editor_action_kinds,
        "commandTargets": &manifest.command_targets,
        "runnableAnalysisDirectives": &manifest.runnable_analysis_directives,
        "artifactAnalysisDirectives": &manifest.artifact_analysis_directives,
        "artifactCapabilities": &manifest.artifact_capabilities,
    })
}

fn app_bootstrap_snapshot_json_value(snapshot: &BerkeleyAppBootstrapSnapshot) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": snapshot.schema_version,
        "packageManifest": app_package_manifest_json_value(&snapshot.package_manifest),
        "hostSurface": host_surface_wire_json_value(&snapshot.host_surface),
    })
}

fn app_startup_summary_json_value(summary: &BerkeleyAppStartupSummary) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": summary.schema_version,
        "packageName": &summary.package_name,
        "sourceFingerprint": &summary.source_fingerprint,
        "title": &summary.title,
        "parsed": summary.parsed,
        "executionAvailable": summary.execution_available,
        "ready": summary.ready,
        "requestedSelectedSyntaxCardIndex": summary.requested_selected_syntax_card_index,
        "requestedActiveCommandId": &summary.requested_active_command_id,
        "resolvedSelectedSyntaxCardIndex": summary.resolved_selected_syntax_card_index,
        "resolvedActiveCommandId": &summary.resolved_active_command_id,
        "selectionStale": summary.selection_stale,
        "commandStale": summary.command_stale,
        "panelCount": summary.panel_count,
        "activePanelId": &summary.active_panel_id,
        "diagnosticCount": summary.diagnostic_count,
        "blockingMessage": &summary.blocking_message,
    })
}

fn manifest_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn editor_command_from_control(
    control: &BerkeleyAppAnalysisControl,
    action: &BerkeleyAppEditorAction,
) -> BerkeleyAppEditorCommand {
    BerkeleyAppEditorCommand {
        id: editor_command_id(control.syntax_card_index, action.kind),
        kind: action.kind,
        syntax_card_index: control.syntax_card_index,
        directive: control.directive.clone(),
        analysis: control.analysis.clone(),
        span: control.span,
        target: editor_command_target(action.kind).to_string(),
        label: action.label.clone(),
        enabled: action.enabled,
        selected: control.selected,
        disabled_reason: action.disabled_reason.clone(),
    }
}

fn editor_command_id(syntax_card_index: usize, kind: BerkeleyAppEditorActionKind) -> String {
    format!(
        "analysis.{}.{}",
        syntax_card_index,
        editor_command_slug(kind)
    )
}

fn editor_command_slug(kind: BerkeleyAppEditorActionKind) -> &'static str {
    match kind {
        BerkeleyAppEditorActionKind::SelectAnalysis => "select",
        BerkeleyAppEditorActionKind::RunAnalysis => "run",
        BerkeleyAppEditorActionKind::InspectTable => "inspect-table",
        BerkeleyAppEditorActionKind::InspectWaveform => "inspect-waveform",
    }
}

fn editor_command_target(kind: BerkeleyAppEditorActionKind) -> &'static str {
    match kind {
        BerkeleyAppEditorActionKind::SelectAnalysis => "analysis-selection",
        BerkeleyAppEditorActionKind::RunAnalysis => "analysis-runner",
        BerkeleyAppEditorActionKind::InspectTable => "analysis-table",
        BerkeleyAppEditorActionKind::InspectWaveform => "analysis-waveform",
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
