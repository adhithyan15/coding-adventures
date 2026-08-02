use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
};

use spice_engine::{
    ac_sweep, dc_op_with_options, dc_sweep, transient_with_method, AcPoint,
    AdaptiveTransientOptions, Bjt, BjtPolarity, Capacitor, Cccs, Ccvs, Circuit, Complex,
    CurrentSource, DcOpOptions, DcResult, DcSweepPoint, Diode, Element, ExpWaveform, Inductor,
    Jfet, JfetPolarity, Mosfet, MosfetLevel1Params, MosfetType, MutualInductor, PulseWaveform,
    PwlWaveform, Resistor, SinWaveform, SpiceError, TransientMethod, TransientPoint,
    TransmissionLine, Vccs, Vcvs, VoltageSource, Waveform,
};

const OXIDE_PERMITTIVITY: f64 = 3.453_133e-11;

mod syntax;

pub use syntax::{
    berkeley_app_package_manifest, berkeley_app_package_manifest_json, parse_berkeley_app_deck,
    parse_berkeley_syntax, BerkeleyAnalysisInventoryEntry, BerkeleyAppAnalysisArtifact,
    BerkeleyAppAnalysisControl, BerkeleyAppBootstrapSnapshot, BerkeleyAppDeck,
    BerkeleyAppEditorAction, BerkeleyAppEditorActionKind, BerkeleyAppEditorCommand,
    BerkeleyAppEditorCommandPlan, BerkeleyAppEditorControls, BerkeleyAppEditorStateSnapshot,
    BerkeleyAppExecution, BerkeleyAppHostDiagnosticWire, BerkeleyAppHostPanel,
    BerkeleyAppHostPanelKind, BerkeleyAppHostPanelWire, BerkeleyAppHostSpanWire,
    BerkeleyAppHostSurface, BerkeleyAppHostSurfaceWire, BerkeleyAppLaunchAction,
    BerkeleyAppLaunchPlan, BerkeleyAppPackageManifest, BerkeleyAppPersistedEditorState,
    BerkeleyAppReadinessReport, BerkeleyAppSessionAnalysis, BerkeleyAppSessionState,
    BerkeleyAppShellDashboardActionDispatch, BerkeleyAppShellDashboardActionDispatchItem,
    BerkeleyAppShellDashboardBreadcrumb, BerkeleyAppShellDashboardBreadcrumbs,
    BerkeleyAppShellDashboardCard, BerkeleyAppShellDashboardCards,
    BerkeleyAppShellDashboardDispatchEvent, BerkeleyAppShellDashboardDispatchEvents,
    BerkeleyAppShellDashboardDispatchQueue, BerkeleyAppShellDashboardDispatchQueueDigest,
    BerkeleyAppShellDashboardDispatchQueueItem, BerkeleyAppShellDashboardDispatchQueueLane,
    BerkeleyAppShellDashboardDispatchQueueLaneTab,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanel,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCard,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardAction,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenu,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroup,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcut,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutBinding,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutBindings,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommand,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPalette,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteItem,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchIndex,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchIndexEntry,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocation,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceipt,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotification,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStack,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummary,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoff,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackage,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedLoaderPlan,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedManifest,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationPlan,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceipt,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournal,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummary,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoff,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceipt,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgement,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecord,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceipt,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecord,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordSummary,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecordSummaryDigest,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimePlan,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptNotificationStackSummaryProductHandoffDeliveryPackageEmbedRuntimeSessionPlan,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceiptSummary,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationReceipts,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchResult,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchResults,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchSelection,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandRegistry,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroupShortcuts,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuGroups,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActionMenuItem,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCardActions,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanelCards,
    BerkeleyAppShellDashboardDispatchQueueLaneTabPanels,
    BerkeleyAppShellDashboardDispatchQueueLaneTabs, BerkeleyAppShellDashboardDispatchQueueLanes,
    BerkeleyAppShellDashboardDispatchQueueSummary, BerkeleyAppShellDashboardLayout,
    BerkeleyAppShellDashboardLayoutRegion, BerkeleyAppShellDashboardNavigation,
    BerkeleyAppShellDashboardNavigationItem, BerkeleyAppShellDashboardPackage,
    BerkeleyAppShellDashboardPanelCard, BerkeleyAppShellDashboardPanelCardAction,
    BerkeleyAppShellDashboardPanelCardActions, BerkeleyAppShellDashboardPanelCards,
    BerkeleyAppShellDashboardRoute, BerkeleyAppShellDashboardRoutes, BerkeleyAppShellDashboardTab,
    BerkeleyAppShellDashboardTabPanel, BerkeleyAppShellDashboardTabPanels,
    BerkeleyAppShellDashboardTabs, BerkeleyAppShellDashboardView, BerkeleyAppShellEvent,
    BerkeleyAppShellEventDashboard, BerkeleyAppShellEventDashboardSection,
    BerkeleyAppShellEventDigest, BerkeleyAppShellEventLog, BerkeleyAppShellEventSummary,
    BerkeleyAppShellHandoff, BerkeleyAppShellStatus, BerkeleyAppShellTelemetry,
    BerkeleyAppStartupSummary, BerkeleyAppWaveformPoint, BerkeleyAppWaveformSeries,
    BerkeleyCardKind, BerkeleyDiagnosticSeverity, BerkeleyGrammarMetadata, BerkeleyLogicalCard,
    BerkeleySyntaxDeck, BerkeleySyntaxDiagnostic, BerkeleySyntaxToken, SourceSpan,
    BERKELEY_APP_BOOTSTRAP_SCHEMA_VERSION, BERKELEY_APP_HOST_SURFACE_WIRE_SCHEMA_VERSION,
    BERKELEY_APP_LAUNCH_PLAN_SCHEMA_VERSION, BERKELEY_APP_PACKAGE_MANIFEST_SCHEMA_VERSION,
    BERKELEY_APP_PACKAGE_NAME, BERKELEY_APP_READINESS_REPORT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_ACTION_DISPATCH_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_BREADCRUMBS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_CARDS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_EVENTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_DIGEST_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANES_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TABS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANELS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARDS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTIONS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUPS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_BINDINGS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INDEX_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_LOADER_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_MANIFEST_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SUMMARY_DIGEST_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_RECORD_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_ACKNOWLEDGEMENT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_RECEIPT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_RECORD_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_ACKNOWLEDGEMENT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_RECEIPT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_HANDOFF_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_JOURNAL_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_ACTIVATION_RECEIPT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_EMBED_RUNTIME_SESSION_PLAN_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_DELIVERY_PACKAGE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_PRODUCT_HANDOFF_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_NOTIFICATION_STACK_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_RECEIPT_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_INVOCATION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_RESULTS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_PALETTE_SEARCH_SELECTION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_GROUP_SHORTCUT_COMMAND_REGISTRY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_LANE_TAB_PANEL_CARD_ACTION_MENU_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_DISPATCH_QUEUE_SUMMARY_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_LAYOUT_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_NAVIGATION_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_PACKAGE_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARDS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_PANEL_CARD_ACTIONS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_ROUTES_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_TABS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_TAB_PANELS_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_DASHBOARD_VIEW_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_EVENT_DASHBOARD_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_EVENT_DIGEST_SCHEMA_VERSION, BERKELEY_APP_SHELL_EVENT_LOG_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_EVENT_SUMMARY_SCHEMA_VERSION, BERKELEY_APP_SHELL_HANDOFF_SCHEMA_VERSION,
    BERKELEY_APP_SHELL_STATUS_SCHEMA_VERSION, BERKELEY_APP_SHELL_TELEMETRY_SCHEMA_VERSION,
    BERKELEY_APP_SOURCE_FINGERPRINT_ALGORITHM, BERKELEY_APP_STARTUP_SUMMARY_SCHEMA_VERSION,
    BERKELEY_SPICE_GRAMMAR_NAME, BERKELEY_SPICE_GRAMMAR_VERSION, BERKELEY_SPICE_PARSER_GRAMMAR,
    BERKELEY_SPICE_TOKEN_GRAMMAR,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetlistParseError {
    message: String,
}

impl NetlistParseError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NetlistParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NetlistParseError {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OpAnalysis;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TranAnalysis {
    pub time_step: f64,
    pub stop_time: f64,
    pub method: Option<TransientMethod>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcAnalysis {
    pub source_name: String,
    pub start: f64,
    pub stop: f64,
    pub step: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AcAnalysis {
    pub mode: String,
    pub points: usize,
    pub start_hz: f64,
    pub stop_hz: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfAnalysis {
    pub output_node: String,
    pub input_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensAnalysis {
    pub output_node: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct McAnalysis {
    pub output_node: String,
    pub n_trials: usize,
    pub tolerance: f64,
    pub distribution: String,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseAnalysis {
    pub output_node: String,
    pub input_source: String,
    pub frequencies_hz: Vec<f64>,
    pub temperature: f64,
    pub temperature_is_explicit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempAnalysis {
    pub temperatures_celsius: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputProbe {
    Voltage { node: String },
    Current { source_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintAnalysis {
    pub analysis: String,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlotAnalysis {
    pub analysis: String,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveAnalysis {
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAnalysis {
    pub analysis: Option<String>,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MeasureOperation {
    Find,
    Max,
    Min,
    Avg,
    Rms,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureAnalysis {
    pub analysis: String,
    pub name: String,
    pub operation: MeasureOperation,
    pub probe: OutputProbe,
    pub at: Option<f64>,
    pub start: Option<f64>,
    pub stop: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourAnalysis {
    pub frequency_hz: f64,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistortionAnalysis {
    pub mode: String,
    pub points: usize,
    pub start_hz: f64,
    pub stop_hz: f64,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PoleZeroKind {
    Pole,
    Zero,
    PoleZero,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoleZeroAnalysis {
    pub output_node: String,
    pub input_source: String,
    pub kind: PoleZeroKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Number(f64),
    Text(String),
    Flag(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionsAnalysis {
    pub values: HashMap<String, OptionValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Analysis {
    Op(OpAnalysis),
    Tran(TranAnalysis),
    Dc(DcAnalysis),
    Ac(AcAnalysis),
    Tf(TfAnalysis),
    Sens(SensAnalysis),
    Mc(McAnalysis),
    Noise(NoiseAnalysis),
    Temp(TempAnalysis),
    Print(PrintAnalysis),
    Plot(PlotAnalysis),
    Save(SaveAnalysis),
    Probe(ProbeAnalysis),
    Measure(MeasureAnalysis),
    Four(FourAnalysis),
    Distortion(DistortionAnalysis),
    PoleZero(PoleZeroAnalysis),
    Options(OptionsAnalysis),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AnalysisKind {
    Op,
    Tran,
    Dc,
    Ac,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunnableAnalysis {
    Op(OpAnalysis),
    Tran(TranAnalysis),
    Dc(DcAnalysis),
    Ac(AcAnalysis),
}

impl RunnableAnalysis {
    pub fn kind(&self) -> AnalysisKind {
        match self {
            Self::Op(_) => AnalysisKind::Op,
            Self::Tran(_) => AnalysisKind::Tran,
            Self::Dc(_) => AnalysisKind::Dc,
            Self::Ac(_) => AnalysisKind::Ac,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisPlanStep {
    pub index: usize,
    pub kind: AnalysisKind,
    pub analysis: RunnableAnalysis,
}

#[derive(Debug, Clone, PartialEq)]
// Boxing the large `Op(DcResult)` variant would ripple through every consumer's
// pattern matches; the size difference is not worth that churn here.
#[allow(clippy::large_enum_variant)]
pub enum AnalysisResult {
    Op(DcResult),
    Tran(Vec<TransientPoint>),
    Dc(Vec<DcSweepPoint>),
    Ac(Vec<AcPoint>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisExecutionResult {
    pub index: usize,
    pub kind: AnalysisKind,
    pub analysis: RunnableAnalysis,
    pub result: AnalysisResult,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SelectedOutputValue {
    Real(f64),
    Complex(Complex),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedOutputRow {
    pub index: usize,
    pub axis_name: Option<String>,
    pub axis_value: Option<f64>,
    pub values: BTreeMap<String, SelectedOutputValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedAnalysisOutput {
    pub index: usize,
    pub kind: AnalysisKind,
    pub probes: Vec<OutputProbe>,
    pub rows: Vec<SelectedOutputRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureResult {
    pub analysis_index: usize,
    pub analysis: String,
    pub name: String,
    pub operation: MeasureOperation,
    pub probe: OutputProbe,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisExecutionError {
    Netlist(NetlistParseError),
    Spice(SpiceError),
}

impl fmt::Display for AnalysisExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Netlist(error) => write!(f, "{error}"),
            Self::Spice(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for AnalysisExecutionError {}

impl From<NetlistParseError> for AnalysisExecutionError {
    fn from(error: NetlistParseError) -> Self {
        Self::Netlist(error)
    }
}

impl From<SpiceError> for AnalysisExecutionError {
    fn from(error: SpiceError) -> Self {
        Self::Spice(error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCard {
    pub name: String,
    pub kind: String,
    pub params: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedNetlist {
    pub circuit: Circuit,
    pub analyses: Vec<Analysis>,
    pub models: HashMap<String, ModelCard>,
    pub title: Option<String>,
}

impl ParsedNetlist {
    pub fn op_cards(&self) -> Vec<&OpAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Op(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn tran_cards(&self) -> Vec<&TranAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Tran(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn dc_cards(&self) -> Vec<&DcAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Dc(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn ac_cards(&self) -> Vec<&AcAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Ac(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn tf_cards(&self) -> Vec<&TfAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Tf(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn sens_cards(&self) -> Vec<&SensAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Sens(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn mc_cards(&self) -> Vec<&McAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Mc(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn noise_cards(&self) -> Vec<&NoiseAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Noise(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn options_cards(&self) -> Vec<&OptionsAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Options(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn temp_cards(&self) -> Vec<&TempAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Temp(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn print_cards(&self) -> Vec<&PrintAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Print(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn plot_cards(&self) -> Vec<&PlotAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Plot(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn save_cards(&self) -> Vec<&SaveAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Save(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn probe_cards(&self) -> Vec<&ProbeAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Probe(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn measure_cards(&self) -> Vec<&MeasureAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Measure(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn four_cards(&self) -> Vec<&FourAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Four(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn distortion_cards(&self) -> Vec<&DistortionAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Distortion(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn pole_zero_cards(&self) -> Vec<&PoleZeroAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::PoleZero(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn transient_method(
        &self,
        tran: Option<&TranAnalysis>,
    ) -> Result<Option<TransientMethod>, NetlistParseError> {
        if let Some(method) = tran.and_then(|card| card.method) {
            return Ok(Some(method));
        }
        for options in self.options_cards() {
            if let Some(OptionValue::Text(value)) = options.values.get("method") {
                return Ok(Some(parse_transient_method(value, ".options method")?));
            }
        }
        Ok(None)
    }

    pub fn dc_op_options(&self) -> Result<DcOpOptions, NetlistParseError> {
        let values = self.merged_options();
        let mut options = DcOpOptions::default();
        if let Some(tolerance) = option_number(&values, &["reltol", "tol"])? {
            options.tolerance = tolerance;
        }
        if let Some(max_iterations) =
            option_usize(&values, &["itl1", "maxiter", "maxiters", "maxiterations"])?
        {
            options.max_iterations = max_iterations;
        }
        if let Some(gmin) = option_number(&values, &["gmin"])? {
            options.pseudo_transient_conductance = gmin;
        }
        if let Some(pseudo_steps) = option_usize(&values, &["srcsteps", "pseudotransientsteps"])? {
            options.pseudo_transient_steps = pseudo_steps;
        }
        if let Some(pseudo_iterations) =
            option_usize(&values, &["itl6", "pseudotransientmaxiterations"])?
        {
            options.pseudo_transient_max_iterations = pseudo_iterations;
        }
        Ok(options)
    }

    pub fn adaptive_transient_options(
        &self,
        tran: Option<&TranAnalysis>,
    ) -> Result<AdaptiveTransientOptions, NetlistParseError> {
        let values = self.merged_options();
        let mut options = AdaptiveTransientOptions::default();
        if let Some(method) = self.transient_method(tran)? {
            options.method = method;
        }
        if let Some(tolerance) = option_number(&values, &["trtol", "lte", "tollte"])? {
            options.tolerance = tolerance;
        }
        if let Some(min_step) = option_number(&values, &["minstep", "tmin"])? {
            options.min_step = Some(min_step);
        }
        if let Some(max_step) = option_number(&values, &["maxstep", "tmax"])? {
            options.max_step = Some(max_step);
        }
        Ok(options)
    }

    pub fn operating_temperature_kelvin(
        &self,
        temperature_index: usize,
        default_temperature_kelvin: f64,
    ) -> Result<f64, NetlistParseError> {
        let temperatures_celsius = self
            .temp_cards()
            .into_iter()
            .flat_map(|card| card.temperatures_celsius.iter().copied())
            .collect::<Vec<_>>();
        if temperatures_celsius.is_empty() {
            return Ok(default_temperature_kelvin);
        }
        let Some(temperature_celsius) = temperatures_celsius.get(temperature_index) else {
            return Err(NetlistParseError::new(format!(
                "temperature index {temperature_index} exceeds .temp entries"
            )));
        };
        Ok(temperature_celsius + 273.15)
    }

    pub fn noise_temperature_kelvin(
        &self,
        noise: Option<&NoiseAnalysis>,
        temperature_index: usize,
        default_temperature_kelvin: f64,
    ) -> Result<f64, NetlistParseError> {
        if let Some(noise) = noise {
            if noise.temperature_is_explicit {
                return Ok(noise.temperature);
            }
        }
        self.operating_temperature_kelvin(temperature_index, default_temperature_kelvin)
    }

    pub fn analysis_plan(&self) -> Vec<AnalysisPlanStep> {
        build_analysis_plan(self)
    }

    pub fn run_analysis_plan(
        &self,
    ) -> Result<Vec<AnalysisExecutionResult>, AnalysisExecutionError> {
        run_analysis_plan(self)
    }

    pub fn select_outputs(
        &self,
        results: &[AnalysisExecutionResult],
    ) -> Result<Vec<SelectedAnalysisOutput>, NetlistParseError> {
        select_outputs(self, results)
    }

    pub fn measure_results(
        &self,
        results: &[AnalysisExecutionResult],
    ) -> Result<Vec<MeasureResult>, NetlistParseError> {
        measure_results(self, results)
    }

    fn merged_options(&self) -> HashMap<String, OptionValue> {
        let mut values = HashMap::new();
        for options in self.options_cards() {
            values.extend(options.values.clone());
        }
        values
    }
}

fn option_number(
    values: &HashMap<String, OptionValue>,
    keys: &[&str],
) -> Result<Option<f64>, NetlistParseError> {
    for key in keys {
        if let Some(value) = values.get(*key) {
            return match value {
                OptionValue::Number(value) => Ok(Some(*value)),
                OptionValue::Text(value) => Err(NetlistParseError::new(format!(
                    ".options {key:?} must be numeric, got {value:?}"
                ))),
                OptionValue::Flag(_) => Err(NetlistParseError::new(format!(
                    ".options {key:?} requires a numeric value"
                ))),
            };
        }
    }
    Ok(None)
}

fn option_usize(
    values: &HashMap<String, OptionValue>,
    keys: &[&str],
) -> Result<Option<usize>, NetlistParseError> {
    let Some(value) = option_number(values, keys)? else {
        return Ok(None);
    };
    if !value.is_finite() || value < 0.0 {
        return Err(NetlistParseError::new(
            ".options iteration counts must be finite and non-negative",
        ));
    }
    Ok(Some(value.trunc() as usize))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Statement {
    line_number: usize,
    fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubcktDefinition {
    name: String,
    pins: Vec<String>,
    body: Vec<Statement>,
    line_number: usize,
}

pub fn parse_netlist(text: &str) -> Result<ParsedNetlist, NetlistParseError> {
    let syntax = syntax::parse_berkeley_syntax(text);
    if let Some(diagnostic) = syntax
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.is_error())
    {
        return Err(syntax_diagnostic_error(diagnostic));
    }

    let mut circuit = Circuit::new();
    let mut analyses = Vec::new();
    let mut models = HashMap::new();
    let mut statements = Vec::new();
    let mut subckts = HashMap::new();
    let mut current_subckt: Option<SubcktDefinition> = None;
    let title = syntax.title.clone();

    for card in syntax.cards {
        let line_number = card.span.start_line;
        if card.kind == BerkeleyCardKind::End {
            break;
        }

        let fields = split_fields(&card.text).map_err(|err| line_error(line_number, err))?;
        if fields.is_empty() {
            continue;
        }
        let head = &fields[0];
        let head_lower = head.to_ascii_lowercase();

        if let Some(definition) = current_subckt.as_mut() {
            if head_lower == ".ends" {
                finish_subckt(definition, &fields).map_err(|err| line_error(line_number, err))?;
                let definition = current_subckt.take().expect("subckt exists");
                subckts.insert(definition.name.to_ascii_lowercase(), definition);
            } else if head_lower == ".subckt" {
                return Err(line_error(
                    line_number,
                    NetlistParseError::new("nested .subckt definitions are not supported"),
                ));
            } else {
                definition.body.push(Statement {
                    line_number,
                    fields,
                });
            }
            continue;
        }
        if head_lower == ".subckt" {
            current_subckt = Some(
                start_subckt(&fields, line_number, &subckts)
                    .map_err(|err| line_error(line_number, err))?,
            );
            continue;
        }
        if head_lower == ".ends" {
            return Err(line_error(
                line_number,
                NetlistParseError::new(".ends without matching .subckt"),
            ));
        }

        if head_lower == ".end" {
            break;
        }
        statements.push(Statement {
            line_number,
            fields,
        });
    }

    if let Some(definition) = current_subckt {
        return Err(NetlistParseError::new(format!(
            "line {}: .subckt {:?} is missing .ends",
            definition.line_number, definition.name
        )));
    }

    for statement in &statements {
        if !statement.fields[0].eq_ignore_ascii_case(".model") {
            continue;
        }
        let model = parse_model_card(&statement.fields)
            .map_err(|err| line_error(statement.line_number, err))?;
        let key = model.name.to_ascii_lowercase();
        if models.contains_key(&key) {
            return Err(line_error(
                statement.line_number,
                NetlistParseError::new(format!("duplicate .model definition {:?}", model.name)),
            ));
        }
        models.insert(key, model);
    }

    for statement in statements {
        if statement.fields[0].eq_ignore_ascii_case(".model") {
            continue;
        }
        if statement.fields[0].starts_with('.') {
            let analysis = parse_directive(&statement.fields)
                .map_err(|err| line_error(statement.line_number, err))?;
            analyses.push(analysis);
        } else if statement.fields[0].to_ascii_uppercase().starts_with('X') {
            let elements = expand_subckt_instance(&statement.fields, &subckts, &[], &models)
                .map_err(|err| line_error(statement.line_number, err))?;
            for element in elements {
                circuit.add(element);
            }
        } else {
            let element = parse_element(&statement.fields, &models)
                .map_err(|err| line_error(statement.line_number, err))?;
            circuit.add(element);
        }
    }
    validate_mutual_inductors(&circuit)?;
    validate_transmission_lines(&circuit)?;

    Ok(ParsedNetlist {
        circuit,
        analyses,
        models,
        title,
    })
}

fn syntax_diagnostic_error(diagnostic: &BerkeleySyntaxDiagnostic) -> NetlistParseError {
    let message = format!("{}: {}", diagnostic.code, diagnostic.message);
    if let Some(span) = diagnostic.span {
        line_error(span.start_line, NetlistParseError::new(message))
    } else {
        NetlistParseError::new(message)
    }
}

pub fn build_analysis_plan(parsed: &ParsedNetlist) -> Vec<AnalysisPlanStep> {
    parsed
        .analyses
        .iter()
        .enumerate()
        .filter_map(|(index, analysis)| analysis_plan_step(index, analysis))
        .collect()
}

pub fn run_analysis_plan(
    parsed: &ParsedNetlist,
) -> Result<Vec<AnalysisExecutionResult>, AnalysisExecutionError> {
    build_analysis_plan(parsed)
        .into_iter()
        .map(|step| {
            let result = execute_analysis_step(parsed, &step)?;
            Ok(AnalysisExecutionResult {
                index: step.index,
                kind: step.kind,
                analysis: step.analysis,
                result,
            })
        })
        .collect()
}

pub fn run_netlist(text: &str) -> Result<Vec<AnalysisExecutionResult>, AnalysisExecutionError> {
    let parsed = parse_netlist(text)?;
    run_analysis_plan(&parsed)
}

pub fn select_outputs(
    parsed: &ParsedNetlist,
    results: &[AnalysisExecutionResult],
) -> Result<Vec<SelectedAnalysisOutput>, NetlistParseError> {
    let mut selected = Vec::new();
    for result in results {
        let probes = selected_output_probes(parsed, result.kind);
        if probes.is_empty() {
            continue;
        }
        selected.push(SelectedAnalysisOutput {
            index: result.index,
            kind: result.kind,
            rows: selected_output_rows(result, &probes)?,
            probes,
        });
    }
    Ok(selected)
}

pub fn measure_results(
    parsed: &ParsedNetlist,
    results: &[AnalysisExecutionResult],
) -> Result<Vec<MeasureResult>, NetlistParseError> {
    parsed
        .measure_cards()
        .into_iter()
        .map(|card| {
            let execution = find_measure_execution_result(card, results)?;
            Ok(MeasureResult {
                analysis_index: execution.index,
                analysis: card.analysis.clone(),
                name: card.name.clone(),
                operation: card.operation,
                probe: card.probe.clone(),
                value: evaluate_measure(card, execution)?,
            })
        })
        .collect()
}

fn analysis_plan_step(index: usize, analysis: &Analysis) -> Option<AnalysisPlanStep> {
    let analysis = match analysis {
        Analysis::Op(card) => RunnableAnalysis::Op(*card),
        Analysis::Tran(card) => RunnableAnalysis::Tran(*card),
        Analysis::Dc(card) => RunnableAnalysis::Dc(card.clone()),
        Analysis::Ac(card) => RunnableAnalysis::Ac(card.clone()),
        _ => return None,
    };
    Some(AnalysisPlanStep {
        index,
        kind: analysis.kind(),
        analysis,
    })
}

fn execute_analysis_step(
    parsed: &ParsedNetlist,
    step: &AnalysisPlanStep,
) -> Result<AnalysisResult, AnalysisExecutionError> {
    match &step.analysis {
        RunnableAnalysis::Op(_) => Ok(AnalysisResult::Op(dc_op_with_options(
            &parsed.circuit,
            parsed.dc_op_options()?,
        )?)),
        RunnableAnalysis::Tran(card) => {
            let method = parsed
                .transient_method(Some(card))?
                .unwrap_or(TransientMethod::Euler);
            Ok(AnalysisResult::Tran(transient_with_method(
                &parsed.circuit,
                card.time_step,
                card.stop_time,
                method,
            )?))
        }
        RunnableAnalysis::Dc(card) => Ok(AnalysisResult::Dc(dc_sweep(
            &parsed.circuit,
            &card.source_name,
            card.start,
            card.stop,
            card.step,
        )?)),
        RunnableAnalysis::Ac(card) => Ok(AnalysisResult::Ac(ac_sweep(
            &parsed.circuit,
            card.start_hz,
            card.stop_hz,
            executable_ac_points_per_decade(card)?,
        )?)),
    }
}

fn executable_ac_points_per_decade(card: &AcAnalysis) -> Result<usize, NetlistParseError> {
    if card.mode == "dec" || card.mode == "log" {
        return Ok(card.points);
    }
    Err(NetlistParseError::new(format!(
        ".ac mode {:?} is not executable; supported modes are \"dec\" and \"log\"",
        card.mode
    )))
}

fn selected_output_probes(parsed: &ParsedNetlist, kind: AnalysisKind) -> Vec<OutputProbe> {
    let mut probes = Vec::new();
    let mut seen = HashSet::new();
    for card in &parsed.analyses {
        let matching = match card {
            Analysis::Save(card) => Some(card.probes.as_slice()),
            Analysis::Probe(card)
                if card
                    .analysis
                    .as_deref()
                    .is_none_or(|name| analysis_name_matches(name, kind)) =>
            {
                Some(card.probes.as_slice())
            }
            Analysis::Print(card) if analysis_name_matches(&card.analysis, kind) => {
                Some(card.probes.as_slice())
            }
            Analysis::Plot(card) if analysis_name_matches(&card.analysis, kind) => {
                Some(card.probes.as_slice())
            }
            _ => None,
        };
        let Some(new_probes) = matching else {
            continue;
        };
        for probe in new_probes {
            let key = probe_key(probe);
            if seen.insert(key) {
                probes.push(probe.clone());
            }
        }
    }
    probes
}

fn analysis_name_matches(requested: &str, kind: AnalysisKind) -> bool {
    match requested.to_ascii_lowercase().as_str() {
        "op" | "dcop" => kind == AnalysisKind::Op,
        "dc" => kind == AnalysisKind::Dc,
        "ac" => kind == AnalysisKind::Ac,
        "tran" | "transient" => kind == AnalysisKind::Tran,
        _ => false,
    }
}

fn selected_output_rows(
    execution: &AnalysisExecutionResult,
    probes: &[OutputProbe],
) -> Result<Vec<SelectedOutputRow>, NetlistParseError> {
    match &execution.result {
        AnalysisResult::Op(result) => Ok(vec![SelectedOutputRow {
            index: 0,
            axis_name: None,
            axis_value: None,
            values: selected_real_output_values(
                &result.node_voltages,
                &result.branch_currents,
                probes,
                ".op output selection",
            )?,
        }]),
        AnalysisResult::Dc(points) => points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                Ok(SelectedOutputRow {
                    index,
                    axis_name: Some("source".to_string()),
                    axis_value: Some(point.value),
                    values: selected_real_output_values(
                        &point.result.node_voltages,
                        &point.result.branch_currents,
                        probes,
                        ".dc output selection",
                    )?,
                })
            })
            .collect(),
        AnalysisResult::Ac(points) => points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                Ok(SelectedOutputRow {
                    index,
                    axis_name: Some("frequency".to_string()),
                    axis_value: Some(point.frequency_hz),
                    values: selected_complex_output_values(
                        &point.node_voltages,
                        &point.branch_currents,
                        probes,
                        ".ac output selection",
                    )?,
                })
            })
            .collect(),
        AnalysisResult::Tran(points) => points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                Ok(SelectedOutputRow {
                    index,
                    axis_name: Some("time".to_string()),
                    axis_value: Some(point.time),
                    values: selected_real_output_values(
                        &point.node_voltages,
                        &point.branch_currents,
                        probes,
                        ".tran output selection",
                    )?,
                })
            })
            .collect(),
    }
}

fn selected_real_output_values(
    node_voltages: &BTreeMap<String, f64>,
    branch_currents: &BTreeMap<String, f64>,
    probes: &[OutputProbe],
    context: &str,
) -> Result<BTreeMap<String, SelectedOutputValue>, NetlistParseError> {
    let mut values = BTreeMap::new();
    for probe in probes {
        values.insert(
            probe_label(probe),
            probe_real_value(probe, node_voltages, branch_currents, context)?,
        );
    }
    Ok(values)
}

fn selected_complex_output_values(
    node_voltages: &BTreeMap<String, Complex>,
    branch_currents: &BTreeMap<String, Complex>,
    probes: &[OutputProbe],
    context: &str,
) -> Result<BTreeMap<String, SelectedOutputValue>, NetlistParseError> {
    let mut values = BTreeMap::new();
    for probe in probes {
        values.insert(
            probe_label(probe),
            probe_complex_value(probe, node_voltages, branch_currents, context)?,
        );
    }
    Ok(values)
}

fn find_measure_execution_result<'a>(
    card: &MeasureAnalysis,
    results: &'a [AnalysisExecutionResult],
) -> Result<&'a AnalysisExecutionResult, NetlistParseError> {
    results
        .iter()
        .find(|result| analysis_name_matches(&card.analysis, result.kind))
        .ok_or_else(|| {
            NetlistParseError::new(format!(
                ".measure {:?} references missing {} analysis",
                card.name, card.analysis
            ))
        })
}

fn evaluate_measure(
    card: &MeasureAnalysis,
    execution: &AnalysisExecutionResult,
) -> Result<f64, NetlistParseError> {
    let samples = measure_samples(card, execution)?;
    if samples.is_empty() {
        return Err(NetlistParseError::new(format!(
            ".measure {:?} has no samples",
            card.name
        )));
    }
    if card.operation == MeasureOperation::Find {
        if execution.kind == AnalysisKind::Op && card.at.is_none() {
            return Ok(measure_numeric_value(samples[0].value));
        }
        let Some(at) = card.at else {
            return Err(NetlistParseError::new(format!(
                ".measure {:?} FIND requires AT=<value>",
                card.name
            )));
        };
        return Ok(measure_numeric_value(interpolate_measure_value(
            &samples, at, card,
        )?));
    }

    let ranged = range_measure_samples(&samples, card)?;
    if ranged.is_empty() {
        return Err(NetlistParseError::new(format!(
            ".measure {:?} range has no samples",
            card.name
        )));
    }
    match card.operation {
        MeasureOperation::Max => Ok(ranged
            .iter()
            .map(|sample| measure_numeric_value(sample.value))
            .fold(f64::NEG_INFINITY, f64::max)),
        MeasureOperation::Min => Ok(ranged
            .iter()
            .map(|sample| measure_numeric_value(sample.value))
            .fold(f64::INFINITY, f64::min)),
        MeasureOperation::Avg => Ok(average_measure_value(&ranged)),
        MeasureOperation::Rms => Ok(rms_measure_value(&ranged)),
        MeasureOperation::Find => unreachable!("handled above"),
    }
}

#[derive(Debug, Copy, Clone)]
struct MeasureSample {
    axis: Option<f64>,
    value: SelectedOutputValue,
}

fn measure_samples(
    card: &MeasureAnalysis,
    execution: &AnalysisExecutionResult,
) -> Result<Vec<MeasureSample>, NetlistParseError> {
    match &execution.result {
        AnalysisResult::Op(result) => Ok(vec![MeasureSample {
            axis: None,
            value: probe_real_value(
                &card.probe,
                &result.node_voltages,
                &result.branch_currents,
                &format!(".measure {}", card.name),
            )?,
        }]),
        AnalysisResult::Dc(points) => points
            .iter()
            .map(|point| {
                Ok(MeasureSample {
                    axis: Some(point.value),
                    value: probe_real_value(
                        &card.probe,
                        &point.result.node_voltages,
                        &point.result.branch_currents,
                        &format!(".measure {}", card.name),
                    )?,
                })
            })
            .collect(),
        AnalysisResult::Ac(points) => points
            .iter()
            .map(|point| {
                Ok(MeasureSample {
                    axis: Some(point.frequency_hz),
                    value: probe_complex_value(
                        &card.probe,
                        &point.node_voltages,
                        &point.branch_currents,
                        &format!(".measure {}", card.name),
                    )?,
                })
            })
            .collect(),
        AnalysisResult::Tran(points) => points
            .iter()
            .map(|point| {
                Ok(MeasureSample {
                    axis: Some(point.time),
                    value: probe_real_value(
                        &card.probe,
                        &point.node_voltages,
                        &point.branch_currents,
                        &format!(".measure {}", card.name),
                    )?,
                })
            })
            .collect(),
    }
}

fn range_measure_samples(
    samples: &[MeasureSample],
    card: &MeasureAnalysis,
) -> Result<Vec<MeasureSample>, NetlistParseError> {
    if samples.iter().any(|sample| sample.axis.is_none()) {
        if card.start.is_some() || card.stop.is_some() {
            return Err(NetlistParseError::new(format!(
                ".measure {:?} range requires swept samples",
                card.name
            )));
        }
        return Ok(samples.to_vec());
    }
    let mut axis_samples = samples.to_vec();
    axis_samples.sort_by(|left, right| left.axis.unwrap().total_cmp(&right.axis.unwrap()));
    let lower = card.start.unwrap_or(axis_samples[0].axis.unwrap());
    let upper = card
        .stop
        .unwrap_or(axis_samples.last().unwrap().axis.unwrap());
    if lower > upper {
        return Err(NetlistParseError::new(format!(
            ".measure {:?} FROM must be <= TO",
            card.name
        )));
    }
    let mut ranged = Vec::new();
    if card.start.is_some() {
        ranged.push(MeasureSample {
            axis: Some(lower),
            value: interpolate_measure_value(samples, lower, card)?,
        });
    }
    for sample in axis_samples {
        let axis = sample.axis.unwrap();
        if axis >= lower && axis <= upper && !axis_already_present(&ranged, axis) {
            ranged.push(sample);
        }
    }
    if card.stop.is_some() && !axis_already_present(&ranged, upper) {
        ranged.push(MeasureSample {
            axis: Some(upper),
            value: interpolate_measure_value(samples, upper, card)?,
        });
    }
    ranged.sort_by(|left, right| left.axis.unwrap().total_cmp(&right.axis.unwrap()));
    Ok(ranged)
}

fn axis_already_present(samples: &[MeasureSample], axis: f64) -> bool {
    samples.iter().any(|sample| {
        sample
            .axis
            .is_some_and(|existing| (existing - axis).abs() <= 1.0e-12)
    })
}

fn interpolate_measure_value(
    samples: &[MeasureSample],
    target: f64,
    card: &MeasureAnalysis,
) -> Result<SelectedOutputValue, NetlistParseError> {
    let mut axis_samples = samples
        .iter()
        .copied()
        .filter(|sample| sample.axis.is_some())
        .collect::<Vec<_>>();
    axis_samples.sort_by(|left, right| left.axis.unwrap().total_cmp(&right.axis.unwrap()));
    if axis_samples.is_empty() {
        return Err(NetlistParseError::new(format!(
            ".measure {:?} AT requires swept samples",
            card.name
        )));
    }
    if target < axis_samples[0].axis.unwrap() || target > axis_samples.last().unwrap().axis.unwrap()
    {
        return Err(NetlistParseError::new(format!(
            ".measure {:?} AT is outside the analysis range",
            card.name
        )));
    }
    for sample in &axis_samples {
        if (sample.axis.unwrap() - target).abs() <= 1.0e-12 {
            return Ok(sample.value);
        }
    }
    for window in axis_samples.windows(2) {
        let left = window[0];
        let right = window[1];
        let left_axis = left.axis.unwrap();
        let right_axis = right.axis.unwrap();
        if left_axis <= target && target <= right_axis {
            let fraction = (target - left_axis) / (right_axis - left_axis);
            return Ok(interpolate_output_values(left.value, right.value, fraction));
        }
    }
    Ok(axis_samples.last().unwrap().value)
}

fn interpolate_output_values(
    left: SelectedOutputValue,
    right: SelectedOutputValue,
    fraction: f64,
) -> SelectedOutputValue {
    match (left, right) {
        (SelectedOutputValue::Real(left), SelectedOutputValue::Real(right)) => {
            SelectedOutputValue::Real(left + (right - left) * fraction)
        }
        (left, right) => {
            let left = output_value_as_complex(left);
            let right = output_value_as_complex(right);
            SelectedOutputValue::Complex(Complex::new(
                left.real + (right.real - left.real) * fraction,
                left.imag + (right.imag - left.imag) * fraction,
            ))
        }
    }
}

fn average_measure_value(samples: &[MeasureSample]) -> f64 {
    if samples.len() < 2 || samples.iter().any(|sample| sample.axis.is_none()) {
        return samples
            .iter()
            .map(|sample| measure_numeric_value(sample.value))
            .sum::<f64>()
            / samples.len() as f64;
    }
    let span = samples.last().unwrap().axis.unwrap() - samples[0].axis.unwrap();
    if span <= 0.0 {
        return samples
            .iter()
            .map(|sample| measure_numeric_value(sample.value))
            .sum::<f64>()
            / samples.len() as f64;
    }
    let area = samples
        .windows(2)
        .map(|window| {
            let left = window[0];
            let right = window[1];
            0.5 * (measure_numeric_value(left.value) + measure_numeric_value(right.value))
                * (right.axis.unwrap() - left.axis.unwrap())
        })
        .sum::<f64>();
    area / span
}

fn rms_measure_value(samples: &[MeasureSample]) -> f64 {
    if samples.len() < 2 || samples.iter().any(|sample| sample.axis.is_none()) {
        return (samples
            .iter()
            .map(|sample| measure_numeric_value(sample.value).powi(2))
            .sum::<f64>()
            / samples.len() as f64)
            .sqrt();
    }
    let span = samples.last().unwrap().axis.unwrap() - samples[0].axis.unwrap();
    if span <= 0.0 {
        return (samples
            .iter()
            .map(|sample| measure_numeric_value(sample.value).powi(2))
            .sum::<f64>()
            / samples.len() as f64)
            .sqrt();
    }
    let area = samples
        .windows(2)
        .map(|window| {
            let left = window[0];
            let right = window[1];
            let left_value = measure_numeric_value(left.value);
            let right_value = measure_numeric_value(right.value);
            0.5 * (left_value.powi(2) + right_value.powi(2))
                * (right.axis.unwrap() - left.axis.unwrap())
        })
        .sum::<f64>();
    (area / span).sqrt()
}

fn measure_numeric_value(value: SelectedOutputValue) -> f64 {
    match value {
        SelectedOutputValue::Real(value) => value,
        SelectedOutputValue::Complex(value) => value.abs(),
    }
}

fn output_value_as_complex(value: SelectedOutputValue) -> Complex {
    match value {
        SelectedOutputValue::Real(value) => Complex::new(value, 0.0),
        SelectedOutputValue::Complex(value) => value,
    }
}

fn probe_real_value(
    probe: &OutputProbe,
    node_voltages: &BTreeMap<String, f64>,
    branch_currents: &BTreeMap<String, f64>,
    context: &str,
) -> Result<SelectedOutputValue, NetlistParseError> {
    match probe {
        OutputProbe::Voltage { node } => {
            if is_probe_ground(node) {
                return Ok(SelectedOutputValue::Real(0.0));
            }
            case_insensitive_get_real(node_voltages, node)
                .map(SelectedOutputValue::Real)
                .ok_or_else(|| {
                    NetlistParseError::new(format!("{context}: missing voltage probe V({node})"))
                })
        }
        OutputProbe::Current { source_name } => {
            let key = branch_current_key(source_name);
            case_insensitive_get_real(branch_currents, &key)
                .map(SelectedOutputValue::Real)
                .ok_or_else(|| {
                    NetlistParseError::new(format!(
                        "{context}: missing branch current probe I({source_name})"
                    ))
                })
        }
    }
}

fn probe_complex_value(
    probe: &OutputProbe,
    node_voltages: &BTreeMap<String, Complex>,
    branch_currents: &BTreeMap<String, Complex>,
    context: &str,
) -> Result<SelectedOutputValue, NetlistParseError> {
    match probe {
        OutputProbe::Voltage { node } => {
            if is_probe_ground(node) {
                return Ok(SelectedOutputValue::Complex(Complex::zero()));
            }
            case_insensitive_get_complex(node_voltages, node)
                .map(SelectedOutputValue::Complex)
                .ok_or_else(|| {
                    NetlistParseError::new(format!("{context}: missing voltage probe V({node})"))
                })
        }
        OutputProbe::Current { source_name } => {
            let key = branch_current_key(source_name);
            case_insensitive_get_complex(branch_currents, &key)
                .map(SelectedOutputValue::Complex)
                .ok_or_else(|| {
                    NetlistParseError::new(format!(
                        "{context}: missing branch current probe I({source_name})"
                    ))
                })
        }
    }
}

fn case_insensitive_get_real(values: &BTreeMap<String, f64>, key: &str) -> Option<f64> {
    values.get(key).copied().or_else(|| {
        let lower = key.to_ascii_lowercase();
        values
            .iter()
            .find(|(candidate, _)| candidate.to_ascii_lowercase() == lower)
            .map(|(_, value)| *value)
    })
}

fn case_insensitive_get_complex(values: &BTreeMap<String, Complex>, key: &str) -> Option<Complex> {
    values.get(key).copied().or_else(|| {
        let lower = key.to_ascii_lowercase();
        values
            .iter()
            .find(|(candidate, _)| candidate.to_ascii_lowercase() == lower)
            .map(|(_, value)| *value)
    })
}

fn branch_current_key(source_name: &str) -> String {
    if source_name.to_ascii_lowercase().starts_with("i(") {
        source_name.to_string()
    } else {
        format!("I({source_name})")
    }
}

fn is_probe_ground(node: &str) -> bool {
    matches!(node.to_ascii_lowercase().as_str(), "0" | "gnd")
}

fn probe_label(probe: &OutputProbe) -> String {
    match probe {
        OutputProbe::Voltage { node } => format!("V({node})"),
        OutputProbe::Current { source_name } => format!("I({source_name})"),
    }
}

fn probe_key(probe: &OutputProbe) -> (String, String) {
    match probe {
        OutputProbe::Voltage { node } => ("voltage".to_string(), node.to_ascii_lowercase()),
        OutputProbe::Current { source_name } => {
            ("current".to_string(), source_name.to_ascii_lowercase())
        }
    }
}

fn validate_mutual_inductors(circuit: &Circuit) -> Result<(), NetlistParseError> {
    let inductors: std::collections::HashSet<String> = circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::Inductor(inductor) => Some(inductor.name.clone()),
            _ => None,
        })
        .collect();

    for element in circuit.elements() {
        let Element::MutualInductor(mutual) = element else {
            continue;
        };
        if !mutual.coupling.is_finite() {
            return Err(NetlistParseError::new(format!(
                "{}: coupling must be finite",
                mutual.name
            )));
        }
        if mutual.coupling.abs() >= 1.0 {
            return Err(NetlistParseError::new(format!(
                "{}: coupling magnitude must be less than one",
                mutual.name
            )));
        }
        if mutual.primary == mutual.secondary {
            return Err(NetlistParseError::new(format!(
                "{}: coupled inductors must be distinct",
                mutual.name
            )));
        }
        if !inductors.contains(&mutual.primary) {
            return Err(NetlistParseError::new(format!(
                "{}: referenced inductor {:?} was not found",
                mutual.name, mutual.primary
            )));
        }
        if !inductors.contains(&mutual.secondary) {
            return Err(NetlistParseError::new(format!(
                "{}: referenced inductor {:?} was not found",
                mutual.name, mutual.secondary
            )));
        }
    }

    Ok(())
}

fn validate_transmission_lines(circuit: &Circuit) -> Result<(), NetlistParseError> {
    for element in circuit.elements() {
        let Element::TransmissionLine(line) = element else {
            continue;
        };
        if !line.characteristic_impedance_ohms.is_finite() {
            return Err(NetlistParseError::new(format!(
                "{}: characteristic impedance must be finite",
                line.name
            )));
        }
        if line.characteristic_impedance_ohms <= 0.0 {
            return Err(NetlistParseError::new(format!(
                "{}: characteristic impedance must be positive",
                line.name
            )));
        }
        if !line.delay_seconds.is_finite() {
            return Err(NetlistParseError::new(format!(
                "{}: delay must be finite",
                line.name
            )));
        }
        if line.delay_seconds <= 0.0 {
            return Err(NetlistParseError::new(format!(
                "{}: delay must be positive",
                line.name
            )));
        }
    }

    Ok(())
}

pub fn parse(text: &str) -> Result<ParsedNetlist, NetlistParseError> {
    parse_netlist(text)
}

pub fn parse_value(token: &str) -> Result<f64, NetlistParseError> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(NetlistParseError::new(
            "expected numeric value, got empty token",
        ));
    }

    for split in trimmed
        .char_indices()
        .map(|(idx, _)| idx)
        .chain([trimmed.len()])
        .rev()
    {
        let number = &trimmed[..split];
        let suffix = trimmed[split..].to_ascii_lowercase();
        if !is_supported_suffix(&suffix) {
            continue;
        }
        if let Ok(value) = number.parse::<f64>() {
            return Ok(value * suffix_multiplier(&suffix));
        }
    }

    Err(NetlistParseError::new(format!(
        "expected numeric value, got {token:?}"
    )))
}

fn parse_model_card(fields: &[String]) -> Result<ModelCard, NetlistParseError> {
    require_min_fields(fields, 3, ".model")?;
    let tail = fields[2..].join(" ");
    let trimmed = tail.trim();
    let kind_end = trimmed
        .find(|ch: char| ch.is_whitespace() || ch == '(')
        .unwrap_or(trimmed.len());
    if kind_end == 0 {
        return Err(NetlistParseError::new(format!(
            "invalid .model kind {trimmed:?}"
        )));
    }
    let kind = trimmed[..kind_end].to_ascii_uppercase();
    let mut params_text = trimmed[kind_end..].trim();
    if params_text.starts_with('(') && params_text.ends_with(')') {
        params_text = &params_text[1..params_text.len() - 1];
    }
    let params = parse_model_params(params_text)?;
    if matches!(kind.as_str(), "NMOS" | "PMOS") {
        if let Some(oxide_thickness) = params.get("TOX") {
            if !oxide_thickness.is_finite() || *oxide_thickness <= 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET TOX must be finite and positive",
                ));
            }
        }
        for surface_mobility in [params.get("U0"), params.get("UO")].into_iter().flatten() {
            if !surface_mobility.is_finite() || *surface_mobility < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET U0 must be finite and non-negative",
                ));
            }
        }
        if let Some(transconductance) = params.get("KP") {
            if !transconductance.is_finite() || *transconductance <= 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET KP must be finite and positive",
                ));
            }
        }
        for threshold_voltage in [params.get("VT0"), params.get("VTO"), params.get("VTH")]
            .into_iter()
            .flatten()
        {
            if !threshold_voltage.is_finite() {
                return Err(NetlistParseError::new("MOSFET VT0 must be finite"));
            }
        }
        for channel_length_modulation in [params.get("LAMBDA"), params.get("LAM")]
            .into_iter()
            .flatten()
        {
            if !channel_length_modulation.is_finite() {
                return Err(NetlistParseError::new("MOSFET LAMBDA must be finite"));
            }
        }
        if let Some(bulk_potential) = params.get("PHI") {
            if !bulk_potential.is_finite() || *bulk_potential <= 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET PHI must be finite and positive",
                ));
            }
        }
        if let Some(body_effect_coefficient) = params.get("GAMMA") {
            if !body_effect_coefficient.is_finite() || *body_effect_coefficient < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET GAMMA must be finite and non-negative",
                ));
            }
        }
        if let Some(bulk_junction_potential) = params.get("PB") {
            if !bulk_junction_potential.is_finite() || *bulk_junction_potential <= 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET PB must be finite and positive",
                ));
            }
        }
        if let Some(bulk_junction_grading_coefficient) = params.get("MJ") {
            if !bulk_junction_grading_coefficient.is_finite()
                || *bulk_junction_grading_coefficient < 0.0
            {
                return Err(NetlistParseError::new(
                    "MOSFET MJ must be finite and non-negative",
                ));
            }
        }
        if let Some(depletion_coefficient) = params.get("FC") {
            if !depletion_coefficient.is_finite()
                || *depletion_coefficient < 0.0
                || *depletion_coefficient >= 1.0
            {
                return Err(NetlistParseError::new(
                    "MOSFET FC must be finite and in [0, 1)",
                ));
            }
        }
        if let Some(sidewall_grading_coefficient) = params.get("MJSW") {
            if !sidewall_grading_coefficient.is_finite() || *sidewall_grading_coefficient < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET MJSW must be finite and non-negative",
                ));
            }
        }
        if let Some(bottom_junction_capacitance) = params.get("CJ") {
            if !bottom_junction_capacitance.is_finite() || *bottom_junction_capacitance < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET CJ must be finite and non-negative",
                ));
            }
        }
        if let Some(sidewall_junction_capacitance) = params.get("CJSW") {
            if !sidewall_junction_capacitance.is_finite() || *sidewall_junction_capacitance < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET CJSW must be finite and non-negative",
                ));
            }
        }
        for source_bulk_capacitance in [params.get("CBS"), params.get("CJS")].into_iter().flatten()
        {
            if !source_bulk_capacitance.is_finite() || *source_bulk_capacitance < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET CBS must be finite and non-negative",
                ));
            }
        }
        for drain_bulk_capacitance in [params.get("CBD"), params.get("CJD")].into_iter().flatten() {
            if !drain_bulk_capacitance.is_finite() || *drain_bulk_capacitance < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET CBD must be finite and non-negative",
                ));
            }
        }
        if let Some(gate_source_overlap_capacitance) = params.get("CGSO") {
            if !gate_source_overlap_capacitance.is_finite()
                || *gate_source_overlap_capacitance < 0.0
            {
                return Err(NetlistParseError::new(
                    "MOSFET CGSO must be finite and non-negative",
                ));
            }
        }
        if let Some(gate_drain_overlap_capacitance) = params.get("CGDO") {
            if !gate_drain_overlap_capacitance.is_finite() || *gate_drain_overlap_capacitance < 0.0
            {
                return Err(NetlistParseError::new(
                    "MOSFET CGDO must be finite and non-negative",
                ));
            }
        }
        if let Some(gate_bulk_overlap_capacitance) = params.get("CGBO") {
            if !gate_bulk_overlap_capacitance.is_finite() || *gate_bulk_overlap_capacitance < 0.0 {
                return Err(NetlistParseError::new(
                    "MOSFET CGBO must be finite and non-negative",
                ));
            }
        }
    }
    Ok(ModelCard {
        name: fields[1].clone(),
        kind,
        params,
    })
}

fn parse_model_params(params_text: &str) -> Result<HashMap<String, f64>, NetlistParseError> {
    let mut params = HashMap::new();
    let mut rest = params_text.trim();
    while !rest.is_empty() {
        rest = rest.trim_start_matches(|ch: char| ch.is_whitespace() || ch == ',');
        if rest.is_empty() {
            break;
        }
        let name_end = rest
            .find(|ch: char| ch.is_whitespace() || ch == '=')
            .unwrap_or(rest.len());
        let name = &rest[..name_end];
        if name.is_empty()
            || !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(NetlistParseError::new(format!(
                "invalid .model parameter syntax {params_text:?}"
            )));
        }
        rest = rest[name_end..].trim_start();
        if !rest.starts_with('=') {
            return Err(NetlistParseError::new(format!(
                "invalid .model parameter syntax {params_text:?}"
            )));
        }
        rest = rest[1..].trim_start();
        let value_end = rest
            .find(|ch: char| ch.is_whitespace() || ch == ',')
            .unwrap_or(rest.len());
        let value = &rest[..value_end];
        if value.is_empty() {
            return Err(NetlistParseError::new(format!(
                "invalid .model parameter syntax {params_text:?}"
            )));
        }
        params.insert(name.to_ascii_uppercase(), parse_value(value)?);
        rest = &rest[value_end..];
    }
    Ok(params)
}

fn parse_element_params(
    fields: &[String],
    label: &str,
) -> Result<HashMap<String, f64>, NetlistParseError> {
    let mut params = HashMap::new();
    for token in fields {
        let Some((name, value)) = token.split_once('=') else {
            return Err(NetlistParseError::new(format!(
                "invalid {label} parameter syntax {token:?}"
            )));
        };
        if name.is_empty() || value.is_empty() {
            return Err(NetlistParseError::new(format!(
                "invalid {label} parameter syntax {token:?}"
            )));
        }
        params.insert(name.to_ascii_uppercase(), parse_value(value)?);
    }
    Ok(params)
}

fn build_mosfet_params(
    model: &ModelCard,
    instance_params: &HashMap<String, f64>,
) -> MosfetLevel1Params {
    let mut params = MosfetLevel1Params::default();
    for (name, value) in model.params.iter().chain(instance_params.iter()) {
        apply_mosfet_param(&mut params, name, *value);
    }
    if !model.params.contains_key("KP") && !instance_params.contains_key("KP") {
        if let Some(oxide_thickness) = model.params.get("TOX").filter(|value| **value > 0.0) {
            params.kp = params.surface_mobility * 1.0e-4 * OXIDE_PERMITTIVITY / oxide_thickness;
        }
    }
    if let Some(value) = instance_params.get("NRD") {
        params.drain_squares = *value;
    }
    if let Some(value) = instance_params.get("NRS") {
        params.source_squares = *value;
    }
    if let Some(value) = instance_params.get("AD") {
        params.drain_area = *value;
    }
    if let Some(value) = instance_params.get("AS") {
        params.source_area = *value;
    }
    if let Some(value) = instance_params.get("PD") {
        params.drain_perimeter = *value;
    }
    if let Some(value) = instance_params.get("PS") {
        params.source_perimeter = *value;
    }
    params
}

fn apply_mosfet_param(params: &mut MosfetLevel1Params, name: &str, value: f64) {
    match name {
        "VT0" | "VTO" | "VTH" => params.vt0 = value,
        "KP" => params.kp = value,
        "LAMBDA" | "LAM" => params.lambda = value,
        "GAMMA" => params.gamma = value,
        "PHI" => params.phi = value,
        "W" => params.w = value,
        "L" => params.l = value,
        "LD" => params.lateral_diffusion_length = value,
        "TOX" => params.oxide_thickness = value,
        "U0" | "UO" => params.surface_mobility = value,
        "RD" => params.drain_resistance = value,
        "RS" => params.source_resistance = value,
        "RSH" => params.sheet_resistance = value,
        "IS" => params.saturation_current = value,
        "JS" => params.saturation_current_density = value,
        "N_SUB" | "NSUB" | "N" => params.n_sub = value,
        "T_NOM" | "TNOM" => params.t_nom = value,
        "CGSO" => params.gate_source_overlap_capacitance = value,
        "CGDO" => params.gate_drain_overlap_capacitance = value,
        "CGBO" => params.gate_bulk_overlap_capacitance = value,
        "CBS" | "CJS" => params.source_bulk_capacitance = value,
        "CBD" | "CJD" => params.drain_bulk_capacitance = value,
        "CJ" => params.bottom_junction_capacitance = value,
        "CJSW" => params.sidewall_junction_capacitance = value,
        "PB" => params.bulk_junction_potential = value,
        "MJ" => params.bulk_junction_grading_coefficient = value,
        "MJSW" => params.sidewall_junction_grading_coefficient = value,
        "FC" => params.forward_bias_depletion_coefficient = value,
        "KF" => params.flicker_noise_coefficient = value,
        "AF" => params.flicker_noise_exponent = value,
        _ => {}
    }
}

fn parse_element(
    fields: &[String],
    models: &HashMap<String, ModelCard>,
) -> Result<Element, NetlistParseError> {
    let name = &fields[0];
    let prefix = element_prefix(name)?;

    match prefix {
        'R' => {
            require_fields(fields, 4, "resistor")?;
            Ok(Element::Resistor(Resistor::new(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
            )))
        }
        'C' => {
            require_min_fields(fields, 4, "capacitor")?;
            let params = parse_element_params(&fields[4..], "capacitor")?;
            if let Some(param_name) = params.keys().find(|name| name.as_str() != "IC") {
                return Err(NetlistParseError::new(format!(
                    "unsupported capacitor parameter {param_name:?}"
                )));
            }
            Ok(Element::Capacitor(Capacitor::with_initial_voltage(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
                *params.get("IC").unwrap_or(&0.0),
            )))
        }
        'L' => {
            require_min_fields(fields, 4, "inductor")?;
            let params = parse_element_params(&fields[4..], "inductor")?;
            if let Some(param_name) = params.keys().find(|name| name.as_str() != "IC") {
                return Err(NetlistParseError::new(format!(
                    "unsupported inductor parameter {param_name:?}"
                )));
            }
            Ok(Element::Inductor(Inductor::with_initial_current(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
                *params.get("IC").unwrap_or(&0.0),
            )))
        }
        'K' => {
            require_fields(fields, 4, "mutual inductor")?;
            Ok(Element::MutualInductor(MutualInductor::new(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
            )))
        }
        'T' => {
            require_min_fields(fields, 6, "transmission line")?;
            let params = parse_element_params(&fields[5..], "transmission line")?;
            if let Some(param_name) = params
                .keys()
                .find(|name| name.as_str() != "Z0" && name.as_str() != "TD")
            {
                return Err(NetlistParseError::new(format!(
                    "unsupported transmission line parameter {param_name:?}"
                )));
            }
            let characteristic_impedance = params.get("Z0").ok_or_else(|| {
                NetlistParseError::new(format!("{name}: transmission line requires Z0"))
            })?;
            let delay = params.get("TD").ok_or_else(|| {
                NetlistParseError::new(format!("{name}: transmission line requires TD"))
            })?;
            Ok(Element::TransmissionLine(TransmissionLine::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                *characteristic_impedance,
                *delay,
            )))
        }
        'V' => {
            require_min_fields(fields, 4, "voltage source")?;
            let (voltage, waveform, ac) = parse_source_value(&fields[3..])?;
            let mut source = match waveform {
                Some(waveform) => {
                    VoltageSource::with_waveform(name, &fields[1], &fields[2], voltage, waveform)
                }
                None => VoltageSource::new(name, &fields[1], &fields[2], voltage),
            };
            source.ac = ac;
            Ok(Element::VoltageSource(source))
        }
        'I' => {
            require_min_fields(fields, 4, "current source")?;
            let (current, waveform, ac) = parse_source_value(&fields[3..])?;
            let mut source = match waveform {
                Some(waveform) => {
                    CurrentSource::with_waveform(name, &fields[1], &fields[2], current, waveform)
                }
                None => CurrentSource::new(name, &fields[1], &fields[2], current),
            };
            source.ac = ac;
            Ok(Element::CurrentSource(source))
        }
        'D' => {
            require_fields(fields, 4, "diode")?;
            let model = models.get(&fields[3].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!(
                    "unknown model {:?} for diode {:?}",
                    fields[3], name
                ))
            })?;
            if model.kind != "D" {
                return Err(NetlistParseError::new(format!(
                    "model {:?} has kind {:?}, expected \"D\"",
                    model.name, model.kind
                )));
            }
            Ok(Element::Diode(Diode::with_model_and_breakdown(
                name,
                &fields[1],
                &fields[2],
                *model.params.get("IS").unwrap_or(&1.0e-15),
                *model.params.get("VT").unwrap_or(&0.02585),
                *model.params.get("N").unwrap_or(&1.0),
                model.params.get("BV").copied(),
                *model.params.get("IBV").unwrap_or(&1.0e-3),
                model
                    .params
                    .get("CJO")
                    .or_else(|| model.params.get("CJ0"))
                    .copied()
                    .unwrap_or(0.0),
                *model.params.get("TT").unwrap_or(&0.0),
            )))
        }
        'Q' => {
            require_fields(fields, 5, "BJT")?;
            let model = models.get(&fields[4].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!("unknown model {:?} for BJT {:?}", fields[4], name))
            })?;
            let polarity = match model.kind.as_str() {
                "NPN" => BjtPolarity::Npn,
                "PNP" => BjtPolarity::Pnp,
                _ => {
                    return Err(NetlistParseError::new(format!(
                        "model {:?} has kind {:?}, expected \"NPN\" or \"PNP\"",
                        model.name, model.kind
                    )));
                }
            };
            let forward_beta = model
                .params
                .get("BF")
                .or_else(|| model.params.get("BETA_F"))
                .copied()
                .unwrap_or(100.0);
            Ok(Element::Bjt(Bjt::with_model(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                polarity,
                *model.params.get("IS").unwrap_or(&1.0e-14),
                forward_beta,
                *model.params.get("VT").unwrap_or(&0.02585),
                *model
                    .params
                    .get("CJE")
                    .or_else(|| model.params.get("CBE"))
                    .unwrap_or(&0.0),
                *model
                    .params
                    .get("CJC")
                    .or_else(|| model.params.get("CBC"))
                    .unwrap_or(&0.0),
                *model.params.get("TF").unwrap_or(&0.0),
                *model.params.get("TR").unwrap_or(&0.0),
            )))
        }
        'J' => {
            require_fields(fields, 5, "JFET")?;
            let model = models.get(&fields[4].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!("unknown model {:?} for JFET {:?}", fields[4], name))
            })?;
            let polarity = match model.kind.as_str() {
                "NJF" => JfetPolarity::Njf,
                "PJF" => JfetPolarity::Pjf,
                _ => {
                    return Err(NetlistParseError::new(format!(
                        "model {:?} has kind {:?}, expected \"NJF\" or \"PJF\"",
                        model.name, model.kind
                    )));
                }
            };
            let beta = model
                .params
                .get("BETA")
                .or_else(|| model.params.get("B"))
                .copied()
                .unwrap_or(1.0e-4);
            let threshold_voltage = model.params.get("VTO").copied().unwrap_or(match polarity {
                JfetPolarity::Njf => -2.0,
                JfetPolarity::Pjf => 2.0,
            });
            Ok(Element::Jfet(Jfet::with_model(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                polarity,
                beta,
                threshold_voltage,
                *model.params.get("LAMBDA").unwrap_or(&0.0),
            )))
        }
        'M' => {
            require_min_fields(fields, 6, "MOSFET")?;
            let model = models.get(&fields[5].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!(
                    "unknown model {:?} for MOSFET {:?}",
                    fields[5], name
                ))
            })?;
            let mosfet_type = match model.kind.as_str() {
                "NMOS" => MosfetType::Nmos,
                "PMOS" => MosfetType::Pmos,
                _ => {
                    return Err(NetlistParseError::new(format!(
                        "model {:?} has kind {:?}, expected \"NMOS\" or \"PMOS\"",
                        model.name, model.kind
                    )));
                }
            };
            let instance_params = parse_element_params(&fields[6..], "MOSFET")?;
            if let Some(param_name) = instance_params.keys().find(|name| {
                !matches!(
                    name.as_str(),
                    "W" | "L" | "NRD" | "NRS" | "AD" | "AS" | "PD" | "PS"
                )
            }) {
                return Err(NetlistParseError::new(format!(
                    "unsupported MOSFET parameter {param_name:?}"
                )));
            }
            if let Some(width) = instance_params.get("W") {
                if !width.is_finite() || *width <= 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET W must be finite and positive",
                    ));
                }
            }
            if let Some(length) = instance_params.get("L") {
                if !length.is_finite() || *length <= 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET L must be finite and positive",
                    ));
                }
            }
            if let Some(drain_squares) = instance_params.get("NRD") {
                if !drain_squares.is_finite() || *drain_squares < 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET NRD must be finite and non-negative",
                    ));
                }
            }
            if let Some(source_squares) = instance_params.get("NRS") {
                if !source_squares.is_finite() || *source_squares < 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET NRS must be finite and non-negative",
                    ));
                }
            }
            if let Some(drain_area) = instance_params.get("AD") {
                if !drain_area.is_finite() || *drain_area < 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET AD must be finite and non-negative",
                    ));
                }
            }
            if let Some(source_area) = instance_params.get("AS") {
                if !source_area.is_finite() || *source_area < 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET AS must be finite and non-negative",
                    ));
                }
            }
            if let Some(drain_perimeter) = instance_params.get("PD") {
                if !drain_perimeter.is_finite() || *drain_perimeter < 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET PD must be finite and non-negative",
                    ));
                }
            }
            if let Some(source_perimeter) = instance_params.get("PS") {
                if !source_perimeter.is_finite() || *source_perimeter < 0.0 {
                    return Err(NetlistParseError::new(
                        "MOSFET PS must be finite and non-negative",
                    ));
                }
            }
            Ok(Element::Mosfet(Mosfet::with_model(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                mosfet_type,
                build_mosfet_params(model, &instance_params),
            )))
        }
        'G' => {
            require_fields(fields, 6, "VCCS")?;
            Ok(Element::Vccs(Vccs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                parse_value(&fields[5])?,
            )))
        }
        'E' => {
            require_fields(fields, 6, "VCVS")?;
            Ok(Element::Vcvs(Vcvs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                parse_value(&fields[5])?,
            )))
        }
        'F' => {
            require_fields(fields, 5, "CCCS")?;
            Ok(Element::Cccs(Cccs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                parse_value(&fields[4])?,
            )))
        }
        'H' => {
            require_fields(fields, 5, "CCVS")?;
            Ok(Element::Ccvs(Ccvs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                parse_value(&fields[4])?,
            )))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported element {name:?}"
        ))),
    }
}

