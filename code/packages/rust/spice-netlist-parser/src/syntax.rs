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
pub const BERKELEY_APP_LAUNCH_PLAN_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_READINESS_REPORT_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_HANDOFF_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_STATUS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_TELEMETRY_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_EVENT_LOG_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_EVENT_SUMMARY_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_EVENT_DIGEST_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_EVENT_DASHBOARD_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_PACKAGE_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_CARDS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_VIEW_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_LAYOUT_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_NAVIGATION_SCHEMA_VERSION: u32 = 1;
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
    "app-launch-plan-json",
    "app-readiness-report-json",
    "app-shell-handoff-json",
    "app-shell-status-json",
    "app-shell-telemetry-json",
    "app-shell-events-json",
    "app-shell-event-summary-json",
    "app-shell-event-digest-json",
    "app-shell-event-dashboard-json",
    "app-shell-dashboard-package-json",
    "app-shell-dashboard-cards-json",
    "app-shell-dashboard-view-json",
    "app-shell-dashboard-layout-json",
    "app-shell-dashboard-navigation-json",
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
pub struct BerkeleyAppLaunchAction {
    pub id: String,
    pub label: String,
    pub panel_id: String,
    pub panel_kind: String,
    pub target: String,
    pub enabled: bool,
    pub primary: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppLaunchPlan {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub entry_panel_id: Option<String>,
    pub entry_panel_kind: Option<String>,
    pub entry_target: Option<String>,
    pub requested_selected_syntax_card_index: Option<usize>,
    pub requested_active_command_id: Option<String>,
    pub resolved_selected_syntax_card_index: Option<usize>,
    pub resolved_active_command_id: Option<String>,
    pub selection_stale: bool,
    pub command_stale: bool,
    pub action_count: usize,
    pub actions: Vec<BerkeleyAppLaunchAction>,
    pub diagnostic_count: usize,
    pub blocking_message: Option<String>,
}

impl BerkeleyAppLaunchPlan {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        let host_surface = &snapshot.host_surface;
        let ready = host_surface.parsed && host_surface.execution_available;
        let startup_route = if ready { "ready" } else { "blocked" }.to_string();
        let entry_panel = launch_entry_panel(host_surface, ready);
        let entry_panel_id = entry_panel.map(|panel| panel.id.clone());
        let entry_panel_kind = entry_panel.map(|panel| panel.kind.clone());
        let entry_target = entry_panel.map(|panel| panel.target.clone());
        let actions = host_surface
            .panels
            .iter()
            .map(|panel| launch_action_from_panel(panel, entry_panel_id.as_deref()))
            .collect::<Vec<_>>();

        Self {
            schema_version: BERKELEY_APP_LAUNCH_PLAN_SCHEMA_VERSION,
            package_name: snapshot.package_manifest.package_name.clone(),
            source_fingerprint: host_surface.source_fingerprint.clone(),
            title: host_surface.title.clone(),
            startup_route,
            ready,
            entry_panel_id,
            entry_panel_kind,
            entry_target,
            requested_selected_syntax_card_index: host_surface.requested_selected_syntax_card_index,
            requested_active_command_id: host_surface.requested_active_command_id.clone(),
            resolved_selected_syntax_card_index: host_surface.resolved_selected_syntax_card_index,
            resolved_active_command_id: host_surface.resolved_active_command_id.clone(),
            selection_stale: host_surface.selection_stale,
            command_stale: host_surface.command_stale,
            action_count: actions.len(),
            actions,
            diagnostic_count: host_surface.diagnostics.len(),
            blocking_message: host_surface.blocking_message.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        app_launch_plan_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppReadinessReport {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub parsed: bool,
    pub execution_available: bool,
    pub entry_panel_id: Option<String>,
    pub entry_target: Option<String>,
    pub primary_action_id: Option<String>,
    pub primary_action_enabled: bool,
    pub panel_count: usize,
    pub enabled_panel_count: usize,
    pub disabled_panel_count: usize,
    pub action_count: usize,
    pub enabled_action_count: usize,
    pub disabled_action_count: usize,
    pub diagnostic_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub note_count: usize,
    pub selection_stale: bool,
    pub command_stale: bool,
    pub repaired_state: bool,
    pub blocking_message: Option<String>,
}

impl BerkeleyAppReadinessReport {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        let host_surface = &snapshot.host_surface;
        let launch_plan = BerkeleyAppLaunchPlan::from_bootstrap_snapshot(snapshot);
        let enabled_panel_count = host_surface
            .panels
            .iter()
            .filter(|panel| panel.enabled)
            .count();
        let primary_action = launch_plan.actions.iter().find(|action| action.primary);
        let primary_action_id = primary_action.map(|action| action.id.clone());
        let primary_action_enabled = primary_action.is_some_and(|action| action.enabled);
        let enabled_action_count = launch_plan
            .actions
            .iter()
            .filter(|action| action.enabled)
            .count();
        let error_count = host_surface
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "error")
            .count();
        let warning_count = host_surface
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "warning")
            .count();
        let note_count = host_surface
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "note")
            .count();
        let repaired_state = launch_plan.selection_stale || launch_plan.command_stale;

