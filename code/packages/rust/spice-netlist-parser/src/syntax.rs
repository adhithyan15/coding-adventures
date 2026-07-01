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
pub const BERKELEY_APP_SHELL_DASHBOARD_ROUTES_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_BREADCRUMBS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_TABS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_TAB_PANELS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARDS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARD_ACTIONS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_ACTION_DISPATCH_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_EVENTS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SUMMARY_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_DIGEST_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANES_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TABS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANELS_SCHEMA_VERSION: u32 = 1;
pub const BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARDS_SCHEMA_VERSION: u32 = 1;
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
    "app-shell-dashboard-routes-json",
    "app-shell-dashboard-breadcrumbs-json",
    "app-shell-dashboard-tabs-json",
    "app-shell-dashboard-tab-panels-json",
    "app-shell-dashboard-panel-cards-json",
    "app-shell-dashboard-panel-card-actions-json",
    "app-shell-dashboard-action-dispatch-json",
    "app-shell-dashboard-dispatch-events-json",
    "app-shell-dashboard-dispatch-queue-json",
    "app-shell-dashboard-dispatch-queue-summary-json",
    "app-shell-dashboard-dispatch-queue-digest-json",
    "app-shell-dashboard-dispatch-queue-lanes-json",
    "app-shell-dashboard-dispatch-queue-lane-tabs-json",
    "app-shell-dashboard-dispatch-queue-lane-tab-panels-json",
    "app-shell-dashboard-dispatch-queue-lane-tab-panel-cards-json",
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
pub struct BerkeleyAppShellDashboardRoute {
    pub id: String,
    pub item_id: String,
    pub region_id: String,
    pub role: String,
    pub label: String,
    pub path: String,
    pub card_ids: Vec<String>,
    pub active: bool,
    pub default_route: bool,
    pub visible: bool,
    pub enabled: bool,
    pub badge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardRoutes {
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
    pub active_route_id: Option<String>,
    pub active_route_path: Option<String>,
    pub default_route_id: Option<String>,
    pub default_route_path: Option<String>,
    pub route_count: usize,
    pub visible_route_count: usize,
    pub enabled_route_count: usize,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub routes: Vec<BerkeleyAppShellDashboardRoute>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub routes_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardRoutes {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_navigation(
            &BerkeleyAppShellDashboardNavigation::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_navigation(&BerkeleyAppShellDashboardNavigation::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dashboard_navigation(navigation: &BerkeleyAppShellDashboardNavigation) -> Self {
        let mut routes = navigation
            .items
            .iter()
            .map(|item| BerkeleyAppShellDashboardRoute {
                id: format!("dashboard.route.{}", item.role),
                item_id: item.id.clone(),
                region_id: item.region_id.clone(),
                role: item.role.clone(),
                label: item.label.clone(),
                path: format!("/dashboard/{}", item.role),
                card_ids: item.card_ids.clone(),
                active: item.active,
                default_route: false,
                visible: item.visible,
                enabled: item.enabled,
                badge_count: item.badge_count,
            })
            .collect::<Vec<_>>();
        let active_route_id = routes
            .iter()
            .find(|route| route.active)
            .map(|route| route.id.clone());
        let default_route_id = active_route_id
            .clone()
            .or_else(|| {
                routes
                    .iter()
                    .find(|route| route.enabled)
                    .map(|route| route.id.clone())
            })
            .or_else(|| {
                routes
                    .iter()
                    .find(|route| route.visible)
                    .map(|route| route.id.clone())
            })
            .or_else(|| routes.first().map(|route| route.id.clone()));
        for route in &mut routes {
            route.default_route = Some(&route.id) == default_route_id.as_ref();
        }
        let active_route_path = active_route_id.as_ref().and_then(|active_id| {
            routes
                .iter()
                .find(|route| &route.id == active_id)
                .map(|route| route.path.clone())
        });
        let default_route_path = default_route_id.as_ref().and_then(|default_id| {
            routes
                .iter()
                .find(|route| &route.id == default_id)
                .map(|route| route.path.clone())
        });
        let route_count = routes.len();
        let visible_route_count = routes.iter().filter(|route| route.visible).count();
        let enabled_route_count = routes.iter().filter(|route| route.enabled).count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_ROUTES_SCHEMA_VERSION,
            package_name: navigation.package_name.clone(),
            source_fingerprint: navigation.source_fingerprint.clone(),
            title: navigation.title.clone(),
            startup_route: navigation.startup_route.clone(),
            ready: navigation.ready,
            severity: navigation.severity.clone(),
            attention_required: navigation.attention_required,
            primary_card_id: navigation.primary_card_id.clone(),
            primary_region_id: navigation.primary_region_id.clone(),
            active_item_id: navigation.active_item_id.clone(),
            active_route_id,
            active_route_path,
            default_route_id,
            default_route_path,
            route_count,
            visible_route_count,
            enabled_route_count,
            item_count: navigation.item_count,
            visible_item_count: navigation.visible_item_count,
            enabled_item_count: navigation.enabled_item_count,
            region_count: navigation.region_count,
            visible_region_count: navigation.visible_region_count,
            card_count: navigation.card_count,
            visible_card_count: navigation.visible_card_count,
            attention_card_count: navigation.attention_card_count,
            metric_card_count: navigation.metric_card_count,
            routes,
            package_capability_id: navigation.package_capability_id.clone(),
            dashboard_capability_id: navigation.dashboard_capability_id.clone(),
            cards_capability_id: navigation.cards_capability_id.clone(),
            view_capability_id: navigation.view_capability_id.clone(),
            layout_capability_id: navigation.layout_capability_id.clone(),
            navigation_capability_id: navigation.navigation_capability_id.clone(),
            routes_capability_id: "app-shell-dashboard-routes-json".to_string(),
            artifact_capability_count: navigation.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_routes_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardBreadcrumb {
    pub id: String,
    pub route_id: String,
    pub item_id: String,
    pub region_id: String,
    pub role: String,
    pub label: String,
    pub path: String,
    pub position: usize,
    pub active: bool,
    pub default_route: bool,
    pub visible: bool,
    pub enabled: bool,
    pub badge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardBreadcrumbs {
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
    pub active_route_id: Option<String>,
    pub active_route_path: Option<String>,
    pub default_route_id: Option<String>,
    pub default_route_path: Option<String>,
    pub active_breadcrumb_id: Option<String>,
    pub active_breadcrumb_path: Option<String>,
    pub default_breadcrumb_id: Option<String>,
    pub default_breadcrumb_path: Option<String>,
    pub route_count: usize,
    pub visible_route_count: usize,
    pub enabled_route_count: usize,
    pub breadcrumb_count: usize,
    pub visible_breadcrumb_count: usize,
    pub enabled_breadcrumb_count: usize,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub breadcrumbs: Vec<BerkeleyAppShellDashboardBreadcrumb>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub routes_capability_id: String,
    pub breadcrumbs_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardBreadcrumbs {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_routes(&BerkeleyAppShellDashboardRoutes::from_bootstrap_snapshot(
            snapshot,
        ))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_routes(&BerkeleyAppShellDashboardRoutes::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dashboard_routes(routes: &BerkeleyAppShellDashboardRoutes) -> Self {
        let breadcrumbs = routes
            .routes
            .iter()
            .enumerate()
            .map(|(index, route)| BerkeleyAppShellDashboardBreadcrumb {
                id: format!("dashboard.breadcrumb.{}", route.role),
                route_id: route.id.clone(),
                item_id: route.item_id.clone(),
                region_id: route.region_id.clone(),
                role: route.role.clone(),
                label: route.label.clone(),
                path: route.path.clone(),
                position: index + 1,
                active: route.active,
                default_route: route.default_route,
                visible: route.visible,
                enabled: route.enabled,
                badge_count: route.badge_count,
            })
            .collect::<Vec<_>>();
        let active_breadcrumb_id = breadcrumbs
            .iter()
            .find(|breadcrumb| breadcrumb.active)
            .map(|breadcrumb| breadcrumb.id.clone());
        let active_breadcrumb_path = active_breadcrumb_id.as_ref().and_then(|active_id| {
            breadcrumbs
                .iter()
                .find(|breadcrumb| &breadcrumb.id == active_id)
                .map(|breadcrumb| breadcrumb.path.clone())
        });
        let default_breadcrumb_id = breadcrumbs
            .iter()
            .find(|breadcrumb| breadcrumb.default_route)
            .map(|breadcrumb| breadcrumb.id.clone());
        let default_breadcrumb_path = default_breadcrumb_id.as_ref().and_then(|default_id| {
            breadcrumbs
                .iter()
                .find(|breadcrumb| &breadcrumb.id == default_id)
                .map(|breadcrumb| breadcrumb.path.clone())
        });
        let breadcrumb_count = breadcrumbs.len();
        let visible_breadcrumb_count = breadcrumbs
            .iter()
            .filter(|breadcrumb| breadcrumb.visible)
            .count();
        let enabled_breadcrumb_count = breadcrumbs
            .iter()
            .filter(|breadcrumb| breadcrumb.enabled)
            .count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_BREADCRUMBS_SCHEMA_VERSION,
            package_name: routes.package_name.clone(),
            source_fingerprint: routes.source_fingerprint.clone(),
            title: routes.title.clone(),
            startup_route: routes.startup_route.clone(),
            ready: routes.ready,
            severity: routes.severity.clone(),
            attention_required: routes.attention_required,
            primary_card_id: routes.primary_card_id.clone(),
            primary_region_id: routes.primary_region_id.clone(),
            active_item_id: routes.active_item_id.clone(),
            active_route_id: routes.active_route_id.clone(),
            active_route_path: routes.active_route_path.clone(),
            default_route_id: routes.default_route_id.clone(),
            default_route_path: routes.default_route_path.clone(),
            active_breadcrumb_id,
            active_breadcrumb_path,
            default_breadcrumb_id,
            default_breadcrumb_path,
            route_count: routes.route_count,
            visible_route_count: routes.visible_route_count,
            enabled_route_count: routes.enabled_route_count,
            breadcrumb_count,
            visible_breadcrumb_count,
            enabled_breadcrumb_count,
            item_count: routes.item_count,
            visible_item_count: routes.visible_item_count,
            enabled_item_count: routes.enabled_item_count,
            region_count: routes.region_count,
            visible_region_count: routes.visible_region_count,
            card_count: routes.card_count,
            visible_card_count: routes.visible_card_count,
            attention_card_count: routes.attention_card_count,
            metric_card_count: routes.metric_card_count,
            breadcrumbs,
            package_capability_id: routes.package_capability_id.clone(),
            dashboard_capability_id: routes.dashboard_capability_id.clone(),
            cards_capability_id: routes.cards_capability_id.clone(),
            view_capability_id: routes.view_capability_id.clone(),
            layout_capability_id: routes.layout_capability_id.clone(),
            navigation_capability_id: routes.navigation_capability_id.clone(),
            routes_capability_id: routes.routes_capability_id.clone(),
            breadcrumbs_capability_id: "app-shell-dashboard-breadcrumbs-json".to_string(),
            artifact_capability_count: routes.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_breadcrumbs_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardTab {
    pub id: String,
    pub breadcrumb_id: String,
    pub route_id: String,
    pub item_id: String,
    pub region_id: String,
    pub role: String,
    pub label: String,
    pub path: String,
    pub position: usize,
    pub selected: bool,
    pub default_tab: bool,
    pub visible: bool,
    pub enabled: bool,
    pub badge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardTabs {
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
    pub active_route_id: Option<String>,
    pub active_route_path: Option<String>,
    pub default_route_id: Option<String>,
    pub default_route_path: Option<String>,
    pub active_breadcrumb_id: Option<String>,
    pub active_breadcrumb_path: Option<String>,
    pub default_breadcrumb_id: Option<String>,
    pub default_breadcrumb_path: Option<String>,
    pub selected_tab_id: Option<String>,
    pub selected_tab_path: Option<String>,
    pub default_tab_id: Option<String>,
    pub default_tab_path: Option<String>,
    pub route_count: usize,
    pub visible_route_count: usize,
    pub enabled_route_count: usize,
    pub breadcrumb_count: usize,
    pub visible_breadcrumb_count: usize,
    pub enabled_breadcrumb_count: usize,
    pub tab_count: usize,
    pub visible_tab_count: usize,
    pub enabled_tab_count: usize,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub tabs: Vec<BerkeleyAppShellDashboardTab>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub routes_capability_id: String,
    pub breadcrumbs_capability_id: String,
    pub tabs_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardTabs {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_breadcrumbs(
            &BerkeleyAppShellDashboardBreadcrumbs::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_breadcrumbs(&BerkeleyAppShellDashboardBreadcrumbs::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dashboard_breadcrumbs(breadcrumbs: &BerkeleyAppShellDashboardBreadcrumbs) -> Self {
        let tabs = breadcrumbs
            .breadcrumbs
            .iter()
            .map(|breadcrumb| BerkeleyAppShellDashboardTab {
                id: format!("dashboard.tab.{}", breadcrumb.role),
                breadcrumb_id: breadcrumb.id.clone(),
                route_id: breadcrumb.route_id.clone(),
                item_id: breadcrumb.item_id.clone(),
                region_id: breadcrumb.region_id.clone(),
                role: breadcrumb.role.clone(),
                label: breadcrumb.label.clone(),
                path: breadcrumb.path.clone(),
                position: breadcrumb.position,
                selected: breadcrumb.active,
                default_tab: breadcrumb.default_route,
                visible: breadcrumb.visible,
                enabled: breadcrumb.enabled,
                badge_count: breadcrumb.badge_count,
            })
            .collect::<Vec<_>>();
        let selected_tab_id = tabs
            .iter()
            .find(|tab| tab.selected)
            .map(|tab| tab.id.clone());
        let selected_tab_path = selected_tab_id.as_ref().and_then(|selected_id| {
            tabs.iter()
                .find(|tab| &tab.id == selected_id)
                .map(|tab| tab.path.clone())
        });
        let default_tab_id = tabs
            .iter()
            .find(|tab| tab.default_tab)
            .map(|tab| tab.id.clone());
        let default_tab_path = default_tab_id.as_ref().and_then(|default_id| {
            tabs.iter()
                .find(|tab| &tab.id == default_id)
                .map(|tab| tab.path.clone())
        });
        let tab_count = tabs.len();
        let visible_tab_count = tabs.iter().filter(|tab| tab.visible).count();
        let enabled_tab_count = tabs.iter().filter(|tab| tab.enabled).count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_TABS_SCHEMA_VERSION,
            package_name: breadcrumbs.package_name.clone(),
            source_fingerprint: breadcrumbs.source_fingerprint.clone(),
            title: breadcrumbs.title.clone(),
            startup_route: breadcrumbs.startup_route.clone(),
            ready: breadcrumbs.ready,
            severity: breadcrumbs.severity.clone(),
            attention_required: breadcrumbs.attention_required,
            primary_card_id: breadcrumbs.primary_card_id.clone(),
            primary_region_id: breadcrumbs.primary_region_id.clone(),
            active_item_id: breadcrumbs.active_item_id.clone(),
            active_route_id: breadcrumbs.active_route_id.clone(),
            active_route_path: breadcrumbs.active_route_path.clone(),
            default_route_id: breadcrumbs.default_route_id.clone(),
            default_route_path: breadcrumbs.default_route_path.clone(),
            active_breadcrumb_id: breadcrumbs.active_breadcrumb_id.clone(),
            active_breadcrumb_path: breadcrumbs.active_breadcrumb_path.clone(),
            default_breadcrumb_id: breadcrumbs.default_breadcrumb_id.clone(),
            default_breadcrumb_path: breadcrumbs.default_breadcrumb_path.clone(),
            selected_tab_id,
            selected_tab_path,
            default_tab_id,
            default_tab_path,
            route_count: breadcrumbs.route_count,
            visible_route_count: breadcrumbs.visible_route_count,
            enabled_route_count: breadcrumbs.enabled_route_count,
            breadcrumb_count: breadcrumbs.breadcrumb_count,
            visible_breadcrumb_count: breadcrumbs.visible_breadcrumb_count,
            enabled_breadcrumb_count: breadcrumbs.enabled_breadcrumb_count,
            tab_count,
            visible_tab_count,
            enabled_tab_count,
            item_count: breadcrumbs.item_count,
            visible_item_count: breadcrumbs.visible_item_count,
            enabled_item_count: breadcrumbs.enabled_item_count,
            region_count: breadcrumbs.region_count,
            visible_region_count: breadcrumbs.visible_region_count,
            card_count: breadcrumbs.card_count,
            visible_card_count: breadcrumbs.visible_card_count,
            attention_card_count: breadcrumbs.attention_card_count,
            metric_card_count: breadcrumbs.metric_card_count,
            tabs,
            package_capability_id: breadcrumbs.package_capability_id.clone(),
            dashboard_capability_id: breadcrumbs.dashboard_capability_id.clone(),
            cards_capability_id: breadcrumbs.cards_capability_id.clone(),
            view_capability_id: breadcrumbs.view_capability_id.clone(),
            layout_capability_id: breadcrumbs.layout_capability_id.clone(),
            navigation_capability_id: breadcrumbs.navigation_capability_id.clone(),
            routes_capability_id: breadcrumbs.routes_capability_id.clone(),
            breadcrumbs_capability_id: breadcrumbs.breadcrumbs_capability_id.clone(),
            tabs_capability_id: "app-shell-dashboard-tabs-json".to_string(),
            artifact_capability_count: breadcrumbs.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_tabs_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardTabPanel {
    pub id: String,
    pub tab_id: String,
    pub breadcrumb_id: String,
    pub route_id: String,
    pub item_id: String,
    pub region_id: String,
    pub role: String,
    pub title: String,
    pub path: String,
    pub position: usize,
    pub selected: bool,
    pub default_panel: bool,
    pub visible: bool,
    pub enabled: bool,
    pub badge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardTabPanels {
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
    pub active_route_id: Option<String>,
    pub active_route_path: Option<String>,
    pub default_route_id: Option<String>,
    pub default_route_path: Option<String>,
    pub active_breadcrumb_id: Option<String>,
    pub active_breadcrumb_path: Option<String>,
    pub default_breadcrumb_id: Option<String>,
    pub default_breadcrumb_path: Option<String>,
    pub selected_tab_id: Option<String>,
    pub selected_tab_path: Option<String>,
    pub default_tab_id: Option<String>,
    pub default_tab_path: Option<String>,
    pub selected_panel_id: Option<String>,
    pub selected_panel_path: Option<String>,
    pub default_panel_id: Option<String>,
    pub default_panel_path: Option<String>,
    pub route_count: usize,
    pub visible_route_count: usize,
    pub enabled_route_count: usize,
    pub breadcrumb_count: usize,
    pub visible_breadcrumb_count: usize,
    pub enabled_breadcrumb_count: usize,
    pub tab_count: usize,
    pub visible_tab_count: usize,
    pub enabled_tab_count: usize,
    pub panel_count: usize,
    pub visible_panel_count: usize,
    pub enabled_panel_count: usize,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub panels: Vec<BerkeleyAppShellDashboardTabPanel>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub routes_capability_id: String,
    pub breadcrumbs_capability_id: String,
    pub tabs_capability_id: String,
    pub tab_panels_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardTabPanels {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dashboard_tabs(&BerkeleyAppShellDashboardTabs::from_bootstrap_snapshot(
            snapshot,
        ))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_tabs(&BerkeleyAppShellDashboardTabs::from_shell_handoff(handoff))
    }

    pub fn from_dashboard_tabs(tabs: &BerkeleyAppShellDashboardTabs) -> Self {
        let panels = tabs
            .tabs
            .iter()
            .map(|tab| BerkeleyAppShellDashboardTabPanel {
                id: format!("dashboard.tab-panel.{}", tab.role),
                tab_id: tab.id.clone(),
                breadcrumb_id: tab.breadcrumb_id.clone(),
                route_id: tab.route_id.clone(),
                item_id: tab.item_id.clone(),
                region_id: tab.region_id.clone(),
                role: tab.role.clone(),
                title: tab.label.clone(),
                path: tab.path.clone(),
                position: tab.position,
                selected: tab.selected,
                default_panel: tab.default_tab,
                visible: tab.visible,
                enabled: tab.enabled,
                badge_count: tab.badge_count,
            })
            .collect::<Vec<_>>();
        let selected_panel_id = panels
            .iter()
            .find(|panel| panel.selected)
            .map(|panel| panel.id.clone());
        let selected_panel_path = selected_panel_id.as_ref().and_then(|selected_id| {
            panels
                .iter()
                .find(|panel| &panel.id == selected_id)
                .map(|panel| panel.path.clone())
        });
        let default_panel_id = panels
            .iter()
            .find(|panel| panel.default_panel)
            .map(|panel| panel.id.clone());
        let default_panel_path = default_panel_id.as_ref().and_then(|default_id| {
            panels
                .iter()
                .find(|panel| &panel.id == default_id)
                .map(|panel| panel.path.clone())
        });
        let panel_count = panels.len();
        let visible_panel_count = panels.iter().filter(|panel| panel.visible).count();
        let enabled_panel_count = panels.iter().filter(|panel| panel.enabled).count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_TAB_PANELS_SCHEMA_VERSION,
            package_name: tabs.package_name.clone(),
            source_fingerprint: tabs.source_fingerprint.clone(),
            title: tabs.title.clone(),
            startup_route: tabs.startup_route.clone(),
            ready: tabs.ready,
            severity: tabs.severity.clone(),
            attention_required: tabs.attention_required,
            primary_card_id: tabs.primary_card_id.clone(),
            primary_region_id: tabs.primary_region_id.clone(),
            active_item_id: tabs.active_item_id.clone(),
            active_route_id: tabs.active_route_id.clone(),
            active_route_path: tabs.active_route_path.clone(),
            default_route_id: tabs.default_route_id.clone(),
            default_route_path: tabs.default_route_path.clone(),
            active_breadcrumb_id: tabs.active_breadcrumb_id.clone(),
            active_breadcrumb_path: tabs.active_breadcrumb_path.clone(),
            default_breadcrumb_id: tabs.default_breadcrumb_id.clone(),
            default_breadcrumb_path: tabs.default_breadcrumb_path.clone(),
            selected_tab_id: tabs.selected_tab_id.clone(),
            selected_tab_path: tabs.selected_tab_path.clone(),
            default_tab_id: tabs.default_tab_id.clone(),
            default_tab_path: tabs.default_tab_path.clone(),
            selected_panel_id,
            selected_panel_path,
            default_panel_id,
            default_panel_path,
            route_count: tabs.route_count,
            visible_route_count: tabs.visible_route_count,
            enabled_route_count: tabs.enabled_route_count,
            breadcrumb_count: tabs.breadcrumb_count,
            visible_breadcrumb_count: tabs.visible_breadcrumb_count,
            enabled_breadcrumb_count: tabs.enabled_breadcrumb_count,
            tab_count: tabs.tab_count,
            visible_tab_count: tabs.visible_tab_count,
            enabled_tab_count: tabs.enabled_tab_count,
            panel_count,
            visible_panel_count,
            enabled_panel_count,
            item_count: tabs.item_count,
            visible_item_count: tabs.visible_item_count,
            enabled_item_count: tabs.enabled_item_count,
            region_count: tabs.region_count,
            visible_region_count: tabs.visible_region_count,
            card_count: tabs.card_count,
            visible_card_count: tabs.visible_card_count,
            attention_card_count: tabs.attention_card_count,
            metric_card_count: tabs.metric_card_count,
            panels,
            package_capability_id: tabs.package_capability_id.clone(),
            dashboard_capability_id: tabs.dashboard_capability_id.clone(),
            cards_capability_id: tabs.cards_capability_id.clone(),
            view_capability_id: tabs.view_capability_id.clone(),
            layout_capability_id: tabs.layout_capability_id.clone(),
            navigation_capability_id: tabs.navigation_capability_id.clone(),
            routes_capability_id: tabs.routes_capability_id.clone(),
            breadcrumbs_capability_id: tabs.breadcrumbs_capability_id.clone(),
            tabs_capability_id: tabs.tabs_capability_id.clone(),
            tab_panels_capability_id: "app-shell-dashboard-tab-panels-json".to_string(),
            artifact_capability_count: tabs.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_tab_panels_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardPanelCard {
    pub id: String,
    pub panel_id: String,
    pub tab_id: String,
    pub breadcrumb_id: String,
    pub route_id: String,
    pub item_id: String,
    pub region_id: String,
    pub card_id: String,
    pub section_id: String,
    pub role: String,
    pub title: String,
    pub severity: String,
    pub path: String,
    pub position: usize,
    pub selected: bool,
    pub default_panel: bool,
    pub visible: bool,
    pub enabled: bool,
    pub primary: bool,
    pub attention: bool,
    pub event_count: usize,
    pub event_ids: Vec<String>,
    pub badge_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardPanelCards {
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
    pub active_route_id: Option<String>,
    pub active_route_path: Option<String>,
    pub default_route_id: Option<String>,
    pub default_route_path: Option<String>,
    pub active_breadcrumb_id: Option<String>,
    pub active_breadcrumb_path: Option<String>,
    pub default_breadcrumb_id: Option<String>,
    pub default_breadcrumb_path: Option<String>,
    pub selected_tab_id: Option<String>,
    pub selected_tab_path: Option<String>,
    pub default_tab_id: Option<String>,
    pub default_tab_path: Option<String>,
    pub selected_panel_id: Option<String>,
    pub selected_panel_path: Option<String>,
    pub default_panel_id: Option<String>,
    pub default_panel_path: Option<String>,
    pub selected_panel_card_id: Option<String>,
    pub selected_card_id: Option<String>,
    pub default_panel_card_id: Option<String>,
    pub default_card_id: Option<String>,
    pub route_count: usize,
    pub visible_route_count: usize,
    pub enabled_route_count: usize,
    pub breadcrumb_count: usize,
    pub visible_breadcrumb_count: usize,
    pub enabled_breadcrumb_count: usize,
    pub tab_count: usize,
    pub visible_tab_count: usize,
    pub enabled_tab_count: usize,
    pub panel_count: usize,
    pub visible_panel_count: usize,
    pub enabled_panel_count: usize,
    pub panel_card_count: usize,
    pub visible_panel_card_count: usize,
    pub enabled_panel_card_count: usize,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub panel_cards: Vec<BerkeleyAppShellDashboardPanelCard>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub routes_capability_id: String,
    pub breadcrumbs_capability_id: String,
    pub tabs_capability_id: String,
    pub tab_panels_capability_id: String,
    pub panel_cards_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardPanelCards {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_shell_handoff(&BerkeleyAppShellHandoff::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_tab_panels_and_cards(
            &BerkeleyAppShellDashboardTabPanels::from_shell_handoff(handoff),
            &BerkeleyAppShellDashboardCards::from_shell_handoff(handoff),
        )
    }

    pub fn from_dashboard_tab_panels_and_cards(
        tab_panels: &BerkeleyAppShellDashboardTabPanels,
        cards: &BerkeleyAppShellDashboardCards,
    ) -> Self {
        let panel_cards = tab_panels
            .panels
            .iter()
            .flat_map(|panel| {
                cards
                    .cards
                    .iter()
                    .filter(move |card| card.section_id == panel.role)
                    .map(move |card| {
                        let visible_card = card.primary || card.attention || card.event_count > 0;
                        BerkeleyAppShellDashboardPanelCard {
                            id: format!("dashboard.panel-card.{}", panel.role),
                            panel_id: panel.id.clone(),
                            tab_id: panel.tab_id.clone(),
                            breadcrumb_id: panel.breadcrumb_id.clone(),
                            route_id: panel.route_id.clone(),
                            item_id: panel.item_id.clone(),
                            region_id: panel.region_id.clone(),
                            card_id: card.id.clone(),
                            section_id: card.section_id.clone(),
                            role: panel.role.clone(),
                            title: card.title.clone(),
                            severity: card.severity.clone(),
                            path: panel.path.clone(),
                            position: panel.position,
                            selected: panel.selected,
                            default_panel: panel.default_panel,
                            visible: panel.visible && visible_card,
                            enabled: panel.enabled,
                            primary: card.primary,
                            attention: card.attention,
                            event_count: card.event_count,
                            event_ids: card.event_ids.clone(),
                            badge_count: panel.badge_count,
                        }
                    })
            })
            .collect::<Vec<_>>();
        let selected_panel_card = panel_cards.iter().find(|panel_card| panel_card.selected);
        let selected_panel_card_id = selected_panel_card.map(|panel_card| panel_card.id.clone());
        let selected_card_id = selected_panel_card.map(|panel_card| panel_card.card_id.clone());
        let default_panel_card = panel_cards
            .iter()
            .find(|panel_card| panel_card.default_panel);
        let default_panel_card_id = default_panel_card.map(|panel_card| panel_card.id.clone());
        let default_card_id = default_panel_card.map(|panel_card| panel_card.card_id.clone());
        let panel_card_count = panel_cards.len();
        let visible_panel_card_count = panel_cards
            .iter()
            .filter(|panel_card| panel_card.visible)
            .count();
        let enabled_panel_card_count = panel_cards
            .iter()
            .filter(|panel_card| panel_card.enabled)
            .count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARDS_SCHEMA_VERSION,
            package_name: tab_panels.package_name.clone(),
            source_fingerprint: tab_panels.source_fingerprint.clone(),
            title: tab_panels.title.clone(),
            startup_route: tab_panels.startup_route.clone(),
            ready: tab_panels.ready,
            severity: tab_panels.severity.clone(),
            attention_required: tab_panels.attention_required,
            primary_card_id: tab_panels.primary_card_id.clone(),
            primary_region_id: tab_panels.primary_region_id.clone(),
            active_item_id: tab_panels.active_item_id.clone(),
            active_route_id: tab_panels.active_route_id.clone(),
            active_route_path: tab_panels.active_route_path.clone(),
            default_route_id: tab_panels.default_route_id.clone(),
            default_route_path: tab_panels.default_route_path.clone(),
            active_breadcrumb_id: tab_panels.active_breadcrumb_id.clone(),
            active_breadcrumb_path: tab_panels.active_breadcrumb_path.clone(),
            default_breadcrumb_id: tab_panels.default_breadcrumb_id.clone(),
            default_breadcrumb_path: tab_panels.default_breadcrumb_path.clone(),
            selected_tab_id: tab_panels.selected_tab_id.clone(),
            selected_tab_path: tab_panels.selected_tab_path.clone(),
            default_tab_id: tab_panels.default_tab_id.clone(),
            default_tab_path: tab_panels.default_tab_path.clone(),
            selected_panel_id: tab_panels.selected_panel_id.clone(),
            selected_panel_path: tab_panels.selected_panel_path.clone(),
            default_panel_id: tab_panels.default_panel_id.clone(),
            default_panel_path: tab_panels.default_panel_path.clone(),
            selected_panel_card_id,
            selected_card_id,
            default_panel_card_id,
            default_card_id,
            route_count: tab_panels.route_count,
            visible_route_count: tab_panels.visible_route_count,
            enabled_route_count: tab_panels.enabled_route_count,
            breadcrumb_count: tab_panels.breadcrumb_count,
            visible_breadcrumb_count: tab_panels.visible_breadcrumb_count,
            enabled_breadcrumb_count: tab_panels.enabled_breadcrumb_count,
            tab_count: tab_panels.tab_count,
            visible_tab_count: tab_panels.visible_tab_count,
            enabled_tab_count: tab_panels.enabled_tab_count,
            panel_count: tab_panels.panel_count,
            visible_panel_count: tab_panels.visible_panel_count,
            enabled_panel_count: tab_panels.enabled_panel_count,
            panel_card_count,
            visible_panel_card_count,
            enabled_panel_card_count,
            item_count: tab_panels.item_count,
            visible_item_count: tab_panels.visible_item_count,
            enabled_item_count: tab_panels.enabled_item_count,
            region_count: tab_panels.region_count,
            visible_region_count: tab_panels.visible_region_count,
            card_count: tab_panels.card_count,
            visible_card_count: tab_panels.visible_card_count,
            attention_card_count: tab_panels.attention_card_count,
            metric_card_count: tab_panels.metric_card_count,
            panel_cards,
            package_capability_id: tab_panels.package_capability_id.clone(),
            dashboard_capability_id: tab_panels.dashboard_capability_id.clone(),
            cards_capability_id: tab_panels.cards_capability_id.clone(),
            view_capability_id: tab_panels.view_capability_id.clone(),
            layout_capability_id: tab_panels.layout_capability_id.clone(),
            navigation_capability_id: tab_panels.navigation_capability_id.clone(),
            routes_capability_id: tab_panels.routes_capability_id.clone(),
            breadcrumbs_capability_id: tab_panels.breadcrumbs_capability_id.clone(),
            tabs_capability_id: tab_panels.tabs_capability_id.clone(),
            tab_panels_capability_id: tab_panels.tab_panels_capability_id.clone(),
            panel_cards_capability_id: "app-shell-dashboard-panel-cards-json".to_string(),
            artifact_capability_count: tab_panels.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_panel_cards_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardPanelCardAction {
    pub id: String,
    pub panel_card_id: String,
    pub panel_id: String,
    pub card_id: String,
    pub action_id: String,
    pub label: String,
    pub target: String,
    pub panel_kind: String,
    pub role: String,
    pub path: String,
    pub position: usize,
    pub selected: bool,
    pub default_panel: bool,
    pub visible: bool,
    pub enabled: bool,
    pub primary: bool,
    pub card_primary: bool,
    pub attention: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardPanelCardActions {
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
    pub active_route_id: Option<String>,
    pub active_route_path: Option<String>,
    pub default_route_id: Option<String>,
    pub default_route_path: Option<String>,
    pub active_breadcrumb_id: Option<String>,
    pub active_breadcrumb_path: Option<String>,
    pub default_breadcrumb_id: Option<String>,
    pub default_breadcrumb_path: Option<String>,
    pub selected_tab_id: Option<String>,
    pub selected_tab_path: Option<String>,
    pub default_tab_id: Option<String>,
    pub default_tab_path: Option<String>,
    pub selected_panel_id: Option<String>,
    pub selected_panel_path: Option<String>,
    pub default_panel_id: Option<String>,
    pub default_panel_path: Option<String>,
    pub selected_panel_card_id: Option<String>,
    pub selected_card_id: Option<String>,
    pub default_panel_card_id: Option<String>,
    pub default_card_id: Option<String>,
    pub selected_panel_card_action_id: Option<String>,
    pub selected_action_id: Option<String>,
    pub default_panel_card_action_id: Option<String>,
    pub default_action_id: Option<String>,
    pub route_count: usize,
    pub visible_route_count: usize,
    pub enabled_route_count: usize,
    pub breadcrumb_count: usize,
    pub visible_breadcrumb_count: usize,
    pub enabled_breadcrumb_count: usize,
    pub tab_count: usize,
    pub visible_tab_count: usize,
    pub enabled_tab_count: usize,
    pub panel_count: usize,
    pub visible_panel_count: usize,
    pub enabled_panel_count: usize,
    pub panel_card_count: usize,
    pub visible_panel_card_count: usize,
    pub enabled_panel_card_count: usize,
    pub action_count: usize,
    pub enabled_action_count: usize,
    pub primary_action_count: usize,
    pub panel_card_action_count: usize,
    pub visible_panel_card_action_count: usize,
    pub enabled_panel_card_action_count: usize,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub panel_card_actions: Vec<BerkeleyAppShellDashboardPanelCardAction>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub routes_capability_id: String,
    pub breadcrumbs_capability_id: String,
    pub tabs_capability_id: String,
    pub tab_panels_capability_id: String,
    pub panel_cards_capability_id: String,
    pub panel_card_actions_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardPanelCardActions {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_shell_handoff(&BerkeleyAppShellHandoff::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dashboard_panel_cards_and_launch_plan(
            &BerkeleyAppShellDashboardPanelCards::from_shell_handoff(handoff),
            &handoff.launch_plan,
        )
    }

    pub fn from_dashboard_panel_cards_and_launch_plan(
        panel_cards: &BerkeleyAppShellDashboardPanelCards,
        launch_plan: &BerkeleyAppLaunchPlan,
    ) -> Self {
        let panel_card_actions = panel_cards
            .panel_cards
            .iter()
            .filter_map(|panel_card| {
                dashboard_panel_card_launch_action(panel_card, launch_plan).map(|action| {
                    BerkeleyAppShellDashboardPanelCardAction {
                        id: format!("dashboard.panel-card-action.{}", panel_card.role),
                        panel_card_id: panel_card.id.clone(),
                        panel_id: panel_card.panel_id.clone(),
                        card_id: panel_card.card_id.clone(),
                        action_id: action.id.clone(),
                        label: action.label.clone(),
                        target: action.target.clone(),
                        panel_kind: action.panel_kind.clone(),
                        role: panel_card.role.clone(),
                        path: panel_card.path.clone(),
                        position: panel_card.position,
                        selected: panel_card.selected,
                        default_panel: panel_card.default_panel,
                        visible: panel_card.visible,
                        enabled: panel_card.enabled && action.enabled,
                        primary: action.primary,
                        card_primary: panel_card.primary,
                        attention: panel_card.attention,
                        disabled_reason: if panel_card.enabled {
                            action.disabled_reason.clone()
                        } else {
                            action
                                .disabled_reason
                                .clone()
                                .or_else(|| Some("panel card is not enabled".to_string()))
                        },
                    }
                })
            })
            .collect::<Vec<_>>();
        let selected_panel_card_action = panel_card_actions.iter().find(|action| action.selected);
        let selected_panel_card_action_id =
            selected_panel_card_action.map(|action| action.id.clone());
        let selected_action_id = selected_panel_card_action.map(|action| action.action_id.clone());
        let default_panel_card_action = panel_card_actions
            .iter()
            .find(|action| action.default_panel);
        let default_panel_card_action_id =
            default_panel_card_action.map(|action| action.id.clone());
        let default_action_id = default_panel_card_action.map(|action| action.action_id.clone());
        let enabled_action_count = launch_plan
            .actions
            .iter()
            .filter(|action| action.enabled)
            .count();
        let primary_action_count = launch_plan
            .actions
            .iter()
            .filter(|action| action.primary)
            .count();
        let panel_card_action_count = panel_card_actions.len();
        let visible_panel_card_action_count = panel_card_actions
            .iter()
            .filter(|action| action.visible)
            .count();
        let enabled_panel_card_action_count = panel_card_actions
            .iter()
            .filter(|action| action.enabled)
            .count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARD_ACTIONS_SCHEMA_VERSION,
            package_name: panel_cards.package_name.clone(),
            source_fingerprint: panel_cards.source_fingerprint.clone(),
            title: panel_cards.title.clone(),
            startup_route: panel_cards.startup_route.clone(),
            ready: panel_cards.ready,
            severity: panel_cards.severity.clone(),
            attention_required: panel_cards.attention_required,
            primary_card_id: panel_cards.primary_card_id.clone(),
            primary_region_id: panel_cards.primary_region_id.clone(),
            active_item_id: panel_cards.active_item_id.clone(),
            active_route_id: panel_cards.active_route_id.clone(),
            active_route_path: panel_cards.active_route_path.clone(),
            default_route_id: panel_cards.default_route_id.clone(),
            default_route_path: panel_cards.default_route_path.clone(),
            active_breadcrumb_id: panel_cards.active_breadcrumb_id.clone(),
            active_breadcrumb_path: panel_cards.active_breadcrumb_path.clone(),
            default_breadcrumb_id: panel_cards.default_breadcrumb_id.clone(),
            default_breadcrumb_path: panel_cards.default_breadcrumb_path.clone(),
            selected_tab_id: panel_cards.selected_tab_id.clone(),
            selected_tab_path: panel_cards.selected_tab_path.clone(),
            default_tab_id: panel_cards.default_tab_id.clone(),
            default_tab_path: panel_cards.default_tab_path.clone(),
            selected_panel_id: panel_cards.selected_panel_id.clone(),
            selected_panel_path: panel_cards.selected_panel_path.clone(),
            default_panel_id: panel_cards.default_panel_id.clone(),
            default_panel_path: panel_cards.default_panel_path.clone(),
            selected_panel_card_id: panel_cards.selected_panel_card_id.clone(),
            selected_card_id: panel_cards.selected_card_id.clone(),
            default_panel_card_id: panel_cards.default_panel_card_id.clone(),
            default_card_id: panel_cards.default_card_id.clone(),
            selected_panel_card_action_id,
            selected_action_id,
            default_panel_card_action_id,
            default_action_id,
            route_count: panel_cards.route_count,
            visible_route_count: panel_cards.visible_route_count,
            enabled_route_count: panel_cards.enabled_route_count,
            breadcrumb_count: panel_cards.breadcrumb_count,
            visible_breadcrumb_count: panel_cards.visible_breadcrumb_count,
            enabled_breadcrumb_count: panel_cards.enabled_breadcrumb_count,
            tab_count: panel_cards.tab_count,
            visible_tab_count: panel_cards.visible_tab_count,
            enabled_tab_count: panel_cards.enabled_tab_count,
            panel_count: panel_cards.panel_count,
            visible_panel_count: panel_cards.visible_panel_count,
            enabled_panel_count: panel_cards.enabled_panel_count,
            panel_card_count: panel_cards.panel_card_count,
            visible_panel_card_count: panel_cards.visible_panel_card_count,
            enabled_panel_card_count: panel_cards.enabled_panel_card_count,
            action_count: launch_plan.action_count,
            enabled_action_count,
            primary_action_count,
            panel_card_action_count,
            visible_panel_card_action_count,
            enabled_panel_card_action_count,
            item_count: panel_cards.item_count,
            visible_item_count: panel_cards.visible_item_count,
            enabled_item_count: panel_cards.enabled_item_count,
            region_count: panel_cards.region_count,
            visible_region_count: panel_cards.visible_region_count,
            card_count: panel_cards.card_count,
            visible_card_count: panel_cards.visible_card_count,
            attention_card_count: panel_cards.attention_card_count,
            metric_card_count: panel_cards.metric_card_count,
            panel_card_actions,
            package_capability_id: panel_cards.package_capability_id.clone(),
            dashboard_capability_id: panel_cards.dashboard_capability_id.clone(),
            cards_capability_id: panel_cards.cards_capability_id.clone(),
            view_capability_id: panel_cards.view_capability_id.clone(),
            layout_capability_id: panel_cards.layout_capability_id.clone(),
            navigation_capability_id: panel_cards.navigation_capability_id.clone(),
            routes_capability_id: panel_cards.routes_capability_id.clone(),
            breadcrumbs_capability_id: panel_cards.breadcrumbs_capability_id.clone(),
            tabs_capability_id: panel_cards.tabs_capability_id.clone(),
            tab_panels_capability_id: panel_cards.tab_panels_capability_id.clone(),
            panel_cards_capability_id: panel_cards.panel_cards_capability_id.clone(),
            panel_card_actions_capability_id: "app-shell-dashboard-panel-card-actions-json"
                .to_string(),
            artifact_capability_count: panel_cards.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_panel_card_actions_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardActionDispatchItem {
    pub id: String,
    pub panel_card_action_id: String,
    pub panel_card_id: String,
    pub panel_id: String,
    pub card_id: String,
    pub action_id: String,
    pub label: String,
    pub target: String,
    pub panel_kind: String,
    pub role: String,
    pub path: String,
    pub position: usize,
    pub selected: bool,
    pub default_panel: bool,
    pub visible: bool,
    pub enabled: bool,
    pub dispatchable: bool,
    pub primary: bool,
    pub card_primary: bool,
    pub attention: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardActionDispatch {
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
    pub active_route_id: Option<String>,
    pub active_route_path: Option<String>,
    pub default_route_id: Option<String>,
    pub default_route_path: Option<String>,
    pub active_breadcrumb_id: Option<String>,
    pub active_breadcrumb_path: Option<String>,
    pub default_breadcrumb_id: Option<String>,
    pub default_breadcrumb_path: Option<String>,
    pub selected_tab_id: Option<String>,
    pub selected_tab_path: Option<String>,
    pub default_tab_id: Option<String>,
    pub default_tab_path: Option<String>,
    pub selected_panel_id: Option<String>,
    pub selected_panel_path: Option<String>,
    pub default_panel_id: Option<String>,
    pub default_panel_path: Option<String>,
    pub selected_panel_card_id: Option<String>,
    pub selected_card_id: Option<String>,
    pub default_panel_card_id: Option<String>,
    pub default_card_id: Option<String>,
    pub selected_panel_card_action_id: Option<String>,
    pub selected_action_dispatch_id: Option<String>,
    pub selected_action_id: Option<String>,
    pub default_panel_card_action_id: Option<String>,
    pub default_action_dispatch_id: Option<String>,
    pub default_action_id: Option<String>,
    pub route_count: usize,
    pub visible_route_count: usize,
    pub enabled_route_count: usize,
    pub breadcrumb_count: usize,
    pub visible_breadcrumb_count: usize,
    pub enabled_breadcrumb_count: usize,
    pub tab_count: usize,
    pub visible_tab_count: usize,
    pub enabled_tab_count: usize,
    pub panel_count: usize,
    pub visible_panel_count: usize,
    pub enabled_panel_count: usize,
    pub panel_card_count: usize,
    pub visible_panel_card_count: usize,
    pub enabled_panel_card_count: usize,
    pub action_count: usize,
    pub enabled_action_count: usize,
    pub primary_action_count: usize,
    pub panel_card_action_count: usize,
    pub visible_panel_card_action_count: usize,
    pub enabled_panel_card_action_count: usize,
    pub action_dispatch_count: usize,
    pub visible_action_dispatch_count: usize,
    pub enabled_action_dispatch_count: usize,
    pub item_count: usize,
    pub visible_item_count: usize,
    pub enabled_item_count: usize,
    pub region_count: usize,
    pub visible_region_count: usize,
    pub card_count: usize,
    pub visible_card_count: usize,
    pub attention_card_count: usize,
    pub metric_card_count: usize,
    pub action_dispatches: Vec<BerkeleyAppShellDashboardActionDispatchItem>,
    pub package_capability_id: String,
    pub dashboard_capability_id: String,
    pub cards_capability_id: String,
    pub view_capability_id: String,
    pub layout_capability_id: String,
    pub navigation_capability_id: String,
    pub routes_capability_id: String,
    pub breadcrumbs_capability_id: String,
    pub tabs_capability_id: String,
    pub tab_panels_capability_id: String,
    pub panel_cards_capability_id: String,
    pub panel_card_actions_capability_id: String,
    pub action_dispatch_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardActionDispatch {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_shell_handoff(&BerkeleyAppShellHandoff::from_bootstrap_snapshot(snapshot))
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_panel_card_actions(
            &BerkeleyAppShellDashboardPanelCardActions::from_shell_handoff(handoff),
        )
    }

    pub fn from_panel_card_actions(
        panel_card_actions: &BerkeleyAppShellDashboardPanelCardActions,
    ) -> Self {
        let action_dispatches = panel_card_actions
            .panel_card_actions
            .iter()
            .map(|action| {
                let dispatchable = action.visible && action.enabled;
                let disabled_reason = if dispatchable {
                    None
                } else {
                    action.disabled_reason.clone().or_else(|| {
                        Some(if !action.visible {
                            "panel card action is not visible".to_string()
                        } else {
                            "panel card action is not enabled".to_string()
                        })
                    })
                };

                BerkeleyAppShellDashboardActionDispatchItem {
                    id: format!("dashboard.action-dispatch.{}", action.role),
                    panel_card_action_id: action.id.clone(),
                    panel_card_id: action.panel_card_id.clone(),
                    panel_id: action.panel_id.clone(),
                    card_id: action.card_id.clone(),
                    action_id: action.action_id.clone(),
                    label: action.label.clone(),
                    target: action.target.clone(),
                    panel_kind: action.panel_kind.clone(),
                    role: action.role.clone(),
                    path: action.path.clone(),
                    position: action.position,
                    selected: action.selected,
                    default_panel: action.default_panel,
                    visible: action.visible,
                    enabled: action.enabled,
                    dispatchable,
                    primary: action.primary,
                    card_primary: action.card_primary,
                    attention: action.attention,
                    disabled_reason,
                }
            })
            .collect::<Vec<_>>();
        let selected_action_dispatch = action_dispatches.iter().find(|dispatch| dispatch.selected);
        let selected_action_dispatch_id =
            selected_action_dispatch.map(|dispatch| dispatch.id.clone());
        let selected_action_id =
            selected_action_dispatch.map(|dispatch| dispatch.action_id.clone());
        let default_action_dispatch = action_dispatches
            .iter()
            .find(|dispatch| dispatch.default_panel);
        let default_action_dispatch_id =
            default_action_dispatch.map(|dispatch| dispatch.id.clone());
        let default_action_id = default_action_dispatch.map(|dispatch| dispatch.action_id.clone());
        let action_dispatch_count = action_dispatches.len();
        let visible_action_dispatch_count = action_dispatches
            .iter()
            .filter(|dispatch| dispatch.visible)
            .count();
        let enabled_action_dispatch_count = action_dispatches
            .iter()
            .filter(|dispatch| dispatch.dispatchable)
            .count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_ACTION_DISPATCH_SCHEMA_VERSION,
            package_name: panel_card_actions.package_name.clone(),
            source_fingerprint: panel_card_actions.source_fingerprint.clone(),
            title: panel_card_actions.title.clone(),
            startup_route: panel_card_actions.startup_route.clone(),
            ready: panel_card_actions.ready,
            severity: panel_card_actions.severity.clone(),
            attention_required: panel_card_actions.attention_required,
            primary_card_id: panel_card_actions.primary_card_id.clone(),
            primary_region_id: panel_card_actions.primary_region_id.clone(),
            active_item_id: panel_card_actions.active_item_id.clone(),
            active_route_id: panel_card_actions.active_route_id.clone(),
            active_route_path: panel_card_actions.active_route_path.clone(),
            default_route_id: panel_card_actions.default_route_id.clone(),
            default_route_path: panel_card_actions.default_route_path.clone(),
            active_breadcrumb_id: panel_card_actions.active_breadcrumb_id.clone(),
            active_breadcrumb_path: panel_card_actions.active_breadcrumb_path.clone(),
            default_breadcrumb_id: panel_card_actions.default_breadcrumb_id.clone(),
            default_breadcrumb_path: panel_card_actions.default_breadcrumb_path.clone(),
            selected_tab_id: panel_card_actions.selected_tab_id.clone(),
            selected_tab_path: panel_card_actions.selected_tab_path.clone(),
            default_tab_id: panel_card_actions.default_tab_id.clone(),
            default_tab_path: panel_card_actions.default_tab_path.clone(),
            selected_panel_id: panel_card_actions.selected_panel_id.clone(),
            selected_panel_path: panel_card_actions.selected_panel_path.clone(),
            default_panel_id: panel_card_actions.default_panel_id.clone(),
            default_panel_path: panel_card_actions.default_panel_path.clone(),
            selected_panel_card_id: panel_card_actions.selected_panel_card_id.clone(),
            selected_card_id: panel_card_actions.selected_card_id.clone(),
            default_panel_card_id: panel_card_actions.default_panel_card_id.clone(),
            default_card_id: panel_card_actions.default_card_id.clone(),
            selected_panel_card_action_id: panel_card_actions.selected_panel_card_action_id.clone(),
            selected_action_dispatch_id,
            selected_action_id,
            default_panel_card_action_id: panel_card_actions.default_panel_card_action_id.clone(),
            default_action_dispatch_id,
            default_action_id,
            route_count: panel_card_actions.route_count,
            visible_route_count: panel_card_actions.visible_route_count,
            enabled_route_count: panel_card_actions.enabled_route_count,
            breadcrumb_count: panel_card_actions.breadcrumb_count,
            visible_breadcrumb_count: panel_card_actions.visible_breadcrumb_count,
            enabled_breadcrumb_count: panel_card_actions.enabled_breadcrumb_count,
            tab_count: panel_card_actions.tab_count,
            visible_tab_count: panel_card_actions.visible_tab_count,
            enabled_tab_count: panel_card_actions.enabled_tab_count,
            panel_count: panel_card_actions.panel_count,
            visible_panel_count: panel_card_actions.visible_panel_count,
            enabled_panel_count: panel_card_actions.enabled_panel_count,
            panel_card_count: panel_card_actions.panel_card_count,
            visible_panel_card_count: panel_card_actions.visible_panel_card_count,
            enabled_panel_card_count: panel_card_actions.enabled_panel_card_count,
            action_count: panel_card_actions.action_count,
            enabled_action_count: panel_card_actions.enabled_action_count,
            primary_action_count: panel_card_actions.primary_action_count,
            panel_card_action_count: panel_card_actions.panel_card_action_count,
            visible_panel_card_action_count: panel_card_actions.visible_panel_card_action_count,
            enabled_panel_card_action_count: panel_card_actions.enabled_panel_card_action_count,
            action_dispatch_count,
            visible_action_dispatch_count,
            enabled_action_dispatch_count,
            item_count: panel_card_actions.item_count,
            visible_item_count: panel_card_actions.visible_item_count,
            enabled_item_count: panel_card_actions.enabled_item_count,
            region_count: panel_card_actions.region_count,
            visible_region_count: panel_card_actions.visible_region_count,
            card_count: panel_card_actions.card_count,
            visible_card_count: panel_card_actions.visible_card_count,
            attention_card_count: panel_card_actions.attention_card_count,
            metric_card_count: panel_card_actions.metric_card_count,
            action_dispatches,
            package_capability_id: panel_card_actions.package_capability_id.clone(),
            dashboard_capability_id: panel_card_actions.dashboard_capability_id.clone(),
            cards_capability_id: panel_card_actions.cards_capability_id.clone(),
            view_capability_id: panel_card_actions.view_capability_id.clone(),
            layout_capability_id: panel_card_actions.layout_capability_id.clone(),
            navigation_capability_id: panel_card_actions.navigation_capability_id.clone(),
            routes_capability_id: panel_card_actions.routes_capability_id.clone(),
            breadcrumbs_capability_id: panel_card_actions.breadcrumbs_capability_id.clone(),
            tabs_capability_id: panel_card_actions.tabs_capability_id.clone(),
            tab_panels_capability_id: panel_card_actions.tab_panels_capability_id.clone(),
            panel_cards_capability_id: panel_card_actions.panel_cards_capability_id.clone(),
            panel_card_actions_capability_id: panel_card_actions
                .panel_card_actions_capability_id
                .clone(),
            action_dispatch_capability_id: "app-shell-dashboard-action-dispatch-json".to_string(),
            artifact_capability_count: panel_card_actions.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_action_dispatch_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchEvent {
    pub id: String,
    pub action_dispatch_id: String,
    pub panel_card_action_id: String,
    pub action_id: String,
    pub kind: String,
    pub severity: String,
    pub message: String,
    pub label: String,
    pub target: String,
    pub role: String,
    pub path: String,
    pub position: usize,
    pub selected: bool,
    pub default_dispatch: bool,
    pub dispatchable: bool,
    pub primary: bool,
    pub attention: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchEvents {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub selected_action_dispatch_id: Option<String>,
    pub selected_dispatch_event_id: Option<String>,
    pub selected_action_id: Option<String>,
    pub default_action_dispatch_id: Option<String>,
    pub default_dispatch_event_id: Option<String>,
    pub default_action_id: Option<String>,
    pub action_dispatch_count: usize,
    pub dispatch_event_count: usize,
    pub dispatch_ready_event_count: usize,
    pub dispatch_blocked_event_count: usize,
    pub attention_dispatch_event_count: usize,
    pub selected_dispatchable: bool,
    pub default_dispatchable: bool,
    pub dispatch_events: Vec<BerkeleyAppShellDashboardDispatchEvent>,
    pub action_dispatch_capability_id: String,
    pub dispatch_events_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchEvents {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_action_dispatch(
            &BerkeleyAppShellDashboardActionDispatch::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_action_dispatch(
            &BerkeleyAppShellDashboardActionDispatch::from_shell_handoff(handoff),
        )
    }

    pub fn from_action_dispatch(dispatch: &BerkeleyAppShellDashboardActionDispatch) -> Self {
        let dispatch_events = dispatch
            .action_dispatches
            .iter()
            .map(|action_dispatch| {
                let kind = if action_dispatch.dispatchable {
                    "dispatch-ready"
                } else {
                    "dispatch-blocked"
                };
                let severity = if action_dispatch.dispatchable {
                    if action_dispatch.attention {
                        "warning"
                    } else {
                        "ready"
                    }
                } else {
                    "blocked"
                };
                let message = if action_dispatch.dispatchable {
                    format!("{} dispatch ready", action_dispatch.label)
                } else {
                    format!(
                        "{} dispatch blocked: {}",
                        action_dispatch.label,
                        action_dispatch
                            .disabled_reason
                            .as_deref()
                            .unwrap_or("action dispatch is not available")
                    )
                };

                BerkeleyAppShellDashboardDispatchEvent {
                    id: format!("dashboard.dispatch-event.{}", action_dispatch.role),
                    action_dispatch_id: action_dispatch.id.clone(),
                    panel_card_action_id: action_dispatch.panel_card_action_id.clone(),
                    action_id: action_dispatch.action_id.clone(),
                    kind: kind.to_string(),
                    severity: severity.to_string(),
                    message,
                    label: action_dispatch.label.clone(),
                    target: action_dispatch.target.clone(),
                    role: action_dispatch.role.clone(),
                    path: action_dispatch.path.clone(),
                    position: action_dispatch.position,
                    selected: action_dispatch.selected,
                    default_dispatch: action_dispatch.default_panel,
                    dispatchable: action_dispatch.dispatchable,
                    primary: action_dispatch.primary,
                    attention: action_dispatch.attention,
                    disabled_reason: action_dispatch.disabled_reason.clone(),
                }
            })
            .collect::<Vec<_>>();
        let selected_dispatch_event = dispatch_events.iter().find(|event| event.selected);
        let default_dispatch_event = dispatch_events.iter().find(|event| event.default_dispatch);
        let dispatch_event_count = dispatch_events.len();
        let dispatch_ready_event_count = dispatch_events
            .iter()
            .filter(|event| event.dispatchable)
            .count();
        let dispatch_blocked_event_count = dispatch_events
            .iter()
            .filter(|event| !event.dispatchable)
            .count();
        let attention_dispatch_event_count = dispatch_events
            .iter()
            .filter(|event| event.attention)
            .count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_EVENTS_SCHEMA_VERSION,
            package_name: dispatch.package_name.clone(),
            source_fingerprint: dispatch.source_fingerprint.clone(),
            title: dispatch.title.clone(),
            startup_route: dispatch.startup_route.clone(),
            ready: dispatch.ready,
            severity: dispatch.severity.clone(),
            attention_required: dispatch.attention_required,
            selected_action_dispatch_id: dispatch.selected_action_dispatch_id.clone(),
            selected_dispatch_event_id: selected_dispatch_event.map(|event| event.id.clone()),
            selected_action_id: dispatch.selected_action_id.clone(),
            default_action_dispatch_id: dispatch.default_action_dispatch_id.clone(),
            default_dispatch_event_id: default_dispatch_event.map(|event| event.id.clone()),
            default_action_id: dispatch.default_action_id.clone(),
            action_dispatch_count: dispatch.action_dispatch_count,
            dispatch_event_count,
            dispatch_ready_event_count,
            dispatch_blocked_event_count,
            attention_dispatch_event_count,
            selected_dispatchable: selected_dispatch_event
                .map(|event| event.dispatchable)
                .unwrap_or(false),
            default_dispatchable: default_dispatch_event
                .map(|event| event.dispatchable)
                .unwrap_or(false),
            dispatch_events,
            action_dispatch_capability_id: dispatch.action_dispatch_capability_id.clone(),
            dispatch_events_capability_id: "app-shell-dashboard-dispatch-events-json".to_string(),
            artifact_capability_count: dispatch.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_events_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueItem {
    pub id: String,
    pub dispatch_event_id: String,
    pub action_dispatch_id: String,
    pub panel_card_action_id: String,
    pub action_id: String,
    pub queue_state: String,
    pub severity: String,
    pub message: String,
    pub label: String,
    pub target: String,
    pub role: String,
    pub path: String,
    pub position: usize,
    pub selected: bool,
    pub default_dispatch: bool,
    pub queued: bool,
    pub blocked: bool,
    pub dispatchable: bool,
    pub primary: bool,
    pub attention: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueue {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub selected_action_dispatch_id: Option<String>,
    pub selected_dispatch_event_id: Option<String>,
    pub selected_dispatch_queue_item_id: Option<String>,
    pub selected_action_id: Option<String>,
    pub default_action_dispatch_id: Option<String>,
    pub default_dispatch_event_id: Option<String>,
    pub default_dispatch_queue_item_id: Option<String>,
    pub default_action_id: Option<String>,
    pub action_dispatch_count: usize,
    pub dispatch_event_count: usize,
    pub dispatch_ready_event_count: usize,
    pub dispatch_blocked_event_count: usize,
    pub dispatch_queue_item_count: usize,
    pub queued_dispatch_count: usize,
    pub blocked_dispatch_count: usize,
    pub attention_dispatch_queue_item_count: usize,
    pub selected_queued: bool,
    pub default_queued: bool,
    pub dispatch_queue_items: Vec<BerkeleyAppShellDashboardDispatchQueueItem>,
    pub action_dispatch_capability_id: String,
    pub dispatch_events_capability_id: String,
    pub dispatch_queue_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchQueue {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dispatch_events(
            &BerkeleyAppShellDashboardDispatchEvents::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dispatch_events(
            &BerkeleyAppShellDashboardDispatchEvents::from_shell_handoff(handoff),
        )
    }

    pub fn from_dispatch_events(dispatch_events: &BerkeleyAppShellDashboardDispatchEvents) -> Self {
        let dispatch_queue_items = dispatch_events
            .dispatch_events
            .iter()
            .map(|event| {
                let queued = event.dispatchable;
                let blocked = !event.dispatchable;
                let queue_state = if queued { "queued" } else { "blocked" };
                let message = if queued {
                    format!("{} queued for dispatch", event.label)
                } else {
                    format!(
                        "{} cannot be queued: {}",
                        event.label,
                        event
                            .disabled_reason
                            .as_deref()
                            .unwrap_or("dispatch is not available")
                    )
                };

                BerkeleyAppShellDashboardDispatchQueueItem {
                    id: format!("dashboard.dispatch-queue.{}", event.role),
                    dispatch_event_id: event.id.clone(),
                    action_dispatch_id: event.action_dispatch_id.clone(),
                    panel_card_action_id: event.panel_card_action_id.clone(),
                    action_id: event.action_id.clone(),
                    queue_state: queue_state.to_string(),
                    severity: event.severity.clone(),
                    message,
                    label: event.label.clone(),
                    target: event.target.clone(),
                    role: event.role.clone(),
                    path: event.path.clone(),
                    position: event.position,
                    selected: event.selected,
                    default_dispatch: event.default_dispatch,
                    queued,
                    blocked,
                    dispatchable: event.dispatchable,
                    primary: event.primary,
                    attention: event.attention,
                    disabled_reason: event.disabled_reason.clone(),
                }
            })
            .collect::<Vec<_>>();
        let selected_dispatch_queue_item = dispatch_queue_items.iter().find(|item| item.selected);
        let default_dispatch_queue_item = dispatch_queue_items
            .iter()
            .find(|item| item.default_dispatch);
        let dispatch_queue_item_count = dispatch_queue_items.len();
        let queued_dispatch_count = dispatch_queue_items
            .iter()
            .filter(|item| item.queued)
            .count();
        let blocked_dispatch_count = dispatch_queue_items
            .iter()
            .filter(|item| item.blocked)
            .count();
        let attention_dispatch_queue_item_count = dispatch_queue_items
            .iter()
            .filter(|item| item.attention)
            .count();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SCHEMA_VERSION,
            package_name: dispatch_events.package_name.clone(),
            source_fingerprint: dispatch_events.source_fingerprint.clone(),
            title: dispatch_events.title.clone(),
            startup_route: dispatch_events.startup_route.clone(),
            ready: dispatch_events.ready,
            severity: dispatch_events.severity.clone(),
            attention_required: dispatch_events.attention_required,
            selected_action_dispatch_id: dispatch_events.selected_action_dispatch_id.clone(),
            selected_dispatch_event_id: dispatch_events.selected_dispatch_event_id.clone(),
            selected_dispatch_queue_item_id: selected_dispatch_queue_item
                .map(|item| item.id.clone()),
            selected_action_id: dispatch_events.selected_action_id.clone(),
            default_action_dispatch_id: dispatch_events.default_action_dispatch_id.clone(),
            default_dispatch_event_id: dispatch_events.default_dispatch_event_id.clone(),
            default_dispatch_queue_item_id: default_dispatch_queue_item.map(|item| item.id.clone()),
            default_action_id: dispatch_events.default_action_id.clone(),
            action_dispatch_count: dispatch_events.action_dispatch_count,
            dispatch_event_count: dispatch_events.dispatch_event_count,
            dispatch_ready_event_count: dispatch_events.dispatch_ready_event_count,
            dispatch_blocked_event_count: dispatch_events.dispatch_blocked_event_count,
            dispatch_queue_item_count,
            queued_dispatch_count,
            blocked_dispatch_count,
            attention_dispatch_queue_item_count,
            selected_queued: selected_dispatch_queue_item
                .map(|item| item.queued)
                .unwrap_or(false),
            default_queued: default_dispatch_queue_item
                .map(|item| item.queued)
                .unwrap_or(false),
            dispatch_queue_items,
            action_dispatch_capability_id: dispatch_events.action_dispatch_capability_id.clone(),
            dispatch_events_capability_id: dispatch_events.dispatch_events_capability_id.clone(),
            dispatch_queue_capability_id: "app-shell-dashboard-dispatch-queue-json".to_string(),
            artifact_capability_count: dispatch_events.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_queue_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueSummary {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub selected_action_dispatch_id: Option<String>,
    pub selected_dispatch_event_id: Option<String>,
    pub selected_dispatch_queue_item_id: Option<String>,
    pub selected_action_id: Option<String>,
    pub default_action_dispatch_id: Option<String>,
    pub default_dispatch_event_id: Option<String>,
    pub default_dispatch_queue_item_id: Option<String>,
    pub default_action_id: Option<String>,
    pub action_dispatch_count: usize,
    pub dispatch_event_count: usize,
    pub dispatch_ready_event_count: usize,
    pub dispatch_blocked_event_count: usize,
    pub dispatch_queue_item_count: usize,
    pub queued_dispatch_count: usize,
    pub blocked_dispatch_count: usize,
    pub attention_dispatch_queue_item_count: usize,
    pub selected_queued: bool,
    pub default_queued: bool,
    pub first_queued_dispatch_queue_item_id: Option<String>,
    pub first_blocked_dispatch_queue_item_id: Option<String>,
    pub first_attention_dispatch_queue_item_id: Option<String>,
    pub queued_dispatch_queue_item_ids: Vec<String>,
    pub blocked_dispatch_queue_item_ids: Vec<String>,
    pub attention_dispatch_queue_item_ids: Vec<String>,
    pub dispatch_queue_capability_id: String,
    pub dispatch_queue_summary_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchQueueSummary {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dispatch_queue(
            &BerkeleyAppShellDashboardDispatchQueue::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dispatch_queue(&BerkeleyAppShellDashboardDispatchQueue::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dispatch_queue(queue: &BerkeleyAppShellDashboardDispatchQueue) -> Self {
        let queued_dispatch_queue_item_ids = queue
            .dispatch_queue_items
            .iter()
            .filter(|item| item.queued)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        let blocked_dispatch_queue_item_ids = queue
            .dispatch_queue_items
            .iter()
            .filter(|item| item.blocked)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        let attention_dispatch_queue_item_ids = queue
            .dispatch_queue_items
            .iter()
            .filter(|item| item.attention)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SUMMARY_SCHEMA_VERSION,
            package_name: queue.package_name.clone(),
            source_fingerprint: queue.source_fingerprint.clone(),
            title: queue.title.clone(),
            startup_route: queue.startup_route.clone(),
            ready: queue.ready,
            severity: queue.severity.clone(),
            attention_required: queue.attention_required,
            selected_action_dispatch_id: queue.selected_action_dispatch_id.clone(),
            selected_dispatch_event_id: queue.selected_dispatch_event_id.clone(),
            selected_dispatch_queue_item_id: queue.selected_dispatch_queue_item_id.clone(),
            selected_action_id: queue.selected_action_id.clone(),
            default_action_dispatch_id: queue.default_action_dispatch_id.clone(),
            default_dispatch_event_id: queue.default_dispatch_event_id.clone(),
            default_dispatch_queue_item_id: queue.default_dispatch_queue_item_id.clone(),
            default_action_id: queue.default_action_id.clone(),
            action_dispatch_count: queue.action_dispatch_count,
            dispatch_event_count: queue.dispatch_event_count,
            dispatch_ready_event_count: queue.dispatch_ready_event_count,
            dispatch_blocked_event_count: queue.dispatch_blocked_event_count,
            dispatch_queue_item_count: queue.dispatch_queue_item_count,
            queued_dispatch_count: queue.queued_dispatch_count,
            blocked_dispatch_count: queue.blocked_dispatch_count,
            attention_dispatch_queue_item_count: queue.attention_dispatch_queue_item_count,
            selected_queued: queue.selected_queued,
            default_queued: queue.default_queued,
            first_queued_dispatch_queue_item_id: queued_dispatch_queue_item_ids.first().cloned(),
            first_blocked_dispatch_queue_item_id: blocked_dispatch_queue_item_ids.first().cloned(),
            first_attention_dispatch_queue_item_id: attention_dispatch_queue_item_ids
                .first()
                .cloned(),
            queued_dispatch_queue_item_ids,
            blocked_dispatch_queue_item_ids,
            attention_dispatch_queue_item_ids,
            dispatch_queue_capability_id: queue.dispatch_queue_capability_id.clone(),
            dispatch_queue_summary_capability_id: "app-shell-dashboard-dispatch-queue-summary-json"
                .to_string(),
            artifact_capability_count: queue.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_queue_summary_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueDigest {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub headline_dispatch_queue_item_id: Option<String>,
    pub headline_dispatch_event_id: Option<String>,
    pub headline_action_dispatch_id: Option<String>,
    pub headline_panel_card_action_id: Option<String>,
    pub headline_action_id: Option<String>,
    pub headline_queue_state: Option<String>,
    pub headline_message: String,
    pub headline_label: Option<String>,
    pub headline_target: Option<String>,
    pub headline_role: Option<String>,
    pub headline_path: Option<String>,
    pub headline_position: Option<usize>,
    pub headline_selected: bool,
    pub headline_default_dispatch: bool,
    pub headline_queued: bool,
    pub headline_blocked: bool,
    pub headline_dispatchable: bool,
    pub headline_primary: bool,
    pub headline_attention: bool,
    pub headline_disabled_reason: Option<String>,
    pub selected_action_dispatch_id: Option<String>,
    pub selected_dispatch_event_id: Option<String>,
    pub selected_dispatch_queue_item_id: Option<String>,
    pub selected_action_id: Option<String>,
    pub default_action_dispatch_id: Option<String>,
    pub default_dispatch_event_id: Option<String>,
    pub default_dispatch_queue_item_id: Option<String>,
    pub default_action_id: Option<String>,
    pub action_dispatch_count: usize,
    pub dispatch_event_count: usize,
    pub dispatch_ready_event_count: usize,
    pub dispatch_blocked_event_count: usize,
    pub dispatch_queue_item_count: usize,
    pub queued_dispatch_count: usize,
    pub blocked_dispatch_count: usize,
    pub attention_dispatch_queue_item_count: usize,
    pub selected_queued: bool,
    pub default_queued: bool,
    pub first_queued_dispatch_queue_item_id: Option<String>,
    pub first_blocked_dispatch_queue_item_id: Option<String>,
    pub first_attention_dispatch_queue_item_id: Option<String>,
    pub dispatch_queue_capability_id: String,
    pub dispatch_queue_summary_capability_id: String,
    pub dispatch_queue_digest_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchQueueDigest {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dispatch_queue(
            &BerkeleyAppShellDashboardDispatchQueue::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dispatch_queue(&BerkeleyAppShellDashboardDispatchQueue::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dispatch_queue(queue: &BerkeleyAppShellDashboardDispatchQueue) -> Self {
        let summary = BerkeleyAppShellDashboardDispatchQueueSummary::from_dispatch_queue(queue);
        let headline_dispatch_queue_item_id = summary
            .selected_dispatch_queue_item_id
            .clone()
            .or_else(|| summary.default_dispatch_queue_item_id.clone())
            .or_else(|| summary.first_attention_dispatch_queue_item_id.clone())
            .or_else(|| summary.first_blocked_dispatch_queue_item_id.clone())
            .or_else(|| summary.first_queued_dispatch_queue_item_id.clone());
        let headline_dispatch_queue_item =
            headline_dispatch_queue_item_id.as_ref().and_then(|id| {
                queue
                    .dispatch_queue_items
                    .iter()
                    .find(|item| item.id == id.as_str())
            });

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_DIGEST_SCHEMA_VERSION,
            package_name: summary.package_name.clone(),
            source_fingerprint: summary.source_fingerprint.clone(),
            title: summary.title.clone(),
            startup_route: summary.startup_route.clone(),
            ready: summary.ready,
            severity: summary.severity.clone(),
            attention_required: summary.attention_required,
            headline_dispatch_queue_item_id,
            headline_dispatch_event_id: headline_dispatch_queue_item
                .map(|item| item.dispatch_event_id.clone()),
            headline_action_dispatch_id: headline_dispatch_queue_item
                .map(|item| item.action_dispatch_id.clone()),
            headline_panel_card_action_id: headline_dispatch_queue_item
                .map(|item| item.panel_card_action_id.clone()),
            headline_action_id: headline_dispatch_queue_item.map(|item| item.action_id.clone()),
            headline_queue_state: headline_dispatch_queue_item.map(|item| item.queue_state.clone()),
            headline_message: headline_dispatch_queue_item
                .map(|item| item.message.clone())
                .unwrap_or_else(|| {
                    if summary.ready {
                        "Berkeley SPICE dashboard dispatch queue ready".to_string()
                    } else {
                        "Berkeley SPICE dashboard dispatch queue blocked".to_string()
                    }
                }),
            headline_label: headline_dispatch_queue_item.map(|item| item.label.clone()),
            headline_target: headline_dispatch_queue_item.map(|item| item.target.clone()),
            headline_role: headline_dispatch_queue_item.map(|item| item.role.clone()),
            headline_path: headline_dispatch_queue_item.map(|item| item.path.clone()),
            headline_position: headline_dispatch_queue_item.map(|item| item.position),
            headline_selected: headline_dispatch_queue_item
                .map(|item| item.selected)
                .unwrap_or(false),
            headline_default_dispatch: headline_dispatch_queue_item
                .map(|item| item.default_dispatch)
                .unwrap_or(false),
            headline_queued: headline_dispatch_queue_item
                .map(|item| item.queued)
                .unwrap_or(false),
            headline_blocked: headline_dispatch_queue_item
                .map(|item| item.blocked)
                .unwrap_or(false),
            headline_dispatchable: headline_dispatch_queue_item
                .map(|item| item.dispatchable)
                .unwrap_or(false),
            headline_primary: headline_dispatch_queue_item
                .map(|item| item.primary)
                .unwrap_or(false),
            headline_attention: headline_dispatch_queue_item
                .map(|item| item.attention)
                .unwrap_or(false),
            headline_disabled_reason: headline_dispatch_queue_item
                .and_then(|item| item.disabled_reason.clone()),
            selected_action_dispatch_id: summary.selected_action_dispatch_id.clone(),
            selected_dispatch_event_id: summary.selected_dispatch_event_id.clone(),
            selected_dispatch_queue_item_id: summary.selected_dispatch_queue_item_id.clone(),
            selected_action_id: summary.selected_action_id.clone(),
            default_action_dispatch_id: summary.default_action_dispatch_id.clone(),
            default_dispatch_event_id: summary.default_dispatch_event_id.clone(),
            default_dispatch_queue_item_id: summary.default_dispatch_queue_item_id.clone(),
            default_action_id: summary.default_action_id.clone(),
            action_dispatch_count: summary.action_dispatch_count,
            dispatch_event_count: summary.dispatch_event_count,
            dispatch_ready_event_count: summary.dispatch_ready_event_count,
            dispatch_blocked_event_count: summary.dispatch_blocked_event_count,
            dispatch_queue_item_count: summary.dispatch_queue_item_count,
            queued_dispatch_count: summary.queued_dispatch_count,
            blocked_dispatch_count: summary.blocked_dispatch_count,
            attention_dispatch_queue_item_count: summary.attention_dispatch_queue_item_count,
            selected_queued: summary.selected_queued,
            default_queued: summary.default_queued,
            first_queued_dispatch_queue_item_id: summary
                .first_queued_dispatch_queue_item_id
                .clone(),
            first_blocked_dispatch_queue_item_id: summary
                .first_blocked_dispatch_queue_item_id
                .clone(),
            first_attention_dispatch_queue_item_id: summary
                .first_attention_dispatch_queue_item_id
                .clone(),
            dispatch_queue_capability_id: summary.dispatch_queue_capability_id.clone(),
            dispatch_queue_summary_capability_id: summary
                .dispatch_queue_summary_capability_id
                .clone(),
            dispatch_queue_digest_capability_id: "app-shell-dashboard-dispatch-queue-digest-json"
                .to_string(),
            artifact_capability_count: summary.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_queue_digest_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLane {
    pub id: String,
    pub title: String,
    pub queue_state: String,
    pub severity: String,
    pub dispatch_queue_item_count: usize,
    pub dispatch_queue_item_ids: Vec<String>,
    pub selected: bool,
    pub default_dispatch: bool,
    pub primary: bool,
    pub attention: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLanes {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub headline_dispatch_queue_item_id: Option<String>,
    pub headline_queue_state: Option<String>,
    pub headline_message: String,
    pub selected_dispatch_queue_item_id: Option<String>,
    pub default_dispatch_queue_item_id: Option<String>,
    pub first_queued_dispatch_queue_item_id: Option<String>,
    pub first_blocked_dispatch_queue_item_id: Option<String>,
    pub first_attention_dispatch_queue_item_id: Option<String>,
    pub dispatch_queue_item_count: usize,
    pub queued_dispatch_count: usize,
    pub blocked_dispatch_count: usize,
    pub attention_dispatch_queue_item_count: usize,
    pub lane_count: usize,
    pub active_lane_id: Option<String>,
    pub attention_lane_id: Option<String>,
    pub lanes: Vec<BerkeleyAppShellDashboardDispatchQueueLane>,
    pub dispatch_queue_capability_id: String,
    pub dispatch_queue_summary_capability_id: String,
    pub dispatch_queue_digest_capability_id: String,
    pub dispatch_queue_lanes_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchQueueLanes {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_dispatch_queue(
            &BerkeleyAppShellDashboardDispatchQueue::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_dispatch_queue(&BerkeleyAppShellDashboardDispatchQueue::from_shell_handoff(
            handoff,
        ))
    }

    pub fn from_dispatch_queue(queue: &BerkeleyAppShellDashboardDispatchQueue) -> Self {
        let summary = BerkeleyAppShellDashboardDispatchQueueSummary::from_dispatch_queue(queue);
        let digest = BerkeleyAppShellDashboardDispatchQueueDigest::from_dispatch_queue(queue);
        let lanes = vec![
            Self::lane_from_item_ids(
                queue,
                "dashboard.dispatch-queue-lane.queued",
                "Queued dispatches",
                "queued",
                summary.queued_dispatch_queue_item_ids.clone(),
            ),
            Self::lane_from_item_ids(
                queue,
                "dashboard.dispatch-queue-lane.blocked",
                "Blocked dispatches",
                "blocked",
                summary.blocked_dispatch_queue_item_ids.clone(),
            ),
            Self::lane_from_item_ids(
                queue,
                "dashboard.dispatch-queue-lane.attention",
                "Attention dispatches",
                "attention",
                summary.attention_dispatch_queue_item_ids.clone(),
            ),
        ];
        let active_lane_id =
            digest
                .headline_dispatch_queue_item_id
                .as_ref()
                .and_then(|headline_id| {
                    lanes
                        .iter()
                        .filter(|lane| lane.dispatch_queue_item_ids.contains(headline_id))
                        .find(|lane| lane.queue_state == "attention")
                        .or_else(|| {
                            lanes
                                .iter()
                                .filter(|lane| lane.dispatch_queue_item_ids.contains(headline_id))
                                .find(|lane| lane.queue_state == "blocked")
                        })
                        .or_else(|| {
                            lanes
                                .iter()
                                .find(|lane| lane.dispatch_queue_item_ids.contains(headline_id))
                        })
                        .map(|lane| lane.id.clone())
                });
        let attention_lane_id = summary
            .first_attention_dispatch_queue_item_id
            .as_ref()
            .map(|_| "dashboard.dispatch-queue-lane.attention".to_string());

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANES_SCHEMA_VERSION,
            package_name: summary.package_name.clone(),
            source_fingerprint: summary.source_fingerprint.clone(),
            title: summary.title.clone(),
            startup_route: summary.startup_route.clone(),
            ready: summary.ready,
            severity: summary.severity.clone(),
            attention_required: summary.attention_required,
            headline_dispatch_queue_item_id: digest.headline_dispatch_queue_item_id.clone(),
            headline_queue_state: digest.headline_queue_state.clone(),
            headline_message: digest.headline_message.clone(),
            selected_dispatch_queue_item_id: summary.selected_dispatch_queue_item_id.clone(),
            default_dispatch_queue_item_id: summary.default_dispatch_queue_item_id.clone(),
            first_queued_dispatch_queue_item_id: summary
                .first_queued_dispatch_queue_item_id
                .clone(),
            first_blocked_dispatch_queue_item_id: summary
                .first_blocked_dispatch_queue_item_id
                .clone(),
            first_attention_dispatch_queue_item_id: summary
                .first_attention_dispatch_queue_item_id
                .clone(),
            dispatch_queue_item_count: summary.dispatch_queue_item_count,
            queued_dispatch_count: summary.queued_dispatch_count,
            blocked_dispatch_count: summary.blocked_dispatch_count,
            attention_dispatch_queue_item_count: summary.attention_dispatch_queue_item_count,
            lane_count: lanes.len(),
            active_lane_id,
            attention_lane_id,
            lanes,
            dispatch_queue_capability_id: summary.dispatch_queue_capability_id.clone(),
            dispatch_queue_summary_capability_id: summary
                .dispatch_queue_summary_capability_id
                .clone(),
            dispatch_queue_digest_capability_id: digest.dispatch_queue_digest_capability_id.clone(),
            dispatch_queue_lanes_capability_id: "app-shell-dashboard-dispatch-queue-lanes-json"
                .to_string(),
            artifact_capability_count: summary.artifact_capability_count,
        }
    }

    fn lane_from_item_ids(
        queue: &BerkeleyAppShellDashboardDispatchQueue,
        id: &str,
        title: &str,
        queue_state: &str,
        dispatch_queue_item_ids: Vec<String>,
    ) -> BerkeleyAppShellDashboardDispatchQueueLane {
        let lane_items = dispatch_queue_item_ids
            .iter()
            .filter_map(|item_id| {
                queue
                    .dispatch_queue_items
                    .iter()
                    .find(|item| item.id == item_id.as_str())
            })
            .collect::<Vec<_>>();
        let severity = if lane_items.iter().any(|item| item.severity == "error") {
            "error"
        } else if lane_items.iter().any(|item| item.severity == "warning") {
            "warning"
        } else {
            "ready"
        };

        BerkeleyAppShellDashboardDispatchQueueLane {
            id: id.to_string(),
            title: title.to_string(),
            queue_state: queue_state.to_string(),
            severity: severity.to_string(),
            dispatch_queue_item_count: dispatch_queue_item_ids.len(),
            dispatch_queue_item_ids,
            selected: lane_items.iter().any(|item| item.selected),
            default_dispatch: lane_items.iter().any(|item| item.default_dispatch),
            primary: lane_items.iter().any(|item| item.primary),
            attention: queue_state == "attention" && !lane_items.is_empty(),
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_queue_lanes_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLaneTab {
    pub id: String,
    pub lane_id: String,
    pub title: String,
    pub queue_state: String,
    pub severity: String,
    pub dispatch_queue_item_count: usize,
    pub selected: bool,
    pub default_dispatch: bool,
    pub active: bool,
    pub attention: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLaneTabs {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub active_lane_id: Option<String>,
    pub active_tab_id: Option<String>,
    pub attention_lane_id: Option<String>,
    pub attention_tab_id: Option<String>,
    pub lane_count: usize,
    pub tab_count: usize,
    pub enabled_tab_count: usize,
    pub disabled_tab_count: usize,
    pub tabs: Vec<BerkeleyAppShellDashboardDispatchQueueLaneTab>,
    pub dispatch_queue_capability_id: String,
    pub dispatch_queue_summary_capability_id: String,
    pub dispatch_queue_digest_capability_id: String,
    pub dispatch_queue_lanes_capability_id: String,
    pub dispatch_queue_lane_tabs_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchQueueLaneTabs {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_lanes(
            &BerkeleyAppShellDashboardDispatchQueueLanes::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_lanes(&BerkeleyAppShellDashboardDispatchQueueLanes::from_shell_handoff(handoff))
    }

    pub fn from_lanes(lanes: &BerkeleyAppShellDashboardDispatchQueueLanes) -> Self {
        let tabs = lanes
            .lanes
            .iter()
            .map(|lane| {
                let tab_id_suffix = lane
                    .id
                    .strip_prefix("dashboard.dispatch-queue-lane.")
                    .unwrap_or(lane.queue_state.as_str());
                BerkeleyAppShellDashboardDispatchQueueLaneTab {
                    id: format!("dashboard.dispatch-queue-lane-tab.{tab_id_suffix}"),
                    lane_id: lane.id.clone(),
                    title: lane.title.clone(),
                    queue_state: lane.queue_state.clone(),
                    severity: lane.severity.clone(),
                    dispatch_queue_item_count: lane.dispatch_queue_item_count,
                    selected: lane.selected,
                    default_dispatch: lane.default_dispatch,
                    active: lanes.active_lane_id.as_deref() == Some(lane.id.as_str()),
                    attention: lane.attention,
                    disabled: lane.dispatch_queue_item_count == 0,
                }
            })
            .collect::<Vec<_>>();
        let active_tab_id = tabs.iter().find(|tab| tab.active).map(|tab| tab.id.clone());
        let attention_tab_id = lanes
            .attention_lane_id
            .as_ref()
            .and_then(|attention_lane_id| {
                tabs.iter()
                    .find(|tab| tab.lane_id == attention_lane_id.as_str())
                    .map(|tab| tab.id.clone())
            });
        let enabled_tab_count = tabs.iter().filter(|tab| !tab.disabled).count();
        let disabled_tab_count = tabs.len().saturating_sub(enabled_tab_count);

        Self {
            schema_version: BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TABS_SCHEMA_VERSION,
            package_name: lanes.package_name.clone(),
            source_fingerprint: lanes.source_fingerprint.clone(),
            title: lanes.title.clone(),
            startup_route: lanes.startup_route.clone(),
            ready: lanes.ready,
            severity: lanes.severity.clone(),
            attention_required: lanes.attention_required,
            active_lane_id: lanes.active_lane_id.clone(),
            active_tab_id,
            attention_lane_id: lanes.attention_lane_id.clone(),
            attention_tab_id,
            lane_count: lanes.lane_count,
            tab_count: tabs.len(),
            enabled_tab_count,
            disabled_tab_count,
            tabs,
            dispatch_queue_capability_id: lanes.dispatch_queue_capability_id.clone(),
            dispatch_queue_summary_capability_id: lanes
                .dispatch_queue_summary_capability_id
                .clone(),
            dispatch_queue_digest_capability_id: lanes.dispatch_queue_digest_capability_id.clone(),
            dispatch_queue_lanes_capability_id: lanes.dispatch_queue_lanes_capability_id.clone(),
            dispatch_queue_lane_tabs_capability_id:
                "app-shell-dashboard-dispatch-queue-lane-tabs-json".to_string(),
            artifact_capability_count: lanes.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_queue_lane_tabs_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLaneTabPanel {
    pub id: String,
    pub tab_id: String,
    pub lane_id: String,
    pub title: String,
    pub queue_state: String,
    pub severity: String,
    pub dispatch_queue_item_count: usize,
    pub selected: bool,
    pub default_dispatch: bool,
    pub active: bool,
    pub attention: bool,
    pub disabled: bool,
    pub empty: bool,
    pub empty_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLaneTabPanels {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub active_lane_id: Option<String>,
    pub active_tab_id: Option<String>,
    pub active_panel_id: Option<String>,
    pub attention_lane_id: Option<String>,
    pub attention_tab_id: Option<String>,
    pub attention_panel_id: Option<String>,
    pub lane_count: usize,
    pub tab_count: usize,
    pub enabled_tab_count: usize,
    pub disabled_tab_count: usize,
    pub panel_count: usize,
    pub enabled_panel_count: usize,
    pub disabled_panel_count: usize,
    pub empty_panel_count: usize,
    pub panels: Vec<BerkeleyAppShellDashboardDispatchQueueLaneTabPanel>,
    pub dispatch_queue_capability_id: String,
    pub dispatch_queue_summary_capability_id: String,
    pub dispatch_queue_digest_capability_id: String,
    pub dispatch_queue_lanes_capability_id: String,
    pub dispatch_queue_lane_tabs_capability_id: String,
    pub dispatch_queue_lane_tab_panels_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchQueueLaneTabPanels {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_lane_tabs(
            &BerkeleyAppShellDashboardDispatchQueueLaneTabs::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_lane_tabs(
            &BerkeleyAppShellDashboardDispatchQueueLaneTabs::from_shell_handoff(handoff),
        )
    }

    pub fn from_lane_tabs(tabs: &BerkeleyAppShellDashboardDispatchQueueLaneTabs) -> Self {
        let panels = tabs
            .tabs
            .iter()
            .map(|tab| {
                let panel_id_suffix = tab
                    .id
                    .strip_prefix("dashboard.dispatch-queue-lane-tab.")
                    .unwrap_or(tab.queue_state.as_str());
                let empty = tab.dispatch_queue_item_count == 0;
                BerkeleyAppShellDashboardDispatchQueueLaneTabPanel {
                    id: format!("dashboard.dispatch-queue-lane-tab-panel.{panel_id_suffix}"),
                    tab_id: tab.id.clone(),
                    lane_id: tab.lane_id.clone(),
                    title: tab.title.clone(),
                    queue_state: tab.queue_state.clone(),
                    severity: tab.severity.clone(),
                    dispatch_queue_item_count: tab.dispatch_queue_item_count,
                    selected: tab.selected,
                    default_dispatch: tab.default_dispatch,
                    active: tab.active,
                    attention: tab.attention,
                    disabled: tab.disabled,
                    empty,
                    empty_message: empty.then(|| format!("No {} dispatches", tab.queue_state)),
                }
            })
            .collect::<Vec<_>>();
        let active_panel_id = panels
            .iter()
            .find(|panel| panel.active)
            .map(|panel| panel.id.clone());
        let attention_panel_id = tabs.attention_tab_id.as_ref().and_then(|attention_tab_id| {
            panels
                .iter()
                .find(|panel| panel.tab_id == attention_tab_id.as_str())
                .map(|panel| panel.id.clone())
        });
        let enabled_panel_count = panels.iter().filter(|panel| !panel.disabled).count();
        let disabled_panel_count = panels.len().saturating_sub(enabled_panel_count);
        let empty_panel_count = panels.iter().filter(|panel| panel.empty).count();

        Self {
            schema_version:
                BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANELS_SCHEMA_VERSION,
            package_name: tabs.package_name.clone(),
            source_fingerprint: tabs.source_fingerprint.clone(),
            title: tabs.title.clone(),
            startup_route: tabs.startup_route.clone(),
            ready: tabs.ready,
            severity: tabs.severity.clone(),
            attention_required: tabs.attention_required,
            active_lane_id: tabs.active_lane_id.clone(),
            active_tab_id: tabs.active_tab_id.clone(),
            active_panel_id,
            attention_lane_id: tabs.attention_lane_id.clone(),
            attention_tab_id: tabs.attention_tab_id.clone(),
            attention_panel_id,
            lane_count: tabs.lane_count,
            tab_count: tabs.tab_count,
            enabled_tab_count: tabs.enabled_tab_count,
            disabled_tab_count: tabs.disabled_tab_count,
            panel_count: panels.len(),
            enabled_panel_count,
            disabled_panel_count,
            empty_panel_count,
            panels,
            dispatch_queue_capability_id: tabs.dispatch_queue_capability_id.clone(),
            dispatch_queue_summary_capability_id: tabs.dispatch_queue_summary_capability_id.clone(),
            dispatch_queue_digest_capability_id: tabs.dispatch_queue_digest_capability_id.clone(),
            dispatch_queue_lanes_capability_id: tabs.dispatch_queue_lanes_capability_id.clone(),
            dispatch_queue_lane_tabs_capability_id: tabs
                .dispatch_queue_lane_tabs_capability_id
                .clone(),
            dispatch_queue_lane_tab_panels_capability_id:
                "app-shell-dashboard-dispatch-queue-lane-tab-panels-json".to_string(),
            artifact_capability_count: tabs.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_queue_lane_tab_panels_json_value(self).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCard {
    pub id: String,
    pub panel_id: String,
    pub tab_id: String,
    pub lane_id: String,
    pub title: String,
    pub queue_state: String,
    pub severity: String,
    pub summary: String,
    pub dispatch_queue_item_count: usize,
    pub badge_count: usize,
    pub selected: bool,
    pub default_dispatch: bool,
    pub active: bool,
    pub attention: bool,
    pub disabled: bool,
    pub empty: bool,
    pub empty_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards {
    pub schema_version: u32,
    pub package_name: String,
    pub source_fingerprint: String,
    pub title: Option<String>,
    pub startup_route: String,
    pub ready: bool,
    pub severity: String,
    pub attention_required: bool,
    pub active_lane_id: Option<String>,
    pub active_tab_id: Option<String>,
    pub active_panel_id: Option<String>,
    pub active_panel_card_id: Option<String>,
    pub attention_lane_id: Option<String>,
    pub attention_tab_id: Option<String>,
    pub attention_panel_id: Option<String>,
    pub attention_panel_card_id: Option<String>,
    pub lane_count: usize,
    pub tab_count: usize,
    pub enabled_tab_count: usize,
    pub disabled_tab_count: usize,
    pub panel_count: usize,
    pub enabled_panel_count: usize,
    pub disabled_panel_count: usize,
    pub empty_panel_count: usize,
    pub panel_card_count: usize,
    pub enabled_panel_card_count: usize,
    pub disabled_panel_card_count: usize,
    pub empty_panel_card_count: usize,
    pub panel_cards: Vec<BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCard>,
    pub dispatch_queue_capability_id: String,
    pub dispatch_queue_summary_capability_id: String,
    pub dispatch_queue_digest_capability_id: String,
    pub dispatch_queue_lanes_capability_id: String,
    pub dispatch_queue_lane_tabs_capability_id: String,
    pub dispatch_queue_lane_tab_panels_capability_id: String,
    pub dispatch_queue_lane_tab_panel_cards_capability_id: String,
    pub artifact_capability_count: usize,
}

impl BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards {
    pub fn from_bootstrap_snapshot(snapshot: &BerkeleyAppBootstrapSnapshot) -> Self {
        Self::from_lane_tab_panels(
            &BerkeleyAppShellDashboardDispatchQueueLaneTabPanels::from_bootstrap_snapshot(snapshot),
        )
    }

    pub fn from_shell_handoff(handoff: &BerkeleyAppShellHandoff) -> Self {
        Self::from_lane_tab_panels(
            &BerkeleyAppShellDashboardDispatchQueueLaneTabPanels::from_shell_handoff(handoff),
        )
    }

    pub fn from_lane_tab_panels(
        panels: &BerkeleyAppShellDashboardDispatchQueueLaneTabPanels,
    ) -> Self {
        let panel_cards = panels
            .panels
            .iter()
            .map(|panel| {
                let panel_card_id_suffix = panel
                    .id
                    .strip_prefix("dashboard.dispatch-queue-lane-tab-panel.")
                    .unwrap_or(panel.queue_state.as_str());
                let dispatch_noun = if panel.dispatch_queue_item_count == 1 {
                    "dispatch"
                } else {
                    "dispatches"
                };
                let summary = if panel.empty {
                    panel
                        .empty_message
                        .clone()
                        .unwrap_or_else(|| format!("No {} dispatches", panel.queue_state))
                } else {
                    format!(
                        "{} {} {}",
                        panel.dispatch_queue_item_count, panel.queue_state, dispatch_noun
                    )
                };

                BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCard {
                    id: format!(
                        "dashboard.dispatch-queue-lane-tab-panel-card.{panel_card_id_suffix}"
                    ),
                    panel_id: panel.id.clone(),
                    tab_id: panel.tab_id.clone(),
                    lane_id: panel.lane_id.clone(),
                    title: panel.title.clone(),
                    queue_state: panel.queue_state.clone(),
                    severity: panel.severity.clone(),
                    summary,
                    dispatch_queue_item_count: panel.dispatch_queue_item_count,
                    badge_count: panel.dispatch_queue_item_count,
                    selected: panel.selected,
                    default_dispatch: panel.default_dispatch,
                    active: panel.active,
                    attention: panel.attention,
                    disabled: panel.disabled,
                    empty: panel.empty,
                    empty_message: panel.empty_message.clone(),
                }
            })
            .collect::<Vec<_>>();
        let active_panel_card_id = panel_cards
            .iter()
            .find(|panel_card| panel_card.active)
            .map(|panel_card| panel_card.id.clone());
        let attention_panel_card_id =
            panels
                .attention_panel_id
                .as_ref()
                .and_then(|attention_panel_id| {
                    panel_cards
                        .iter()
                        .find(|panel_card| panel_card.panel_id == attention_panel_id.as_str())
                        .map(|panel_card| panel_card.id.clone())
                });
        let enabled_panel_card_count = panel_cards
            .iter()
            .filter(|panel_card| !panel_card.disabled)
            .count();
        let disabled_panel_card_count = panel_cards.len().saturating_sub(enabled_panel_card_count);
        let empty_panel_card_count = panel_cards
            .iter()
            .filter(|panel_card| panel_card.empty)
            .count();

        Self {
            schema_version:
                BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARDS_SCHEMA_VERSION,
            package_name: panels.package_name.clone(),
            source_fingerprint: panels.source_fingerprint.clone(),
            title: panels.title.clone(),
            startup_route: panels.startup_route.clone(),
            ready: panels.ready,
            severity: panels.severity.clone(),
            attention_required: panels.attention_required,
            active_lane_id: panels.active_lane_id.clone(),
            active_tab_id: panels.active_tab_id.clone(),
            active_panel_id: panels.active_panel_id.clone(),
            active_panel_card_id,
            attention_lane_id: panels.attention_lane_id.clone(),
            attention_tab_id: panels.attention_tab_id.clone(),
            attention_panel_id: panels.attention_panel_id.clone(),
            attention_panel_card_id,
            lane_count: panels.lane_count,
            tab_count: panels.tab_count,
            enabled_tab_count: panels.enabled_tab_count,
            disabled_tab_count: panels.disabled_tab_count,
            panel_count: panels.panel_count,
            enabled_panel_count: panels.enabled_panel_count,
            disabled_panel_count: panels.disabled_panel_count,
            empty_panel_count: panels.empty_panel_count,
            panel_card_count: panel_cards.len(),
            enabled_panel_card_count,
            disabled_panel_card_count,
            empty_panel_card_count,
            panel_cards,
            dispatch_queue_capability_id: panels.dispatch_queue_capability_id.clone(),
            dispatch_queue_summary_capability_id: panels
                .dispatch_queue_summary_capability_id
                .clone(),
            dispatch_queue_digest_capability_id: panels.dispatch_queue_digest_capability_id.clone(),
            dispatch_queue_lanes_capability_id: panels.dispatch_queue_lanes_capability_id.clone(),
            dispatch_queue_lane_tabs_capability_id: panels
                .dispatch_queue_lane_tabs_capability_id
                .clone(),
            dispatch_queue_lane_tab_panels_capability_id: panels
                .dispatch_queue_lane_tab_panels_capability_id
                .clone(),
            dispatch_queue_lane_tab_panel_cards_capability_id:
                "app-shell-dashboard-dispatch-queue-lane-tab-panel-cards-json".to_string(),
            artifact_capability_count: panels.artifact_capability_count,
        }
    }

    pub fn to_json(&self) -> String {
        app_shell_dashboard_dispatch_queue_lane_tab_panel_cards_json_value(self).to_string()
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

    pub fn app_shell_dashboard_routes(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardRoutes {
        BerkeleyAppShellDashboardRoutes::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_routes(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardRoutes, AnalysisExecutionError> {
        Ok(BerkeleyAppShellDashboardRoutes::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_dashboard_routes_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_routes(persisted_state).to_json()
    }

    pub fn run_app_shell_dashboard_routes_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_routes(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_breadcrumbs(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardBreadcrumbs {
        BerkeleyAppShellDashboardBreadcrumbs::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_breadcrumbs(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardBreadcrumbs, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardBreadcrumbs::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_breadcrumbs_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_breadcrumbs(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_breadcrumbs_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_breadcrumbs(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_tabs(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardTabs {
        BerkeleyAppShellDashboardTabs::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_tabs(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardTabs, AnalysisExecutionError> {
        Ok(BerkeleyAppShellDashboardTabs::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_dashboard_tabs_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_tabs(persisted_state).to_json()
    }

    pub fn run_app_shell_dashboard_tabs_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_tabs(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_tab_panels(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardTabPanels {
        BerkeleyAppShellDashboardTabPanels::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_tab_panels(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardTabPanels, AnalysisExecutionError> {
        Ok(BerkeleyAppShellDashboardTabPanels::from_bootstrap_snapshot(
            &self.run_app_bootstrap_snapshot(persisted_state)?,
        ))
    }

    pub fn app_shell_dashboard_tab_panels_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_tab_panels(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_tab_panels_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_tab_panels(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_panel_cards(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardPanelCards {
        BerkeleyAppShellDashboardPanelCards::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_panel_cards(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardPanelCards, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardPanelCards::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_panel_cards_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_panel_cards(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_panel_cards_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_panel_cards(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_panel_card_actions(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardPanelCardActions {
        BerkeleyAppShellDashboardPanelCardActions::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_panel_card_actions(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardPanelCardActions, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardPanelCardActions::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_panel_card_actions_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_panel_card_actions(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_panel_card_actions_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_panel_card_actions(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_action_dispatch(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardActionDispatch {
        BerkeleyAppShellDashboardActionDispatch::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_action_dispatch(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardActionDispatch, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardActionDispatch::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_action_dispatch_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_action_dispatch(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_action_dispatch_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_action_dispatch(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_events(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchEvents {
        BerkeleyAppShellDashboardDispatchEvents::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_events(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchEvents, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardDispatchEvents::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_events_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_events(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_events_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_events(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_queue(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchQueue {
        BerkeleyAppShellDashboardDispatchQueue::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_queue(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchQueue, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardDispatchQueue::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_queue_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_queue(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_queue_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_queue(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_queue_summary(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchQueueSummary {
        BerkeleyAppShellDashboardDispatchQueueSummary::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_queue_summary(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchQueueSummary, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardDispatchQueueSummary::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_queue_summary_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_queue_summary(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_queue_summary_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_queue_summary(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_queue_digest(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchQueueDigest {
        BerkeleyAppShellDashboardDispatchQueueDigest::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_queue_digest(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchQueueDigest, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardDispatchQueueDigest::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_queue_digest_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_queue_digest(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_queue_digest_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_queue_digest(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_queue_lanes(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchQueueLanes {
        BerkeleyAppShellDashboardDispatchQueueLanes::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lanes(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchQueueLanes, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardDispatchQueueLanes::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_queue_lanes_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_queue_lanes(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lanes_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_queue_lanes(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_queue_lane_tabs(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchQueueLaneTabs {
        BerkeleyAppShellDashboardDispatchQueueLaneTabs::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lane_tabs(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchQueueLaneTabs, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardDispatchQueueLaneTabs::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_queue_lane_tabs_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_queue_lane_tabs(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lane_tabs_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_queue_lane_tabs(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_queue_lane_tab_panels(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchQueueLaneTabPanels {
        BerkeleyAppShellDashboardDispatchQueueLaneTabPanels::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lane_tab_panels(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchQueueLaneTabPanels, AnalysisExecutionError> {
        Ok(
            BerkeleyAppShellDashboardDispatchQueueLaneTabPanels::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_queue_lane_tab_panels_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_queue_lane_tab_panels(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lane_tab_panels_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_queue_lane_tab_panels(persisted_state)?
            .to_json())
    }

    pub fn app_shell_dashboard_dispatch_queue_lane_tab_panel_cards(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards {
        BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards::from_bootstrap_snapshot(
            &self.app_bootstrap_snapshot(persisted_state),
        )
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lane_tab_panel_cards(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards, AnalysisExecutionError>
    {
        Ok(
            BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards::from_bootstrap_snapshot(
                &self.run_app_bootstrap_snapshot(persisted_state)?,
            ),
        )
    }

    pub fn app_shell_dashboard_dispatch_queue_lane_tab_panel_cards_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> String {
        self.app_shell_dashboard_dispatch_queue_lane_tab_panel_cards(persisted_state)
            .to_json()
    }

    pub fn run_app_shell_dashboard_dispatch_queue_lane_tab_panel_cards_json(
        &self,
        persisted_state: BerkeleyAppPersistedEditorState,
    ) -> Result<String, AnalysisExecutionError> {
        Ok(self
            .run_app_shell_dashboard_dispatch_queue_lane_tab_panel_cards(persisted_state)?
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

fn dashboard_panel_card_launch_action<'a>(
    panel_card: &BerkeleyAppShellDashboardPanelCard,
    launch_plan: &'a BerkeleyAppLaunchPlan,
) -> Option<&'a BerkeleyAppLaunchAction> {
    let preferred_panel_id =
        dashboard_panel_card_preferred_action_panel_id(&panel_card.role, launch_plan);
    let preferred_action = launch_plan
        .actions
        .iter()
        .find(|action| action.panel_id == preferred_panel_id);

    if let Some(action) = preferred_action {
        if action.enabled
            || panel_card.selected
            || panel_card.default_panel
            || panel_card.role == "attention"
        {
            return Some(action);
        }
    }

    launch_plan
        .actions
        .iter()
        .find(|action| action.primary && action.enabled)
        .or_else(|| launch_plan.actions.iter().find(|action| action.enabled))
        .or(preferred_action)
        .or_else(|| launch_plan.actions.first())
}

fn dashboard_panel_card_preferred_action_panel_id<'a>(
    role: &str,
    launch_plan: &'a BerkeleyAppLaunchPlan,
) -> &'a str {
    match role {
        "status" if launch_plan.ready => "analysis",
        "status" => "source",
        "attention" => "diagnostics",
        "metrics" => "waveform",
        _ => launch_plan.entry_panel_id.as_deref().unwrap_or("source"),
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

fn app_shell_dashboard_routes_json_value(
    routes: &BerkeleyAppShellDashboardRoutes,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": routes.schema_version,
        "packageName": &routes.package_name,
        "sourceFingerprint": &routes.source_fingerprint,
        "title": &routes.title,
        "startupRoute": &routes.startup_route,
        "ready": routes.ready,
        "severity": &routes.severity,
        "attentionRequired": routes.attention_required,
        "primaryCardId": &routes.primary_card_id,
        "primaryRegionId": &routes.primary_region_id,
        "activeItemId": &routes.active_item_id,
        "activeRouteId": &routes.active_route_id,
        "activeRoutePath": &routes.active_route_path,
        "defaultRouteId": &routes.default_route_id,
        "defaultRoutePath": &routes.default_route_path,
        "routeCount": routes.route_count,
        "visibleRouteCount": routes.visible_route_count,
        "enabledRouteCount": routes.enabled_route_count,
        "itemCount": routes.item_count,
        "visibleItemCount": routes.visible_item_count,
        "enabledItemCount": routes.enabled_item_count,
        "regionCount": routes.region_count,
        "visibleRegionCount": routes.visible_region_count,
        "cardCount": routes.card_count,
        "visibleCardCount": routes.visible_card_count,
        "attentionCardCount": routes.attention_card_count,
        "metricCardCount": routes.metric_card_count,
        "routes": routes
            .routes
            .iter()
            .map(app_shell_dashboard_route_json_value)
            .collect::<Vec<_>>(),
        "packageCapabilityId": &routes.package_capability_id,
        "dashboardCapabilityId": &routes.dashboard_capability_id,
        "cardsCapabilityId": &routes.cards_capability_id,
        "viewCapabilityId": &routes.view_capability_id,
        "layoutCapabilityId": &routes.layout_capability_id,
        "navigationCapabilityId": &routes.navigation_capability_id,
        "routesCapabilityId": &routes.routes_capability_id,
        "artifactCapabilityCount": routes.artifact_capability_count,
    })
}

fn app_shell_dashboard_route_json_value(
    route: &BerkeleyAppShellDashboardRoute,
) -> serde_json::Value {
    serde_json::json!({
        "id": &route.id,
        "itemId": &route.item_id,
        "regionId": &route.region_id,
        "role": &route.role,
        "label": &route.label,
        "path": &route.path,
        "cardIds": &route.card_ids,
        "active": route.active,
        "default": route.default_route,
        "visible": route.visible,
        "enabled": route.enabled,
        "badgeCount": route.badge_count,
    })
}

fn app_shell_dashboard_breadcrumbs_json_value(
    breadcrumbs: &BerkeleyAppShellDashboardBreadcrumbs,
) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    macro_rules! insert_json {
        ($key:literal, $field:expr) => {
            value.insert($key.to_string(), serde_json::json!($field));
        };
    }

    insert_json!("schemaVersion", breadcrumbs.schema_version);
    insert_json!("packageName", &breadcrumbs.package_name);
    insert_json!("sourceFingerprint", &breadcrumbs.source_fingerprint);
    insert_json!("title", &breadcrumbs.title);
    insert_json!("startupRoute", &breadcrumbs.startup_route);
    insert_json!("ready", breadcrumbs.ready);
    insert_json!("severity", &breadcrumbs.severity);
    insert_json!("attentionRequired", breadcrumbs.attention_required);
    insert_json!("primaryCardId", &breadcrumbs.primary_card_id);
    insert_json!("primaryRegionId", &breadcrumbs.primary_region_id);
    insert_json!("activeItemId", &breadcrumbs.active_item_id);
    insert_json!("activeRouteId", &breadcrumbs.active_route_id);
    insert_json!("activeRoutePath", &breadcrumbs.active_route_path);
    insert_json!("defaultRouteId", &breadcrumbs.default_route_id);
    insert_json!("defaultRoutePath", &breadcrumbs.default_route_path);
    insert_json!("activeBreadcrumbId", &breadcrumbs.active_breadcrumb_id);
    insert_json!("activeBreadcrumbPath", &breadcrumbs.active_breadcrumb_path);
    insert_json!("defaultBreadcrumbId", &breadcrumbs.default_breadcrumb_id);
    insert_json!(
        "defaultBreadcrumbPath",
        &breadcrumbs.default_breadcrumb_path
    );
    insert_json!("routeCount", breadcrumbs.route_count);
    insert_json!("visibleRouteCount", breadcrumbs.visible_route_count);
    insert_json!("enabledRouteCount", breadcrumbs.enabled_route_count);
    insert_json!("breadcrumbCount", breadcrumbs.breadcrumb_count);
    insert_json!(
        "visibleBreadcrumbCount",
        breadcrumbs.visible_breadcrumb_count
    );
    insert_json!(
        "enabledBreadcrumbCount",
        breadcrumbs.enabled_breadcrumb_count
    );
    insert_json!("itemCount", breadcrumbs.item_count);
    insert_json!("visibleItemCount", breadcrumbs.visible_item_count);
    insert_json!("enabledItemCount", breadcrumbs.enabled_item_count);
    insert_json!("regionCount", breadcrumbs.region_count);
    insert_json!("visibleRegionCount", breadcrumbs.visible_region_count);
    insert_json!("cardCount", breadcrumbs.card_count);
    insert_json!("visibleCardCount", breadcrumbs.visible_card_count);
    insert_json!("attentionCardCount", breadcrumbs.attention_card_count);
    insert_json!("metricCardCount", breadcrumbs.metric_card_count);
    value.insert(
        "breadcrumbs".to_string(),
        serde_json::Value::Array(
            breadcrumbs
                .breadcrumbs
                .iter()
                .map(app_shell_dashboard_breadcrumb_json_value)
                .collect(),
        ),
    );
    insert_json!("packageCapabilityId", &breadcrumbs.package_capability_id);
    insert_json!(
        "dashboardCapabilityId",
        &breadcrumbs.dashboard_capability_id
    );
    insert_json!("cardsCapabilityId", &breadcrumbs.cards_capability_id);
    insert_json!("viewCapabilityId", &breadcrumbs.view_capability_id);
    insert_json!("layoutCapabilityId", &breadcrumbs.layout_capability_id);
    insert_json!(
        "navigationCapabilityId",
        &breadcrumbs.navigation_capability_id
    );
    insert_json!("routesCapabilityId", &breadcrumbs.routes_capability_id);
    insert_json!(
        "breadcrumbsCapabilityId",
        &breadcrumbs.breadcrumbs_capability_id
    );
    insert_json!(
        "artifactCapabilityCount",
        breadcrumbs.artifact_capability_count
    );

    serde_json::Value::Object(value)
}

fn app_shell_dashboard_breadcrumb_json_value(
    breadcrumb: &BerkeleyAppShellDashboardBreadcrumb,
) -> serde_json::Value {
    serde_json::json!({
        "id": &breadcrumb.id,
        "routeId": &breadcrumb.route_id,
        "itemId": &breadcrumb.item_id,
        "regionId": &breadcrumb.region_id,
        "role": &breadcrumb.role,
        "label": &breadcrumb.label,
        "path": &breadcrumb.path,
        "position": breadcrumb.position,
        "active": breadcrumb.active,
        "default": breadcrumb.default_route,
        "visible": breadcrumb.visible,
        "enabled": breadcrumb.enabled,
        "badgeCount": breadcrumb.badge_count,
    })
}

fn app_shell_dashboard_tabs_json_value(tabs: &BerkeleyAppShellDashboardTabs) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    macro_rules! insert_json {
        ($key:literal, $field:expr) => {
            value.insert($key.to_string(), serde_json::json!($field));
        };
    }

    insert_json!("schemaVersion", tabs.schema_version);
    insert_json!("packageName", &tabs.package_name);
    insert_json!("sourceFingerprint", &tabs.source_fingerprint);
    insert_json!("title", &tabs.title);
    insert_json!("startupRoute", &tabs.startup_route);
    insert_json!("ready", tabs.ready);
    insert_json!("severity", &tabs.severity);
    insert_json!("attentionRequired", tabs.attention_required);
    insert_json!("primaryCardId", &tabs.primary_card_id);
    insert_json!("primaryRegionId", &tabs.primary_region_id);
    insert_json!("activeItemId", &tabs.active_item_id);
    insert_json!("activeRouteId", &tabs.active_route_id);
    insert_json!("activeRoutePath", &tabs.active_route_path);
    insert_json!("defaultRouteId", &tabs.default_route_id);
    insert_json!("defaultRoutePath", &tabs.default_route_path);
    insert_json!("activeBreadcrumbId", &tabs.active_breadcrumb_id);
    insert_json!("activeBreadcrumbPath", &tabs.active_breadcrumb_path);
    insert_json!("defaultBreadcrumbId", &tabs.default_breadcrumb_id);
    insert_json!("defaultBreadcrumbPath", &tabs.default_breadcrumb_path);
    insert_json!("selectedTabId", &tabs.selected_tab_id);
    insert_json!("selectedTabPath", &tabs.selected_tab_path);
    insert_json!("defaultTabId", &tabs.default_tab_id);
    insert_json!("defaultTabPath", &tabs.default_tab_path);
    insert_json!("routeCount", tabs.route_count);
    insert_json!("visibleRouteCount", tabs.visible_route_count);
    insert_json!("enabledRouteCount", tabs.enabled_route_count);
    insert_json!("breadcrumbCount", tabs.breadcrumb_count);
    insert_json!("visibleBreadcrumbCount", tabs.visible_breadcrumb_count);
    insert_json!("enabledBreadcrumbCount", tabs.enabled_breadcrumb_count);
    insert_json!("tabCount", tabs.tab_count);
    insert_json!("visibleTabCount", tabs.visible_tab_count);
    insert_json!("enabledTabCount", tabs.enabled_tab_count);
    insert_json!("itemCount", tabs.item_count);
    insert_json!("visibleItemCount", tabs.visible_item_count);
    insert_json!("enabledItemCount", tabs.enabled_item_count);
    insert_json!("regionCount", tabs.region_count);
    insert_json!("visibleRegionCount", tabs.visible_region_count);
    insert_json!("cardCount", tabs.card_count);
    insert_json!("visibleCardCount", tabs.visible_card_count);
    insert_json!("attentionCardCount", tabs.attention_card_count);
    insert_json!("metricCardCount", tabs.metric_card_count);
    value.insert(
        "tabs".to_string(),
        serde_json::Value::Array(
            tabs.tabs
                .iter()
                .map(app_shell_dashboard_tab_json_value)
                .collect(),
        ),
    );
    insert_json!("packageCapabilityId", &tabs.package_capability_id);
    insert_json!("dashboardCapabilityId", &tabs.dashboard_capability_id);
    insert_json!("cardsCapabilityId", &tabs.cards_capability_id);
    insert_json!("viewCapabilityId", &tabs.view_capability_id);
    insert_json!("layoutCapabilityId", &tabs.layout_capability_id);
    insert_json!("navigationCapabilityId", &tabs.navigation_capability_id);
    insert_json!("routesCapabilityId", &tabs.routes_capability_id);
    insert_json!("breadcrumbsCapabilityId", &tabs.breadcrumbs_capability_id);
    insert_json!("tabsCapabilityId", &tabs.tabs_capability_id);
    insert_json!("artifactCapabilityCount", tabs.artifact_capability_count);

    serde_json::Value::Object(value)
}

fn app_shell_dashboard_tab_json_value(tab: &BerkeleyAppShellDashboardTab) -> serde_json::Value {
    serde_json::json!({
        "id": &tab.id,
        "breadcrumbId": &tab.breadcrumb_id,
        "routeId": &tab.route_id,
        "itemId": &tab.item_id,
        "regionId": &tab.region_id,
        "role": &tab.role,
        "label": &tab.label,
        "path": &tab.path,
        "position": tab.position,
        "selected": tab.selected,
        "default": tab.default_tab,
        "visible": tab.visible,
        "enabled": tab.enabled,
        "badgeCount": tab.badge_count,
    })
}

fn app_shell_dashboard_tab_panels_json_value(
    panels: &BerkeleyAppShellDashboardTabPanels,
) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    macro_rules! insert_json {
        ($key:literal, $field:expr) => {
            value.insert($key.to_string(), serde_json::json!($field));
        };
    }

    insert_json!("schemaVersion", panels.schema_version);
    insert_json!("packageName", &panels.package_name);
    insert_json!("sourceFingerprint", &panels.source_fingerprint);
    insert_json!("title", &panels.title);
    insert_json!("startupRoute", &panels.startup_route);
    insert_json!("ready", panels.ready);
    insert_json!("severity", &panels.severity);
    insert_json!("attentionRequired", panels.attention_required);
    insert_json!("primaryCardId", &panels.primary_card_id);
    insert_json!("primaryRegionId", &panels.primary_region_id);
    insert_json!("activeItemId", &panels.active_item_id);
    insert_json!("activeRouteId", &panels.active_route_id);
    insert_json!("activeRoutePath", &panels.active_route_path);
    insert_json!("defaultRouteId", &panels.default_route_id);
    insert_json!("defaultRoutePath", &panels.default_route_path);
    insert_json!("activeBreadcrumbId", &panels.active_breadcrumb_id);
    insert_json!("activeBreadcrumbPath", &panels.active_breadcrumb_path);
    insert_json!("defaultBreadcrumbId", &panels.default_breadcrumb_id);
    insert_json!("defaultBreadcrumbPath", &panels.default_breadcrumb_path);
    insert_json!("selectedTabId", &panels.selected_tab_id);
    insert_json!("selectedTabPath", &panels.selected_tab_path);
    insert_json!("defaultTabId", &panels.default_tab_id);
    insert_json!("defaultTabPath", &panels.default_tab_path);
    insert_json!("selectedPanelId", &panels.selected_panel_id);
    insert_json!("selectedPanelPath", &panels.selected_panel_path);
    insert_json!("defaultPanelId", &panels.default_panel_id);
    insert_json!("defaultPanelPath", &panels.default_panel_path);
    insert_json!("routeCount", panels.route_count);
    insert_json!("visibleRouteCount", panels.visible_route_count);
    insert_json!("enabledRouteCount", panels.enabled_route_count);
    insert_json!("breadcrumbCount", panels.breadcrumb_count);
    insert_json!("visibleBreadcrumbCount", panels.visible_breadcrumb_count);
    insert_json!("enabledBreadcrumbCount", panels.enabled_breadcrumb_count);
    insert_json!("tabCount", panels.tab_count);
    insert_json!("visibleTabCount", panels.visible_tab_count);
    insert_json!("enabledTabCount", panels.enabled_tab_count);
    insert_json!("panelCount", panels.panel_count);
    insert_json!("visiblePanelCount", panels.visible_panel_count);
    insert_json!("enabledPanelCount", panels.enabled_panel_count);
    insert_json!("itemCount", panels.item_count);
    insert_json!("visibleItemCount", panels.visible_item_count);
    insert_json!("enabledItemCount", panels.enabled_item_count);
    insert_json!("regionCount", panels.region_count);
    insert_json!("visibleRegionCount", panels.visible_region_count);
    insert_json!("cardCount", panels.card_count);
    insert_json!("visibleCardCount", panels.visible_card_count);
    insert_json!("attentionCardCount", panels.attention_card_count);
    insert_json!("metricCardCount", panels.metric_card_count);
    value.insert(
        "panels".to_string(),
        serde_json::Value::Array(
            panels
                .panels
                .iter()
                .map(app_shell_dashboard_tab_panel_json_value)
                .collect(),
        ),
    );
    insert_json!("packageCapabilityId", &panels.package_capability_id);
    insert_json!("dashboardCapabilityId", &panels.dashboard_capability_id);
    insert_json!("cardsCapabilityId", &panels.cards_capability_id);
    insert_json!("viewCapabilityId", &panels.view_capability_id);
    insert_json!("layoutCapabilityId", &panels.layout_capability_id);
    insert_json!("navigationCapabilityId", &panels.navigation_capability_id);
    insert_json!("routesCapabilityId", &panels.routes_capability_id);
    insert_json!("breadcrumbsCapabilityId", &panels.breadcrumbs_capability_id);
    insert_json!("tabsCapabilityId", &panels.tabs_capability_id);
    insert_json!("tabPanelsCapabilityId", &panels.tab_panels_capability_id);
    insert_json!("artifactCapabilityCount", panels.artifact_capability_count);

    serde_json::Value::Object(value)
}

fn app_shell_dashboard_tab_panel_json_value(
    panel: &BerkeleyAppShellDashboardTabPanel,
) -> serde_json::Value {
    serde_json::json!({
        "id": &panel.id,
        "tabId": &panel.tab_id,
        "breadcrumbId": &panel.breadcrumb_id,
        "routeId": &panel.route_id,
        "itemId": &panel.item_id,
        "regionId": &panel.region_id,
        "role": &panel.role,
        "title": &panel.title,
        "path": &panel.path,
        "position": panel.position,
        "selected": panel.selected,
        "default": panel.default_panel,
        "visible": panel.visible,
        "enabled": panel.enabled,
        "badgeCount": panel.badge_count,
    })
}

fn app_shell_dashboard_panel_cards_json_value(
    panel_cards: &BerkeleyAppShellDashboardPanelCards,
) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    macro_rules! insert_json {
        ($key:literal, $field:expr) => {
            value.insert($key.to_string(), serde_json::json!($field));
        };
    }

    insert_json!("schemaVersion", panel_cards.schema_version);
    insert_json!("packageName", &panel_cards.package_name);
    insert_json!("sourceFingerprint", &panel_cards.source_fingerprint);
    insert_json!("title", &panel_cards.title);
    insert_json!("startupRoute", &panel_cards.startup_route);
    insert_json!("ready", panel_cards.ready);
    insert_json!("severity", &panel_cards.severity);
    insert_json!("attentionRequired", panel_cards.attention_required);
    insert_json!("primaryCardId", &panel_cards.primary_card_id);
    insert_json!("primaryRegionId", &panel_cards.primary_region_id);
    insert_json!("activeItemId", &panel_cards.active_item_id);
    insert_json!("activeRouteId", &panel_cards.active_route_id);
    insert_json!("activeRoutePath", &panel_cards.active_route_path);
    insert_json!("defaultRouteId", &panel_cards.default_route_id);
    insert_json!("defaultRoutePath", &panel_cards.default_route_path);
    insert_json!("activeBreadcrumbId", &panel_cards.active_breadcrumb_id);
    insert_json!("activeBreadcrumbPath", &panel_cards.active_breadcrumb_path);
    insert_json!("defaultBreadcrumbId", &panel_cards.default_breadcrumb_id);
    insert_json!(
        "defaultBreadcrumbPath",
        &panel_cards.default_breadcrumb_path
    );
    insert_json!("selectedTabId", &panel_cards.selected_tab_id);
    insert_json!("selectedTabPath", &panel_cards.selected_tab_path);
    insert_json!("defaultTabId", &panel_cards.default_tab_id);
    insert_json!("defaultTabPath", &panel_cards.default_tab_path);
    insert_json!("selectedPanelId", &panel_cards.selected_panel_id);
    insert_json!("selectedPanelPath", &panel_cards.selected_panel_path);
    insert_json!("defaultPanelId", &panel_cards.default_panel_id);
    insert_json!("defaultPanelPath", &panel_cards.default_panel_path);
    insert_json!("selectedPanelCardId", &panel_cards.selected_panel_card_id);
    insert_json!("selectedCardId", &panel_cards.selected_card_id);
    insert_json!("defaultPanelCardId", &panel_cards.default_panel_card_id);
    insert_json!("defaultCardId", &panel_cards.default_card_id);
    insert_json!("routeCount", panel_cards.route_count);
    insert_json!("visibleRouteCount", panel_cards.visible_route_count);
    insert_json!("enabledRouteCount", panel_cards.enabled_route_count);
    insert_json!("breadcrumbCount", panel_cards.breadcrumb_count);
    insert_json!(
        "visibleBreadcrumbCount",
        panel_cards.visible_breadcrumb_count
    );
    insert_json!(
        "enabledBreadcrumbCount",
        panel_cards.enabled_breadcrumb_count
    );
    insert_json!("tabCount", panel_cards.tab_count);
    insert_json!("visibleTabCount", panel_cards.visible_tab_count);
    insert_json!("enabledTabCount", panel_cards.enabled_tab_count);
    insert_json!("panelCount", panel_cards.panel_count);
    insert_json!("visiblePanelCount", panel_cards.visible_panel_count);
    insert_json!("enabledPanelCount", panel_cards.enabled_panel_count);
    insert_json!("panelCardCount", panel_cards.panel_card_count);
    insert_json!(
        "visiblePanelCardCount",
        panel_cards.visible_panel_card_count
    );
    insert_json!(
        "enabledPanelCardCount",
        panel_cards.enabled_panel_card_count
    );
    insert_json!("itemCount", panel_cards.item_count);
    insert_json!("visibleItemCount", panel_cards.visible_item_count);
    insert_json!("enabledItemCount", panel_cards.enabled_item_count);
    insert_json!("regionCount", panel_cards.region_count);
    insert_json!("visibleRegionCount", panel_cards.visible_region_count);
    insert_json!("cardCount", panel_cards.card_count);
    insert_json!("visibleCardCount", panel_cards.visible_card_count);
    insert_json!("attentionCardCount", panel_cards.attention_card_count);
    insert_json!("metricCardCount", panel_cards.metric_card_count);
    value.insert(
        "panelCards".to_string(),
        serde_json::Value::Array(
            panel_cards
                .panel_cards
                .iter()
                .map(app_shell_dashboard_panel_card_json_value)
                .collect(),
        ),
    );
    insert_json!("packageCapabilityId", &panel_cards.package_capability_id);
    insert_json!(
        "dashboardCapabilityId",
        &panel_cards.dashboard_capability_id
    );
    insert_json!("cardsCapabilityId", &panel_cards.cards_capability_id);
    insert_json!("viewCapabilityId", &panel_cards.view_capability_id);
    insert_json!("layoutCapabilityId", &panel_cards.layout_capability_id);
    insert_json!(
        "navigationCapabilityId",
        &panel_cards.navigation_capability_id
    );
    insert_json!("routesCapabilityId", &panel_cards.routes_capability_id);
    insert_json!(
        "breadcrumbsCapabilityId",
        &panel_cards.breadcrumbs_capability_id
    );
    insert_json!("tabsCapabilityId", &panel_cards.tabs_capability_id);
    insert_json!(
        "tabPanelsCapabilityId",
        &panel_cards.tab_panels_capability_id
    );
    insert_json!(
        "panelCardsCapabilityId",
        &panel_cards.panel_cards_capability_id
    );
    insert_json!(
        "artifactCapabilityCount",
        panel_cards.artifact_capability_count
    );

    serde_json::Value::Object(value)
}

fn app_shell_dashboard_panel_card_json_value(
    panel_card: &BerkeleyAppShellDashboardPanelCard,
) -> serde_json::Value {
    serde_json::json!({
        "id": &panel_card.id,
        "panelId": &panel_card.panel_id,
        "tabId": &panel_card.tab_id,
        "breadcrumbId": &panel_card.breadcrumb_id,
        "routeId": &panel_card.route_id,
        "itemId": &panel_card.item_id,
        "regionId": &panel_card.region_id,
        "cardId": &panel_card.card_id,
        "sectionId": &panel_card.section_id,
        "role": &panel_card.role,
        "title": &panel_card.title,
        "severity": &panel_card.severity,
        "path": &panel_card.path,
        "position": panel_card.position,
        "selected": panel_card.selected,
        "default": panel_card.default_panel,
        "visible": panel_card.visible,
        "enabled": panel_card.enabled,
        "primary": panel_card.primary,
        "attention": panel_card.attention,
        "eventCount": panel_card.event_count,
        "eventIds": &panel_card.event_ids,
        "badgeCount": panel_card.badge_count,
    })
}

fn app_shell_dashboard_panel_card_actions_json_value(
    panel_card_actions: &BerkeleyAppShellDashboardPanelCardActions,
) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    macro_rules! insert_json {
        ($key:literal, $field:expr) => {
            value.insert($key.to_string(), serde_json::json!($field));
        };
    }

    insert_json!("schemaVersion", panel_card_actions.schema_version);
    insert_json!("packageName", &panel_card_actions.package_name);
    insert_json!("sourceFingerprint", &panel_card_actions.source_fingerprint);
    insert_json!("title", &panel_card_actions.title);
    insert_json!("startupRoute", &panel_card_actions.startup_route);
    insert_json!("ready", panel_card_actions.ready);
    insert_json!("severity", &panel_card_actions.severity);
    insert_json!("attentionRequired", panel_card_actions.attention_required);
    insert_json!("primaryCardId", &panel_card_actions.primary_card_id);
    insert_json!("primaryRegionId", &panel_card_actions.primary_region_id);
    insert_json!("activeItemId", &panel_card_actions.active_item_id);
    insert_json!("activeRouteId", &panel_card_actions.active_route_id);
    insert_json!("activeRoutePath", &panel_card_actions.active_route_path);
    insert_json!("defaultRouteId", &panel_card_actions.default_route_id);
    insert_json!("defaultRoutePath", &panel_card_actions.default_route_path);
    insert_json!(
        "activeBreadcrumbId",
        &panel_card_actions.active_breadcrumb_id
    );
    insert_json!(
        "activeBreadcrumbPath",
        &panel_card_actions.active_breadcrumb_path
    );
    insert_json!(
        "defaultBreadcrumbId",
        &panel_card_actions.default_breadcrumb_id
    );
    insert_json!(
        "defaultBreadcrumbPath",
        &panel_card_actions.default_breadcrumb_path
    );
    insert_json!("selectedTabId", &panel_card_actions.selected_tab_id);
    insert_json!("selectedTabPath", &panel_card_actions.selected_tab_path);
    insert_json!("defaultTabId", &panel_card_actions.default_tab_id);
    insert_json!("defaultTabPath", &panel_card_actions.default_tab_path);
    insert_json!("selectedPanelId", &panel_card_actions.selected_panel_id);
    insert_json!("selectedPanelPath", &panel_card_actions.selected_panel_path);
    insert_json!("defaultPanelId", &panel_card_actions.default_panel_id);
    insert_json!("defaultPanelPath", &panel_card_actions.default_panel_path);
    insert_json!(
        "selectedPanelCardId",
        &panel_card_actions.selected_panel_card_id
    );
    insert_json!("selectedCardId", &panel_card_actions.selected_card_id);
    insert_json!(
        "defaultPanelCardId",
        &panel_card_actions.default_panel_card_id
    );
    insert_json!("defaultCardId", &panel_card_actions.default_card_id);
    insert_json!(
        "selectedPanelCardActionId",
        &panel_card_actions.selected_panel_card_action_id
    );
    insert_json!("selectedActionId", &panel_card_actions.selected_action_id);
    insert_json!(
        "defaultPanelCardActionId",
        &panel_card_actions.default_panel_card_action_id
    );
    insert_json!("defaultActionId", &panel_card_actions.default_action_id);
    insert_json!("routeCount", panel_card_actions.route_count);
    insert_json!("visibleRouteCount", panel_card_actions.visible_route_count);
    insert_json!("enabledRouteCount", panel_card_actions.enabled_route_count);
    insert_json!("breadcrumbCount", panel_card_actions.breadcrumb_count);
    insert_json!(
        "visibleBreadcrumbCount",
        panel_card_actions.visible_breadcrumb_count
    );
    insert_json!(
        "enabledBreadcrumbCount",
        panel_card_actions.enabled_breadcrumb_count
    );
    insert_json!("tabCount", panel_card_actions.tab_count);
    insert_json!("visibleTabCount", panel_card_actions.visible_tab_count);
    insert_json!("enabledTabCount", panel_card_actions.enabled_tab_count);
    insert_json!("panelCount", panel_card_actions.panel_count);
    insert_json!("visiblePanelCount", panel_card_actions.visible_panel_count);
    insert_json!("enabledPanelCount", panel_card_actions.enabled_panel_count);
    insert_json!("panelCardCount", panel_card_actions.panel_card_count);
    insert_json!(
        "visiblePanelCardCount",
        panel_card_actions.visible_panel_card_count
    );
    insert_json!(
        "enabledPanelCardCount",
        panel_card_actions.enabled_panel_card_count
    );
    insert_json!("actionCount", panel_card_actions.action_count);
    insert_json!(
        "enabledActionCount",
        panel_card_actions.enabled_action_count
    );
    insert_json!(
        "primaryActionCount",
        panel_card_actions.primary_action_count
    );
    insert_json!(
        "panelCardActionCount",
        panel_card_actions.panel_card_action_count
    );
    insert_json!(
        "visiblePanelCardActionCount",
        panel_card_actions.visible_panel_card_action_count
    );
    insert_json!(
        "enabledPanelCardActionCount",
        panel_card_actions.enabled_panel_card_action_count
    );
    insert_json!("itemCount", panel_card_actions.item_count);
    insert_json!("visibleItemCount", panel_card_actions.visible_item_count);
    insert_json!("enabledItemCount", panel_card_actions.enabled_item_count);
    insert_json!("regionCount", panel_card_actions.region_count);
    insert_json!(
        "visibleRegionCount",
        panel_card_actions.visible_region_count
    );
    insert_json!("cardCount", panel_card_actions.card_count);
    insert_json!("visibleCardCount", panel_card_actions.visible_card_count);
    insert_json!(
        "attentionCardCount",
        panel_card_actions.attention_card_count
    );
    insert_json!("metricCardCount", panel_card_actions.metric_card_count);
    value.insert(
        "panelCardActions".to_string(),
        serde_json::Value::Array(
            panel_card_actions
                .panel_card_actions
                .iter()
                .map(app_shell_dashboard_panel_card_action_json_value)
                .collect(),
        ),
    );
    insert_json!(
        "packageCapabilityId",
        &panel_card_actions.package_capability_id
    );
    insert_json!(
        "dashboardCapabilityId",
        &panel_card_actions.dashboard_capability_id
    );
    insert_json!("cardsCapabilityId", &panel_card_actions.cards_capability_id);
    insert_json!("viewCapabilityId", &panel_card_actions.view_capability_id);
    insert_json!(
        "layoutCapabilityId",
        &panel_card_actions.layout_capability_id
    );
    insert_json!(
        "navigationCapabilityId",
        &panel_card_actions.navigation_capability_id
    );
    insert_json!(
        "routesCapabilityId",
        &panel_card_actions.routes_capability_id
    );
    insert_json!(
        "breadcrumbsCapabilityId",
        &panel_card_actions.breadcrumbs_capability_id
    );
    insert_json!("tabsCapabilityId", &panel_card_actions.tabs_capability_id);
    insert_json!(
        "tabPanelsCapabilityId",
        &panel_card_actions.tab_panels_capability_id
    );
    insert_json!(
        "panelCardsCapabilityId",
        &panel_card_actions.panel_cards_capability_id
    );
    insert_json!(
        "panelCardActionsCapabilityId",
        &panel_card_actions.panel_card_actions_capability_id
    );
    insert_json!(
        "artifactCapabilityCount",
        panel_card_actions.artifact_capability_count
    );

    serde_json::Value::Object(value)
}

fn app_shell_dashboard_panel_card_action_json_value(
    panel_card_action: &BerkeleyAppShellDashboardPanelCardAction,
) -> serde_json::Value {
    serde_json::json!({
        "id": &panel_card_action.id,
        "panelCardId": &panel_card_action.panel_card_id,
        "panelId": &panel_card_action.panel_id,
        "cardId": &panel_card_action.card_id,
        "actionId": &panel_card_action.action_id,
        "label": &panel_card_action.label,
        "target": &panel_card_action.target,
        "panelKind": &panel_card_action.panel_kind,
        "role": &panel_card_action.role,
        "path": &panel_card_action.path,
        "position": panel_card_action.position,
        "selected": panel_card_action.selected,
        "default": panel_card_action.default_panel,
        "visible": panel_card_action.visible,
        "enabled": panel_card_action.enabled,
        "primary": panel_card_action.primary,
        "cardPrimary": panel_card_action.card_primary,
        "attention": panel_card_action.attention,
        "disabledReason": &panel_card_action.disabled_reason,
    })
}

fn app_shell_dashboard_action_dispatch_json_value(
    dispatch: &BerkeleyAppShellDashboardActionDispatch,
) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    macro_rules! insert_json {
        ($key:literal, $field:expr) => {
            value.insert($key.to_string(), serde_json::json!($field));
        };
    }

    insert_json!("schemaVersion", dispatch.schema_version);
    insert_json!("packageName", &dispatch.package_name);
    insert_json!("sourceFingerprint", &dispatch.source_fingerprint);
    insert_json!("title", &dispatch.title);
    insert_json!("startupRoute", &dispatch.startup_route);
    insert_json!("ready", dispatch.ready);
    insert_json!("severity", &dispatch.severity);
    insert_json!("attentionRequired", dispatch.attention_required);
    insert_json!("primaryCardId", &dispatch.primary_card_id);
    insert_json!("primaryRegionId", &dispatch.primary_region_id);
    insert_json!("activeItemId", &dispatch.active_item_id);
    insert_json!("activeRouteId", &dispatch.active_route_id);
    insert_json!("activeRoutePath", &dispatch.active_route_path);
    insert_json!("defaultRouteId", &dispatch.default_route_id);
    insert_json!("defaultRoutePath", &dispatch.default_route_path);
    insert_json!("activeBreadcrumbId", &dispatch.active_breadcrumb_id);
    insert_json!("activeBreadcrumbPath", &dispatch.active_breadcrumb_path);
    insert_json!("defaultBreadcrumbId", &dispatch.default_breadcrumb_id);
    insert_json!("defaultBreadcrumbPath", &dispatch.default_breadcrumb_path);
    insert_json!("selectedTabId", &dispatch.selected_tab_id);
    insert_json!("selectedTabPath", &dispatch.selected_tab_path);
    insert_json!("defaultTabId", &dispatch.default_tab_id);
    insert_json!("defaultTabPath", &dispatch.default_tab_path);
    insert_json!("selectedPanelId", &dispatch.selected_panel_id);
    insert_json!("selectedPanelPath", &dispatch.selected_panel_path);
    insert_json!("defaultPanelId", &dispatch.default_panel_id);
    insert_json!("defaultPanelPath", &dispatch.default_panel_path);
    insert_json!("selectedPanelCardId", &dispatch.selected_panel_card_id);
    insert_json!("selectedCardId", &dispatch.selected_card_id);
    insert_json!("defaultPanelCardId", &dispatch.default_panel_card_id);
    insert_json!("defaultCardId", &dispatch.default_card_id);
    insert_json!(
        "selectedPanelCardActionId",
        &dispatch.selected_panel_card_action_id
    );
    insert_json!(
        "selectedActionDispatchId",
        &dispatch.selected_action_dispatch_id
    );
    insert_json!("selectedActionId", &dispatch.selected_action_id);
    insert_json!(
        "defaultPanelCardActionId",
        &dispatch.default_panel_card_action_id
    );
    insert_json!(
        "defaultActionDispatchId",
        &dispatch.default_action_dispatch_id
    );
    insert_json!("defaultActionId", &dispatch.default_action_id);
    insert_json!("routeCount", dispatch.route_count);
    insert_json!("visibleRouteCount", dispatch.visible_route_count);
    insert_json!("enabledRouteCount", dispatch.enabled_route_count);
    insert_json!("breadcrumbCount", dispatch.breadcrumb_count);
    insert_json!("visibleBreadcrumbCount", dispatch.visible_breadcrumb_count);
    insert_json!("enabledBreadcrumbCount", dispatch.enabled_breadcrumb_count);
    insert_json!("tabCount", dispatch.tab_count);
    insert_json!("visibleTabCount", dispatch.visible_tab_count);
    insert_json!("enabledTabCount", dispatch.enabled_tab_count);
    insert_json!("panelCount", dispatch.panel_count);
    insert_json!("visiblePanelCount", dispatch.visible_panel_count);
    insert_json!("enabledPanelCount", dispatch.enabled_panel_count);
    insert_json!("panelCardCount", dispatch.panel_card_count);
    insert_json!("visiblePanelCardCount", dispatch.visible_panel_card_count);
    insert_json!("enabledPanelCardCount", dispatch.enabled_panel_card_count);
    insert_json!("actionCount", dispatch.action_count);
    insert_json!("enabledActionCount", dispatch.enabled_action_count);
    insert_json!("primaryActionCount", dispatch.primary_action_count);
    insert_json!("panelCardActionCount", dispatch.panel_card_action_count);
    insert_json!(
        "visiblePanelCardActionCount",
        dispatch.visible_panel_card_action_count
    );
    insert_json!(
        "enabledPanelCardActionCount",
        dispatch.enabled_panel_card_action_count
    );
    insert_json!("actionDispatchCount", dispatch.action_dispatch_count);
    insert_json!(
        "visibleActionDispatchCount",
        dispatch.visible_action_dispatch_count
    );
    insert_json!(
        "enabledActionDispatchCount",
        dispatch.enabled_action_dispatch_count
    );
    insert_json!("itemCount", dispatch.item_count);
    insert_json!("visibleItemCount", dispatch.visible_item_count);
    insert_json!("enabledItemCount", dispatch.enabled_item_count);
    insert_json!("regionCount", dispatch.region_count);
    insert_json!("visibleRegionCount", dispatch.visible_region_count);
    insert_json!("cardCount", dispatch.card_count);
    insert_json!("visibleCardCount", dispatch.visible_card_count);
    insert_json!("attentionCardCount", dispatch.attention_card_count);
    insert_json!("metricCardCount", dispatch.metric_card_count);
    value.insert(
        "actionDispatches".to_string(),
        serde_json::Value::Array(
            dispatch
                .action_dispatches
                .iter()
                .map(app_shell_dashboard_action_dispatch_item_json_value)
                .collect(),
        ),
    );
    insert_json!("packageCapabilityId", &dispatch.package_capability_id);
    insert_json!("dashboardCapabilityId", &dispatch.dashboard_capability_id);
    insert_json!("cardsCapabilityId", &dispatch.cards_capability_id);
    insert_json!("viewCapabilityId", &dispatch.view_capability_id);
    insert_json!("layoutCapabilityId", &dispatch.layout_capability_id);
    insert_json!("navigationCapabilityId", &dispatch.navigation_capability_id);
    insert_json!("routesCapabilityId", &dispatch.routes_capability_id);
    insert_json!(
        "breadcrumbsCapabilityId",
        &dispatch.breadcrumbs_capability_id
    );
    insert_json!("tabsCapabilityId", &dispatch.tabs_capability_id);
    insert_json!("tabPanelsCapabilityId", &dispatch.tab_panels_capability_id);
    insert_json!(
        "panelCardsCapabilityId",
        &dispatch.panel_cards_capability_id
    );
    insert_json!(
        "panelCardActionsCapabilityId",
        &dispatch.panel_card_actions_capability_id
    );
    insert_json!(
        "actionDispatchCapabilityId",
        &dispatch.action_dispatch_capability_id
    );
    insert_json!(
        "artifactCapabilityCount",
        dispatch.artifact_capability_count
    );

    serde_json::Value::Object(value)
}

fn app_shell_dashboard_action_dispatch_item_json_value(
    dispatch: &BerkeleyAppShellDashboardActionDispatchItem,
) -> serde_json::Value {
    serde_json::json!({
        "id": &dispatch.id,
        "panelCardActionId": &dispatch.panel_card_action_id,
        "panelCardId": &dispatch.panel_card_id,
        "panelId": &dispatch.panel_id,
        "cardId": &dispatch.card_id,
        "actionId": &dispatch.action_id,
        "label": &dispatch.label,
        "target": &dispatch.target,
        "panelKind": &dispatch.panel_kind,
        "role": &dispatch.role,
        "path": &dispatch.path,
        "position": dispatch.position,
        "selected": dispatch.selected,
        "default": dispatch.default_panel,
        "visible": dispatch.visible,
        "enabled": dispatch.enabled,
        "dispatchable": dispatch.dispatchable,
        "primary": dispatch.primary,
        "cardPrimary": dispatch.card_primary,
        "attention": dispatch.attention,
        "disabledReason": &dispatch.disabled_reason,
    })
}

fn app_shell_dashboard_dispatch_events_json_value(
    dispatch_events: &BerkeleyAppShellDashboardDispatchEvents,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": dispatch_events.schema_version,
        "packageName": &dispatch_events.package_name,
        "sourceFingerprint": &dispatch_events.source_fingerprint,
        "title": &dispatch_events.title,
        "startupRoute": &dispatch_events.startup_route,
        "ready": dispatch_events.ready,
        "severity": &dispatch_events.severity,
        "attentionRequired": dispatch_events.attention_required,
        "selectedActionDispatchId": &dispatch_events.selected_action_dispatch_id,
        "selectedDispatchEventId": &dispatch_events.selected_dispatch_event_id,
        "selectedActionId": &dispatch_events.selected_action_id,
        "defaultActionDispatchId": &dispatch_events.default_action_dispatch_id,
        "defaultDispatchEventId": &dispatch_events.default_dispatch_event_id,
        "defaultActionId": &dispatch_events.default_action_id,
        "actionDispatchCount": dispatch_events.action_dispatch_count,
        "dispatchEventCount": dispatch_events.dispatch_event_count,
        "dispatchReadyEventCount": dispatch_events.dispatch_ready_event_count,
        "dispatchBlockedEventCount": dispatch_events.dispatch_blocked_event_count,
        "attentionDispatchEventCount": dispatch_events.attention_dispatch_event_count,
        "selectedDispatchable": dispatch_events.selected_dispatchable,
        "defaultDispatchable": dispatch_events.default_dispatchable,
        "dispatchEvents": dispatch_events
            .dispatch_events
            .iter()
            .map(app_shell_dashboard_dispatch_event_json_value)
            .collect::<Vec<_>>(),
        "actionDispatchCapabilityId": &dispatch_events.action_dispatch_capability_id,
        "dispatchEventsCapabilityId": &dispatch_events.dispatch_events_capability_id,
        "artifactCapabilityCount": dispatch_events.artifact_capability_count,
    })
}

fn app_shell_dashboard_dispatch_event_json_value(
    event: &BerkeleyAppShellDashboardDispatchEvent,
) -> serde_json::Value {
    serde_json::json!({
        "id": &event.id,
        "actionDispatchId": &event.action_dispatch_id,
        "panelCardActionId": &event.panel_card_action_id,
        "actionId": &event.action_id,
        "kind": &event.kind,
        "severity": &event.severity,
        "message": &event.message,
        "label": &event.label,
        "target": &event.target,
        "role": &event.role,
        "path": &event.path,
        "position": event.position,
        "selected": event.selected,
        "default": event.default_dispatch,
        "dispatchable": event.dispatchable,
        "primary": event.primary,
        "attention": event.attention,
        "disabledReason": &event.disabled_reason,
    })
}

fn app_shell_dashboard_dispatch_queue_json_value(
    dispatch_queue: &BerkeleyAppShellDashboardDispatchQueue,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": dispatch_queue.schema_version,
        "packageName": &dispatch_queue.package_name,
        "sourceFingerprint": &dispatch_queue.source_fingerprint,
        "title": &dispatch_queue.title,
        "startupRoute": &dispatch_queue.startup_route,
        "ready": dispatch_queue.ready,
        "severity": &dispatch_queue.severity,
        "attentionRequired": dispatch_queue.attention_required,
        "selectedActionDispatchId": &dispatch_queue.selected_action_dispatch_id,
        "selectedDispatchEventId": &dispatch_queue.selected_dispatch_event_id,
        "selectedDispatchQueueItemId": &dispatch_queue.selected_dispatch_queue_item_id,
        "selectedActionId": &dispatch_queue.selected_action_id,
        "defaultActionDispatchId": &dispatch_queue.default_action_dispatch_id,
        "defaultDispatchEventId": &dispatch_queue.default_dispatch_event_id,
        "defaultDispatchQueueItemId": &dispatch_queue.default_dispatch_queue_item_id,
        "defaultActionId": &dispatch_queue.default_action_id,
        "actionDispatchCount": dispatch_queue.action_dispatch_count,
        "dispatchEventCount": dispatch_queue.dispatch_event_count,
        "dispatchReadyEventCount": dispatch_queue.dispatch_ready_event_count,
        "dispatchBlockedEventCount": dispatch_queue.dispatch_blocked_event_count,
        "dispatchQueueItemCount": dispatch_queue.dispatch_queue_item_count,
        "queuedDispatchCount": dispatch_queue.queued_dispatch_count,
        "blockedDispatchCount": dispatch_queue.blocked_dispatch_count,
        "attentionDispatchQueueItemCount": dispatch_queue.attention_dispatch_queue_item_count,
        "selectedQueued": dispatch_queue.selected_queued,
        "defaultQueued": dispatch_queue.default_queued,
        "dispatchQueueItems": dispatch_queue
            .dispatch_queue_items
            .iter()
            .map(app_shell_dashboard_dispatch_queue_item_json_value)
            .collect::<Vec<_>>(),
        "actionDispatchCapabilityId": &dispatch_queue.action_dispatch_capability_id,
        "dispatchEventsCapabilityId": &dispatch_queue.dispatch_events_capability_id,
        "dispatchQueueCapabilityId": &dispatch_queue.dispatch_queue_capability_id,
        "artifactCapabilityCount": dispatch_queue.artifact_capability_count,
    })
}

fn app_shell_dashboard_dispatch_queue_item_json_value(
    item: &BerkeleyAppShellDashboardDispatchQueueItem,
) -> serde_json::Value {
    serde_json::json!({
        "id": &item.id,
        "dispatchEventId": &item.dispatch_event_id,
        "actionDispatchId": &item.action_dispatch_id,
        "panelCardActionId": &item.panel_card_action_id,
        "actionId": &item.action_id,
        "queueState": &item.queue_state,
        "severity": &item.severity,
        "message": &item.message,
        "label": &item.label,
        "target": &item.target,
        "role": &item.role,
        "path": &item.path,
        "position": item.position,
        "selected": item.selected,
        "default": item.default_dispatch,
        "queued": item.queued,
        "blocked": item.blocked,
        "dispatchable": item.dispatchable,
        "primary": item.primary,
        "attention": item.attention,
        "disabledReason": &item.disabled_reason,
    })
}

fn app_shell_dashboard_dispatch_queue_summary_json_value(
    summary: &BerkeleyAppShellDashboardDispatchQueueSummary,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": summary.schema_version,
        "packageName": &summary.package_name,
        "sourceFingerprint": &summary.source_fingerprint,
        "title": &summary.title,
        "startupRoute": &summary.startup_route,
        "ready": summary.ready,
        "severity": &summary.severity,
        "attentionRequired": summary.attention_required,
        "selectedActionDispatchId": &summary.selected_action_dispatch_id,
        "selectedDispatchEventId": &summary.selected_dispatch_event_id,
        "selectedDispatchQueueItemId": &summary.selected_dispatch_queue_item_id,
        "selectedActionId": &summary.selected_action_id,
        "defaultActionDispatchId": &summary.default_action_dispatch_id,
        "defaultDispatchEventId": &summary.default_dispatch_event_id,
        "defaultDispatchQueueItemId": &summary.default_dispatch_queue_item_id,
        "defaultActionId": &summary.default_action_id,
        "actionDispatchCount": summary.action_dispatch_count,
        "dispatchEventCount": summary.dispatch_event_count,
        "dispatchReadyEventCount": summary.dispatch_ready_event_count,
        "dispatchBlockedEventCount": summary.dispatch_blocked_event_count,
        "dispatchQueueItemCount": summary.dispatch_queue_item_count,
        "queuedDispatchCount": summary.queued_dispatch_count,
        "blockedDispatchCount": summary.blocked_dispatch_count,
        "attentionDispatchQueueItemCount": summary.attention_dispatch_queue_item_count,
        "selectedQueued": summary.selected_queued,
        "defaultQueued": summary.default_queued,
        "firstQueuedDispatchQueueItemId": &summary.first_queued_dispatch_queue_item_id,
        "firstBlockedDispatchQueueItemId": &summary.first_blocked_dispatch_queue_item_id,
        "firstAttentionDispatchQueueItemId": &summary.first_attention_dispatch_queue_item_id,
        "queuedDispatchQueueItemIds": &summary.queued_dispatch_queue_item_ids,
        "blockedDispatchQueueItemIds": &summary.blocked_dispatch_queue_item_ids,
        "attentionDispatchQueueItemIds": &summary.attention_dispatch_queue_item_ids,
        "dispatchQueueCapabilityId": &summary.dispatch_queue_capability_id,
        "dispatchQueueSummaryCapabilityId": &summary.dispatch_queue_summary_capability_id,
        "artifactCapabilityCount": summary.artifact_capability_count,
    })
}

fn app_shell_dashboard_dispatch_queue_digest_json_value(
    digest: &BerkeleyAppShellDashboardDispatchQueueDigest,
) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    macro_rules! insert_json {
        ($key:literal, $field:expr) => {
            value.insert($key.to_string(), serde_json::json!($field));
        };
    }

    insert_json!("schemaVersion", digest.schema_version);
    insert_json!("packageName", &digest.package_name);
    insert_json!("sourceFingerprint", &digest.source_fingerprint);
    insert_json!("title", &digest.title);
    insert_json!("startupRoute", &digest.startup_route);
    insert_json!("ready", digest.ready);
    insert_json!("severity", &digest.severity);
    insert_json!("attentionRequired", digest.attention_required);
    insert_json!(
        "headlineDispatchQueueItemId",
        &digest.headline_dispatch_queue_item_id
    );
    insert_json!(
        "headlineDispatchEventId",
        &digest.headline_dispatch_event_id
    );
    insert_json!(
        "headlineActionDispatchId",
        &digest.headline_action_dispatch_id
    );
    insert_json!(
        "headlinePanelCardActionId",
        &digest.headline_panel_card_action_id
    );
    insert_json!("headlineActionId", &digest.headline_action_id);
    insert_json!("headlineQueueState", &digest.headline_queue_state);
    insert_json!("headlineMessage", &digest.headline_message);
    insert_json!("headlineLabel", &digest.headline_label);
    insert_json!("headlineTarget", &digest.headline_target);
    insert_json!("headlineRole", &digest.headline_role);
    insert_json!("headlinePath", &digest.headline_path);
    insert_json!("headlinePosition", digest.headline_position);
    insert_json!("headlineSelected", digest.headline_selected);
    insert_json!("headlineDefaultDispatch", digest.headline_default_dispatch);
    insert_json!("headlineQueued", digest.headline_queued);
    insert_json!("headlineBlocked", digest.headline_blocked);
    insert_json!("headlineDispatchable", digest.headline_dispatchable);
    insert_json!("headlinePrimary", digest.headline_primary);
    insert_json!("headlineAttention", digest.headline_attention);
    insert_json!("headlineDisabledReason", &digest.headline_disabled_reason);
    insert_json!(
        "selectedActionDispatchId",
        &digest.selected_action_dispatch_id
    );
    insert_json!(
        "selectedDispatchEventId",
        &digest.selected_dispatch_event_id
    );
    insert_json!(
        "selectedDispatchQueueItemId",
        &digest.selected_dispatch_queue_item_id
    );
    insert_json!("selectedActionId", &digest.selected_action_id);
    insert_json!(
        "defaultActionDispatchId",
        &digest.default_action_dispatch_id
    );
    insert_json!("defaultDispatchEventId", &digest.default_dispatch_event_id);
    insert_json!(
        "defaultDispatchQueueItemId",
        &digest.default_dispatch_queue_item_id
    );
    insert_json!("defaultActionId", &digest.default_action_id);
    insert_json!("actionDispatchCount", digest.action_dispatch_count);
    insert_json!("dispatchEventCount", digest.dispatch_event_count);
    insert_json!("dispatchReadyEventCount", digest.dispatch_ready_event_count);
    insert_json!(
        "dispatchBlockedEventCount",
        digest.dispatch_blocked_event_count
    );
    insert_json!("dispatchQueueItemCount", digest.dispatch_queue_item_count);
    insert_json!("queuedDispatchCount", digest.queued_dispatch_count);
    insert_json!("blockedDispatchCount", digest.blocked_dispatch_count);
    insert_json!(
        "attentionDispatchQueueItemCount",
        digest.attention_dispatch_queue_item_count
    );
    insert_json!("selectedQueued", digest.selected_queued);
    insert_json!("defaultQueued", digest.default_queued);
    insert_json!(
        "firstQueuedDispatchQueueItemId",
        &digest.first_queued_dispatch_queue_item_id
    );
    insert_json!(
        "firstBlockedDispatchQueueItemId",
        &digest.first_blocked_dispatch_queue_item_id
    );
    insert_json!(
        "firstAttentionDispatchQueueItemId",
        &digest.first_attention_dispatch_queue_item_id
    );
    insert_json!(
        "dispatchQueueCapabilityId",
        &digest.dispatch_queue_capability_id
    );
    insert_json!(
        "dispatchQueueSummaryCapabilityId",
        &digest.dispatch_queue_summary_capability_id
    );
    insert_json!(
        "dispatchQueueDigestCapabilityId",
        &digest.dispatch_queue_digest_capability_id
    );
    insert_json!("artifactCapabilityCount", digest.artifact_capability_count);

    serde_json::Value::Object(value)
}

fn app_shell_dashboard_dispatch_queue_lanes_json_value(
    lanes: &BerkeleyAppShellDashboardDispatchQueueLanes,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": lanes.schema_version,
        "packageName": &lanes.package_name,
        "sourceFingerprint": &lanes.source_fingerprint,
        "title": &lanes.title,
        "startupRoute": &lanes.startup_route,
        "ready": lanes.ready,
        "severity": &lanes.severity,
        "attentionRequired": lanes.attention_required,
        "headlineDispatchQueueItemId": &lanes.headline_dispatch_queue_item_id,
        "headlineQueueState": &lanes.headline_queue_state,
        "headlineMessage": &lanes.headline_message,
        "selectedDispatchQueueItemId": &lanes.selected_dispatch_queue_item_id,
        "defaultDispatchQueueItemId": &lanes.default_dispatch_queue_item_id,
        "firstQueuedDispatchQueueItemId": &lanes.first_queued_dispatch_queue_item_id,
        "firstBlockedDispatchQueueItemId": &lanes.first_blocked_dispatch_queue_item_id,
        "firstAttentionDispatchQueueItemId": &lanes.first_attention_dispatch_queue_item_id,
        "dispatchQueueItemCount": lanes.dispatch_queue_item_count,
        "queuedDispatchCount": lanes.queued_dispatch_count,
        "blockedDispatchCount": lanes.blocked_dispatch_count,
        "attentionDispatchQueueItemCount": lanes.attention_dispatch_queue_item_count,
        "laneCount": lanes.lane_count,
        "activeLaneId": &lanes.active_lane_id,
        "attentionLaneId": &lanes.attention_lane_id,
        "lanes": lanes
            .lanes
            .iter()
            .map(app_shell_dashboard_dispatch_queue_lane_json_value)
            .collect::<Vec<_>>(),
        "dispatchQueueCapabilityId": &lanes.dispatch_queue_capability_id,
        "dispatchQueueSummaryCapabilityId": &lanes.dispatch_queue_summary_capability_id,
        "dispatchQueueDigestCapabilityId": &lanes.dispatch_queue_digest_capability_id,
        "dispatchQueueLanesCapabilityId": &lanes.dispatch_queue_lanes_capability_id,
        "artifactCapabilityCount": lanes.artifact_capability_count,
    })
}

fn app_shell_dashboard_dispatch_queue_lane_json_value(
    lane: &BerkeleyAppShellDashboardDispatchQueueLane,
) -> serde_json::Value {
    serde_json::json!({
        "id": &lane.id,
        "title": &lane.title,
        "queueState": &lane.queue_state,
        "severity": &lane.severity,
        "dispatchQueueItemCount": lane.dispatch_queue_item_count,
        "dispatchQueueItemIds": &lane.dispatch_queue_item_ids,
        "selected": lane.selected,
        "defaultDispatch": lane.default_dispatch,
        "primary": lane.primary,
        "attention": lane.attention,
    })
}

fn app_shell_dashboard_dispatch_queue_lane_tabs_json_value(
    tabs: &BerkeleyAppShellDashboardDispatchQueueLaneTabs,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": tabs.schema_version,
        "packageName": &tabs.package_name,
        "sourceFingerprint": &tabs.source_fingerprint,
        "title": &tabs.title,
        "startupRoute": &tabs.startup_route,
        "ready": tabs.ready,
        "severity": &tabs.severity,
        "attentionRequired": tabs.attention_required,
        "activeLaneId": &tabs.active_lane_id,
        "activeTabId": &tabs.active_tab_id,
        "attentionLaneId": &tabs.attention_lane_id,
        "attentionTabId": &tabs.attention_tab_id,
        "laneCount": tabs.lane_count,
        "tabCount": tabs.tab_count,
        "enabledTabCount": tabs.enabled_tab_count,
        "disabledTabCount": tabs.disabled_tab_count,
        "tabs": tabs
            .tabs
            .iter()
            .map(app_shell_dashboard_dispatch_queue_lane_tab_json_value)
            .collect::<Vec<_>>(),
        "dispatchQueueCapabilityId": &tabs.dispatch_queue_capability_id,
        "dispatchQueueSummaryCapabilityId": &tabs.dispatch_queue_summary_capability_id,
        "dispatchQueueDigestCapabilityId": &tabs.dispatch_queue_digest_capability_id,
        "dispatchQueueLanesCapabilityId": &tabs.dispatch_queue_lanes_capability_id,
        "dispatchQueueLaneTabsCapabilityId": &tabs.dispatch_queue_lane_tabs_capability_id,
        "artifactCapabilityCount": tabs.artifact_capability_count,
    })
}

fn app_shell_dashboard_dispatch_queue_lane_tab_json_value(
    tab: &BerkeleyAppShellDashboardDispatchQueueLaneTab,
) -> serde_json::Value {
    serde_json::json!({
        "id": &tab.id,
        "laneId": &tab.lane_id,
        "title": &tab.title,
        "queueState": &tab.queue_state,
        "severity": &tab.severity,
        "dispatchQueueItemCount": tab.dispatch_queue_item_count,
        "selected": tab.selected,
        "defaultDispatch": tab.default_dispatch,
        "active": tab.active,
        "attention": tab.attention,
        "disabled": tab.disabled,
    })
}

fn app_shell_dashboard_dispatch_queue_lane_tab_panels_json_value(
    panels: &BerkeleyAppShellDashboardDispatchQueueLaneTabPanels,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": panels.schema_version,
        "packageName": &panels.package_name,
        "sourceFingerprint": &panels.source_fingerprint,
        "title": &panels.title,
        "startupRoute": &panels.startup_route,
        "ready": panels.ready,
        "severity": &panels.severity,
        "attentionRequired": panels.attention_required,
        "activeLaneId": &panels.active_lane_id,
        "activeTabId": &panels.active_tab_id,
        "activePanelId": &panels.active_panel_id,
        "attentionLaneId": &panels.attention_lane_id,
        "attentionTabId": &panels.attention_tab_id,
        "attentionPanelId": &panels.attention_panel_id,
        "laneCount": panels.lane_count,
        "tabCount": panels.tab_count,
        "enabledTabCount": panels.enabled_tab_count,
        "disabledTabCount": panels.disabled_tab_count,
        "panelCount": panels.panel_count,
        "enabledPanelCount": panels.enabled_panel_count,
        "disabledPanelCount": panels.disabled_panel_count,
        "emptyPanelCount": panels.empty_panel_count,
        "panels": panels
            .panels
            .iter()
            .map(app_shell_dashboard_dispatch_queue_lane_tab_panel_json_value)
            .collect::<Vec<_>>(),
        "dispatchQueueCapabilityId": &panels.dispatch_queue_capability_id,
        "dispatchQueueSummaryCapabilityId": &panels.dispatch_queue_summary_capability_id,
        "dispatchQueueDigestCapabilityId": &panels.dispatch_queue_digest_capability_id,
        "dispatchQueueLanesCapabilityId": &panels.dispatch_queue_lanes_capability_id,
        "dispatchQueueLaneTabsCapabilityId": &panels.dispatch_queue_lane_tabs_capability_id,
        "dispatchQueueLaneTabPanelsCapabilityId": &panels.dispatch_queue_lane_tab_panels_capability_id,
        "artifactCapabilityCount": panels.artifact_capability_count,
    })
}

fn app_shell_dashboard_dispatch_queue_lane_tab_panel_json_value(
    panel: &BerkeleyAppShellDashboardDispatchQueueLaneTabPanel,
) -> serde_json::Value {
    serde_json::json!({
        "id": &panel.id,
        "tabId": &panel.tab_id,
        "laneId": &panel.lane_id,
        "title": &panel.title,
        "queueState": &panel.queue_state,
        "severity": &panel.severity,
        "dispatchQueueItemCount": panel.dispatch_queue_item_count,
        "selected": panel.selected,
        "defaultDispatch": panel.default_dispatch,
        "active": panel.active,
        "attention": panel.attention,
        "disabled": panel.disabled,
        "empty": panel.empty,
        "emptyMessage": &panel.empty_message,
    })
}

fn app_shell_dashboard_dispatch_queue_lane_tab_panel_cards_json_value(
    panel_cards: &BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": panel_cards.schema_version,
        "packageName": &panel_cards.package_name,
        "sourceFingerprint": &panel_cards.source_fingerprint,
        "title": &panel_cards.title,
        "startupRoute": &panel_cards.startup_route,
        "ready": panel_cards.ready,
        "severity": &panel_cards.severity,
        "attentionRequired": panel_cards.attention_required,
        "activeLaneId": &panel_cards.active_lane_id,
        "activeTabId": &panel_cards.active_tab_id,
        "activePanelId": &panel_cards.active_panel_id,
        "activePanelCardId": &panel_cards.active_panel_card_id,
        "attentionLaneId": &panel_cards.attention_lane_id,
        "attentionTabId": &panel_cards.attention_tab_id,
        "attentionPanelId": &panel_cards.attention_panel_id,
        "attentionPanelCardId": &panel_cards.attention_panel_card_id,
        "laneCount": panel_cards.lane_count,
        "tabCount": panel_cards.tab_count,
        "enabledTabCount": panel_cards.enabled_tab_count,
        "disabledTabCount": panel_cards.disabled_tab_count,
        "panelCount": panel_cards.panel_count,
        "enabledPanelCount": panel_cards.enabled_panel_count,
        "disabledPanelCount": panel_cards.disabled_panel_count,
        "emptyPanelCount": panel_cards.empty_panel_count,
        "panelCardCount": panel_cards.panel_card_count,
        "enabledPanelCardCount": panel_cards.enabled_panel_card_count,
        "disabledPanelCardCount": panel_cards.disabled_panel_card_count,
        "emptyPanelCardCount": panel_cards.empty_panel_card_count,
        "panelCards": panel_cards
            .panel_cards
            .iter()
            .map(app_shell_dashboard_dispatch_queue_lane_tab_panel_card_json_value)
            .collect::<Vec<_>>(),
        "dispatchQueueCapabilityId": &panel_cards.dispatch_queue_capability_id,
        "dispatchQueueSummaryCapabilityId": &panel_cards.dispatch_queue_summary_capability_id,
        "dispatchQueueDigestCapabilityId": &panel_cards.dispatch_queue_digest_capability_id,
        "dispatchQueueLanesCapabilityId": &panel_cards.dispatch_queue_lanes_capability_id,
        "dispatchQueueLaneTabsCapabilityId": &panel_cards.dispatch_queue_lane_tabs_capability_id,
        "dispatchQueueLaneTabPanelsCapabilityId": &panel_cards.dispatch_queue_lane_tab_panels_capability_id,
        "dispatchQueueLaneTabPanelCardsCapabilityId": &panel_cards.dispatch_queue_lane_tab_panel_cards_capability_id,
        "artifactCapabilityCount": panel_cards.artifact_capability_count,
    })
}

fn app_shell_dashboard_dispatch_queue_lane_tab_panel_card_json_value(
    panel_card: &BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCard,
) -> serde_json::Value {
    serde_json::json!({
        "id": &panel_card.id,
        "panelId": &panel_card.panel_id,
        "tabId": &panel_card.tab_id,
        "laneId": &panel_card.lane_id,
        "title": &panel_card.title,
        "queueState": &panel_card.queue_state,
        "severity": &panel_card.severity,
        "summary": &panel_card.summary,
        "dispatchQueueItemCount": panel_card.dispatch_queue_item_count,
        "badgeCount": panel_card.badge_count,
        "selected": panel_card.selected,
        "defaultDispatch": panel_card.default_dispatch,
        "active": panel_card.active,
        "attention": panel_card.attention,
        "disabled": panel_card.disabled,
        "empty": panel_card.empty,
        "emptyMessage": &panel_card.empty_message,
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