fn start_subckt(
    fields: &[String],
    line_number: usize,
    subckts: &HashMap<String, SubcktDefinition>,
) -> Result<SubcktDefinition, NetlistParseError> {
    require_min_fields(fields, 3, ".subckt")?;
    let name = fields[1].clone();
    if subckts.contains_key(&name.to_ascii_lowercase()) {
        return Err(NetlistParseError::new(format!(
            "duplicate .subckt definition {name:?}"
        )));
    }
    Ok(SubcktDefinition {
        name,
        pins: fields[2..].to_vec(),
        body: Vec::new(),
        line_number,
    })
}

fn finish_subckt(
    definition: &SubcktDefinition,
    fields: &[String],
) -> Result<(), NetlistParseError> {
    if fields.len() > 2 {
        return Err(NetlistParseError::new(
            ".ends expects at most a subcircuit name",
        ));
    }
    if fields.len() == 2 && !fields[1].eq_ignore_ascii_case(&definition.name) {
        return Err(NetlistParseError::new(format!(
            ".ends {:?} does not match .subckt {:?}",
            fields[1], definition.name
        )));
    }
    Ok(())
}

fn expand_subckt_instance(
    fields: &[String],
    subckts: &HashMap<String, SubcktDefinition>,
    stack: &[String],
    models: &HashMap<String, ModelCard>,
) -> Result<Vec<Element>, NetlistParseError> {
    require_min_fields(fields, 3, "subcircuit instance")?;
    let instance_name = &fields[0];
    let subckt_name = fields.last().expect("minimum fields checked");
    let definition = subckts
        .get(&subckt_name.to_ascii_lowercase())
        .ok_or_else(|| NetlistParseError::new(format!("unknown subcircuit {subckt_name:?}")))?;
    let definition_key = definition.name.to_ascii_lowercase();
    if stack.contains(&definition_key) {
        let mut cycle = stack.to_vec();
        cycle.push(definition_key);
        return Err(NetlistParseError::new(format!(
            "recursive subcircuit expansion is not supported: {}",
            cycle.join(" -> ")
        )));
    }

    let actual_nodes = &fields[1..fields.len() - 1];
    if actual_nodes.len() != definition.pins.len() {
        return Err(NetlistParseError::new(format!(
            "subcircuit {:?} expects {} pins, got {}",
            definition.name,
            definition.pins.len(),
            actual_nodes.len()
        )));
    }

    let mut node_map = HashMap::new();
    for (pin, actual) in definition.pins.iter().zip(actual_nodes.iter()) {
        node_map.insert(pin.clone(), actual.clone());
        node_map.insert(pin.to_ascii_lowercase(), actual.clone());
    }

    let mut elements = Vec::new();
    let mut next_stack = stack.to_vec();
    next_stack.push(definition.name.to_ascii_lowercase());
    for statement in &definition.body {
        if statement.fields[0].starts_with('.') {
            return Err(NetlistParseError::new(format!(
                "line {}: directives inside .subckt are not supported",
                statement.line_number
            )));
        }
        let local_fields = map_subckt_fields(&statement.fields, instance_name, &node_map)?;
        if element_prefix(&statement.fields[0])? == 'X' {
            elements.extend(expand_subckt_instance(
                &local_fields,
                subckts,
                &next_stack,
                models,
            )?);
        } else {
            elements.push(parse_element(&local_fields, models)?);
        }
    }
    Ok(elements)
}