        Self {
            schema_version: BERKELEY_APP_READINESS_REPORT_SCHEMA_VERSION,
            package_name: launch_plan.package_name,
            source_fingerprint: launch_plan.source_fingerprint,
            title: launch_plan.title,
            startup_route: launch_plan.startup_route,
            ready: launch_plan.ready,
            parsed: host_surface.parsed,
            execution_available: host_surface.execution_available,
            entry_panel_id: launch_plan.entry_panel_id,
            entry_target: launch_plan.entry_target,
            primary_action_id,
            primary_action_enabled,
            panel_count: host_surface.panel_count,
            enabled_panel_count,
            disabled_panel_count: host_surface.panel_count.saturating_sub(enabled_panel_count),
            action_count: launch_plan.action_count,
            enabled_action_count,
            disabled_action_count: launch_plan
                .action_count
                .saturating_sub(enabled_action_count),
            diagnostic_count: host_surface.diagnostics.len(),
            error_count,
            warning_count,
            note_count,
            selection_stale: launch_plan.selection_stale,
            command_stale: launch_plan.command_stale,
            repaired_state,
            blocking_message: launch_plan.blocking_message,
        }
    }

    pub fn to_json(&self) -> String {
        app_readiness_report_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellHandoff {
    pub schema_version: u32,
    pub package_manifest: BerkeleyAppPackageManifest,
    pub startup_summary: BerkeleyAppStartupSummary,
    pub launch_plan: BerkeleyAppLaunchPlan,
    pub readiness_report: BerkeleyAppReadinessReport,
}

impl BerkeleyAppShellHandoff {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self {
            schema_version: BERKELEY_APP_SHELL_HANDOFF_SCHEMA_VERSION,
            package_manifest: snapshot.package_manifest.clone(),
            startup_summary: BerkeleyAppStartupSummary::from_bootstrap_snapshot(snapshot),
            launch_plan: BerkeleyAppLaunchPlan::from_bootstrap_snapshot(snapshot),
            readiness_report: BerkeleyAppReadinessReport::from_bootstrap_snapshot(snapshot),
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_handoff_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellStatus {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub message: String,
    pub entry_panel_id: Option<String>,
    pub entry_target: Option<String>,
    pub primary_action_id: Option<String>,
    pub diagnostic_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub note_count: usize,
    pub blocking_message: Option<String>,
}

impl BerkeleyAppShellStatus {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_shell_handoff(&BerkeleyAppShellHandoff::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        let readiness = &handoff.readiness_report;
        let severity = shell_status_severity(readiness).to_string();
        let message = shell_status_message(readiness);

        Self {
            schema_version: BERKELEY_APP_SHELL_STATUS_SCHEMA_VERSION,
            package_name: readiness.package_name.clone(),
            source_fingerprint: readiness.source_fingerprint.clone(),
            title: readiness.title.clone(),
            startup_route: readiness.startup_route.clone(),
            ready: readiness.ready,
            severity,
            message,
            entry_panel_id: readiness.entry_panel_id.clone(),
            entry_target: readiness.entry_target.clone(),
            primary_action_id: readiness.primary_action_id.clone(),
            diagnostic_count: readiness.diagnostic_count,
            error_count: readiness.error_count,
            warning_count: readiness.warning_count,
            note_count: readiness.note_count,
            blocking_message: readiness.blocking_message.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_status_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellTelemetry {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub message: String,
    pub entry_panel_id: Option<String>,
    pub primary_action_id: Option<String>,
    pub panel_count: usize,
    pub enabled_panel_count: usize,
    pub disabled_panel_count: usize,
    pub action_count: usize,
    pub enabled_action_count: usize,
    pub disabled_action_count: usize,
    pub diagnostic_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub note_count: usize,
    pub selection_stale: bool,
    pub command_stale: bool,
    pub repaired_state: bool,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellTelemetry {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_shell_handoff(&BerkeleyAppShellHandoff::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        let status = BerkeleyAppShellStatus::from_shell_handoff(handoff);
        let readiness = &handoff.readiness_report;

        Self {
            schema_version: BERKELEY_APP_SHELL_TELEMETRY_SCHEMA_VERSION,
            package_name: status.package_name,
            source_fingerprint: status.source_fingerprint,
            title: status.title,
            startup_route: status.startup_route,
            ready: status.ready,
            severity: status.severity,
            message: status.message,
            entry_panel_id: status.entry_panel_id,
            primary_action_id: status.primary_action_id,
            panel_count: readiness.panel_count,
            enabled_panel_count: readiness.enabled_panel_count,
            disabled_panel_count: readiness.disabled_panel_count,
            action_count: readiness.action_count,
            enabled_action_count: readiness.enabled_action_count,
            disabled_action_count: readiness.disabled_action_count,
            diagnostic_count: readiness.diagnostic_count,
            error_count: readiness.error_count,
            warning_count: readiness.warning_count,
            note_count: readiness.note_count,
            selection_stale: readiness.selection_stale,
            command_stale: readiness.command_stale,
            repaired_state: readiness.repaired_state,
            artifact_capability_count: handoff.package_manifest.artifact_capabilities.len(),
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_telemetry_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellEvent {
    pub id: String,
    pub kind: String,
    pub severity: String,
    pub message: String,
    pub panel_id: Option<String>,
    pub action_id: Option<String>,
    pub count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellEventLog {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub event_count: usize,
    pub events: Vec<BerkeleyAppShellEvent>,
}

impl BerkeleyAppShellEventLog {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_shell_handoff(&BerkeleyAppShellHandoff::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        let status = BerkeleyAppShellStatus::from_shell_handoff(handoff);
        let telemetry = BerkeleyAppShellTelemetry::from_shell_handoff(handoff);
        let readiness = &handoff.readiness_report;
        let primary_action_enabled = readiness.primary_action_enabled;
        let repaired_state_count = if readiness.repaired_state { 1 } else { 0 };
        let diagnostics_severity = if readiness.error_count > 0 {
            "error"
        } else if readiness.warning_count > 0 {
            "warning"
        } else {
            "info"
        };
        let state_severity = if readiness.repaired_state {
            "warning"
        } else {
            "info"
        };

        let events = vec![
            BerkeleyAppShellEvent {
                id: "shell.status".to_string(),
                kind: "status".to_string(),
                severity: status.severity.clone(),
                message: status.message.clone(),
                panel_id: status.entry_panel_id.clone(),
                action_id: status.primary_action_id.clone(),
                count: None,
            },
            BerkeleyAppShellEvent {
                id: format!("shell.route.{}", status.startup_route),
                kind: "route".to_string(),
                severity: status.severity.clone(),
                message: if status.ready {
                    "Ready startup route selected".to_string()
                } else {
                    "Blocked startup route selected".to_string()
                },
                panel_id: status.entry_panel_id.clone(),
                action_id: status.primary_action_id.clone(),
                count: None,
            },
            BerkeleyAppShellEvent {
                id: "shell.action.primary".to_string(),
                kind: "action".to_string(),
                severity: if primary_action_enabled {
                    "ready".to_string()
                } else {
                    "blocked".to_string()
                },
                message: match &status.primary_action_id {
                    Some(action_id) if primary_action_enabled => {
                        format!("Primary action {action_id} enabled")
                    }
                    Some(action_id) => format!("Primary action {action_id} disabled"),
                    None => "No primary action available".to_string(),
                },
                panel_id: status.entry_panel_id.clone(),
                action_id: status.primary_action_id.clone(),
                count: None,
            },
            BerkeleyAppShellEvent {
                id: "shell.diagnostics".to_string(),
                kind: "diagnostics".to_string(),
                severity: diagnostics_severity.to_string(),
                message: format!(
                    "{} diagnostics: {} errors, {} warnings, {} notes",
                    readiness.diagnostic_count,
                    readiness.error_count,
                    readiness.warning_count,
                    readiness.note_count
                ),
                panel_id: None,
                action_id: None,
                count: Some(readiness.diagnostic_count),
            },
            BerkeleyAppShellEvent {
                id: "shell.state".to_string(),
                kind: "state".to_string(),
                severity: state_severity.to_string(),
                message: if readiness.repaired_state {
                    "Persisted editor state repaired".to_string()
                } else {
                    "Persisted editor state current".to_string()
                },
                panel_id: readiness.entry_panel_id.clone(),
                action_id: readiness.primary_action_id.clone(),
                count: Some(repaired_state_count),
            },
            BerkeleyAppShellEvent {
                id: "shell.capabilities".to_string(),
                kind: "capability".to_string(),
                severity: "info".to_string(),
                message: format!(
                    "{} artifact capabilities advertised",
                    telemetry.artifact_capability_count
                ),
                panel_id: None,
                action_id: None,
                count: Some(telemetry.artifact_capability_count),
            },
        ];

        let event_count = events.len();
        Self {
            schema_version: BERKELEY_APP_SHELL_EVENT_LOG_SCHEMA_VERSION,
            package_name: status.package_name,
            source_fingerprint: status.source_fingerprint,
            title: status.title,
            startup_route: status.startup_route,
            ready: status.ready,
            event_count,
            events,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_event_log_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellEventSummary {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub status_event_id: Option<String>,
    pub primary_action_id: Option<String>,
    pub event_count: usize,
    pub status_event_count: usize,
    pub route_event_count: usize,
    pub action_event_count: usize,
    pub diagnostic_event_count: usize,
    pub state_event_count: usize,
    pub capability_event_count: usize,
    pub ready_event_count: usize,
    pub blocked_event_count: usize,
    pub info_event_count: usize,
    pub warning_event_count: usize,
    pub error_event_count: usize,
    pub counted_event_total: usize,
    pub diagnostic_count: usize,
    pub repaired_state_count: usize,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellEventSummary {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_event_log(&BerkeleyAppShellEventLog::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_event_log(&BerkeleyAppShellEventLog::from_shell_handoff(handoff))
    }

    pub fn from_event_log(event_log: &BerkeleyAppShellEventLog) -> Self {
        let status_event = event_log.events.iter().find(|event| event.kind == "status");
        let action_event = event_log.events.iter().find(|event| event.kind == "action");
        let diagnostic_event = event_log
            .events
            .iter()
            .find(|event| event.id == "shell.diagnostics");
        let state_event = event_log
            .events
            .iter()
            .find(|event| event.id == "shell.state");
        let capability_event = event_log
            .events
            .iter()
            .find(|event| event.id == "shell.capabilities");

        let mut status_event_count = 0;
        let mut route_event_count = 0;
        let mut action_event_count = 0;
        let mut diagnostic_event_count = 0;
        let mut state_event_count = 0;
        let mut capability_event_count = 0;
        let mut ready_event_count = 0;
        let mut blocked_event_count = 0;
        let mut info_event_count = 0;
        let mut warning_event_count = 0;
        let mut error_event_count = 0;
        let mut counted_event_total = 0;

        for event in &event_log.events {
            match event.kind.as_str() {
                "status" => status_event_count += 1,
                "route" => route_event_count += 1,
                "action" => action_event_count += 1,
                "diagnostics" => diagnostic_event_count += 1,
                "state" => state_event_count += 1,
                "capability" => capability_event_count += 1,
                _ => {}
            }

            match event.severity.as_str() {
                "ready" => ready_event_count += 1,
                "blocked" => blocked_event_count += 1,
                "info" => info_event_count += 1,
                "warning" => warning_event_count += 1,
                "error" => error_event_count += 1,
                _ => {}
            }

            counted_event_total += event.count.unwrap_or(0);
        }

        Self {
            schema_version: BERKELEY_APP_SHELL_EVENT_SUMMARY_SCHEMA_VERSION,
            package_name: event_log.package_name.clone(),
            source_fingerprint: event_log.source_fingerprint.clone(),
            title: event_log.title.clone(),
            startup_route: event_log.startup_route.clone(),
            ready: event_log.ready,
            severity: status_event
                .map(|event| event.severity.clone())
                .unwrap_or_else(|| {
                    if event_log.ready {
                        "ready".to_string()
                    } else {
                        "blocked".to_string()
                    }
                }),
            status_event_id: status_event.map(|event| event.id.clone()),
            primary_action_id: action_event.and_then(|event| event.action_id.clone()),
            event_count: event_log.event_count,
            status_event_count,
            route_event_count,
            action_event_count,
            diagnostic_event_count,
            state_event_count,
            capability_event_count,
            ready_event_count,
            blocked_event_count,
            info_event_count,
            warning_event_count,
            error_event_count,
            counted_event_total,
            diagnostic_count: diagnostic_event.and_then(|event| event.count).unwrap_or(0),
            repaired_state_count: state_event.and_then(|event| event.count).unwrap_or(0),
            artifact_capability_count: capability_event.and_then(|event| event.count).unwrap_or(0),
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_event_summary_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellEventDigest {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub headline_event_id: Option<String>,
    pub headline_message: String,
    pub primary_action_id: Option<String>,
    pub attention_event_count: usize,
    pub attention_event_ids: Vec<String>,
    pub metric_event_count: usize,
    pub metric_event_ids: Vec<String>,
    pub event_count: usize,
    pub counted_event_total: usize,
    pub diagnostic_count: usize,
    pub repaired_state_count: usize,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellEventDigest {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_event_log(&BerkeleyAppShellEventLog::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_event_log(&BerkeleyAppShellEventLog::from_shell_handoff(handoff))
    }

    pub fn from_event_log(event_log: &BerkeleyAppShellEventLog) -> Self {
        let summary = BerkeleyAppShellEventSummary::from_event_log(event_log);
        let headline_event = summary
            .status_event_id
            .as_ref()
            .and_then(|status_event_id| {
                event_log
                    .events
                    .iter()
                    .find(|event| event.id == *status_event_id)
            });
        let attention_event_ids = event_log
            .events
            .iter()
            .filter(|event| matches!(event.severity.as_str(), "blocked" | "warning" | "error"))
            .map(|event| event.id.clone())
            .collect::<Vec<_>>();
        let metric_event_ids = event_log
            .events
            .iter()
            .filter(|event| event.count.is_some())
            .map(|event| event.id.clone())
            .collect::<Vec<_>>();

        Self {
            schema_version: BERKELEY_APP_SHELL_EVENT_DIGEST_SCHEMA_VERSION,
            package_name: summary.package_name,
            source_fingerprint: summary.source_fingerprint,
            title: summary.title,
            startup_route: summary.startup_route,
            ready: summary.ready,
            severity: summary.severity,
            headline_event_id: headline_event.map(|event| event.id.clone()),
            headline_message: headline_event
                .map(|event| event.message.clone())
                .unwrap_or_else(|| {
                    if event_log.ready {
                        "Berkeley SPICE Mosaic app ready".to_string()
                    } else {
                        "Berkeley SPICE Mosaic app blocked".to_string()
                    }
                }),
            primary_action_id: summary.primary_action_id,
            attention_event_count: attention_event_ids.len(),
            attention_event_ids,
            metric_event_count: metric_event_ids.len(),
            metric_event_ids,
            event_count: summary.event_count,
            counted_event_total: summary.counted_event_total,
            diagnostic_count: summary.diagnostic_count,
            repaired_state_count: summary.repaired_state_count,
            artifact_capability_count: summary.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_event_digest_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellEventDashboardSection {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub event_count: usize,
    pub event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellEventDashboard {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub headline_event_id: Option<String>,
    pub headline_message: String,
    pub primary_action_id: Option<String>,
    pub attention_required: bool,
    pub section_count: usize,
    pub sections: Vec<BerkeleyAppShellEventDashboardSection>,
    pub event_count: usize,
    pub diagnostic_count: usize,
    pub repaired_state_count: usize,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellEventDashboard {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_event_log(&BerkeleyAppShellEventLog::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_event_log(&BerkeleyAppShellEventLog::from_shell_handoff(handoff))
    }

    pub fn from_event_log(event_log: &BerkeleyAppShellEventLog) -> Self {
        Self::from_event_digest(&BerkeleyAppShellEventDigest::from_event_log(event_log))
    }

    pub fn from_event_digest(digest: &BerkeleyAppShellEventDigest) -> Self {
        let status_event_ids = digest.headline_event_id.iter().cloned().collect::<Vec<_>>();
        let sections = vec![
            BerkeleyAppShellEventDashboardSection {
                id: "status".to_string(),
                title: "Startup status".to_string(),
                severity: digest.severity.clone(),
                event_count: status_event_ids.len(),
                event_ids: status_event_ids,
            },
            BerkeleyAppShellEventDashboardSection {
                id: "attention".to_string(),
                title: "Attention".to_string(),
                severity: if digest.attention_event_count > 0 {
                    digest.severity.clone()
                } else {
                    "ready".to_string()
                },
                event_count: digest.attention_event_count,
                event_ids: digest.attention_event_ids.clone(),
            },
            BerkeleyAppShellEventDashboardSection {
                id: "metrics".to_string(),
                title: "Metrics".to_string(),
                severity: "info".to_string(),
                event_count: digest.metric_event_count,
                event_ids: digest.metric_event_ids.clone(),
            },
        ];
        let section_count = sections.len();

        Self {
            schema_version: BERKELEY_APP_SHELL_EVENT_DASHBOARD_SCHEMA_VERSION,
            package_name: digest.package_name.clone(),
            source_fingerprint: digest.source_fingerprint.clone(),
            title: digest.title.clone(),
            startup_route: digest.startup_route.clone(),
            ready: digest.ready,
            severity: digest.severity.clone(),
            headline_event_id: digest.headline_event_id.clone(),
            headline_message: digest.headline_message.clone(),
            primary_action_id: digest.primary_action_id.clone(),
            attention_required: digest.attention_event_count > 0,
            section_count,
            sections,
            event_count: digest.event_count,
            diagnostic_count: digest.diagnostic_count,
            repaired_state_count: digest.repaired_state_count,
            artifact_capability_count: digest.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_event_dashboard_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardPackage {
    pub schema_version: u32,
    pub package_manifest: BerkeleyAppPackageManifest,
    pub event_dashboard: BerkeleyAppShellEventDashboard,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub section_count: usize,
    pub artifact_capability_count: usize,
    pub dashboard_capability_id: String,
    pub package_capability_id: String,
}

impl BerkeleyAppShellDashboardPackage {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_package_and_dashboard(
            snapshot.package_manifest.clone(),
            BerkeleyAppShellEventDashboard::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_package_and_dashboard(
            handoff.package_manifest.clone(),
            BerkeleyAppShellEventDashboard::from_shell_handoff(handoff),
        )
    }

    fn from_package_and_dashboard(
        package_manifest: BerkeleyAppPackageManifest,
        event_dashboard: BerkeleyAppShellEventDashboard,
    ) -> Self {
        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_PACKAGE_SCHEMA_VERSION,
            package_name: event_dashboard.package_name.clone(),
            source_fingerprint: event_dashboard.source_fingerprint.clone(),
            title: event_dashboard.title.clone(),
            startup_route: event_dashboard.startup_route.clone(),
            ready: event_dashboard.ready,
            severity: event_dashboard.severity.clone(),
            attention_required: event_dashboard.attention_required,
            section_count: event_dashboard.section_count,
            artifact_capability_count: package_manifest.artifact_capabilities.len(),
            dashboard_capability_id: "app-shell-event-dashboard-json".to_string(),
            package_capability_id: "app-shell-dashboard-package-json".to_string(),
            package_manifest,
            event_dashboard,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_package_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardCard {
    pub id: String,
    pub section_id: String,
    pub title: String,
    pub severity: String,
    pub event_count: usize,
    pub event_ids: Vec<String>,
    pub primary: bool,
    pub attention: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardCards {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub card_count: usize,
    pub primary_card_id: Option<String>,
    pub cards: Vec<BerkeleyAppShellDashboardCard>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardCards {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_package(&BerkeleyAppShellDashboardPackage::from_bootstrap_snapshot(
            snapshot,
        ))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_package(&BerkeleyAppShellDashboardPackage::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dashboard_package(package: &BerkeleyAppShellDashboardPackage) -> Self {
        let primary_section_id = if package.attention_required {
            "attention"
        } else {
            "status"
        };
        let cards = package
            .event_dashboard
            .sections
            .iter()
            .map(|section| {
                let primary = section.id == primary_section_id;
                let attention = section.id == "attention" && package.attention_required;
                BerkeleyAppShellDashboardCard {
                    id: format!("dashboard.{}", section.id),
                    section_id: section.id.clone(),
                    title: section.title.clone(),
                    severity: section.severity.clone(),
                    event_count: section.event_count,
                    event_ids: section.event_ids.clone(),
                    primary,
                    attention,
                }
            })
            .collect::<Vec<_>>();
        let primary_card_id = cards
            .iter()
            .find(|card| card.primary)
            .map(|card| card.id.clone());
        let card_count = cards.len();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_CARDS_SCHEMA_VERSION,
            package_name: package.package_name.clone(),
            source_fingerprint: package.source_fingerprint.clone(),
            title: package.title.clone(),
            startup_route: package.startup_route.clone(),
            ready: package.ready,
            severity: package.severity.clone(),
            attention_required: package.attention_required,
            card_count,
            primary_card_id,
            cards,
            package_capability_id: package.package_capability_id.clone(),
            dashboard_capability_id: package.dashboard_capability_id.clone(),
            cards_capability_id: "app-shell-dashboard-cards-json".to_string(),
            artifact_capability_count: package.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_cards_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardView {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub primary_card_id: Option<String>,
    pub primary_card_title: Option<String>,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub card_ids: Vec<String>,
    pub visible_card_ids: Vec<String>,
    pub attention_card_ids: Vec<String>,
    pub metric_card_ids: Vec<String>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardView {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_cards(&BerkeleyAppShellDashboardCards::from_bootstrap_snapshot(
            snapshot,
        ))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_cards(&BerkeleyAppShellDashboardCards::from_shell_handoff(handoff))
    }

    pub fn from_dashboard_cards(cards: &BerkeleyAppShellDashboardCards) -> Self {
        let primary_card_title = cards
            .primary_card_id
            .as_deref()
            .and_then(|primary_card_id| {
                cards
                    .cards
                    .iter()
                    .find(|card| card.id == primary_card_id)
                    .map(|card| card.title.clone())
            });
        let card_ids = cards
            .cards
            .iter()
            .map(|card| card.id.clone())
            .collect::<Vec<_>>();
        let visible_card_ids = cards
            .cards
            .iter()
            .filter(|card| card.primary || card.attention || card.event_count > 0)
            .map(|card| card.id.clone())
            .collect::<Vec<_>>();
        let attention_card_ids = cards
            .cards
            .iter()
            .filter(|card| card.attention)
            .map(|card| card.id.clone())
            .collect::<Vec<_>>();
        let metric_card_ids = cards
            .cards
            .iter()
            .filter(|card| card.section_id == "metrics")
            .map(|card| card.id.clone())
            .collect::<Vec<_>>();
        let visible_card_count = visible_card_ids.len();
        let attention_card_count = attention_card_ids.len();
        let metric_card_count = metric_card_ids.len();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_VIEW_SCHEMA_VERSION,
            package_name: cards.package_name.clone(),
            source_fingerprint: cards.source_fingerprint.clone(),
            title: cards.title.clone(),
            startup_route: cards.startup_route.clone(),
            ready: cards.ready,
            severity: cards.severity.clone(),
            attention_required: cards.attention_required,
            primary_card_id: cards.primary_card_id.clone(),
            primary_card_title,
            card_count: cards.card_count,
            visible_card_count,
            attention_card_count,
            metric_card_count,
            card_ids,
            visible_card_ids,
            attention_card_ids,
            metric_card_ids,
            package_capability_id: cards.package_capability_id.clone(),
            dashboard_capability_id: cards.dashboard_capability_id.clone(),
            cards_capability_id: cards.cards_capability_id.clone(),
            view_capability_id: "app-shell-dashboard-view-json".to_string(),
            artifact_capability_count: cards.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_view_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardLayoutRegion {
    pub id: String,
    pub role: String,
    pub title: String,
    pub card_ids: Vec<String>,
    pub primary: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardLayout {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub primary_card_id: Option<String>,
    pub primary_region_id: Option<String>,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub regions: Vec<BerkeleyAppShellDashboardLayoutRegion>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardLayout {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_cards(&BerkeleyAppShellDashboardCards::from_bootstrap_snapshot(
            snapshot,
        ))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_cards(&BerkeleyAppShellDashboardCards::from_shell_handoff(handoff))
    }

    pub fn from_dashboard_cards(cards: &BerkeleyAppShellDashboardCards) -> Self {
        let view = BerkeleyAppShellDashboardView::from_dashboard_cards(cards);
        let regions = ["status", "attention", "metrics"]
            .iter()
            .map(|role| {
                let card_ids = cards
                    .cards
                    .iter()
                    .filter(|card| card.section_id == *role)
                    .map(|card| card.id.clone())
                    .collect::<Vec<_>>();
                let primary = card_ids
                    .iter()
                    .any(|card_id| Some(card_id) == view.primary_card_id.as_ref());
                let visible = card_ids.iter().any(|card_id| {
                    view.visible_card_ids
                        .iter()
                        .any(|visible_id| visible_id == card_id)
                });
                BerkeleyAppShellDashboardLayoutRegion {
                    id: format!("dashboard.layout.{}", role),
                    role: (*role).to_string(),
                    title: match *role {
                        "status" => "Status".to_string(),
                        "attention" => "Attention".to_string(),
                        "metrics" => "Metrics".to_string(),
                        _ => (*role).to_string(),
                    },
                    card_ids,
                    primary,
                    visible,
                }
            })
            .collect::<Vec<_>>();
        let primary_region_id = regions
            .iter()
            .find(|region| region.primary)
            .map(|region| region.id.clone());
        let region_count = regions.len();
        let visible_region_count = regions.iter().filter(|region| region.visible).count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_LAYOUT_SCHEMA_VERSION,
            package_name: view.package_name,
            source_fingerprint: view.source_fingerprint,
            title: view.title,
            startup_route: view.startup_route,
            ready: view.ready,
            severity: view.severity,
            attention_required: view.attention_required,
            primary_card_id: view.primary_card_id,
            primary_region_id,
            region_count,
            visible_region_count,
            card_count: view.card_count,
            visible_card_count: view.visible_card_count,
            attention_card_count: view.attention_card_count,
            metric_card_count: view.metric_card_count,
            regions,
            package_capability_id: view.package_capability_id,
            dashboard_capability_id: view.dashboard_capability_id,
            cards_capability_id: view.cards_capability_id,
            view_capability_id: view.view_capability_id,
            layout_capability_id: "app-shell-dashboard-layout-json".to_string(),
            artifact_capability_count: view.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_layout_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardNavigationItem {
    pub id: String,
    pub region_id: String,
    pub role: String,
    pub label: String,
    pub card_ids: Vec<String>,
    pub active: bool,
    pub visible: bool,
    pub enabled: bool,
    pub badge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardNavigation {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub primary_card_id: Option<String>,
    pub primary_region_id: Option<String>,
    pub active_item_id: Option<String>,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub items: Vec<BerkeleyAppShellDashboardNavigationItem>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardNavigation {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_layout(&BerkeleyAppShellDashboardLayout::from_bootstrap_snapshot(
            snapshot,
        ))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_layout(&BerkeleyAppShellDashboardLayout::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dashboard_layout(layout: &BerkeleyAppShellDashboardLayout) -> Self {
        let items = layout
            .regions
            .iter()
            .map(|region| BerkeleyAppShellDashboardNavigationItem {
                id: format!("dashboard.nav.{}", region.role),
                region_id: region.id.clone(),
                role: region.role.clone(),
                label: region.title.clone(),
                card_ids: region.card_ids.clone(),
                active: region.primary,
                visible: region.visible,
                enabled: region.visible,
                badge_count: region.card_ids.len(),
            })
            .collect::<Vec<_>>();
        let active_item_id = items
            .iter()
            .find(|item| item.active)
            .map(|item| item.id.clone());
        let item_count = items.len();
        let visible_item_count = items.iter().filter(|item| item.visible).count();
        let enabled_item_count = items.iter().filter(|item| item.enabled).count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_NAVIGATION_SCHEMA_VERSION,
            package_name: layout.package_name.clone(),
            source_fingerprint: layout.source_fingerprint.clone(),
            title: layout.title.clone(),
            startup_route: layout.startup_route.clone(),
            ready: layout.ready,
            severity: layout.severity.clone(),
            attention_required: layout.attention_required,
            primary_card_id: layout.primary_card_id.clone(),
            primary_region_id: layout.primary_region_id.clone(),
            active_item_id,
            item_count,
            visible_item_count,
            enabled_item_count,
            region_count: layout.region_count,
            visible_region_count: layout.visible_region_count,
            card_count: layout.card_count,
            visible_card_count: layout.visible_card_count,
            attention_card_count: layout.attention_card_count,
            metric_card_count: layout.metric_card_count,
            items,
            package_capability_id: layout.package_capability_id.clone(),
            dashboard_capability_id: layout.dashboard_capability_id.clone(),
            cards_capability_id: layout.cards_capability_id.clone(),
            view_capability_id: layout.view_capability_id.clone(),
            layout_capability_id: layout.layout_capability_id.clone(),
            navigation_capability_id: "app-shell-dashboard-navigation-json".to_string(),
            artifact_capability_count: layout.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_navigation_json_value(self).to_string()
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

    pub fn app_launch_plan(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppLaunchPlan {
        BerkeleyAppLaunchPlan::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_launch_plan(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppLaunchPlan, AnalysisExecutionError> {
        Ok(BerkeleyAppLaunchPlan::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_launch_plan_json(&self, persisted_state: BerkeleyAppPersistedEditorState) -> String {
        self.app_launch_plan(persisted_state).to_json()
    }

    pub fn run_app_launch_plan_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_launch_plan(persisted_state)?.to_json())
    }

    pub fn app_readiness_report(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppReadinessReport {
        BerkeleyAppReadinessReport::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_readiness_report(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppReadinessReport, AnalysisExecutionError> {
        Ok(BerkeleyAppReadinessReport::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_readiness_report_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_readiness_report(persisted_state).to_json()
    }

    pub fn run_app_readiness_report_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_readiness_report(persisted_state)?.to_json())
    }

    pub fn app_shell_handoff(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellHandoff {
        BerkeleyAppShellHandoff::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_handoff(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellHandoff, AnalysisExecutionError> {
        Ok(BerkeleyAppShellHandoff::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_handoff_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_handoff(persisted_state).to_json()
    }

    pub fn run_app_shell_handoff_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_shell_handoff(persisted_state)?.to_json())
    }

    pub fn app_shell_status(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellStatus {
        BerkeleyAppShellStatus::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_status(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellStatus, AnalysisExecutionError> {
        Ok(BerkeleyAppShellStatus::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_status_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_status(persisted_state).to_json()
    }

    pub fn run_app_shell_status_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_shell_status(persisted_state)?.to_json())
    }

    pub fn app_shell_telemetry(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellTelemetry {
        BerkeleyAppShellTelemetry::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_telemetry(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellTelemetry, AnalysisExecutionError> {
        Ok(BerkeleyAppShellTelemetry::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_telemetry_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_telemetry(persisted_state).to_json()
    }

    pub fn run_app_shell_telemetry_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_shell_telemetry(persisted_state)?.to_json())
    }

    pub fn app_shell_event_log(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellEventLog {
        BerkeleyAppShellEventLog::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_event_log(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellEventLog, AnalysisExecutionError> {
        Ok(BerkeleyAppShellEventLog::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_event_log_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_event_log(persisted_state).to_json()
    }

    pub fn run_app_shell_event_log_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_shell_event_log(persisted_state)?.to_json())
    }

    pub fn app_shell_event_summary(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellEventSummary {
        BerkeleyAppShellEventSummary::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_event_summary(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellEventSummary, AnalysisExecutionError> {
        Ok(BerkeleyAppShellEventSummary::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_event_summary_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_event_summary(persisted_state).to_json()
    }

    pub fn run_app_shell_event_summary_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_shell_event_summary(persisted_state)?.to_json())
    }

    pub fn app_shell_event_digest(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellEventDigest {
        BerkeleyAppShellEventDigest::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_event_digest(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellEventDigest, AnalysisExecutionError> {
        Ok(BerkeleyAppShellEventDigest::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_event_digest_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_event_digest(persisted_state).to_json()
    }

    pub fn run_app_shell_event_digest_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self.run_app_shell_event_digest(persisted_state)?.to_json())
    }

    pub fn app_shell_event_dashboard(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellEventDashboard {
        BerkeleyAppShellEventDashboard::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_event_dashboard(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellEventDashboard, AnalysisExecutionError> {
        Ok(BerkeleyAppShellEventDashboard::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_event_dashboard_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_event_dashboard(persisted_state).to_json()
    }

    pub fn run_app_shell_event_dashboard_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_event_dashboard(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_package(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardPackage {
        BerkeleyAppShellDashboardPackage::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_package(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardPackage, AnalysisExecutionError> {
        Ok(BerkeleyAppShellDashboardPackage::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_dashboard_package_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_package(persisted_state).to_json()
    }

    pub fn run_app_shell_dashboard_package_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_package(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_cards(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardCards {
        BerkeleyAppShellDashboardCards::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_cards(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardCards, AnalysisExecutionError> {
        Ok(BerkeleyAppShellDashboardCards::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_dashboard_cards_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_cards(persisted_state).to_json()
    }

    pub fn run_app_shell_dashboard_cards_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_cards(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_view(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardView {
        BerkeleyAppShellDashboardView::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_view(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardView, AnalysisExecutionError> {
        Ok(BerkeleyAppShellDashboardView::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_dashboard_view_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_view(persisted_state).to_json()
    }

    pub fn run_app_shell_dashboard_view_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_view(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_layout(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardLayout {
        BerkeleyAppShellDashboardLayout::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_layout(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardLayout, AnalysisExecutionError> {
        Ok(BerkeleyAppShellDashboardLayout::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_dashboard_layout_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_layout(persisted_state).to_json()
    }

    pub fn run_app_shell_dashboard_layout_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_layout(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_navigation(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardNavigation {
        BerkeleyAppShellDashboardNavigation::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_navigation(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardNavigation, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardNavigation::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_navigation_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_navigation(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_navigation_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_navigation(persisted_state)?
            .to_json())
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

fn launch_entry_panel(
    host_surface: &BerkeleyAppHostSurfaceWire,
    ready: bool,
) -> Option<&BerkeleyAppHostPanelWire> {
    host_surface
        .active_panel_id
        .as_deref()
        .and_then(|active_panel_id| {
            host_surface
                .panels
                .iter()
                .find(|panel| panel.id == active_panel_id)
        })
        .or_else(|| {
            let preferred_panel_id = if ready { "analysis" } else { "diagnostics" };
            host_surface
                .panels
                .iter()
                .find(|panel| panel.id == preferred_panel_id && panel.enabled)
        })
        .or_else(|| host_surface.panels.iter().find(|panel| panel.enabled))
}

fn launch_action_from_panel(
    panel: &BerkeleyAppHostPanelWire,
    entry_panel_id: Option<&str>,
) -> BerkeleyAppLaunchAction {
    BerkeleyAppLaunchAction {
        id: format!("launch.{}", panel.id),
        label: panel.title.clone(),
        panel_id: panel.id.clone(),
        panel_kind: panel.kind.clone(),
        target: panel.target.clone(),
        enabled: panel.enabled,
        primary: entry_panel_id == Some(panel.id.as_str()),
        disabled_reason: panel.disabled_reason.clone(),
    }
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

fn app_launch_plan_json_value(plan: &BerkeleyAppLaunchPlan) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": plan.schema_version,
        "packageName": &plan.package_name,
        "sourceFingerprint": &plan.source_fingerprint,
        "title": &plan.title,
        "startupRoute": &plan.startup_route,
        "ready": plan.ready,
        "entryPanelId": &plan.entry_panel_id,
        "entryPanelKind": &plan.entry_panel_kind,
        "entryTarget": &plan.entry_target,
        "requestedSelectedSyntaxCardIndex": plan.requested_selected_syntax_card_index,
        "requestedActiveCommandId": &plan.requested_active_command_id,
        "resolvedSelectedSyntaxCardIndex": plan.resolved_selected_syntax_card_index,
        "resolvedActiveCommandId": &plan.resolved_active_command_id,
        "selectionStale": plan.selection_stale,
        "commandStale": plan.command_stale,
        "actionCount": plan.action_count,
        "actions": plan
            .actions
            .iter()
            .map(app_launch_action_json_value)
            .collect::<Vec<_>>(),
        "diagnosticCount": plan.diagnostic_count,
        "blockingMessage": &plan.blocking_message,
    })
}

fn app_launch_action_json_value(action: &BerkeleyAppLaunchAction) -> serde_json::Value {
    serde_json::json!({
        "id": &action.id,
        "label": &action.label,
        "panelId": &action.panel_id,
        "panelKind": &action.panel_kind,
        "target": &action.target,
        "enabled": action.enabled,
        "primary": action.primary,
        "disabledReason": &action.disabled_reason,
    })
}

fn app_readiness_report_json_value(report: &BerkeleyAppReadinessReport) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": report.schema_version,
        "packageName": &report.package_name,
        "sourceFingerprint": &report.source_fingerprint,
        "title": &report.title,
        "startupRoute": &report.startup_route,
        "ready": report.ready,
        "parsed": report.parsed,
        "executionAvailable": report.execution_available,
        "entryPanelId": &report.entry_panel_id,
        "entryTarget": &report.entry_target,
        "primaryActionId": &report.primary_action_id,
        "primaryActionEnabled": report.primary_action_enabled,
        "panelCount": report.panel_count,
        "enabledPanelCount": report.enabled_panel_count,
        "disabledPanelCount": report.disabled_panel_count,
        "actionCount": report.action_count,
        "enabledActionCount": report.enabled_action_count,
        "disabledActionCount": report.disabled_action_count,
        "diagnosticCount": report.diagnostic_count,
        "errorCount": report.error_count,
        "warningCount": report.warning_count,
        "noteCount": report.note_count,
        "selectionStale": report.selection_stale,
        "commandStale": report.command_stale,
        "repairedState": report.repaired_state,
        "blockingMessage": &report.blocking_message,
    })
}

fn app_shell_handoff_json_value(handoff: &BerkeleyAppShellHandoff) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": handoff.schema_version,
        "packageManifest": app_package_manifest_json_value(&handoff.package_manifest),
        "startupSummary": app_startup_summary_json_value(&handoff.startup_summary),
        "launchPlan": app_launch_plan_json_value(&handoff.launch_plan),
        "readinessReport": app_readiness_report_json_value(&handoff.readiness_report),
    })
}

fn app_shell_status_json_value(status: &BerkeleyAppShellStatus) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": status.schema_version,
        "packageName": &status.package_name,
        "sourceFingerprint": &status.source_fingerprint,
        "title": &status.title,
        "startupRoute": &status.startup_route,
        "ready": status.ready,
        "severity": &status.severity,
        "message": &status.message,
        "entryPanelId": &status.entry_panel_id,
        "entryTarget": &status.entry_target,
        "primaryActionId": &status.primary_action_id,
        "diagnosticCount": status.diagnostic_count,
        "errorCount": status.error_count,
        "warningCount": status.warning_count,
        "noteCount": status.note_count,
        "blockingMessage": &status.blocking_message,
    })
}

fn app_shell_telemetry_json_value(telemetry: &BerkeleyAppShellTelemetry) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": telemetry.schema_version,
        "packageName": &telemetry.package_name,
        "sourceFingerprint": &telemetry.source_fingerprint,
        "title": &telemetry.title,
        "startupRoute": &telemetry.startup_route,
        "ready": telemetry.ready,
        "severity": &telemetry.severity,
        "message": &telemetry.message,
        "entryPanelId": &telemetry.entry_panel_id,
        "primaryActionId": &telemetry.primary_action_id,
        "panelCount": telemetry.panel_count,
        "enabledPanelCount": telemetry.enabled_panel_count,
        "disabledPanelCount": telemetry.disabled_panel_count,
        "actionCount": telemetry.action_count,
        "enabledActionCount": telemetry.enabled_action_count,
        "disabledActionCount": telemetry.disabled_action_count,
        "diagnosticCount": telemetry.diagnostic_count,
        "errorCount": telemetry.error_count,
        "warningCount": telemetry.warning_count,
        "noteCount": telemetry.note_count,
        "selectionStale": telemetry.selection_stale,
        "commandStale": telemetry.command_stale,
        "repairedState": telemetry.repaired_state,
        "artifactCapabilityCount": telemetry.artifact_capability_count,
    })
}

fn app_shell_event_log_json_value(event_log: &BerkeleyAppShellEventLog) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": event_log.schema_version,
        "packageName": &event_log.package_name,
        "sourceFingerprint": &event_log.source_fingerprint,
        "title": &event_log.title,
        "startupRoute": &event_log.startup_route,
        "ready": event_log.ready,
        "eventCount": event_log.event_count,
        "events": event_log
            .events
            .iter()
            .map(app_shell_event_json_value)
            .collect::<Vec<_>>(),
    })
}

fn app_shell_event_summary_json_value(summary: &BerkeleyAppShellEventSummary) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": summary.schema_version,
        "packageName": &summary.package_name,
        "sourceFingerprint": &summary.source_fingerprint,
        "title": &summary.title,
        "startupRoute": &summary.startup_route,
        "ready": summary.ready,
        "severity": &summary.severity,
        "statusEventId": &summary.status_event_id,
        "primaryActionId": &summary.primary_action_id,
        "eventCount": summary.event_count,
        "statusEventCount": summary.status_event_count,
        "routeEventCount": summary.route_event_count,
        "actionEventCount": summary.action_event_count,
        "diagnosticEventCount": summary.diagnostic_event_count,
        "stateEventCount": summary.state_event_count,
        "capabilityEventCount": summary.capability_event_count,
        "readyEventCount": summary.ready_event_count,
        "blockedEventCount": summary.blocked_event_count,
        "infoEventCount": summary.info_event_count,
        "warningEventCount": summary.warning_event_count,
        "errorEventCount": summary.error_event_count,
        "countedEventTotal": summary.counted_event_total,
        "diagnosticCount": summary.diagnostic_count,
        "repairedStateCount": summary.repaired_state_count,
        "artifactCapabilityCount": summary.artifact_capability_count,
    })
}

fn app_shell_event_digest_json_value(digest: &BerkeleyAppShellEventDigest) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": digest.schema_version,
        "packageName": &digest.package_name,
        "sourceFingerprint": &digest.source_fingerprint,
        "title": &digest.title,
        "startupRoute": &digest.startup_route,
        "ready": digest.ready,
        "severity": &digest.severity,
        "headlineEventId": &digest.headline_event_id,
        "headlineMessage": &digest.headline_message,
        "primaryActionId": &digest.primary_action_id,
        "attentionEventCount": digest.attention_event_count,
        "attentionEventIds": &digest.attention_event_ids,
        "metricEventCount": digest.metric_event_count,
        "metricEventIds": &digest.metric_event_ids,
        "eventCount": digest.event_count,
        "countedEventTotal": digest.counted_event_total,
        "diagnosticCount": digest.diagnostic_count,
        "repairedStateCount": digest.repaired_state_count,
        "artifactCapabilityCount": digest.artifact_capability_count,
    })
}

fn app_shell_event_dashboard_json_value(
    dashboard: &BerkeleyAppShellEventDashboard,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": dashboard.schema_version,
        "packageName": &dashboard.package_name,
        "sourceFingerprint": &dashboard.source_fingerprint,
        "title": &dashboard.title,
        "startupRoute": &dashboard.startup_route,
        "ready": dashboard.ready,
        "severity": &dashboard.severity,
        "headlineEventId": &dashboard.headline_event_id,
        "headlineMessage": &dashboard.headline_message,
        "primaryActionId": &dashboard.primary_action_id,
        "attentionRequired": dashboard.attention_required,
        "sectionCount": dashboard.section_count,
        "sections": dashboard
            .sections
            .iter()
            .map(app_shell_event_dashboard_section_json_value)
            .collect::<Vec<_>>(),
        "eventCount": dashboard.event_count,
        "diagnosticCount": dashboard.diagnostic_count,
        "repairedStateCount": dashboard.repaired_state_count,
        "artifactCapabilityCount": dashboard.artifact_capability_count,
    })
}

fn app_shell_event_dashboard_section_json_value(
    section: &BerkeleyAppShellEventDashboardSection,
) -> serde_json::Value {
    serde_json::json!({
        "id": &section.id,
        "title": &section.title,
        "severity": &section.severity,
        "eventCount": section.event_count,
        "eventIds": &section.event_ids,
    })
}

fn app_shell_dashboard_package_json_value(
    package: &BerkeleyAppShellDashboardPackage,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": package.schema_version,
        "packageName": &package.package_name,
        "sourceFingerprint": &package.source_fingerprint,
        "title": &package.title,
        "startupRoute": &package.startup_route,
        "ready": package.ready,
        "severity": &package.severity,
        "attentionRequired": package.attention_required,
        "sectionCount": package.section_count,
        "artifactCapabilityCount": package.artifact_capability_count,
        "dashboardCapabilityId": &package.dashboard_capability_id,
        "packageCapabilityId": &package.package_capability_id,
        "packageManifest": app_package_manifest_json_value(&package.package_manifest),
        "eventDashboard": app_shell_event_dashboard_json_value(&package.event_dashboard),
    })
}

fn app_shell_dashboard_cards_json_value(
    cards: &BerkeleyAppShellDashboardCards,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": cards.schema_version,
        "packageName": &cards.package_name,
        "sourceFingerprint": &cards.source_fingerprint,
        "title": &cards.title,
        "startupRoute": &cards.startup_route,
        "ready": cards.ready,
        "severity": &cards.severity,
        "attentionRequired": cards.attention_required,
        "cardCount": cards.card_count,
        "primaryCardId": &cards.primary_card_id,
        "cards": cards
            .cards
            .iter()
            .map(app_shell_dashboard_card_json_value)
            .collect::<Vec<_>>(),
        "packageCapabilityId": &cards.package_capability_id,
        "dashboardCapabilityId": &cards.dashboard_capability_id,
        "cardsCapabilityId": &cards.cards_capability_id,
        "artifactCapabilityCount": cards.artifact_capability_count,
    })
}

fn app_shell_dashboard_view_json_value(view: &BerkeleyAppShellDashboardView) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": view.schema_version,
        "packageName": &view.package_name,
        "sourceFingerprint": &view.source_fingerprint,
        "title": &view.title,
        "startupRoute": &view.startup_route,
        "ready": view.ready,
        "severity": &view.severity,
        "attentionRequired": view.attention_required,
        "primaryCardId": &view.primary_card_id,
        "primaryCardTitle": &view.primary_card_title,
        "cardCount": view.card_count,
        "visibleCardCount": view.visible_card_count,
        "attentionCardCount": view.attention_card_count,
        "metricCardCount": view.metric_card_count,
        "cardIds": &view.card_ids,
        "visibleCardIds": &view.visible_card_ids,
        "attentionCardIds": &view.attention_card_ids,
        "metricCardIds": &view.metric_card_ids,
        "packageCapabilityId": &view.package_capability_id,
        "dashboardCapabilityId": &view.dashboard_capability_id,
        "cardsCapabilityId": &view.cards_capability_id,
        "viewCapabilityId": &view.view_capability_id,
        "artifactCapabilityCount": view.artifact_capability_count,
    })
}

fn app_shell_dashboard_layout_json_value(
    layout: &BerkeleyAppShellDashboardLayout,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": layout.schema_version,
        "packageName": &layout.package_name,
        "sourceFingerprint": &layout.source_fingerprint,
        "title": &layout.title,
        "startupRoute": &layout.startup_route,
        "ready": layout.ready,
        "severity": &layout.severity,
        "attentionRequired": layout.attention_required,
        "primaryCardId": &layout.primary_card_id,
        "primaryRegionId": &layout.primary_region_id,
        "regionCount": layout.region_count,
        "visibleRegionCount": layout.visible_region_count,
        "cardCount": layout.card_count,
        "visibleCardCount": layout.visible_card_count,
        "attentionCardCount": layout.attention_card_count,
        "metricCardCount": layout.metric_card_count,
        "regions": layout
            .regions
            .iter()
            .map(app_shell_dashboard_layout_region_json_value)
            .collect::<Vec<_>>(),
        "packageCapabilityId": &layout.package_capability_id,
        "dashboardCapabilityId": &layout.dashboard_capability_id,
        "cardsCapabilityId": &layout.cards_capability_id,
        "viewCapabilityId": &layout.view_capability_id,
        "layoutCapabilityId": &layout.layout_capability_id,
        "artifactCapabilityCount": layout.artifact_capability_count,
    })
}

fn app_shell_dashboard_layout_region_json_value(
    region: &BerkeleyAppShellDashboardLayoutRegion,
) -> serde_json::Value {
    serde_json::json!({
        "id": &region.id,
        "role": &region.role,
        "title": &region.title,
        "cardIds": &region.card_ids,
        "primary": region.primary,
        "visible": region.visible,
    })
}

fn app_shell_dashboard_navigation_json_value(
    navigation: &BerkeleyAppShellDashboardNavigation,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": navigation.schema_version,
        "packageName": &navigation.package_name,
        "sourceFingerprint": &navigation.source_fingerprint,
        "title": &navigation.title,
        "startupRoute": &navigation.startup_route,
        "ready": navigation.ready,
        "severity": &navigation.severity,
        "attentionRequired": navigation.attention_required,
        "primaryCardId": &navigation.primary_card_id,
        "primaryRegionId": &navigation.primary_region_id,
        "activeItemId": &navigation.active_item_id,
        "itemCount": navigation.item_count,
        "visibleItemCount": navigation.visible_item_count,
        "enabledItemCount": navigation.enabled_item_count,
        "regionCount": navigation.region_count,
        "visibleRegionCount": navigation.visible_region_count,
        "cardCount": navigation.card_count,
        "visibleCardCount": navigation.visible_card_count,
        "attentionCardCount": navigation.attention_card_count,
        "metricCardCount": navigation.metric_card_count,
        "items": navigation
            .items
            .iter()
            .map(app_shell_dashboard_navigation_item_json_value)
            .collect::<Vec<_>>(),
        "packageCapabilityId": &navigation.package_capability_id,
        "dashboardCapabilityId": &navigation.dashboard_capability_id,
        "cardsCapabilityId": &navigation.cards_capability_id,
        "viewCapabilityId": &navigation.view_capability_id,
        "layoutCapabilityId": &navigation.layout_capability_id,
        "navigationCapabilityId": &navigation.navigation_capability_id,
        "artifactCapabilityCount": navigation.artifact_capability_count,
    })
}

fn app_shell_dashboard_navigation_item_json_value(
    item: &BerkeleyAppShellDashboardNavigationItem,
) -> serde_json::Value {
    serde_json::json!({
        "id": &item.id,
        "regionId": &item.region_id,
        "role": &item.role,
        "label": &item.label,
        "cardIds": &item.card_ids,
        "active": item.active,
        "visible": item.visible,
        "enabled": item.enabled,
        "badgeCount": item.badge_count,
    })
}

fn app_shell_dashboard_card_json_value(card: &BerkeleyAppShellDashboardCard) -> serde_json::Value {
    serde_json::json!({
        "id": &card.id,
        "sectionId": &card.section_id,
        "title": &card.title,
        "severity": &card.severity,
        "eventCount": card.event_count,
        "eventIds": &card.event_ids,
        "primary": card.primary,
        "attention": card.attention,
    })
}

fn app_shell_event_json_value(event: &BerkeleyAppShellEvent) -> serde_json::Value {
    serde_json::json!({
        "id": &event.id,
        "kind": &event.kind,
        "severity": &event.severity,
        "message": &event.message,
        "panelId": &event.panel_id,
        "actionId": &event.action_id,
        "count": event.count,
    })
}

fn shell_status_severity(readiness: &BerkeleyAppReadinessReport) -> &'static str {
    if readiness.error_count > 0 {
        "error"
    } else if readiness.warning_count > 0 {
        "warning"
    } else if !readiness.ready {
        "blocked"
    } else {
        "ready"
    }
}

fn shell_status_message(readiness: &BerkeleyAppReadinessReport) -> String {
    if readiness.ready {
        return readiness
            .entry_panel_id
            .as_deref()
            .map(|panel_id| format!("Ready to launch {panel_id} panel"))
            .unwrap_or_else(|| "Ready to launch Berkeley SPICE Mosaic app".to_string());
    }

    readiness
        .blocking_message
        .clone()
        .unwrap_or_else(|| "Deck is blocked before launch".to_string())
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