fn map_subckt_fields(
    fields: &[String],
    instance_name: &str,
    node_map: &HashMap<String, String>,
) -> Result<Vec<String>, NetlistParseError> {
    let mut mapped = Vec::with_capacity(fields.len());
    mapped.push(format!("{instance_name}.{}", fields[0]));
    mapped.extend(fields[1..].iter().cloned());
    let prefix = fields[0]
        .chars()
        .next()
        .ok_or_else(|| NetlistParseError::new("element name is empty"))?
        .to_ascii_uppercase();
    match prefix {
        'R' | 'C' | 'L' | 'V' | 'I' | 'D' => {
            require_min_fields(fields, 3, "subcircuit element")?;
            mapped[1] = map_subckt_node(&fields[1], instance_name, node_map);
            mapped[2] = map_subckt_node(&fields[2], instance_name, node_map);
        }
        'Q' | 'J' => {
            require_min_fields(
                fields,
                4,
                if prefix == 'Q' {
                    "subcircuit BJT"
                } else {
                    "subcircuit JFET"
                },
            )?;
            for index in 1..4 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'M' => {
            require_min_fields(fields, 5, "subcircuit MOSFET")?;
            for index in 1..5 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'E' | 'G' => {
            require_min_fields(fields, 5, "subcircuit controlled source")?;
            for index in 1..5 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'F' | 'H' => {
            require_min_fields(fields, 4, "subcircuit current-controlled source")?;
            mapped[1] = map_subckt_node(&fields[1], instance_name, node_map);
            mapped[2] = map_subckt_node(&fields[2], instance_name, node_map);
            mapped[3] = map_subckt_source_ref(&fields[3], instance_name);
        }
        'K' => {
            require_fields(fields, 4, "subcircuit mutual inductor")?;
            mapped[1] = map_subckt_source_ref(&fields[1], instance_name);
            mapped[2] = map_subckt_source_ref(&fields[2], instance_name);
        }
        'T' => {
            require_min_fields(fields, 6, "subcircuit transmission line")?;
            for index in 1..5 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'X' => {
            for index in 1..fields.len() - 1 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        _ => {}
    }
    Ok(mapped)
}

fn map_subckt_node(node: &str, instance_name: &str, node_map: &HashMap<String, String>) -> String {
    if node.eq_ignore_ascii_case("0") || node.eq_ignore_ascii_case("gnd") {
        return node.to_string();
    }
    node_map
        .get(node)
        .or_else(|| node_map.get(&node.to_ascii_lowercase()))
        .cloned()
        .unwrap_or_else(|| format!("{instance_name}.{node}"))
}

fn map_subckt_source_ref(source_name: &str, instance_name: &str) -> String {
    if source_name.contains('.') {
        source_name.to_string()
    } else {
        format!("{instance_name}.{source_name}")
    }
}

fn element_prefix(name: &str) -> Result<char, NetlistParseError> {
    name.rsplit('.')
        .next()
        .and_then(|local_name| local_name.chars().next())
        .map(|ch| ch.to_ascii_uppercase())
        .ok_or_else(|| NetlistParseError::new("element name is empty"))
}

fn parse_source_value(
    fields: &[String],
) -> Result<(f64, Option<Waveform>, Option<spice_engine::AcSource>), NetlistParseError> {
    if fields.is_empty() {
        return Err(NetlistParseError::new("source is missing a value"));
    }
    let ac_index = fields
        .iter()
        .position(|field| field.eq_ignore_ascii_case("AC"));
    if let Some(ac_index) = ac_index {
        let (value_fields, ac_fields_with_marker) = fields.split_at(ac_index);
        let ac_fields = &ac_fields_with_marker[1..];
        if ac_fields.is_empty() {
            return Err(NetlistParseError::new(
                "AC source form requires a magnitude",
            ));
        }
        if ac_fields.len() > 2 {
            return Err(NetlistParseError::new(
                "AC source form accepts magnitude and optional phase",
            ));
        }
        let (value, waveform) = if value_fields.is_empty() {
            (0.0, None)
        } else {
            parse_source_dc_value(value_fields)?
        };
        let magnitude = parse_value(&ac_fields[0])?;
        let phase_degrees = if ac_fields.len() == 2 {
            parse_value(&ac_fields[1])?
        } else {
            0.0
        };
        return Ok((
            value,
            waveform,
            Some(spice_engine::AcSource::new(magnitude, phase_degrees)),
        ));
    }
    let (value, waveform) = parse_source_dc_value(fields)?;
    Ok((value, waveform, None))
}

fn parse_source_dc_value(fields: &[String]) -> Result<(f64, Option<Waveform>), NetlistParseError> {
    if fields[0].eq_ignore_ascii_case("DC") {
        if fields.len() < 2 {
            return Err(NetlistParseError::new("DC source form requires a value"));
        }
        if fields.len() > 2 {
            return Err(NetlistParseError::new("DC source form accepts one value"));
        }
        return Ok((parse_value(&fields[1])?, None));
    }
    if fields.len() == 1 && fields[0].contains('(') {
        let waveform = parse_waveform(&fields[0])?;
        return Ok((waveform.value_at(0.0), Some(waveform)));
    }
    if starts_with_waveform(&fields[0]) {
        let waveform = parse_waveform(&fields.join(" "))?;
        return Ok((waveform.value_at(0.0), Some(waveform)));
    }
    Ok((parse_value(&fields[0])?, None))
}

fn parse_waveform(token: &str) -> Result<Waveform, NetlistParseError> {
    let trimmed = token.trim();
    let open = trimmed
        .find('(')
        .ok_or_else(|| NetlistParseError::new(format!("invalid source waveform {token:?}")))?;
    if !trimmed.ends_with(')') {
        return Err(NetlistParseError::new(format!(
            "invalid source waveform {token:?}"
        )));
    }
    let kind = trimmed[..open].to_ascii_uppercase();
    let inner = &trimmed[open + 1..trimmed.len() - 1];
    let values = parse_waveform_values(inner)?;

    match kind.as_str() {
        "PWL" => {
            if values.len() < 4 || values.len() % 2 != 0 {
                return Err(NetlistParseError::new("PWL requires time/value pairs"));
            }
            let points = values
                .chunks_exact(2)
                .map(|pair| (pair[0], pair[1]))
                .collect::<Vec<_>>();
            Ok(Waveform::Pwl(PwlWaveform::new(points)))
        }
        "SIN" => {
            let padded = pad(&values, 5, 0.0);
            Ok(Waveform::Sin(SinWaveform::with_delay_damping(
                padded[0],
                if values.len() >= 2 { padded[1] } else { 1.0 },
                if values.len() >= 3 { padded[2] } else { 1.0 },
                padded[3],
                padded[4],
            )))
        }
        "PULSE" => {
            let padded = pad(&values, 7, 0.0);
            Ok(Waveform::Pulse(PulseWaveform::new(
                padded[0],
                if values.len() >= 2 { padded[1] } else { 1.0 },
                padded[2],
                padded[3],
                padded[4],
                if values.len() >= 6 { padded[5] } else { 0.5 },
                if values.len() >= 7 { padded[6] } else { 1.0 },
            )))
        }
        "EXP" => {
            let padded = pad(&values, 6, 0.0);
            Ok(Waveform::Exp(ExpWaveform::new(
                padded[0],
                if values.len() >= 2 { padded[1] } else { 1.0 },
                padded[2],
                if values.len() >= 4 { padded[3] } else { 1.0 },
                if values.len() >= 5 { padded[4] } else { 1.0 },
                if values.len() >= 6 { padded[5] } else { 1.0 },
            )))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported source waveform {kind:?}"
        ))),
    }
}

fn parse_directive(fields: &[String]) -> Result<Analysis, NetlistParseError> {
    match fields[0].to_ascii_lowercase().as_str() {
        ".op" => {
            require_fields(fields, 1, ".op")?;
            Ok(Analysis::Op(OpAnalysis))
        }
        ".tran" => {
            require_min_fields(fields, 3, ".tran")?;
            Ok(Analysis::Tran(TranAnalysis {
                time_step: parse_value(&fields[1])?,
                stop_time: parse_value(&fields[2])?,
                method: parse_tran_method_options(&fields[3..])?,
            }))
        }
        ".dc" => {
            require_fields(fields, 5, ".dc")?;
            Ok(Analysis::Dc(DcAnalysis {
                source_name: fields[1].clone(),
                start: parse_value(&fields[2])?,
                stop: parse_value(&fields[3])?,
                step: parse_value(&fields[4])?,
            }))
        }
        ".ac" => {
            require_fields(fields, 5, ".ac")?;
            Ok(Analysis::Ac(AcAnalysis {
                mode: fields[1].to_ascii_lowercase(),
                points: parse_value(&fields[2])? as usize,
                start_hz: parse_value(&fields[3])?,
                stop_hz: parse_value(&fields[4])?,
            }))
        }
        ".tf" => {
            require_fields(fields, 3, ".tf")?;
            Ok(Analysis::Tf(TfAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".tf")?,
                input_source: fields[2].clone(),
            }))
        }
        ".sens" => {
            require_fields(fields, 2, ".sens")?;
            Ok(Analysis::Sens(SensAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".sens")?,
            }))
        }
        ".mc" => {
            require_min_fields(fields, 3, ".mc")?;
            require_max_fields(fields, 6, ".mc")?;
            let distribution = fields
                .get(4)
                .map(|field| field.to_ascii_lowercase())
                .unwrap_or_else(|| "gaussian".to_string());
            if distribution != "gaussian" && distribution != "uniform" {
                return Err(NetlistParseError::new(format!(
                    ".mc distribution must be \"gaussian\" or \"uniform\", got {:?}",
                    fields[4]
                )));
            }
            Ok(Analysis::Mc(McAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".mc")?,
                n_trials: parse_value(&fields[2])? as usize,
                tolerance: if fields.len() >= 4 {
                    parse_value(&fields[3])?
                } else {
                    0.05
                },
                distribution,
                seed: if fields.len() >= 6 {
                    Some(parse_value(&fields[5])? as u64)
                } else {
                    None
                },
            }))
        }
        ".noise" => {
            require_min_fields(fields, 3, ".noise")?;
            let mut frequencies_hz = Vec::new();
            let mut temperature = 300.0;
            let mut temperature_is_explicit = false;
            let mut tail_index = 3;
            while tail_index < fields.len() {
                let token = &fields[tail_index];
                let lower_token = token.to_ascii_lowercase();
                if lower_token == "temp" {
                    if tail_index + 1 >= fields.len() {
                        return Err(NetlistParseError::new(
                            ".noise temp requires a temperature value",
                        ));
                    }
                    temperature = parse_value(&fields[tail_index + 1])?;
                    temperature_is_explicit = true;
                    tail_index += 2;
                } else if let Some(value) = lower_token.strip_prefix("temp=") {
                    temperature = parse_value(value)?;
                    temperature_is_explicit = true;
                    tail_index += 1;
                } else {
                    frequencies_hz.push(parse_value(token)?);
                    tail_index += 1;
                }
            }
            Ok(Analysis::Noise(NoiseAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".noise")?,
                input_source: fields[2].clone(),
                frequencies_hz,
                temperature,
                temperature_is_explicit,
            }))
        }
        ".temp" => {
            require_min_fields(fields, 2, ".temp")?;
            let temperatures_celsius = fields[1..]
                .iter()
                .map(|field| parse_value(field))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Analysis::Temp(TempAnalysis {
                temperatures_celsius,
            }))
        }
        ".print" => {
            require_min_fields(fields, 3, ".print")?;
            Ok(Analysis::Print(PrintAnalysis {
                analysis: fields[1].to_ascii_lowercase(),
                probes: parse_output_probes(&fields[2..], ".print")?,
            }))
        }
        ".plot" => {
            require_min_fields(fields, 3, ".plot")?;
            Ok(Analysis::Plot(PlotAnalysis {
                analysis: fields[1].to_ascii_lowercase(),
                probes: parse_output_probes(&fields[2..], ".plot")?,
            }))
        }
        ".save" => {
            require_min_fields(fields, 2, ".save")?;
            Ok(Analysis::Save(SaveAnalysis {
                probes: parse_output_probes(&fields[1..], ".save")?,
            }))
        }
        ".probe" => Ok(Analysis::Probe(parse_probe_card(fields)?)),
        ".measure" | ".meas" => Ok(Analysis::Measure(parse_measure_card(fields)?)),
        ".four" => {
            require_min_fields(fields, 3, ".four")?;
            Ok(Analysis::Four(FourAnalysis {
                frequency_hz: parse_value(&fields[1])?,
                probes: parse_output_probes(&fields[2..], ".four")?,
            }))
        }
        ".disto" => {
            require_min_fields(fields, 6, ".disto")?;
            Ok(Analysis::Distortion(DistortionAnalysis {
                mode: fields[1].to_ascii_lowercase(),
                points: parse_value(&fields[2])? as usize,
                start_hz: parse_value(&fields[3])?,
                stop_hz: parse_value(&fields[4])?,
                probes: parse_output_probes(&fields[5..], ".disto")?,
            }))
        }
        ".pz" => {
            require_min_fields(fields, 3, ".pz")?;
            require_max_fields(fields, 4, ".pz")?;
            let kind = if let Some(raw_kind) = fields.get(3) {
                parse_pole_zero_kind(raw_kind)?
            } else {
                PoleZeroKind::PoleZero
            };
            Ok(Analysis::PoleZero(PoleZeroAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".pz")?,
                input_source: fields[2].clone(),
                kind,
            }))
        }
        ".options" => {
            require_min_fields(fields, 2, ".options")?;
            Ok(Analysis::Options(OptionsAnalysis {
                values: parse_options(&fields[1..])?,
            }))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported directive {:?}",
            fields[0]
        ))),
    }
}

fn parse_probe_card(fields: &[String]) -> Result<ProbeAnalysis, NetlistParseError> {
    require_min_fields(fields, 2, ".probe")?;
    let (analysis, probe_tokens) = if fields.len() >= 3 && is_analysis_selector(&fields[1]) {
        (Some(fields[1].to_ascii_lowercase()), &fields[2..])
    } else {
        (None, &fields[1..])
    };
    Ok(ProbeAnalysis {
        analysis,
        probes: parse_output_probes(probe_tokens, ".probe")?,
    })
}

fn parse_measure_card(fields: &[String]) -> Result<MeasureAnalysis, NetlistParseError> {
    let directive = fields[0].to_ascii_lowercase();
    require_min_fields(fields, 5, &directive)?;
    let operation = parse_measure_operation(&fields[3], &directive)?;
    let options = parse_measure_options(&fields[5..], &directive)?;
    let analysis = fields[1].to_ascii_lowercase();
    if operation == MeasureOperation::Find
        && !options.contains_key("at")
        && analysis != "op"
        && analysis != "dcop"
    {
        return Err(NetlistParseError::new(format!(
            "{directive} FIND requires AT=<value>"
        )));
    }
    if operation != MeasureOperation::Find && options.contains_key("at") {
        return Err(NetlistParseError::new(format!(
            "{directive} {} does not support AT=<value>",
            measure_operation_name(operation).to_ascii_uppercase()
        )));
    }
    Ok(MeasureAnalysis {
        analysis,
        name: fields[2].clone(),
        operation,
        probe: parse_output_probe(&fields[4], &directive)?,
        at: options.get("at").copied(),
        start: options.get("from").copied(),
        stop: options.get("to").copied(),
    })
}

fn parse_measure_operation(
    token: &str,
    directive: &str,
) -> Result<MeasureOperation, NetlistParseError> {
    match token.to_ascii_lowercase().as_str() {
        "find" => Ok(MeasureOperation::Find),
        "max" => Ok(MeasureOperation::Max),
        "min" => Ok(MeasureOperation::Min),
        "avg" => Ok(MeasureOperation::Avg),
        "rms" => Ok(MeasureOperation::Rms),
        _ => Err(NetlistParseError::new(format!(
            "{directive} operation must be FIND, MAX, MIN, AVG, or RMS, got {token:?}"
        ))),
    }
}

fn measure_operation_name(operation: MeasureOperation) -> &'static str {
    match operation {
        MeasureOperation::Find => "find",
        MeasureOperation::Max => "max",
        MeasureOperation::Min => "min",
        MeasureOperation::Avg => "avg",
        MeasureOperation::Rms => "rms",
    }
}

fn parse_measure_options(
    tokens: &[String],
    directive: &str,
) -> Result<HashMap<String, f64>, NetlistParseError> {
    let mut options = HashMap::new();
    for token in tokens {
        let Some((raw_key, raw_value)) = token.split_once('=') else {
            return Err(NetlistParseError::new(format!(
                "{directive} option must be KEY=value, got {token:?}"
            )));
        };
        let key = raw_key.trim().to_ascii_lowercase();
        if !matches!(key.as_str(), "at" | "from" | "to") {
            return Err(NetlistParseError::new(format!(
                "{directive} unsupported option {key:?}"
            )));
        }
        if options.contains_key(&key) {
            return Err(NetlistParseError::new(format!(
                "{directive} duplicate option {key:?}"
            )));
        }
        if raw_value.is_empty() {
            return Err(NetlistParseError::new(format!(
                "{directive} option {key:?} requires a value"
            )));
        }
        options.insert(key, parse_value(raw_value)?);
    }
    Ok(options)
}

fn is_analysis_selector(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "op" | "dcop" | "dc" | "ac" | "tran" | "transient"
    )
}

fn parse_pole_zero_kind(raw_kind: &str) -> Result<PoleZeroKind, NetlistParseError> {
    match raw_kind.to_ascii_lowercase().as_str() {
        "pole" => Ok(PoleZeroKind::Pole),
        "zero" => Ok(PoleZeroKind::Zero),
        "pz" => Ok(PoleZeroKind::PoleZero),
        _ => Err(NetlistParseError::new(format!(
            ".pz kind must be \"pole\", \"zero\", or \"pz\", got {raw_kind:?}"
        ))),
    }
}

fn parse_options(tokens: &[String]) -> Result<HashMap<String, OptionValue>, NetlistParseError> {
    let mut values = HashMap::new();
    for token in tokens {
        if let Some((raw_key, raw_value)) = token.split_once('=') {
            let key = raw_key.trim().to_ascii_lowercase();
            if key.is_empty() {
                return Err(NetlistParseError::new(format!(
                    ".options contains empty option name in {token:?}"
                )));
            }
            if raw_value.is_empty() {
                return Err(NetlistParseError::new(format!(
                    ".options {key:?} requires a value"
                )));
            }
            let value = if key == "method" {
                let method = parse_transient_method(raw_value, ".options method")?;
                OptionValue::Text(transient_method_name(method).to_string())
            } else {
                parse_option_value(raw_value)
            };
            values.insert(key, value);
        } else {
            let key = token.trim().to_ascii_lowercase();
            if key.is_empty() {
                return Err(NetlistParseError::new(".options contains an empty flag"));
            }
            values.insert(key, OptionValue::Flag(true));
        }
    }
    Ok(values)
}

fn parse_tran_method_options(
    tokens: &[String],
) -> Result<Option<TransientMethod>, NetlistParseError> {
    let mut method = None;
    for token in tokens {
        let Some((raw_key, raw_value)) = token.split_once('=') else {
            return Err(NetlistParseError::new(format!(
                ".tran unsupported trailing option {token:?}; use method=<euler|trap|gear2>"
            )));
        };
        let key = raw_key.trim().to_ascii_lowercase();
        if key != "method" {
            return Err(NetlistParseError::new(format!(
                ".tran unsupported option {key:?}"
            )));
        }
        if raw_value.is_empty() {
            return Err(NetlistParseError::new(".tran method requires a value"));
        }
        method = Some(parse_transient_method(raw_value, ".tran method")?);
    }
    Ok(method)
}

fn parse_transient_method(
    raw_value: &str,
    context: &str,
) -> Result<TransientMethod, NetlistParseError> {
    match raw_value.trim().to_ascii_lowercase().as_str() {
        "euler" => Ok(TransientMethod::Euler),
        "trap" => Ok(TransientMethod::Trap),
        "gear2" => Ok(TransientMethod::Gear2),
        _ => Err(NetlistParseError::new(format!(
            "{context} must be euler, trap, or gear2, got {raw_value:?}"
        ))),
    }
}

fn transient_method_name(method: TransientMethod) -> &'static str {
    match method {
        TransientMethod::Euler => "euler",
        TransientMethod::Trap => "trap",
        TransientMethod::Gear2 => "gear2",
    }
}

fn parse_option_value(raw_value: &str) -> OptionValue {
    match parse_value(raw_value) {
        Ok(value) => OptionValue::Number(value),
        Err(_) => OptionValue::Text(raw_value.to_string()),
    }
}

fn parse_voltage_probe(token: &str, directive: &str) -> Result<String, NetlistParseError> {
    let lower = token.to_ascii_lowercase();
    if !lower.starts_with("v(") || !token.ends_with(')') {
        return Err(NetlistParseError::new(format!(
            "{directive} output must be a voltage probe V(node), got {token:?}"
        )));
    }
    let node = &token[2..token.len() - 1];
    if node.is_empty()
        || node.contains('(')
        || node.contains(')')
        || node.chars().any(char::is_whitespace)
    {
        return Err(NetlistParseError::new(format!(
            "{directive} output must be a voltage probe V(node), got {token:?}"
        )));
    }
    Ok(node.to_string())
}

fn parse_output_probes(
    tokens: &[String],
    directive: &str,
) -> Result<Vec<OutputProbe>, NetlistParseError> {
    tokens
        .iter()
        .map(|token| parse_output_probe(token, directive))
        .collect()
}

fn parse_output_probe(token: &str, directive: &str) -> Result<OutputProbe, NetlistParseError> {
    let lower = token.to_ascii_lowercase();
    let (kind, prefix_len) = if lower.starts_with("v(") {
        ("voltage", 2)
    } else if lower.starts_with("i(") {
        ("current", 2)
    } else {
        return Err(output_probe_error(token, directive));
    };
    if !token.ends_with(')') {
        return Err(output_probe_error(token, directive));
    }
    let target = &token[prefix_len..token.len() - 1];
    if target.is_empty()
        || target.contains('(')
        || target.contains(')')
        || target.chars().any(char::is_whitespace)
    {
        return Err(output_probe_error(token, directive));
    }
    match kind {
        "voltage" => Ok(OutputProbe::Voltage {
            node: target.to_string(),
        }),
        _ => Ok(OutputProbe::Current {
            source_name: target.to_string(),
        }),
    }
}

fn output_probe_error(token: &str, directive: &str) -> NetlistParseError {
    NetlistParseError::new(format!(
        "{directive} probe must be V(node) or I(source), got {token:?}"
    ))
}

fn split_fields(line: &str) -> Result<Vec<String>, NetlistParseError> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut depth = 0_i32;

    for ch in line.chars() {
        if ch.is_whitespace() && depth == 0 {
            if !current.is_empty() {
                fields.push(std::mem::take(&mut current));
            }
            continue;
        }
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(NetlistParseError::new("unmatched closing parenthesis"));
                }
            }
            _ => {}
        }
        current.push(ch);
    }

    if depth != 0 {
        return Err(NetlistParseError::new("unclosed parenthesis"));
    }
    if !current.is_empty() {
        fields.push(current);
    }
    Ok(fields)
}

fn parse_waveform_values(inner: &str) -> Result<Vec<f64>, NetlistParseError> {
    inner
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(parse_value)
        .collect()
}

fn starts_with_waveform(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    ["PWL(", "SIN(", "PULSE(", "EXP("]
        .iter()
        .any(|prefix| upper.starts_with(prefix))
}

fn line_error(line_number: usize, error: NetlistParseError) -> NetlistParseError {
    NetlistParseError::new(format!("line {line_number}: {error}"))
}

fn require_fields(fields: &[String], count: usize, label: &str) -> Result<(), NetlistParseError> {
    if fields.len() != count {
        return Err(NetlistParseError::new(format!(
            "{label} expects {count} fields, got {}",
            fields.len()
        )));
    }
    Ok(())
}

fn require_min_fields(
    fields: &[String],
    count: usize,
    label: &str,
) -> Result<(), NetlistParseError> {
    if fields.len() < count {
        return Err(NetlistParseError::new(format!(
            "{label} expects at least {count} fields, got {}",
            fields.len()
        )));
    }
    Ok(())
}

fn require_max_fields(
    fields: &[String],
    count: usize,
    label: &str,
) -> Result<(), NetlistParseError> {
    if fields.len() > count {
        return Err(NetlistParseError::new(format!(
            "{label} expects at most {count} fields, got {}",
            fields.len()
        )));
    }
    Ok(())
}

fn pad(values: &[f64], count: usize, default_value: f64) -> Vec<f64> {
    let mut padded = values.to_vec();
    padded.resize(count, default_value);
    padded
}

fn is_supported_suffix(suffix: &str) -> bool {
    matches!(
        suffix,
        "t" | "g" | "meg" | "k" | "" | "m" | "u" | "n" | "p" | "f"
    )
}

fn suffix_multiplier(suffix: &str) -> f64 {
    match suffix {
        "t" => 1.0e12,
        "g" => 1.0e9,
        "meg" => 1.0e6,
        "k" => 1.0e3,
        "" => 1.0,
        "m" => 1.0e-3,
        "u" => 1.0e-6,
        "n" => 1.0e-9,
        "p" => 1.0e-12,
        "f" => 1.0e-15,
        _ => unreachable!("suffix support checked before multiplier lookup"),
    }
}
