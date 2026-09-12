//! Host-neutral browser form-control interaction semantics.

use coding_adventures_html_parser::{BrowserDatalistOption, BrowserRenderNode, BrowserRenderTree};
use layout_controls::{ControlAppearance, ControlKind, ControlState};
use text_flow::{first_strong_direction, graphemes, Direction};
use url_parser::Url;

mod custom_elements;
mod typed_values;

use custom_elements::CustomElementInternalsRegistry;
pub use custom_elements::{
    CustomElementAccessibilityAction, CustomElementAccessibilityProjection,
    CustomElementAccessibilityState, CustomElementAccessibilityValue, CustomElementDiagnostic,
    CustomElementFormAssociation, CustomElementFormEntry, CustomElementFormEntryValue,
    CustomElementFormValue, CustomElementInternalsError, CustomElementLifecycleEvent,
    CustomElementRestorationEntry, CustomElementStateRestoreMode, CustomElementSubmissionGroup,
    CustomElementValidity, FormAssociatedCustomElementState, MAX_CUSTOM_ELEMENT_ENTRIES,
    MAX_CUSTOM_ELEMENT_TEXT_BYTES,
};
pub use typed_values::{
    format_typed_value, normalize_color, parse_typed_step, parse_typed_value, step_typed_value,
    typed_constraints, TypedValue, TypedValueConstraints,
};

const EDIT_HISTORY_LIMIT: usize = 100;
const FORM_STATE_CONTROL_LIMIT: usize = 512;
const FORM_STATE_BYTE_LIMIT: usize = 1024 * 1024;
pub const MAX_AUTOFILL_FIELDS: usize = 128;
pub const MAX_AUTOFILL_VALUE_BYTES: usize = 16 * 1024;
pub const MAX_DATALIST_OPTIONS: usize = 512;
pub const MAX_SUGGESTION_RESULTS: usize = 64;
pub const MAX_SUGGESTION_QUERY_BYTES: usize = 4096;
pub const MAX_SELECTED_FILES: usize = 256;
pub const MAX_SELECTED_FILE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SELECTED_TOTAL_BYTES: usize = 16 * 1024 * 1024;

pub const VERSION: &str = "0.1.0";

/// Deterministic editor geometry supplied by a host or shared text backend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlTextMetrics {
    pub advance: f64,
    pub line_height: f64,
    pub inset_x: f64,
    pub inset_y: f64,
    pub caret_width: f64,
}

impl Default for ControlTextMetrics {
    fn default() -> Self {
        Self {
            advance: 8.0,
            line_height: 18.0,
            inset_x: 8.0,
            inset_y: 5.0,
            caret_width: 1.5,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Backend-neutral retained overlay geometry for one editable control.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ControlEditorPresentation {
    pub key: String,
    pub viewport: ControlRect,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub selection: Vec<ControlRect>,
    pub caret: Option<ControlRect>,
    pub composition_underlines: Vec<ControlRect>,
    pub candidate_rect: Option<ControlRect>,
    pub invalid_message: Option<String>,
    pub accessible_description: Option<String>,
}

/// Clipboard payload plus the mutation caused by a successful cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlClipboardCut {
    pub text: String,
    pub effect: ControlEffect,
}

/// Clipboard flavors exchanged with a host. Plain text wins when both are
/// present; HTML-only payloads are reduced by the shared, bounded sanitizer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlClipboardPayload {
    pub plain_text: Option<String>,
    pub html: Option<String>,
}

impl ControlClipboardPayload {
    pub fn from_plain_text(text: impl Into<String>) -> Self {
        let plain_text = text.into();
        Self {
            html: Some(text_to_html_fragment(&plain_text)),
            plain_text: Some(plain_text),
        }
    }

    pub fn preferred_text(&self) -> Option<String> {
        self.plain_text
            .clone()
            .or_else(|| self.html.as_deref().map(html_fragment_to_text))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlNavigationUnit {
    Grapheme,
    Word,
    Line,
    Document,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlAccessibilityAction {
    Activate,
    Move {
        forward: bool,
        unit: ControlNavigationUnit,
        extend: bool,
    },
    SetSelection(ControlSelection),
    ReplaceSelection(String),
    SetValue(String),
    Increment,
    Decrement,
    Toggle,
    SetIndeterminate(bool),
    SelectOption {
        index: usize,
        extend: bool,
        toggle: bool,
    },
    SelectAll,
    ShowSuggestions(String),
    MoveSuggestion {
        forward: bool,
    },
    CommitSuggestion,
    DismissSuggestions,
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlMutationEventKind {
    Input,
    Change,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlMutationSource {
    Autofill,
    SuggestionPicker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlMutationEvent {
    pub key: String,
    pub kind: ControlMutationEventKind,
    pub source: ControlMutationSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlStatePrivacy {
    #[default]
    Public,
    Credentials,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlAutofillDescriptor {
    pub key: String,
    pub section: Option<String>,
    pub address_type: Option<String>,
    pub contact_type: Option<String>,
    pub purpose: String,
    pub enabled: bool,
    pub sensitive: bool,
    pub document_order: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlAutofillValue {
    pub section: Option<String>,
    pub purpose: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlAutofillTransaction {
    pub values: Vec<ControlAutofillValue>,
    pub privacy: ControlStatePrivacy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlStateDiagnostic {
    pub code: &'static str,
    pub key: Option<String>,
    pub message: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlAutofillOutcome {
    pub effects: Vec<ControlEffect>,
    pub events: Vec<ControlMutationEvent>,
    pub diagnostics: Vec<ControlStateDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlDefaultState {
    pub key: String,
    pub default_value: String,
    pub value: String,
    pub default_checked: bool,
    pub checked: bool,
    pub dirty_value: bool,
    pub dirty_checkedness: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlRestorationEntry {
    pub key: String,
    pub control_type: String,
    pub value: String,
    pub checked: bool,
    pub indeterminate: bool,
    pub selected_indices: Vec<usize>,
    pub selected_index: usize,
    pub dirty_value: bool,
    pub dirty_checkedness: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlStateSnapshot {
    pub controls: Vec<ControlRestorationEntry>,
    pub custom_elements: Vec<CustomElementRestorationEntry>,
    pub diagnostics: Vec<ControlStateDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlSuggestionOption {
    pub value: String,
    pub label: Option<String>,
    pub text: String,
    pub source_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlSuggestionDiagnostic {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlSuggestionState {
    pub key: String,
    pub open: bool,
    pub query: String,
    pub options: Vec<ControlSuggestionOption>,
    pub active_index: Option<usize>,
    pub diagnostics: Vec<ControlSuggestionDiagnostic>,
}

impl ControlSuggestionState {
    pub fn to_host_json(&self) -> String {
        let options = self
            .options
            .iter()
            .map(|option| {
                format!(
                    "{{\"value\":\"{}\",\"label\":{},\"text\":\"{}\",\"sourceIndex\":{}}}",
                    json_string(&option.value),
                    option
                        .label
                        .as_deref()
                        .map(|label| format!("\"{}\"", json_string(label)))
                        .unwrap_or_else(|| "null".to_string()),
                    json_string(&option.text),
                    option.source_index,
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let active_index = self
            .active_index
            .map(|index| index.to_string())
            .unwrap_or_else(|| "null".to_string());
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{{\"code\":\"{}\",\"message\":\"{}\"}}",
                    json_string(diagnostic.code),
                    json_string(diagnostic.message),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"key\":\"{}\",\"open\":{},\"query\":\"{}\",\"activeIndex\":{},\"options\":[{}],\"diagnostics\":[{}]}}",
            json_string(&self.key),
            self.open,
            json_string(&self.query),
            active_index,
            options,
            diagnostics,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlSuggestionPickerAction {
    MovePrevious,
    MoveNext,
    CommitActive,
    CommitIndex(usize),
    Cancel,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlTextDirection {
    #[default]
    Ltr,
    Rtl,
}

impl ControlTextDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlValueDiagnostic {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlValueState {
    pub key: String,
    pub value: String,
    pub input_mode: Option<String>,
    pub selection_supported: bool,
    pub numeric_value: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub step: Option<f64>,
    pub value_text: Option<String>,
    pub diagnostics: Vec<ControlValueDiagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveValueKind {
    Output,
    Meter,
    Progress,
}

impl LiveValueKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::Meter => "meter",
            Self::Progress => "progress",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeterValueRegion {
    Optimum,
    Suboptimal,
    EvenLessGood,
}

impl MeterValueRegion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Optimum => "optimum",
            Self::Suboptimal => "suboptimal",
            Self::EvenLessGood => "even-less-good",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDependencyValue {
    pub id: String,
    pub key: Option<String>,
    pub value: String,
}

/// Host-neutral state for HTML output, meter, and progress elements.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveValueState {
    pub key: String,
    pub id: Option<String>,
    pub kind: LiveValueKind,
    pub form_owner: Option<String>,
    pub form_index: Option<usize>,
    pub text: String,
    pub value: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub low: Option<f64>,
    pub high: Option<f64>,
    pub optimum: Option<f64>,
    pub position: Option<f64>,
    pub indeterminate: bool,
    pub meter_region: Option<MeterValueRegion>,
    pub dependencies: Vec<OutputDependencyValue>,
    pub accessible_name: Option<String>,
    pub accessible_description: Option<String>,
    pub value_text: String,
    pub diagnostics: Vec<ControlValueDiagnostic>,
}

impl LiveValueState {
    pub fn to_host_json(&self) -> String {
        let number = |value: Option<f64>| {
            value.map_or_else(|| "null".to_string(), |value| value.to_string())
        };
        let string = |value: Option<&str>| {
            value.map_or_else(
                || "null".to_string(),
                |value| format!("\"{}\"", json_string(value)),
            )
        };
        let dependencies = self
            .dependencies
            .iter()
            .map(|dependency| {
                format!(
                    "{{\"id\":\"{}\",\"key\":{},\"value\":\"{}\"}}",
                    json_string(&dependency.id),
                    dependency.key.as_ref().map_or_else(
                        || "null".to_string(),
                        |key| format!("\"{}\"", json_string(key))
                    ),
                    json_string(&dependency.value)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{{\"code\":\"{}\",\"message\":\"{}\"}}",
                    json_string(diagnostic.code),
                    json_string(diagnostic.message)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"key\":\"{}\",\"kind\":\"{}\",\"text\":\"{}\",\"value\":{},\"minimum\":{},\"maximum\":{},\"low\":{},\"high\":{},\"optimum\":{},\"position\":{},\"indeterminate\":{},\"meterRegion\":{},\"accessibleName\":{},\"accessibleDescription\":{},\"valueText\":\"{}\",\"dependencies\":[{}],\"diagnostics\":[{}]}}",
            json_string(&self.key),
            self.kind.as_str(),
            json_string(&self.text),
            number(self.value),
            number(self.minimum),
            number(self.maximum),
            number(self.low),
            number(self.high),
            number(self.optimum),
            number(self.position),
            self.indeterminate,
            self.meter_region.map_or_else(
                || "null".to_string(),
                |region| format!("\"{}\"", region.as_str())
            ),
            string(self.accessible_name.as_deref()),
            string(self.accessible_description.as_deref()),
            json_string(&self.value_text),
            dependencies,
            diagnostics
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlChoiceOptionState {
    pub index: usize,
    pub value: String,
    pub disabled: bool,
    pub selected: bool,
}

/// Reusable accessibility and interaction state for non-text controls.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlChoiceState {
    pub key: String,
    pub role: &'static str,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub indeterminate: bool,
    pub multiple: bool,
    pub active_index: Option<usize>,
    pub options: Vec<ControlChoiceOptionState>,
    pub value: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub step: Option<f64>,
    pub diagnostics: Vec<ControlValueDiagnostic>,
}

/// One path-free file selected by a host picker. The opaque identifier is
/// meaningful only to the host; shared form code owns the bounded bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFileSelection {
    pub opaque_id: String,
    pub name: String,
    pub media_type: Option<String>,
    pub bytes: Vec<u8>,
}

impl HostFileSelection {
    pub fn new(
        opaque_id: impl Into<String>,
        name: impl Into<String>,
        media_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            opaque_id: opaque_id.into(),
            name: sanitize_file_name(&name.into()),
            media_type: media_type.and_then(|value| normalize_media_type(&value)),
            bytes,
        }
    }

    pub const fn size(&self) -> usize {
        self.bytes.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileAcceptFilter {
    Extension(String),
    MediaType(String),
    MediaRange(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlFilePickerRequest {
    pub key: String,
    pub accept: Vec<FileAcceptFilter>,
    pub multiple: bool,
}

impl FileAcceptFilter {
    pub fn as_token(&self) -> String {
        match self {
            Self::Extension(value) | Self::MediaType(value) => value.clone(),
            Self::MediaRange(category) => format!("{category}/*"),
        }
    }
}

impl ControlFilePickerRequest {
    pub fn accept_attribute(&self) -> String {
        self.accept
            .iter()
            .map(FileAcceptFilter::as_token)
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn to_host_json(&self) -> String {
        format!(
            "{{\"key\":\"{}\",\"accept\":\"{}\",\"multiple\":{}}}",
            json_string(&self.key),
            json_string(&self.accept_attribute()),
            self.multiple
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlFileItemState {
    pub opaque_id: String,
    pub name: String,
    pub media_type: Option<String>,
    pub size: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlFileState {
    pub key: String,
    pub files: Vec<ControlFileItemState>,
    pub multiple: bool,
    pub value_text: String,
    pub diagnostics: Vec<ControlValueDiagnostic>,
}

impl ControlValueState {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKey {
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Enter,
    Escape,
    Space,
    WordLeft,
    WordRight,
    SelectAll,
    Undo,
    Redo,
}

impl ControlKey {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "backspace" => Self::Backspace,
            "delete" => Self::Delete,
            "arrow-left" => Self::ArrowLeft,
            "arrow-right" => Self::ArrowRight,
            "arrow-up" => Self::ArrowUp,
            "arrow-down" => Self::ArrowDown,
            "home" => Self::Home,
            "end" => Self::End,
            "enter" => Self::Enter,
            "escape" => Self::Escape,
            "space" => Self::Space,
            "word-left" => Self::WordLeft,
            "word-right" => Self::WordRight,
            "select-all" => Self::SelectAll,
            "undo" => Self::Undo,
            "redo" => Self::Redo,
            _ => return None,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Backspace => "backspace",
            Self::Delete => "delete",
            Self::ArrowLeft => "arrow-left",
            Self::ArrowRight => "arrow-right",
            Self::ArrowUp => "arrow-up",
            Self::ArrowDown => "arrow-down",
            Self::Home => "home",
            Self::End => "end",
            Self::Enter => "enter",
            Self::Escape => "escape",
            Self::Space => "space",
            Self::WordLeft => "word-left",
            Self::WordRight => "word-right",
            Self::SelectAll => "select-all",
            Self::Undo => "undo",
            Self::Redo => "redo",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlEffect {
    Focused(String),
    ValueChanged {
        key: String,
        value: String,
    },
    CheckedChanged {
        key: String,
        checked: bool,
    },
    Activated(String),
    FilePickerRequested(ControlFilePickerRequest),
    FilesChanged {
        key: String,
        files: Vec<ControlFileItemState>,
    },
    SuggestionPickerChanged {
        key: String,
        open: bool,
        active_index: Option<usize>,
    },
    SuggestionCommitted {
        key: String,
        value: String,
    },
    LiveValueChanged {
        key: String,
        value: String,
    },
    SelectionChanged {
        key: String,
        selection: ControlSelection,
    },
    CompositionChanged {
        key: String,
        composition: Option<String>,
    },
}

/// Character-indexed selection. Navigation and deletion snap to UAX #29
/// grapheme boundaries while explicit accessibility ranges remain scalar-based.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl ControlSelection {
    pub const fn collapsed(offset: usize) -> Self {
        Self {
            anchor: offset,
            focus: offset,
        }
    }

    pub fn ordered(self) -> (usize, usize) {
        (self.anchor.min(self.focus), self.anchor.max(self.focus))
    }

    pub const fn is_collapsed(self) -> bool {
        self.anchor == self.focus
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ControlEditorState {
    pub key: String,
    pub selection: ControlSelection,
    pub composition: Option<String>,
    pub composition_range: Option<ControlSelection>,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub caret_phase_ms: u64,
    pub pointer_anchor: Option<usize>,
    pub invalid_message: Option<String>,
    pub accessible_description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrowserControlModel {
    controls: Vec<ControlState>,
    initial_controls: Vec<ControlState>,
    bindings: Vec<ControlBinding>,
    editors: Vec<ControlEditorState>,
    histories: Vec<ControlEditHistory>,
    choice_anchors: Vec<Option<usize>>,
    file_selections: Vec<Vec<HostFileSelection>>,
    file_diagnostics: Vec<Vec<ControlValueDiagnostic>>,
    suggestions: Vec<ControlSuggestionState>,
    live_values: Vec<LiveValueState>,
    initial_live_values: Vec<LiveValueState>,
    dirty_values: Vec<bool>,
    dirty_checkedness: Vec<bool>,
    document_controls: Vec<DocumentControlBinding>,
    custom_elements: CustomElementInternalsRegistry,
    focused_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ControlEditSnapshot {
    value: String,
    selection: ControlSelection,
}

impl ControlEditSnapshot {
    fn from_parts(control: &ControlState, editor: &ControlEditorState) -> Self {
        Self {
            value: control.value.clone(),
            selection: editor.selection,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ControlEditHistory {
    undo: Vec<ControlEditSnapshot>,
    redo: Vec<ControlEditSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlBinding {
    pub key: String,
    pub id: Option<String>,
    pub control_type: String,
    pub form_owner: Option<String>,
    pub form_index: Option<usize>,
    pub form_action: Option<String>,
    pub resolved_form_action: Option<String>,
    pub form_enctype: Option<String>,
    pub form_method: Option<String>,
    pub form_novalidate: bool,
    pub pattern: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub minlength: Option<String>,
    pub maxlength: Option<String>,
    pub step: Option<String>,
    pub inputmode: Option<String>,
    pub accept: Option<String>,
    pub datalist_options: Vec<BrowserDatalistOption>,
    pub autocomplete: Option<String>,
    pub autocomplete_tokens: Vec<String>,
    pub form_autocomplete_enabled: bool,
    pub dirname: Option<String>,
    pub direction: ControlTextDirection,
    pub direction_auto: bool,
    pub document_order: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DocumentControlBinding {
    id: Option<String>,
    name: Option<String>,
    control_type: String,
    form_owner: Option<String>,
    form_index: Option<usize>,
    document_order: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FormAutocompleteBinding {
    id: Option<String>,
    form_index: usize,
    enabled: bool,
}

impl BrowserControlModel {
    pub fn from_render_tree(tree: &BrowserRenderTree) -> Self {
        let form_autocomplete = collect_form_autocomplete_bindings(&tree.children);
        let mut controls = Vec::new();
        let mut bindings = Vec::new();
        let mut document_controls = Vec::new();
        let mut control_index = 0;
        let mut form_index = 0;
        let mut document_order = 0;
        collect_model_nodes(
            &tree.children,
            &mut ModelCollection {
                states: &mut controls,
                bindings: &mut bindings,
                document_controls: &mut document_controls,
                control_index: &mut control_index,
                next_form_index: &mut form_index,
                document_order: &mut document_order,
                form_autocomplete: &form_autocomplete,
            },
            None,
            ControlTextDirection::Ltr,
            false,
        );
        let focused_key = controls
            .iter()
            .find(|control| control.focused && !control.disabled)
            .map(|control| control.key.clone());
        let descriptions = control_descriptions(tree);
        let editors = controls
            .iter()
            .zip(descriptions)
            .map(|(control, accessible_description)| ControlEditorState {
                key: control.key.clone(),
                selection: ControlSelection::collapsed(control.value.chars().count()),
                composition: None,
                composition_range: None,
                scroll_x: 0.0,
                scroll_y: 0.0,
                caret_phase_ms: 0,
                pointer_anchor: None,
                invalid_message: None,
                accessible_description,
            })
            .collect();
        let histories = vec![ControlEditHistory::default(); bindings.len()];
        let choice_anchors = vec![None; bindings.len()];
        let file_selections = vec![Vec::new(); bindings.len()];
        let file_diagnostics = vec![Vec::new(); bindings.len()];
        let suggestions = bindings
            .iter()
            .map(|binding| ControlSuggestionState {
                key: binding.key.clone(),
                ..ControlSuggestionState::default()
            })
            .collect();
        let dirty_values = vec![false; bindings.len()];
        let dirty_checkedness = vec![false; bindings.len()];
        let mut live_values = Vec::new();
        let mut live_index = 0;
        let mut live_form_index = 0;
        collect_live_value_nodes(
            &tree.children,
            &mut live_values,
            &mut live_index,
            None,
            &mut live_form_index,
        );
        Self {
            initial_controls: controls.clone(),
            controls,
            bindings,
            editors,
            histories,
            choice_anchors,
            file_selections,
            file_diagnostics,
            suggestions,
            initial_live_values: live_values.clone(),
            live_values,
            dirty_values,
            dirty_checkedness,
            document_controls,
            custom_elements: CustomElementInternalsRegistry::from_render_tree(tree),
            focused_key,
        }
    }

    pub fn controls(&self) -> &[ControlState] {
        &self.controls
    }

    pub fn live_value_states(&self) -> Vec<LiveValueState> {
        self.live_values
            .iter()
            .cloned()
            .map(|mut state| {
                if state.kind == LiveValueKind::Output {
                    state.dependencies = self.output_dependencies(&state.key);
                    if state
                        .dependencies
                        .iter()
                        .any(|dependency| dependency.key.is_none())
                    {
                        state.diagnostics.push(ControlValueDiagnostic {
                            code: "unresolved-output-dependency",
                            message: "output dependency does not resolve to a form control",
                        });
                    }
                }
                state
            })
            .collect()
    }

    pub fn live_value_state(&self, key: &str) -> Option<LiveValueState> {
        let mut state = self
            .live_values
            .iter()
            .find(|state| state.key == key)?
            .clone();
        if state.kind == LiveValueKind::Output {
            state.dependencies = self.output_dependencies(key);
            if state
                .dependencies
                .iter()
                .any(|dependency| dependency.key.is_none())
            {
                state.diagnostics.push(ControlValueDiagnostic {
                    code: "unresolved-output-dependency",
                    message: "output dependency does not resolve to a form control",
                });
            }
        }
        Some(state)
    }

    pub fn live_value_states_host_json(&self) -> String {
        format!(
            "[{}]",
            self.live_value_states()
                .iter()
                .map(LiveValueState::to_host_json)
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    pub fn output_dependencies(&self, key: &str) -> Vec<OutputDependencyValue> {
        let Some(output) = self.live_values.iter().find(|state| state.key == key) else {
            return Vec::new();
        };
        output
            .dependencies
            .iter()
            .map(|dependency| {
                let control = self
                    .bindings
                    .iter()
                    .position(|binding| binding.id.as_deref() == Some(&dependency.id))
                    .and_then(|index| self.controls.get(index).map(|control| (index, control)));
                OutputDependencyValue {
                    id: dependency.id.clone(),
                    key: control.map(|(_, control)| control.key.clone()),
                    value: control
                        .map(|(_, control)| control.value.clone())
                        .unwrap_or_default(),
                }
            })
            .collect()
    }

    /// Apply script-owned output calculation through shared dependency state.
    pub fn recalculate_output<F>(&mut self, key: &str, calculate: F) -> Option<ControlEffect>
    where
        F: FnOnce(&[OutputDependencyValue]) -> String,
    {
        if !self
            .live_values
            .iter()
            .any(|state| state.key == key && state.kind == LiveValueKind::Output)
        {
            return None;
        }
        let dependencies = self.output_dependencies(key);
        let value = calculate(&dependencies);
        self.set_live_value(key, Some(&value))
    }

    /// Set an output, meter, or progress value through shared normalization.
    /// `None` makes progress indeterminate and restores output fallback text.
    pub fn set_live_value(&mut self, key: &str, value: Option<&str>) -> Option<ControlEffect> {
        let index = self.live_values.iter().position(|state| state.key == key)?;
        let initial = self.initial_live_values.get(index)?.clone();
        let current = &self.live_values[index];
        let updated = match current.kind {
            LiveValueKind::Output => {
                let mut state = current.clone();
                state.text = value.unwrap_or(&initial.text).to_string();
                state.value_text = state.text.clone();
                state
            }
            LiveValueKind::Meter => normalized_meter_state(current, value),
            LiveValueKind::Progress => normalized_progress_state(current, value),
        };
        if *current == updated {
            return None;
        }
        let effect = ControlEffect::LiveValueChanged {
            key: key.to_string(),
            value: updated.value_text.clone(),
        };
        self.live_values[index] = updated;
        Some(effect)
    }

    pub fn bindings(&self) -> &[ControlBinding] {
        &self.bindings
    }

    pub fn binding(&self, key: &str) -> Option<&ControlBinding> {
        self.bindings.iter().find(|binding| binding.key == key)
    }

    pub fn form_associated_custom_elements(&self) -> &[FormAssociatedCustomElementState] {
        self.custom_elements.elements()
    }

    pub fn form_associated_custom_element(
        &self,
        key: &str,
    ) -> Option<&FormAssociatedCustomElementState> {
        self.custom_elements.element(key)
    }

    pub fn attach_form_associated_custom_element(
        &mut self,
        key: &str,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements.attach(key)
    }

    pub fn reassociate_form_associated_custom_element(
        &mut self,
        key: &str,
        association: CustomElementFormAssociation,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements.reassociate(key, association)
    }

    pub fn set_custom_element_form_value(
        &mut self,
        key: &str,
        value: Option<CustomElementFormValue>,
        restoration_state: Option<CustomElementFormValue>,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements
            .set_form_value(key, value, restoration_state)
    }

    pub fn set_custom_element_validity(
        &mut self,
        key: &str,
        validity: CustomElementValidity,
        message: Option<String>,
        anchor: Option<String>,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements
            .set_validity(key, validity, message, anchor)
    }

    pub fn set_custom_element_disabled(
        &mut self,
        key: &str,
        disabled: bool,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements.set_disabled(key, disabled)
    }

    pub fn set_custom_element_accessibility_value(
        &mut self,
        key: &str,
        value: CustomElementAccessibilityValue,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements.set_accessibility_value(key, value)
    }

    pub fn set_custom_element_accessibility_projection(
        &mut self,
        key: &str,
        projection: CustomElementAccessibilityProjection,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements
            .set_accessibility_projection(key, projection)
    }

    pub fn custom_element_accessibility_action(
        &mut self,
        key: &str,
        action: CustomElementAccessibilityAction,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements.accessibility_action(key, action)
    }

    pub fn custom_element_accessibility_state(
        &self,
        key: &str,
    ) -> Option<CustomElementAccessibilityState> {
        self.custom_elements.accessibility_state(key)
    }

    pub fn custom_element_diagnostics(
        &self,
        form_id: Option<&str>,
        form_index: usize,
    ) -> Vec<CustomElementDiagnostic> {
        self.custom_elements.diagnostics(form_id, form_index)
    }

    pub fn custom_element_submission_groups(
        &self,
        form_id: Option<&str>,
        form_index: usize,
    ) -> Vec<CustomElementSubmissionGroup> {
        self.custom_elements.submission_groups(form_id, form_index)
    }

    pub fn restore_custom_element_state(
        &mut self,
        key: &str,
        mode: CustomElementStateRestoreMode,
    ) -> Result<(), CustomElementInternalsError> {
        self.custom_elements.restore_state(key, mode)
    }

    pub fn take_custom_element_lifecycle_events(&mut self) -> Vec<CustomElementLifecycleEvent> {
        self.custom_elements.take_lifecycle_events()
    }

    pub fn form_control_document_order(
        &self,
        form_id: Option<&str>,
        form_index: usize,
        id: Option<&str>,
        name: Option<&str>,
        control_type: &str,
        used_orders: &[usize],
    ) -> Option<usize> {
        self.document_controls
            .iter()
            .filter(|binding| !used_orders.contains(&binding.document_order))
            .filter(|binding| match binding.form_owner.as_deref() {
                Some(owner) => form_id == Some(owner),
                None => binding.form_index == Some(form_index),
            })
            .find(|binding| {
                id.is_some_and(|id| binding.id.as_deref() == Some(id))
                    || (binding.name.as_deref() == name && binding.control_type == control_type)
            })
            .map(|binding| binding.document_order)
    }

    /// Resolve the live directionality used by HTML's `dirname` form entry.
    pub fn directionality(&self, key: &str) -> Option<ControlTextDirection> {
        let index = self
            .bindings
            .iter()
            .position(|binding| binding.key == key)?;
        let binding = &self.bindings[index];
        if binding.direction_auto {
            Some(control_text_direction(&self.controls[index].value).unwrap_or(binding.direction))
        } else {
            Some(binding.direction)
        }
    }

    pub fn editor(&self, key: &str) -> Option<&ControlEditorState> {
        self.editors.iter().find(|editor| editor.key == key)
    }

    pub fn control(&self, key: &str) -> Option<&ControlState> {
        self.controls.iter().find(|control| control.key == key)
    }

    pub fn default_state(&self, key: &str) -> Option<ControlDefaultState> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        Some(ControlDefaultState {
            key: key.to_string(),
            default_value: self.initial_controls[index].value.clone(),
            value: self.controls[index].value.clone(),
            default_checked: self.initial_controls[index].checked,
            checked: self.controls[index].checked,
            dirty_value: self.dirty_values[index],
            dirty_checkedness: self.dirty_checkedness[index],
        })
    }

    pub fn autofill_descriptors(&self) -> Vec<ControlAutofillDescriptor> {
        self.controls
            .iter()
            .zip(&self.bindings)
            .filter_map(|(control, binding)| autofill_descriptor(control, binding))
            .collect()
    }

    pub fn capture_state(&self, privacy: ControlStatePrivacy) -> ControlStateSnapshot {
        let mut snapshot = ControlStateSnapshot::default();
        let mut bytes = 0_usize;
        for (index, (control, binding)) in self.controls.iter().zip(&self.bindings).enumerate() {
            if snapshot.controls.len() >= FORM_STATE_CONTROL_LIMIT {
                snapshot.diagnostics.push(ControlStateDiagnostic {
                    code: "state-control-limit",
                    key: None,
                    message: "form state exceeds the shared control limit",
                });
                break;
            }
            if matches!(control.kind, ControlKind::Button | ControlKind::File)
                || (control.kind == ControlKind::Password && privacy == ControlStatePrivacy::Public)
            {
                continue;
            }
            let entry_bytes = control.key.len() + binding.control_type.len() + control.value.len();
            if bytes.saturating_add(entry_bytes) > FORM_STATE_BYTE_LIMIT {
                snapshot.diagnostics.push(ControlStateDiagnostic {
                    code: "state-byte-limit",
                    key: Some(control.key.clone()),
                    message: "form state exceeds the shared byte limit",
                });
                break;
            }
            bytes += entry_bytes;
            snapshot.controls.push(ControlRestorationEntry {
                key: control.key.clone(),
                control_type: binding.control_type.clone(),
                value: control.value.clone(),
                checked: control.checked,
                indeterminate: control.indeterminate,
                selected_indices: control.selected_indices.clone(),
                selected_index: control.selected_index,
                dirty_value: self.dirty_values[index],
                dirty_checkedness: self.dirty_checkedness[index],
            });
        }
        for entry in self.custom_elements.restoration_entries() {
            if snapshot.controls.len() + snapshot.custom_elements.len() >= FORM_STATE_CONTROL_LIMIT
            {
                snapshot.diagnostics.push(ControlStateDiagnostic {
                    code: "state-control-limit",
                    key: None,
                    message: "form state exceeds the shared control limit",
                });
                break;
            }
            let Some(state_bytes) = custom_restoration_state_bytes(&entry.state) else {
                snapshot.diagnostics.push(ControlStateDiagnostic {
                    code: "state-file-omitted",
                    key: Some(entry.key),
                    message: "file-backed custom-element state is not persisted",
                });
                continue;
            };
            let entry_bytes = entry.key.len().saturating_add(state_bytes);
            if bytes.saturating_add(entry_bytes) > FORM_STATE_BYTE_LIMIT {
                snapshot.diagnostics.push(ControlStateDiagnostic {
                    code: "state-byte-limit",
                    key: Some(entry.key),
                    message: "form state exceeds the shared byte limit",
                });
                break;
            }
            bytes += entry_bytes;
            snapshot.custom_elements.push(entry);
        }
        snapshot
    }

    pub fn restore_state(&mut self, snapshot: &ControlStateSnapshot) -> Vec<ControlEffect> {
        let mut effects = Vec::new();
        for entry in snapshot.controls.iter().take(FORM_STATE_CONTROL_LIMIT) {
            let Some(index) = self.bindings.iter().position(|binding| {
                binding.key == entry.key && binding.control_type == entry.control_type
            }) else {
                continue;
            };
            if matches!(
                self.controls[index].kind,
                ControlKind::Button | ControlKind::File
            ) {
                continue;
            }
            let changed = restoration_changed(&self.controls[index], entry);
            restore_control(&mut self.controls[index], entry);
            self.dirty_values[index] = entry.dirty_value;
            self.dirty_checkedness[index] = entry.dirty_checkedness;
            reset_editor_after_value_change(
                &mut self.editors[index],
                self.controls[index].value.chars().count(),
            );
            self.histories[index] = ControlEditHistory::default();
            self.choice_anchors[index] = None;
            self.suggestions[index] = ControlSuggestionState {
                key: self.controls[index].key.clone(),
                ..ControlSuggestionState::default()
            };
            if changed {
                effects.push(effect_for_restored_control(&self.controls[index]));
            }
        }
        self.custom_elements.restore_entries(
            &snapshot.custom_elements,
            CustomElementStateRestoreMode::Restore,
        );
        effects
    }

    pub fn apply_autofill(
        &mut self,
        transaction: &ControlAutofillTransaction,
    ) -> ControlAutofillOutcome {
        let mut outcome = ControlAutofillOutcome::default();
        if transaction.values.len() > MAX_AUTOFILL_FIELDS {
            outcome.diagnostics.push(ControlStateDiagnostic {
                code: "autofill-field-limit",
                key: None,
                message: "autofill transaction exceeds the shared field limit",
            });
        }
        let descriptors = self.autofill_descriptors();
        for descriptor in descriptors {
            if !descriptor.enabled
                || (descriptor.sensitive && transaction.privacy == ControlStatePrivacy::Public)
            {
                continue;
            }
            let value = transaction
                .values
                .iter()
                .take(MAX_AUTOFILL_FIELDS)
                .find(|value| {
                    value.purpose.eq_ignore_ascii_case(&descriptor.purpose)
                        && (value.section.is_none() || value.section == descriptor.section)
                });
            let Some(value) = value else {
                continue;
            };
            if value.value.len() > MAX_AUTOFILL_VALUE_BYTES {
                outcome.diagnostics.push(ControlStateDiagnostic {
                    code: "autofill-value-limit",
                    key: Some(descriptor.key.clone()),
                    message: "autofill value exceeds the shared byte limit",
                });
                continue;
            }
            let Some(index) = self
                .controls
                .iter()
                .position(|control| control.key == descriptor.key)
            else {
                continue;
            };
            if !apply_autofill_value(
                &mut self.controls[index],
                &self.bindings[index],
                &value.value,
            ) {
                continue;
            }
            self.dirty_values[index] = true;
            reset_editor_after_value_change(
                &mut self.editors[index],
                self.controls[index].value.chars().count(),
            );
            self.histories[index] = ControlEditHistory::default();
            self.suggestions[index] = ControlSuggestionState {
                key: self.controls[index].key.clone(),
                ..ControlSuggestionState::default()
            };
            outcome
                .effects
                .push(effect_for_restored_control(&self.controls[index]));
            for kind in [
                ControlMutationEventKind::Input,
                ControlMutationEventKind::Change,
            ] {
                outcome.events.push(ControlMutationEvent {
                    key: descriptor.key.clone(),
                    kind,
                    source: ControlMutationSource::Autofill,
                });
            }
        }
        self.custom_elements
            .restore_all(CustomElementStateRestoreMode::Autocomplete);
        outcome
    }

    pub fn suggestion_state(&self, key: &str) -> Option<&ControlSuggestionState> {
        let index = self
            .bindings
            .iter()
            .position(|binding| binding.key == key)?;
        (!self.bindings[index].datalist_options.is_empty()).then_some(&self.suggestions[index])
    }

    pub fn focused_suggestion_state(&self) -> Option<&ControlSuggestionState> {
        self.focused_key()
            .and_then(|key| self.suggestion_state(key))
    }

    /// Build a bounded picker projection from one datalist query. Hosts never
    /// receive the unfiltered source list and do not normalize typed values.
    pub fn open_suggestions(
        &mut self,
        key: &str,
        query: &str,
        limit: usize,
    ) -> Option<ControlEffect> {
        let index = self
            .bindings
            .iter()
            .position(|binding| binding.key == key)?;
        if !suggestions_enabled(&self.controls[index], &self.bindings[index]) {
            return None;
        }
        self.suggestions[index] =
            build_suggestion_state(&self.controls[index], &self.bindings[index], query, limit);
        Some(suggestion_picker_effect(&self.suggestions[index]))
    }

    pub fn apply_suggestion_picker_action(
        &mut self,
        key: &str,
        action: ControlSuggestionPickerAction,
    ) -> Option<ControlEffect> {
        let index = self
            .bindings
            .iter()
            .position(|binding| binding.key == key)?;
        match action {
            ControlSuggestionPickerAction::MovePrevious => self.move_suggestion_at(index, false),
            ControlSuggestionPickerAction::MoveNext => self.move_suggestion_at(index, true),
            ControlSuggestionPickerAction::CommitActive => {
                let option_index = self.suggestions[index].active_index?;
                self.commit_suggestion_at(index, option_index)
            }
            ControlSuggestionPickerAction::CommitIndex(option_index) => {
                self.commit_suggestion_at(index, option_index)
            }
            ControlSuggestionPickerAction::Cancel => self.dismiss_suggestions_at(index),
        }
    }

    fn move_suggestion_at(&mut self, index: usize, forward: bool) -> Option<ControlEffect> {
        if !self.suggestions[index].open {
            let key = self.bindings[index].key.clone();
            let query = self.controls[index].value.clone();
            self.open_suggestions(&key, &query, MAX_SUGGESTION_RESULTS)?;
            if !forward && !self.suggestions[index].options.is_empty() {
                self.suggestions[index].active_index =
                    Some(self.suggestions[index].options.len() - 1);
            }
            return Some(suggestion_picker_effect(&self.suggestions[index]));
        }
        let option_count = self.suggestions[index].options.len();
        if option_count == 0 {
            return Some(suggestion_picker_effect(&self.suggestions[index]));
        }
        self.suggestions[index].active_index = Some(match self.suggestions[index].active_index {
            Some(active) if forward => (active + 1) % option_count,
            Some(0) if !forward => option_count - 1,
            Some(active) => active - 1,
            None if forward => 0,
            None => option_count - 1,
        });
        Some(suggestion_picker_effect(&self.suggestions[index]))
    }

    fn commit_suggestion_at(&mut self, index: usize, option_index: usize) -> Option<ControlEffect> {
        if !self.suggestions[index].open {
            return None;
        }
        let option = self.suggestions[index].options.get(option_index)?.clone();
        let changed = self.controls[index].value != option.value;
        if changed {
            self.record_history(index);
            self.controls[index].value = option.value.clone();
            self.dirty_values[index] = true;
            reset_editor_after_value_change(&mut self.editors[index], option.value.chars().count());
        }
        self.suggestions[index].open = false;
        self.suggestions[index].active_index = None;
        if changed {
            Some(ControlEffect::SuggestionCommitted {
                key: self.controls[index].key.clone(),
                value: option.value,
            })
        } else {
            Some(suggestion_picker_effect(&self.suggestions[index]))
        }
    }

    fn dismiss_suggestions_at(&mut self, index: usize) -> Option<ControlEffect> {
        if !self.suggestions[index].open {
            return None;
        }
        self.suggestions[index].open = false;
        self.suggestions[index].active_index = None;
        Some(suggestion_picker_effect(&self.suggestions[index]))
    }

    pub fn selected_files(&self, key: &str) -> Option<&[HostFileSelection]> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        (self.controls[index].kind == ControlKind::File)
            .then_some(self.file_selections[index].as_slice())
    }

    pub fn file_picker_request(&self, key: &str) -> Option<ControlFilePickerRequest> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        let control = &self.controls[index];
        if control.kind != ControlKind::File || control.disabled {
            return None;
        }
        Some(ControlFilePickerRequest {
            key: key.to_string(),
            accept: parse_accept_filters(self.bindings[index].accept.as_deref()),
            multiple: control.multiple,
        })
    }

    pub fn file_state(&self, key: &str) -> Option<ControlFileState> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        let control = &self.controls[index];
        if control.kind != ControlKind::File {
            return None;
        }
        let files = self.file_selections[index]
            .iter()
            .map(file_item_state)
            .collect::<Vec<_>>();
        let value_text = match files.as_slice() {
            [] => "No file selected".to_string(),
            [file] => file.name.clone(),
            files => format!("{} files selected", files.len()),
        };
        Some(ControlFileState {
            key: key.to_string(),
            files,
            multiple: control.multiple,
            value_text,
            diagnostics: self.file_diagnostics[index].clone(),
        })
    }

    /// Apply one host picker result after shared accept/multiple validation.
    /// Rejected entries never enter retained state or multipart planning.
    pub fn apply_file_selection(
        &mut self,
        key: &str,
        selections: Vec<HostFileSelection>,
    ) -> Option<ControlEffect> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        if self.controls[index].kind != ControlKind::File || self.controls[index].disabled {
            return None;
        }
        let filters = parse_accept_filters(self.bindings[index].accept.as_deref());
        let mut diagnostics = Vec::new();
        let mut accepted = Vec::new();
        let mut accepted_bytes = 0_usize;
        for selection in selections.into_iter().take(MAX_SELECTED_FILES + 1) {
            if accepted.len() >= MAX_SELECTED_FILES {
                push_file_diagnostic(
                    &mut diagnostics,
                    "too-many-files",
                    "file selection exceeds the shared item limit",
                );
                break;
            }
            if selection.size() > MAX_SELECTED_FILE_BYTES {
                push_file_diagnostic(
                    &mut diagnostics,
                    "file-too-large",
                    "selected file exceeds the shared byte limit",
                );
                continue;
            }
            if accepted_bytes.saturating_add(selection.size()) > MAX_SELECTED_TOTAL_BYTES {
                push_file_diagnostic(
                    &mut diagnostics,
                    "selection-too-large",
                    "selected files exceed the shared aggregate byte limit",
                );
                continue;
            }
            if !filters.is_empty() && !file_matches_accept(&selection, &filters) {
                push_file_diagnostic(
                    &mut diagnostics,
                    "accept-mismatch",
                    "selected file does not match the accept filter",
                );
                continue;
            }
            accepted_bytes += selection.size();
            accepted.push(selection);
            if !self.controls[index].multiple {
                break;
            }
        }
        self.file_selections[index] = accepted;
        self.file_diagnostics[index] = diagnostics;
        self.controls[index].value = self.file_selections[index]
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        self.dirty_values[index] = true;
        Some(ControlEffect::FilesChanged {
            key: key.to_string(),
            files: self.file_selections[index]
                .iter()
                .map(file_item_state)
                .collect(),
        })
    }

    pub fn focused_key(&self) -> Option<&str> {
        self.focused_key.as_deref()
    }

    pub fn reset_form(
        &mut self,
        form_id: Option<&str>,
        form_index: Option<usize>,
    ) -> Vec<ControlEffect> {
        let mut effects = Vec::new();
        for (index, (((control, initial), binding), editor)) in self
            .controls
            .iter_mut()
            .zip(&self.initial_controls)
            .zip(&self.bindings)
            .zip(&mut self.editors)
            .enumerate()
        {
            let associated = match binding.form_owner.as_deref() {
                Some(owner) => form_id == Some(owner),
                None => binding.form_index == form_index,
            };
            if !associated {
                continue;
            }
            let focused = control.focused;
            let changed = control.value != initial.value
                || control.checked != initial.checked
                || control.indeterminate != initial.indeterminate
                || control.selected_indices != initial.selected_indices
                || control.selected_index != initial.selected_index
                || !self.file_selections[index].is_empty();
            *control = initial.clone();
            control.focused = focused;
            editor.selection = ControlSelection::collapsed(control.value.chars().count());
            editor.composition = None;
            editor.composition_range = None;
            editor.scroll_x = 0.0;
            editor.scroll_y = 0.0;
            editor.caret_phase_ms = 0;
            editor.pointer_anchor = None;
            editor.invalid_message = None;
            self.histories[index] = ControlEditHistory::default();
            self.choice_anchors[index] = None;
            self.file_selections[index].clear();
            self.file_diagnostics[index].clear();
            self.suggestions[index] = ControlSuggestionState {
                key: control.key.clone(),
                ..ControlSuggestionState::default()
            };
            self.dirty_values[index] = false;
            self.dirty_checkedness[index] = false;
            if changed {
                effects.push(ControlEffect::ValueChanged {
                    key: control.key.clone(),
                    value: control.value.clone(),
                });
            }
        }
        for (state, initial) in self.live_values.iter_mut().zip(&self.initial_live_values) {
            if state.kind != LiveValueKind::Output {
                continue;
            }
            let associated = match state.form_owner.as_deref() {
                Some(owner) => form_id == Some(owner),
                None => state.form_index == form_index,
            };
            if associated && state.text != initial.text {
                *state = initial.clone();
                effects.push(ControlEffect::LiveValueChanged {
                    key: state.key.clone(),
                    value: state.value_text.clone(),
                });
            }
        }
        if let Some(form_index) = form_index {
            self.custom_elements.reset_form(form_id, form_index);
        }
        effects
    }

    pub fn focused(&self) -> Option<&ControlState> {
        let key = self.focused_key.as_deref()?;
        self.controls.iter().find(|control| control.key == key)
    }

    pub fn focus(&mut self, key: &str) -> Option<ControlEffect> {
        let target = self
            .controls
            .iter()
            .position(|control| control.key == key && !control.disabled)?;
        for control in &mut self.controls {
            control.focused = false;
        }
        for (index, suggestion) in self.suggestions.iter_mut().enumerate() {
            if index != target {
                suggestion.open = false;
                suggestion.active_index = None;
            }
        }
        self.controls[target].focused = true;
        self.editors[target].caret_phase_ms = 0;
        self.focused_key = Some(key.to_string());
        Some(ControlEffect::Focused(key.to_string()))
    }

    pub fn focus_next(&mut self, reverse: bool) -> Option<ControlEffect> {
        let enabled = self
            .controls
            .iter()
            .enumerate()
            .filter_map(|(index, control)| (!control.disabled).then_some(index))
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            return None;
        }
        let current = self.focused_key.as_deref().and_then(|key| {
            enabled
                .iter()
                .position(|index| self.controls[*index].key == key)
        });
        let position = match (current, reverse) {
            (Some(0), true) | (None, true) => enabled.len() - 1,
            (Some(position), true) => position - 1,
            (Some(position), false) => (position + 1) % enabled.len(),
            (None, false) => 0,
        };
        let key = self.controls[enabled[position]].key.clone();
        self.focus(&key)
    }

    pub fn pointer_activate(&mut self, key: &str) -> Option<ControlEffect> {
        self.focus(key)?;
        self.activate_focused()
            .or_else(|| Some(ControlEffect::Focused(key.to_string())))
    }

    pub fn text_input(&mut self, text: &str) -> Option<ControlEffect> {
        if text.is_empty() {
            return None;
        }
        self.replace_focused_selection(text)
    }

    pub fn key_down(&mut self, key: ControlKey) -> Option<ControlEffect> {
        self.key_down_with_shift(key, false)
    }

    pub fn key_down_with_shift(&mut self, key: ControlKey, shift: bool) -> Option<ControlEffect> {
        let focused_index = self.focused_key.as_deref().and_then(|focused| {
            self.bindings
                .iter()
                .position(|binding| binding.key == focused)
        })?;
        let kind = self.focused()?.kind;
        match key {
            ControlKey::Undo => return self.undo(),
            ControlKey::Redo => return self.redo(),
            ControlKey::SelectAll if kind.supports_selection() => {
                let key = self.focused_key()?.to_string();
                let length = self.focused()?.value.chars().count();
                return self.set_selection(&key, 0, length);
            }
            _ => {}
        }
        if self.suggestions[focused_index].open {
            match key {
                ControlKey::Escape => return self.dismiss_suggestions_at(focused_index),
                ControlKey::Enter => {
                    let option_index = self.suggestions[focused_index].active_index?;
                    return self.commit_suggestion_at(focused_index, option_index);
                }
                ControlKey::ArrowUp => return self.move_suggestion_at(focused_index, false),
                ControlKey::ArrowDown => return self.move_suggestion_at(focused_index, true),
                _ => {}
            }
        } else if suggestions_enabled(&self.controls[focused_index], &self.bindings[focused_index])
        {
            match key {
                ControlKey::ArrowUp => return self.move_suggestion_at(focused_index, false),
                ControlKey::ArrowDown => return self.move_suggestion_at(focused_index, true),
                _ => {}
            }
        }
        if kind == ControlKind::Range {
            return match key {
                ControlKey::ArrowUp | ControlKey::ArrowRight => self.step_focused_numeric(true),
                ControlKey::ArrowDown | ControlKey::ArrowLeft => self.step_focused_numeric(false),
                ControlKey::Home => self.set_focused_range_boundary(false),
                ControlKey::End => self.set_focused_range_boundary(true),
                _ => None,
            };
        }
        if kind.is_temporal() {
            return match key {
                ControlKey::ArrowUp | ControlKey::ArrowRight => self.step_focused_value(true),
                ControlKey::ArrowDown | ControlKey::ArrowLeft => self.step_focused_value(false),
                ControlKey::Home => self.set_focused_typed_boundary(false),
                ControlKey::End => self.set_focused_typed_boundary(true),
                _ => None,
            };
        }
        if kind == ControlKind::Radio {
            return match key {
                ControlKey::ArrowUp | ControlKey::ArrowLeft => self.move_radio_group(false),
                ControlKey::ArrowDown | ControlKey::ArrowRight => self.move_radio_group(true),
                _ => None,
            };
        }
        match (key, kind) {
            (ControlKey::Space, kind) if kind.accepts_text() => return self.text_input(" "),
            (ControlKey::Enter, ControlKind::TextArea) => return self.text_input("\n"),
            (ControlKey::Enter | ControlKey::Space, _) => return self.activate_focused(),
            _ => {}
        }
        if kind.accepts_text() {
            match key {
                ControlKey::ArrowUp if kind == ControlKind::Number => {
                    return self.step_focused_numeric(true)
                }
                ControlKey::ArrowDown if kind == ControlKind::Number => {
                    return self.step_focused_numeric(false)
                }
                ControlKey::ArrowLeft => return self.move_caret(-1, shift, false),
                ControlKey::ArrowRight => return self.move_caret(1, shift, false),
                ControlKey::WordLeft => {
                    return self.move_by_unit(false, ControlNavigationUnit::Word, shift)
                }
                ControlKey::WordRight => {
                    return self.move_by_unit(true, ControlNavigationUnit::Word, shift)
                }
                ControlKey::Home => return self.move_caret(0, shift, true),
                ControlKey::End => return self.move_caret(0, shift, false),
                ControlKey::Backspace => return self.delete_from_focused(true),
                ControlKey::Delete => return self.delete_from_focused(false),
                _ => {}
            }
        }
        if kind == ControlKind::Select {
            return match key {
                ControlKey::ArrowUp | ControlKey::ArrowLeft => {
                    self.move_select(false, false, shift)
                }
                ControlKey::ArrowDown | ControlKey::ArrowRight => {
                    self.move_select(true, false, shift)
                }
                ControlKey::Home => self.move_select(true, true, shift),
                ControlKey::End => self.move_select(false, true, shift),
                _ => None,
            };
        }
        None
    }

    pub fn sync_render_tree(&self, tree: &mut BrowserRenderTree) {
        let mut index = 0;
        sync_nodes(
            &mut tree.children,
            &self.controls,
            &self.editors,
            &mut index,
        );
        let mut live_index = 0;
        sync_live_value_nodes(&mut tree.children, &self.live_values, &mut live_index);
    }

    pub fn set_selection(
        &mut self,
        key: &str,
        anchor: usize,
        focus: usize,
    ) -> Option<ControlEffect> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        if !self.controls[index].kind.supports_selection() {
            return None;
        }
        let length = self.controls[index].value.chars().count();
        let selection = ControlSelection {
            anchor: anchor.min(length),
            focus: focus.min(length),
        };
        self.editors[index].selection = selection;
        self.editors[index].caret_phase_ms = 0;
        Some(ControlEffect::SelectionChanged {
            key: key.to_string(),
            selection,
        })
    }

    pub fn update_composition(&mut self, text: &str) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if control.disabled || control.readonly || !control.kind.accepts_text() {
            return None;
        }
        self.editors[index].composition = Some(text.to_string());
        self.editors[index].composition_range = Some(self.editors[index].selection);
        self.editors[index].caret_phase_ms = 0;
        Some(ControlEffect::CompositionChanged {
            key: control.key.clone(),
            composition: Some(text.to_string()),
        })
    }

    pub fn commit_composition(&mut self) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let composition = self.editors[index].composition.take()?;
        self.replace_focused_selection(&composition)
    }

    pub fn cancel_composition(&mut self) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        self.editors[index].composition.take()?;
        self.editors[index].composition_range = None;
        Some(ControlEffect::CompositionChanged {
            key: self.controls[index].key.clone(),
            composition: None,
        })
    }

    pub fn clear_validation(&mut self) {
        for editor in &mut self.editors {
            editor.invalid_message = None;
        }
    }

    pub fn set_invalid(&mut self, key: &str, message: impl Into<String>) -> bool {
        let Some(editor) = self.editors.iter_mut().find(|editor| editor.key == key) else {
            return false;
        };
        editor.invalid_message = Some(message.into());
        true
    }

    pub fn focus_first_invalid(&mut self) -> Option<ControlEffect> {
        let key = self
            .editors
            .iter()
            .find(|editor| editor.invalid_message.is_some())?
            .key
            .clone();
        self.focus(&key)
    }

    /// Return the selected plain text without exposing password values.
    pub fn copy_selection(&self) -> Option<String> {
        self.copy_selection_payload()?.plain_text
    }

    pub fn copy_selection_payload(&self) -> Option<ControlClipboardPayload> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if control.kind == ControlKind::Password || !control.kind.supports_selection() {
            return None;
        }
        let (start, end) = self.editors[index].selection.ordered();
        (start != end).then(|| {
            ControlClipboardPayload::from_plain_text(char_range(&control.value, start, end))
        })
    }

    /// Delete and return the selected text. Password selections are never
    /// placed on a host clipboard.
    pub fn cut_selection(&mut self) -> Option<ControlClipboardCut> {
        let text = self.copy_selection()?;
        let effect = self.replace_focused_selection("")?;
        Some(ControlClipboardCut { text, effect })
    }

    pub fn paste_text(&mut self, text: &str) -> Option<ControlEffect> {
        self.replace_focused_selection(text)
    }

    pub fn paste_payload(&mut self, payload: &ControlClipboardPayload) -> Option<ControlEffect> {
        self.replace_focused_selection(&payload.preferred_text()?)
    }

    /// Place a caret from control-local coordinates and begin drag selection.
    pub fn pointer_place(
        &mut self,
        key: &str,
        x: f64,
        y: f64,
        metrics: ControlTextMetrics,
    ) -> Option<ControlEffect> {
        self.pointer_select(key, x, y, metrics, 1)
    }

    /// Apply platform-independent click-count selection policy. A double click
    /// selects a Unicode-aware word class and a triple click selects a line.
    pub fn pointer_select(
        &mut self,
        key: &str,
        x: f64,
        y: f64,
        metrics: ControlTextMetrics,
        click_count: u8,
    ) -> Option<ControlEffect> {
        self.focus(key)?;
        let index = self.focused_index()?;
        let offset = point_to_offset(
            &self.controls[index].value,
            x + self.editors[index].scroll_x,
            y + self.editors[index].scroll_y,
            metrics,
        );
        let selection = match click_count.min(3) {
            2 => {
                let (start, end) = word_boundary(&self.controls[index].value, offset);
                ControlSelection {
                    anchor: start,
                    focus: end,
                }
            }
            3 => ControlSelection {
                anchor: line_boundary(&self.controls[index].value, offset, true),
                focus: line_boundary(&self.controls[index].value, offset, false),
            },
            _ => ControlSelection::collapsed(offset),
        };
        self.editors[index].selection = selection;
        self.editors[index].pointer_anchor = Some(selection.anchor);
        self.editors[index].caret_phase_ms = 0;
        Some(ControlEffect::SelectionChanged {
            key: key.to_string(),
            selection,
        })
    }

    /// Extend the active pointer selection, clamping outside-control drags to
    /// the nearest character position.
    pub fn pointer_drag(
        &mut self,
        x: f64,
        y: f64,
        metrics: ControlTextMetrics,
    ) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let anchor = self.editors[index].pointer_anchor?;
        let focus = point_to_offset(
            &self.controls[index].value,
            x + self.editors[index].scroll_x,
            y + self.editors[index].scroll_y,
            metrics,
        );
        let selection = ControlSelection { anchor, focus };
        self.editors[index].selection = selection;
        self.editors[index].caret_phase_ms = 0;
        Some(ControlEffect::SelectionChanged {
            key: self.controls[index].key.clone(),
            selection,
        })
    }

    /// Continue a drag and deterministically scroll when the pointer leaves
    /// the editable viewport. Hosts provide only local geometry.
    pub fn pointer_drag_autoscroll(
        &mut self,
        x: f64,
        y: f64,
        viewport_width: f64,
        viewport_height: f64,
        metrics: ControlTextMetrics,
    ) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        self.editors[index].pointer_anchor?;
        let metrics = sanitize_metrics(metrics);
        let (content_width, content_height) = text_extent(&self.controls[index].value, metrics);
        let max_x = (content_width - viewport_width.max(metrics.caret_width)).max(0.0);
        let max_y = if self.controls[index].kind == ControlKind::TextArea {
            (content_height - viewport_height.max(metrics.line_height)).max(0.0)
        } else {
            0.0
        };
        self.editors[index].scroll_x = drag_autoscroll_offset(
            self.editors[index].scroll_x,
            x,
            viewport_width,
            metrics.advance * 3.0,
            max_x,
        );
        self.editors[index].scroll_y = drag_autoscroll_offset(
            self.editors[index].scroll_y,
            y,
            viewport_height,
            metrics.line_height * 3.0,
            max_y,
        );
        self.pointer_drag(x, y, metrics)
    }

    pub fn accessibility_action(
        &mut self,
        action: ControlAccessibilityAction,
    ) -> Option<ControlEffect> {
        match action {
            ControlAccessibilityAction::Activate => self.activate_focused(),
            ControlAccessibilityAction::Move {
                forward,
                unit,
                extend,
            } => self.move_by_unit(forward, unit, extend),
            ControlAccessibilityAction::SetSelection(selection) => {
                let key = self.focused_key()?.to_string();
                self.set_selection(&key, selection.anchor, selection.focus)
            }
            ControlAccessibilityAction::ReplaceSelection(value) => {
                self.replace_focused_selection(&value)
            }
            ControlAccessibilityAction::SetValue(value) => self.set_focused_value(&value),
            ControlAccessibilityAction::Increment => self.step_focused_value(true),
            ControlAccessibilityAction::Decrement => self.step_focused_value(false),
            ControlAccessibilityAction::Toggle => self.activate_focused(),
            ControlAccessibilityAction::SetIndeterminate(indeterminate) => {
                let key = self.focused_key()?.to_string();
                self.set_indeterminate(&key, indeterminate)
            }
            ControlAccessibilityAction::SelectOption {
                index,
                extend,
                toggle,
            } => {
                let key = self.focused_key()?.to_string();
                self.select_option(&key, index, extend, toggle)
            }
            ControlAccessibilityAction::SelectAll => self.key_down(ControlKey::SelectAll),
            ControlAccessibilityAction::Undo => self.undo(),
            ControlAccessibilityAction::Redo => self.redo(),
            ControlAccessibilityAction::ShowSuggestions(query) => {
                let key = self.focused_key()?.to_string();
                self.open_suggestions(&key, &query, MAX_SUGGESTION_RESULTS)
            }
            ControlAccessibilityAction::MoveSuggestion { forward } => {
                let key = self.focused_key()?.to_string();
                self.apply_suggestion_picker_action(
                    &key,
                    if forward {
                        ControlSuggestionPickerAction::MoveNext
                    } else {
                        ControlSuggestionPickerAction::MovePrevious
                    },
                )
            }
            ControlAccessibilityAction::CommitSuggestion => {
                let key = self.focused_key()?.to_string();
                self.apply_suggestion_picker_action(
                    &key,
                    ControlSuggestionPickerAction::CommitActive,
                )
            }
            ControlAccessibilityAction::DismissSuggestions => {
                let key = self.focused_key()?.to_string();
                self.apply_suggestion_picker_action(&key, ControlSuggestionPickerAction::Cancel)
            }
        }
    }

    pub fn undo(&mut self) -> Option<ControlEffect> {
        self.restore_history(true)
    }

    pub fn redo(&mut self) -> Option<ControlEffect> {
        self.restore_history(false)
    }

    pub fn value_state(&self, key: &str) -> Option<ControlValueState> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        Some(control_value_state(
            &self.controls[index],
            &self.bindings[index],
        ))
    }

    pub fn choice_state(&self, key: &str) -> Option<ControlChoiceState> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        choice_state(&self.controls[index], &self.bindings[index])
    }

    pub fn selected_values(&self, key: &str) -> Option<Vec<String>> {
        let control = self.control(key)?;
        (control.kind == ControlKind::Select).then(|| selected_values(control))
    }

    pub fn set_indeterminate(&mut self, key: &str, indeterminate: bool) -> Option<ControlEffect> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key && control.kind == ControlKind::Checkbox)?;
        let control = &mut self.controls[index];
        if control.disabled || control.indeterminate == indeterminate {
            return None;
        }
        control.indeterminate = indeterminate;
        self.dirty_checkedness[index] = true;
        Some(ControlEffect::CheckedChanged {
            key: key.to_string(),
            checked: control.checked,
        })
    }

    pub fn select_option(
        &mut self,
        key: &str,
        option_index: usize,
        extend: bool,
        toggle: bool,
    ) -> Option<ControlEffect> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        let control = &self.controls[index];
        if control.disabled
            || control.kind != ControlKind::Select
            || option_index >= control.options.len()
            || option_is_disabled(control, option_index)
        {
            return None;
        }
        let multiple = control.multiple;
        let anchor = self.choice_anchors[index].unwrap_or(option_index);
        let control = &mut self.controls[index];
        control.selected_index = option_index;
        if multiple {
            if extend {
                let (start, end) = (anchor.min(option_index), anchor.max(option_index));
                control.selected_indices = (start..=end)
                    .filter(|candidate| !option_is_disabled(control, *candidate))
                    .collect();
            } else if toggle {
                if let Some(position) = control
                    .selected_indices
                    .iter()
                    .position(|selected| *selected == option_index)
                {
                    control.selected_indices.remove(position);
                } else {
                    control.selected_indices.push(option_index);
                    control.selected_indices.sort_unstable();
                }
                self.choice_anchors[index] = Some(option_index);
            } else {
                control.selected_indices = vec![option_index];
                self.choice_anchors[index] = Some(option_index);
            }
        } else {
            control.selected_indices = vec![option_index];
            self.choice_anchors[index] = Some(option_index);
        }
        sync_selected_value(control);
        self.dirty_values[index] = true;
        Some(ControlEffect::ValueChanged {
            key: key.to_string(),
            value: control.value.clone(),
        })
    }

    pub fn pointer_release(&mut self) {
        for editor in &mut self.editors {
            editor.pointer_anchor = None;
        }
    }

    /// Advance the shared, wall-clock-free caret blink timeline.
    pub fn advance_caret_blink(&mut self, elapsed_ms: u64) -> bool {
        let Some(index) = self.focused_index() else {
            return false;
        };
        let before = self.editors[index].caret_phase_ms < 500;
        self.editors[index].caret_phase_ms =
            (self.editors[index].caret_phase_ms + elapsed_ms) % 1000;
        before != (self.editors[index].caret_phase_ms < 500)
    }

    /// Resolve presentation geometry and scroll the control viewport just
    /// enough to keep the focused caret visible.
    pub fn editor_presentation(
        &mut self,
        key: &str,
        bounds: ControlRect,
        metrics: ControlTextMetrics,
    ) -> Option<ControlEditorPresentation> {
        let index = self
            .controls
            .iter()
            .position(|control| control.key == key)?;
        if !self.controls[index].kind.accepts_text() {
            return None;
        }
        let metrics = sanitize_metrics(metrics);
        let viewport = ControlRect {
            x: bounds.x + metrics.inset_x,
            y: bounds.y + metrics.inset_y,
            width: (bounds.width - metrics.inset_x * 2.0).max(metrics.caret_width),
            height: (bounds.height - metrics.inset_y * 2.0).max(metrics.line_height),
        };
        let value = self.controls[index].display_value();
        let selection = self.editors[index].selection;
        let caret_point = rect_for_offset(
            &value,
            selection.focus,
            ControlRect::default(),
            0.0,
            0.0,
            metrics,
        );
        let editor = &mut self.editors[index];
        if self.controls[index].focused {
            editor.scroll_x = reveal_axis(
                editor.scroll_x,
                caret_point.x,
                metrics.caret_width,
                viewport.width,
            );
            editor.scroll_y = if self.controls[index].kind == ControlKind::TextArea {
                reveal_axis(
                    editor.scroll_y,
                    caret_point.y,
                    metrics.line_height,
                    viewport.height,
                )
            } else {
                0.0
            };
        } else {
            editor.scroll_x = 0.0;
            editor.scroll_y = 0.0;
        }
        let selection_rects = selection_rects(
            &value,
            selection,
            viewport,
            editor.scroll_x,
            editor.scroll_y,
            metrics,
        );
        let caret = (self.controls[index].focused && editor.caret_phase_ms < 500).then_some(
            rect_for_offset(
                &value,
                selection.focus,
                viewport,
                editor.scroll_x,
                editor.scroll_y,
                metrics,
            ),
        );
        let composition_underlines = editor
            .composition
            .as_deref()
            .map(|composition| {
                let range = editor.composition_range.unwrap_or(selection);
                composition_rects(
                    &value,
                    composition,
                    range.ordered().0,
                    viewport,
                    editor.scroll_x,
                    editor.scroll_y,
                    metrics,
                )
            })
            .unwrap_or_default();
        let candidate_rect = editor.composition.as_deref().map(|composition| {
            let start = editor.composition_range.unwrap_or(selection).ordered().0;
            let mut composed = value.clone();
            replace_char_range(&mut composed, start, start, composition);
            let mut rect = rect_for_offset(
                &composed,
                start + composition.chars().count(),
                viewport,
                editor.scroll_x,
                editor.scroll_y,
                metrics,
            );
            rect.y += metrics.line_height;
            rect
        });
        Some(ControlEditorPresentation {
            key: key.to_string(),
            viewport,
            scroll_x: editor.scroll_x,
            scroll_y: editor.scroll_y,
            selection: selection_rects,
            caret,
            composition_underlines,
            candidate_rect,
            invalid_message: editor.invalid_message.clone(),
            accessible_description: editor.accessible_description.clone(),
        })
    }

    fn focused_index(&self) -> Option<usize> {
        let key = self.focused_key.as_deref()?;
        self.controls.iter().position(|control| control.key == key)
    }

    fn replace_focused_selection(&mut self, text: &str) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        if self.controls[index].disabled
            || self.controls[index].readonly
            || !self.controls[index].kind.accepts_text()
        {
            return None;
        }
        let selection = self.editors[index].selection;
        let (start, end) = selection.ordered();
        if start == end && text.is_empty() {
            return None;
        }
        let text = constrained_replacement(
            &self.controls[index],
            &self.bindings[index],
            start,
            end,
            text,
        );
        if text.is_empty() && start == end {
            return None;
        }
        self.record_history(index);
        let control = &mut self.controls[index];
        replace_char_range(&mut control.value, start, end, &text);
        self.dirty_values[index] = true;
        let caret = start + text.chars().count();
        self.editors[index].selection = ControlSelection::collapsed(caret);
        self.editors[index].composition = None;
        self.editors[index].composition_range = None;
        self.editors[index].caret_phase_ms = 0;
        self.editors[index].invalid_message = None;
        Some(ControlEffect::ValueChanged {
            key: control.key.clone(),
            value: control.value.clone(),
        })
    }

    fn set_focused_value(&mut self, value: &str) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if control.disabled
            || control.readonly
            || (!control.kind.accepts_text()
                && control.kind != ControlKind::Range
                && !control.kind.has_typed_value())
        {
            return None;
        }
        if control.kind == ControlKind::Range {
            let requested = parse_finite(value)?;
            let value = format_number(normalize_range_value(
                requested,
                control_numeric_constraints(&self.controls[index], &self.bindings[index]),
            ));
            if self.controls[index].value == value {
                return None;
            }
            self.controls[index].value = value.clone();
            self.dirty_values[index] = true;
            return Some(ControlEffect::ValueChanged {
                key: self.controls[index].key.clone(),
                value,
            });
        }
        if control.kind.is_temporal() {
            let value = parse_typed_value(control.kind, value)?.normalized;
            if self.controls[index].value == value {
                return None;
            }
            self.controls[index].value = value.clone();
            self.dirty_values[index] = true;
            return Some(ControlEffect::ValueChanged {
                key: self.controls[index].key.clone(),
                value,
            });
        }
        if control.kind == ControlKind::Color {
            let value = normalize_color(value)?;
            if self.controls[index].value == value {
                return None;
            }
            self.controls[index].value = value.clone();
            self.dirty_values[index] = true;
            return Some(ControlEffect::ValueChanged {
                key: self.controls[index].key.clone(),
                value,
            });
        }
        if control.value == value {
            return None;
        }
        let length = control.value.chars().count();
        self.editors[index].selection = ControlSelection {
            anchor: 0,
            focus: length,
        };
        self.replace_focused_selection(value)
    }

    fn step_focused_numeric(&mut self, forward: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if control.disabled
            || control.readonly
            || !matches!(control.kind, ControlKind::Number | ControlKind::Range)
        {
            return None;
        }
        let constraints = control_numeric_constraints(control, &self.bindings[index]);
        let base = constraints.minimum.unwrap_or(0.0);
        let mut next = match parse_finite(&control.value) {
            Some(current) if step_mismatch(current, base, constraints.step) => {
                let quotient = (current - base) / constraints.step;
                base + if forward {
                    quotient.ceil()
                } else {
                    quotient.floor()
                } * constraints.step
            }
            Some(current) => {
                current
                    + if forward {
                        constraints.step
                    } else {
                        -constraints.step
                    }
            }
            None if forward => constraints.minimum.unwrap_or(constraints.step),
            None => constraints.maximum.unwrap_or(-constraints.step),
        };
        if let Some(minimum) = constraints.minimum {
            next = next.max(minimum);
        }
        if let Some(maximum) = constraints.maximum {
            next = next.min(maximum);
        }
        self.set_focused_value(&format_number(next))
    }

    fn step_focused_value(&mut self, forward: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let kind = self.controls[index].kind;
        if matches!(kind, ControlKind::Number | ControlKind::Range) {
            return self.step_focused_numeric(forward);
        }
        if self.controls[index].disabled || self.controls[index].readonly || !kind.is_temporal() {
            return None;
        }
        let binding = &self.bindings[index];
        let constraints = typed_constraints(
            kind,
            binding.min.as_deref(),
            binding.max.as_deref(),
            binding.step.as_deref(),
        );
        let value = step_typed_value(kind, &self.controls[index].value, constraints, forward)?;
        self.set_focused_value(&value)
    }

    fn set_focused_typed_boundary(&mut self, maximum: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let kind = self.controls[index].kind;
        if !kind.is_temporal() {
            return None;
        }
        let binding = &self.bindings[index];
        let constraints = typed_constraints(
            kind,
            binding.min.as_deref(),
            binding.max.as_deref(),
            binding.step.as_deref(),
        );
        let scalar = if maximum {
            constraints.maximum?
        } else {
            constraints.minimum?
        };
        let value = format_typed_value(kind, scalar)?;
        self.set_focused_value(&value)
    }

    fn set_focused_range_boundary(&mut self, maximum: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        if self.controls[index].kind != ControlKind::Range {
            return None;
        }
        let constraints = control_numeric_constraints(&self.controls[index], &self.bindings[index]);
        let value = if maximum {
            constraints.maximum.unwrap_or(100.0)
        } else {
            constraints.minimum.unwrap_or(0.0)
        };
        self.set_focused_value(&format_number(value))
    }

    fn move_select(
        &mut self,
        forward: bool,
        boundary: bool,
        extend: bool,
    ) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        let option = if boundary {
            enabled_boundary_option(control, forward)
        } else {
            next_enabled_option(control, control.selected_index, forward)
        }?;
        let key = control.key.clone();
        let multiple = control.multiple;
        if extend && multiple && self.choice_anchors[index].is_none() {
            self.choice_anchors[index] = Some(control.selected_index);
        }
        self.select_option(&key, option, extend && multiple, false)
    }

    fn move_radio_group(&mut self, forward: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let name = self.controls[index].name.clone();
        let owner = self.bindings[index].form_owner.clone();
        let form_index = self.bindings[index].form_index;
        let group = self
            .controls
            .iter()
            .zip(&self.bindings)
            .enumerate()
            .filter_map(|(candidate, (control, binding))| {
                let same_form = match owner.as_deref() {
                    Some(owner) => binding.form_owner.as_deref() == Some(owner),
                    None => binding.form_owner.is_none() && binding.form_index == form_index,
                };
                (same_form
                    && control.kind == ControlKind::Radio
                    && control.name == name
                    && !control.disabled)
                    .then_some(candidate)
            })
            .collect::<Vec<_>>();
        let position = group.iter().position(|candidate| *candidate == index)?;
        let target = if forward {
            group[(position + 1) % group.len()]
        } else {
            group[(position + group.len() - 1) % group.len()]
        };
        for candidate in &group {
            self.controls[*candidate].checked = false;
            self.controls[*candidate].focused = false;
            self.dirty_checkedness[*candidate] = true;
        }
        self.controls[target].checked = true;
        self.dirty_checkedness[target] = true;
        self.controls[target].focused = true;
        self.focused_key = Some(self.controls[target].key.clone());
        Some(ControlEffect::CheckedChanged {
            key: self.controls[target].key.clone(),
            checked: true,
        })
    }

    fn delete_from_focused(&mut self, backward: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if control.disabled || control.readonly || !control.kind.accepts_text() {
            return None;
        }
        let (mut start, mut end) = self.editors[index].selection.ordered();
        if start == end {
            if backward {
                if start == 0 {
                    return None;
                }
                start = previous_grapheme_offset(&control.value, start);
            } else {
                if end >= control.value.chars().count() {
                    return None;
                }
                end = next_grapheme_offset(&control.value, end);
            }
        }
        self.editors[index].selection = ControlSelection {
            anchor: start,
            focus: end,
        };
        self.replace_focused_selection("")
    }

    fn move_caret(&mut self, delta: isize, extend: bool, home: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        let current = self.editors[index].selection;
        let focus = if delta < 0 && !extend && !current.is_collapsed() {
            current.ordered().0
        } else if delta > 0 && !extend && !current.is_collapsed() {
            current.ordered().1
        } else if home {
            line_boundary(&control.value, current.focus, true)
        } else if delta == 0 {
            line_boundary(&control.value, current.focus, false)
        } else {
            if delta < 0 {
                previous_grapheme_offset(&control.value, current.focus)
            } else {
                next_grapheme_offset(&control.value, current.focus)
            }
        };
        let selection = if extend {
            ControlSelection {
                anchor: current.anchor,
                focus,
            }
        } else {
            ControlSelection::collapsed(focus)
        };
        self.editors[index].selection = selection;
        self.editors[index].caret_phase_ms = 0;
        Some(ControlEffect::SelectionChanged {
            key: control.key.clone(),
            selection,
        })
    }

    fn move_by_unit(
        &mut self,
        forward: bool,
        unit: ControlNavigationUnit,
        extend: bool,
    ) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if !control.kind.accepts_text() {
            return None;
        }
        let current = self.editors[index].selection;
        let length = control.value.chars().count();
        let focus = if !extend && !current.is_collapsed() {
            if forward {
                current.ordered().1
            } else {
                current.ordered().0
            }
        } else {
            match unit {
                ControlNavigationUnit::Grapheme => {
                    if forward {
                        next_grapheme_offset(&control.value, current.focus)
                    } else {
                        previous_grapheme_offset(&control.value, current.focus)
                    }
                }
                ControlNavigationUnit::Word => {
                    word_navigation_offset(&control.value, current.focus, forward)
                }
                ControlNavigationUnit::Line => {
                    line_boundary(&control.value, current.focus, !forward)
                }
                ControlNavigationUnit::Document => {
                    if forward {
                        length
                    } else {
                        0
                    }
                }
            }
        };
        let selection = if extend {
            ControlSelection {
                anchor: current.anchor,
                focus,
            }
        } else {
            ControlSelection::collapsed(focus)
        };
        self.editors[index].selection = selection;
        self.editors[index].caret_phase_ms = 0;
        Some(ControlEffect::SelectionChanged {
            key: control.key.clone(),
            selection,
        })
    }

    fn record_history(&mut self, index: usize) {
        let snapshot = ControlEditSnapshot::from_parts(&self.controls[index], &self.editors[index]);
        let history = &mut self.histories[index];
        if history.undo.last() != Some(&snapshot) {
            history.undo.push(snapshot);
            if history.undo.len() > EDIT_HISTORY_LIMIT {
                history.undo.remove(0);
            }
        }
        history.redo.clear();
    }

    fn restore_history(&mut self, undo: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        if self.controls[index].disabled
            || self.controls[index].readonly
            || !self.controls[index].kind.accepts_text()
        {
            return None;
        }
        let current = ControlEditSnapshot::from_parts(&self.controls[index], &self.editors[index]);
        let history = &mut self.histories[index];
        let target = if undo {
            let target = history.undo.pop()?;
            history.redo.push(current);
            target
        } else {
            let target = history.redo.pop()?;
            history.undo.push(current);
            target
        };
        self.controls[index].value = target.value;
        self.dirty_values[index] = true;
        self.editors[index].selection = target.selection;
        self.editors[index].composition = None;
        self.editors[index].composition_range = None;
        self.editors[index].caret_phase_ms = 0;
        self.editors[index].invalid_message = None;
        Some(ControlEffect::ValueChanged {
            key: self.controls[index].key.clone(),
            value: self.controls[index].value.clone(),
        })
    }

    fn activate_focused(&mut self) -> Option<ControlEffect> {
        let index = self.controls.iter().position(|control| {
            self.focused_key.as_deref() == Some(control.key.as_str()) && !control.disabled
        })?;
        match self.controls[index].kind {
            ControlKind::Checkbox => {
                self.controls[index].indeterminate = false;
                self.controls[index].checked = !self.controls[index].checked;
                self.dirty_checkedness[index] = true;
                Some(ControlEffect::CheckedChanged {
                    key: self.controls[index].key.clone(),
                    checked: self.controls[index].checked,
                })
            }
            ControlKind::Radio => {
                let name = self.controls[index].name.clone();
                let owner = self.bindings[index].form_owner.clone();
                let form_index = self.bindings[index].form_index;
                for (candidate, (control, binding)) in
                    self.controls.iter_mut().zip(&self.bindings).enumerate()
                {
                    let same_form = match owner.as_deref() {
                        Some(owner) => binding.form_owner.as_deref() == Some(owner),
                        None => binding.form_owner.is_none() && binding.form_index == form_index,
                    };
                    if same_form && control.kind == ControlKind::Radio && control.name == name {
                        control.checked = false;
                        self.dirty_checkedness[candidate] = true;
                    }
                }
                self.controls[index].checked = true;
                self.dirty_checkedness[index] = true;
                Some(ControlEffect::CheckedChanged {
                    key: self.controls[index].key.clone(),
                    checked: true,
                })
            }
            ControlKind::Button => Some(ControlEffect::Activated(self.controls[index].key.clone())),
            ControlKind::File => self
                .file_picker_request(&self.controls[index].key)
                .map(ControlEffect::FilePickerRequested),
            ControlKind::Select => {
                if self.controls[index].options.is_empty() {
                    return None;
                }
                let active = self.controls[index].selected_index;
                let option = if self.controls[index].multiple {
                    Some(active)
                } else {
                    next_enabled_option(&self.controls[index], active, true)
                }?;
                let key = self.controls[index].key.clone();
                self.select_option(&key, option, false, self.controls[index].multiple)
            }
            _ => None,
        }
    }
}

pub fn control_states(tree: &BrowserRenderTree) -> Vec<ControlState> {
    let mut states = Vec::new();
    let mut index = 0;
    collect_nodes(&tree.children, &mut states, &mut index, false);
    states
}

pub fn control_key(node: &BrowserRenderNode, index: usize) -> String {
    node.id
        .as_ref()
        .map(|id| format!("control:{index}:id:{id}"))
        .unwrap_or_else(|| format!("control:{index}"))
}

pub fn project_control(node: &BrowserRenderNode, key: impl Into<String>) -> ControlState {
    let kind = control_kind(node);
    let mut state = ControlState::new(key, kind);
    state.name = node.control_name.clone();
    state.value = match kind {
        ControlKind::Button => node
            .accessible_name
            .clone()
            .or_else(|| node.text.clone())
            .or_else(|| node.value.clone())
            .unwrap_or_else(|| "Submit".into()),
        ControlKind::TextArea => node
            .value
            .clone()
            .or_else(|| node.text.clone())
            .unwrap_or_default(),
        ControlKind::File => String::new(),
        _ => node.value.clone().unwrap_or_default(),
    };
    state.placeholder = node.placeholder.clone();
    let choice_options = collect_choice_options(&node.children);
    state.options = if choice_options.is_empty() {
        node.options.clone()
    } else {
        choice_options
            .iter()
            .map(|option| option.value.clone())
            .collect()
    };
    state.option_disabled = choice_options
        .iter()
        .map(|option| option.disabled)
        .collect();
    state.multiple = node.multiple;
    state.selected_indices = choice_options
        .iter()
        .enumerate()
        .filter_map(|(index, option)| option.selected.then_some(index))
        .collect();
    state.selected_index = state
        .selected_indices
        .first()
        .copied()
        .or_else(|| {
            node.value
                .as_ref()
                .and_then(|value| state.options.iter().position(|option| option == value))
        })
        .unwrap_or_else(|| enabled_boundary_option(&state, true).unwrap_or(0));
    if kind == ControlKind::Select && state.selected_indices.is_empty() && !state.multiple {
        state.selected_indices.push(state.selected_index);
    }
    state.columns = node
        .cols
        .as_deref()
        .or(node.size.as_deref())
        .and_then(positive_count)
        .unwrap_or(20);
    state.rows = node
        .rows
        .as_deref()
        .and_then(positive_count)
        .unwrap_or(if kind == ControlKind::TextArea { 2 } else { 1 });
    state.disabled = node.disabled || node.aria_disabled.as_deref() == Some("true");
    state.readonly = node.readonly;
    state.required = node.required || node.aria_required.as_deref() == Some("true");
    state.checked = node.checked;
    state.focused = node.control_focused;
    state.appearance = ControlAppearance::Auto;
    if kind == ControlKind::Range {
        let constraints = range_constraints(
            node.min.as_deref(),
            node.max.as_deref(),
            node.step.as_deref(),
        );
        let initial = state
            .value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .unwrap_or((constraints.minimum.unwrap() + constraints.maximum.unwrap()) / 2.0);
        state.value = format_number(normalize_range_value(initial, constraints));
    } else if kind.is_temporal() {
        state.value = parse_typed_value(kind, &state.value)
            .map(|value| value.normalized)
            .unwrap_or_default();
    } else if kind == ControlKind::Color {
        state.value = normalize_color(&state.value).unwrap_or_else(|| "#000000".into());
    }
    state
}

fn collect_nodes(
    nodes: &[BrowserRenderNode],
    states: &mut Vec<ControlState>,
    index: &mut usize,
    inherited_disabled: bool,
) {
    for node in nodes {
        if node.role == "control" && node.control_type.as_deref() != Some("hidden") && !node.hidden
        {
            let key = control_key(node, *index);
            *index += 1;
            let mut state = project_control(node, key);
            state.disabled |= inherited_disabled;
            states.push(state);
        }
        let first_legend = disabled_fieldset_first_legend(node);
        for (child_index, child) in node.children.iter().enumerate() {
            collect_nodes(
                std::slice::from_ref(child),
                states,
                index,
                inherited_disabled
                    || (node.name.as_deref() == Some("fieldset")
                        && node.disabled
                        && Some(child_index) != first_legend),
            );
        }
    }
}

fn disabled_fieldset_first_legend(node: &BrowserRenderNode) -> Option<usize> {
    (node.name.as_deref() == Some("fieldset") && node.disabled)
        .then(|| {
            node.children
                .iter()
                .position(|child| child.name.as_deref() == Some("legend"))
        })
        .flatten()
}

struct ModelCollection<'a> {
    states: &'a mut Vec<ControlState>,
    bindings: &'a mut Vec<ControlBinding>,
    document_controls: &'a mut Vec<DocumentControlBinding>,
    control_index: &'a mut usize,
    next_form_index: &'a mut usize,
    document_order: &'a mut usize,
    form_autocomplete: &'a [FormAutocompleteBinding],
}

fn collect_model_nodes(
    nodes: &[BrowserRenderNode],
    collection: &mut ModelCollection<'_>,
    containing_form: Option<usize>,
    inherited_direction: ControlTextDirection,
    inherited_disabled: bool,
) {
    for node in nodes {
        let node_order = *collection.document_order;
        *collection.document_order += 1;
        let (node_direction, direction_auto) = node_direction(node, inherited_direction);
        let containing_form = if node.name.as_deref() == Some("form") {
            let index = *collection.next_form_index;
            *collection.next_form_index += 1;
            Some(index)
        } else {
            containing_form
        };
        if node.role == "control" {
            let control_type = node
                .control_type
                .clone()
                .unwrap_or_else(|| node.name.clone().unwrap_or_else(|| "text".into()));
            collection.document_controls.push(DocumentControlBinding {
                id: node.id.clone(),
                name: node.control_name.clone(),
                control_type: control_type.clone(),
                form_owner: node.form_owner.clone(),
                form_index: containing_form,
                document_order: node_order,
            });
        }
        if node.role == "control" && node.control_type.as_deref() != Some("hidden") && !node.hidden
        {
            let key = control_key(node, *collection.control_index);
            *collection.control_index += 1;
            let mut state = project_control(node, key.clone());
            state.disabled |= inherited_disabled;
            collection.states.push(state);
            collection.bindings.push(ControlBinding {
                key,
                id: node.id.clone(),
                control_type: node
                    .control_type
                    .clone()
                    .unwrap_or_else(|| node.name.clone().unwrap_or_else(|| "text".into())),
                form_owner: node.form_owner.clone(),
                form_index: containing_form,
                form_action: node.form_action.clone(),
                resolved_form_action: node.resolved_form_action.clone(),
                form_enctype: node.form_enctype.clone(),
                form_method: node.form_method.clone(),
                form_novalidate: node.form_novalidate,
                pattern: node.pattern.clone(),
                min: node.min.clone(),
                max: node.max.clone(),
                minlength: node.minlength.clone(),
                maxlength: node.maxlength.clone(),
                step: node.step.clone(),
                inputmode: node.inputmode.clone(),
                accept: node.accept.clone(),
                datalist_options: node.datalist_options.clone(),
                autocomplete: node.autocomplete.clone(),
                autocomplete_tokens: autocomplete_tokens(node.autocomplete.as_deref()),
                form_autocomplete_enabled: effective_form_autocomplete(
                    collection.form_autocomplete,
                    node.form_owner.as_deref(),
                    containing_form,
                ),
                dirname: node.dirname.clone(),
                direction: if direction_auto {
                    inherited_direction
                } else {
                    node_direction
                },
                direction_auto,
                document_order: node_order,
            });
        }
        let first_legend = disabled_fieldset_first_legend(node);
        for (child_index, child) in node.children.iter().enumerate() {
            collect_model_nodes(
                std::slice::from_ref(child),
                collection,
                containing_form,
                node_direction,
                inherited_disabled
                    || (node.name.as_deref() == Some("fieldset")
                        && node.disabled
                        && Some(child_index) != first_legend),
            );
        }
    }
}

fn collect_live_value_nodes(
    nodes: &[BrowserRenderNode],
    states: &mut Vec<LiveValueState>,
    index: &mut usize,
    containing_form: Option<usize>,
    next_form_index: &mut usize,
) {
    for node in nodes {
        let containing_form = if node.name.as_deref() == Some("form") {
            let form_index = *next_form_index;
            *next_form_index += 1;
            Some(form_index)
        } else {
            containing_form
        };
        let kind = match node.name.as_deref() {
            Some("output") => Some(LiveValueKind::Output),
            Some("meter") => Some(LiveValueKind::Meter),
            Some("progress") => Some(LiveValueKind::Progress),
            _ => None,
        };
        if let Some(kind) = kind {
            let key = node
                .id
                .as_ref()
                .map(|id| format!("live:{}:id:{id}", *index))
                .unwrap_or_else(|| format!("live:{}", *index));
            *index += 1;
            let base = LiveValueState {
                key,
                id: node.id.clone(),
                kind,
                form_owner: node.form_owner.clone(),
                form_index: containing_form,
                text: if kind == LiveValueKind::Output {
                    node.text.clone().unwrap_or_default()
                } else {
                    node.value
                        .clone()
                        .or_else(|| node.text.clone())
                        .unwrap_or_default()
                },
                value: None,
                minimum: parse_live_finite(node.min.as_deref()),
                maximum: parse_live_finite(node.max.as_deref()),
                low: parse_live_finite(node.low.as_deref()),
                high: parse_live_finite(node.high.as_deref()),
                optimum: parse_live_finite(node.optimum.as_deref()),
                position: None,
                indeterminate: false,
                meter_region: None,
                dependencies: node
                    .output_for
                    .iter()
                    .map(|id| OutputDependencyValue {
                        id: id.clone(),
                        key: None,
                        value: String::new(),
                    })
                    .collect(),
                accessible_name: node.accessible_name.clone(),
                accessible_description: node.accessible_description.clone(),
                value_text: String::new(),
                diagnostics: Vec::new(),
            };
            let mut state = match kind {
                LiveValueKind::Output => LiveValueState {
                    value_text: base.text.clone(),
                    ..base
                },
                LiveValueKind::Meter => normalized_meter_state(&base, node.value.as_deref()),
                LiveValueKind::Progress => normalized_progress_state(&base, node.value.as_deref()),
            };
            append_live_constraint_diagnostics(&mut state, node);
            states.push(state);
        }
        collect_live_value_nodes(
            &node.children,
            states,
            index,
            containing_form,
            next_form_index,
        );
    }
}

fn append_live_constraint_diagnostics(state: &mut LiveValueState, node: &BrowserRenderNode) {
    for (value, code, message) in [
        (
            node.min.as_deref(),
            "invalid-minimum",
            "minimum must be a finite number",
        ),
        (
            node.max.as_deref(),
            "invalid-maximum",
            "maximum must be a finite number",
        ),
        (
            node.low.as_deref(),
            "invalid-low",
            "low boundary must be a finite number",
        ),
        (
            node.high.as_deref(),
            "invalid-high",
            "high boundary must be a finite number",
        ),
        (
            node.optimum.as_deref(),
            "invalid-optimum",
            "optimum must be a finite number",
        ),
    ] {
        if value.is_some() && parse_live_finite(value).is_none() {
            state
                .diagnostics
                .push(ControlValueDiagnostic { code, message });
        }
    }
    if state.kind == LiveValueKind::Progress
        && node.max.is_some()
        && parse_live_finite(node.max.as_deref()).is_some_and(|maximum| maximum <= 0.0)
    {
        state.diagnostics.push(ControlValueDiagnostic {
            code: "non-positive-progress-maximum",
            message: "progress maximum must be greater than zero",
        });
    }
    if state.kind == LiveValueKind::Meter {
        if parse_live_finite(node.min.as_deref())
            .zip(parse_live_finite(node.max.as_deref()))
            .is_some_and(|(minimum, maximum)| maximum < minimum)
        {
            state.diagnostics.push(ControlValueDiagnostic {
                code: "invalid-meter-range",
                message: "meter maximum cannot be less than its minimum",
            });
        }
        if parse_live_finite(node.low.as_deref())
            .zip(parse_live_finite(node.high.as_deref()))
            .is_some_and(|(low, high)| high < low)
        {
            state.diagnostics.push(ControlValueDiagnostic {
                code: "invalid-meter-threshold-order",
                message: "meter high boundary cannot be less than its low boundary",
            });
        }
    }
}

fn parse_live_finite(value: Option<&str>) -> Option<f64> {
    value?
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn normalized_meter_state(base: &LiveValueState, value: Option<&str>) -> LiveValueState {
    let mut state = base.clone();
    state
        .diagnostics
        .retain(|diagnostic| diagnostic.code != "invalid-meter-value");
    let minimum = state.minimum.unwrap_or(0.0);
    let maximum = state.maximum.unwrap_or(1.0).max(minimum);
    let low = state.low.unwrap_or(minimum).clamp(minimum, maximum);
    let high = state.high.unwrap_or(maximum).clamp(low, maximum);
    let optimum = state
        .optimum
        .unwrap_or((minimum + maximum) / 2.0)
        .clamp(minimum, maximum);
    let parsed = parse_live_finite(value);
    if value.is_some() && parsed.is_none() {
        state.diagnostics.push(ControlValueDiagnostic {
            code: "invalid-meter-value",
            message: "meter value must be a finite number",
        });
    }
    let current = parsed.unwrap_or(0.0).clamp(minimum, maximum);
    state.minimum = Some(minimum);
    state.maximum = Some(maximum);
    state.low = Some(low);
    state.high = Some(high);
    state.optimum = Some(optimum);
    state.value = Some(current);
    state.position = (maximum > minimum).then_some((current - minimum) / (maximum - minimum));
    state.indeterminate = false;
    state.meter_region = Some(meter_value_region(current, low, high, optimum));
    state.value_text = format!("{} of {}", format_number(current), format_number(maximum));
    state
}

fn meter_value_region(value: f64, low: f64, high: f64, optimum: f64) -> MeterValueRegion {
    if optimum < low {
        if value <= low {
            MeterValueRegion::Optimum
        } else if value <= high {
            MeterValueRegion::Suboptimal
        } else {
            MeterValueRegion::EvenLessGood
        }
    } else if optimum > high {
        if value >= high {
            MeterValueRegion::Optimum
        } else if value >= low {
            MeterValueRegion::Suboptimal
        } else {
            MeterValueRegion::EvenLessGood
        }
    } else if (low..=high).contains(&value) {
        MeterValueRegion::Optimum
    } else {
        MeterValueRegion::Suboptimal
    }
}

fn normalized_progress_state(base: &LiveValueState, value: Option<&str>) -> LiveValueState {
    let mut state = base.clone();
    state
        .diagnostics
        .retain(|diagnostic| diagnostic.code != "invalid-progress-value");
    let maximum = state.maximum.filter(|value| *value > 0.0).unwrap_or(1.0);
    state.minimum = Some(0.0);
    state.maximum = Some(maximum);
    state.low = None;
    state.high = None;
    state.optimum = None;
    state.meter_region = None;
    match value {
        None => {
            state.value = None;
            state.position = None;
            state.indeterminate = true;
            state.value_text = "indeterminate".to_string();
        }
        Some(value) => {
            let parsed = parse_live_finite(Some(value));
            if parsed.is_none() {
                state.diagnostics.push(ControlValueDiagnostic {
                    code: "invalid-progress-value",
                    message: "progress value must be a finite number",
                });
            }
            let current = parsed.unwrap_or(0.0).clamp(0.0, maximum);
            state.value = Some(current);
            state.position = Some(current / maximum);
            state.indeterminate = false;
            state.value_text = format!("{} of {}", format_number(current), format_number(maximum));
        }
    }
    state
}

fn collect_form_autocomplete_bindings(nodes: &[BrowserRenderNode]) -> Vec<FormAutocompleteBinding> {
    fn collect(
        nodes: &[BrowserRenderNode],
        bindings: &mut Vec<FormAutocompleteBinding>,
        next_form_index: &mut usize,
    ) {
        for node in nodes {
            if node.name.as_deref() == Some("form") {
                let form_index = *next_form_index;
                *next_form_index += 1;
                bindings.push(FormAutocompleteBinding {
                    id: node.id.clone(),
                    form_index,
                    enabled: !autocomplete_is_off(node.autocomplete.as_deref()),
                });
            }
            collect(&node.children, bindings, next_form_index);
        }
    }

    let mut bindings = Vec::new();
    let mut next_form_index = 0;
    collect(nodes, &mut bindings, &mut next_form_index);
    bindings
}

fn effective_form_autocomplete(
    forms: &[FormAutocompleteBinding],
    form_owner: Option<&str>,
    form_index: Option<usize>,
) -> bool {
    forms
        .iter()
        .find(|form| match form_owner {
            Some(owner) => form.id.as_deref() == Some(owner),
            None => Some(form.form_index) == form_index,
        })
        .is_none_or(|form| form.enabled)
}

fn autocomplete_tokens(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect()
}

fn autocomplete_is_off(value: Option<&str>) -> bool {
    let tokens = autocomplete_tokens(value);
    tokens.len() == 1 && tokens[0] == "off"
}

fn autofill_descriptor(
    control: &ControlState,
    binding: &ControlBinding,
) -> Option<ControlAutofillDescriptor> {
    if matches!(
        control.kind,
        ControlKind::Button | ControlKind::File | ControlKind::Checkbox | ControlKind::Radio
    ) {
        return None;
    }
    let tokens = &binding.autocomplete_tokens;
    let section = tokens
        .iter()
        .find(|token| token.starts_with("section-"))
        .cloned();
    let address_type = tokens
        .iter()
        .find(|token| matches!(token.as_str(), "shipping" | "billing"))
        .cloned();
    let contact_type = tokens
        .iter()
        .find(|token| matches!(token.as_str(), "home" | "work" | "mobile" | "fax" | "pager"))
        .cloned();
    let purpose = tokens
        .iter()
        .rev()
        .find(|token| is_autofill_purpose(token))
        .cloned()
        .unwrap_or_else(|| infer_autofill_purpose(control, binding));
    if purpose.is_empty() {
        return None;
    }
    let sensitive = control.kind == ControlKind::Password
        || matches!(
            purpose.as_str(),
            "current-password" | "new-password" | "one-time-code" | "cc-number" | "cc-csc"
        );
    Some(ControlAutofillDescriptor {
        key: control.key.clone(),
        section,
        address_type,
        contact_type,
        purpose,
        enabled: binding.form_autocomplete_enabled
            && !autocomplete_is_off(binding.autocomplete.as_deref())
            && !control.disabled
            && !control.readonly,
        sensitive,
        document_order: binding.document_order,
    })
}

fn is_autofill_purpose(token: &str) -> bool {
    matches!(
        token,
        "name"
            | "honorific-prefix"
            | "given-name"
            | "additional-name"
            | "family-name"
            | "honorific-suffix"
            | "nickname"
            | "username"
            | "new-password"
            | "current-password"
            | "one-time-code"
            | "organization-title"
            | "organization"
            | "street-address"
            | "address-line1"
            | "address-line2"
            | "address-line3"
            | "address-level4"
            | "address-level3"
            | "address-level2"
            | "address-level1"
            | "country"
            | "country-name"
            | "postal-code"
            | "cc-name"
            | "cc-given-name"
            | "cc-additional-name"
            | "cc-family-name"
            | "cc-number"
            | "cc-exp"
            | "cc-exp-month"
            | "cc-exp-year"
            | "cc-csc"
            | "cc-type"
            | "transaction-currency"
            | "transaction-amount"
            | "language"
            | "bday"
            | "bday-day"
            | "bday-month"
            | "bday-year"
            | "sex"
            | "url"
            | "photo"
            | "tel"
            | "tel-country-code"
            | "tel-national"
            | "tel-area-code"
            | "tel-local"
            | "tel-local-prefix"
            | "tel-local-suffix"
            | "tel-extension"
            | "email"
            | "impp"
            | "search"
    )
}

fn infer_autofill_purpose(control: &ControlState, binding: &ControlBinding) -> String {
    let name = control
        .name
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    for purpose in [
        "given-name",
        "family-name",
        "street-address",
        "postal-code",
        "organization",
        "username",
        "email",
        "tel",
        "search",
        "url",
        "name",
    ] {
        if name == purpose || name.contains(purpose) {
            return purpose.to_string();
        }
    }
    match binding.control_type.as_str() {
        "email" | "tel" | "url" | "search" => binding.control_type.clone(),
        "password" => "current-password".to_string(),
        _ => String::new(),
    }
}

fn apply_autofill_value(control: &mut ControlState, binding: &ControlBinding, value: &str) -> bool {
    let normalized = match control.kind {
        ControlKind::Button | ControlKind::File | ControlKind::Checkbox | ControlKind::Radio => {
            return false
        }
        ControlKind::Select => {
            let Some(index) = control
                .options
                .iter()
                .enumerate()
                .find_map(|(index, option)| {
                    (option == value && !option_is_disabled(control, index)).then_some(index)
                })
            else {
                return false;
            };
            if control.selected_indices == [index] && control.selected_index == index {
                return false;
            }
            control.selected_index = index;
            control.selected_indices = vec![index];
            sync_selected_value(control);
            return true;
        }
        ControlKind::Range => {
            let Some(value) = parse_finite(value) else {
                return false;
            };
            format_number(normalize_range_value(
                value,
                control_numeric_constraints(control, binding),
            ))
        }
        kind if kind.is_temporal() => {
            let Some(value) = parse_typed_value(kind, value) else {
                return false;
            };
            value.normalized
        }
        ControlKind::Color => {
            let Some(value) = normalize_color(value) else {
                return false;
            };
            value
        }
        ControlKind::Number => {
            let Some(value) = parse_finite(value) else {
                return false;
            };
            format_number(value)
        }
        _ => binding
            .maxlength
            .as_deref()
            .and_then(|maximum| maximum.parse::<usize>().ok())
            .filter(|_| control.kind.supports_maxlength())
            .map_or_else(
                || value.to_string(),
                |maximum| value.chars().take(maximum).collect(),
            ),
    };
    if control.value == normalized {
        return false;
    }
    control.value = normalized;
    true
}

fn suggestions_enabled(control: &ControlState, binding: &ControlBinding) -> bool {
    !binding.datalist_options.is_empty()
        && !control.disabled
        && !control.readonly
        && !matches!(
            control.kind,
            ControlKind::Button
                | ControlKind::File
                | ControlKind::Checkbox
                | ControlKind::Radio
                | ControlKind::Select
                | ControlKind::TextArea
                | ControlKind::Password
        )
}

fn build_suggestion_state(
    control: &ControlState,
    binding: &ControlBinding,
    query: &str,
    requested_limit: usize,
) -> ControlSuggestionState {
    let (query, query_truncated) = bounded_utf8(query, MAX_SUGGESTION_QUERY_BYTES);
    let folded_query = query.to_lowercase();
    let limit = requested_limit.min(MAX_SUGGESTION_RESULTS);
    let mut options = Vec::new();
    let mut diagnostics = Vec::new();
    if query_truncated {
        diagnostics.push(ControlSuggestionDiagnostic {
            code: "query-truncated",
            message: "suggestion query exceeds the shared byte limit",
        });
    }
    if binding.datalist_options.len() > MAX_DATALIST_OPTIONS {
        diagnostics.push(ControlSuggestionDiagnostic {
            code: "source-option-limit",
            message: "datalist exceeds the shared source option limit",
        });
    }
    let mut result_limited = false;
    for (source_index, option) in binding
        .datalist_options
        .iter()
        .take(MAX_DATALIST_OPTIONS)
        .enumerate()
    {
        if option.disabled {
            continue;
        }
        let Some(value) = normalize_suggestion_value(control, binding, &option.value) else {
            continue;
        };
        let matches_query = folded_query.is_empty()
            || value.to_lowercase().contains(&folded_query)
            || option
                .label
                .as_deref()
                .is_some_and(|label| label.to_lowercase().contains(&folded_query))
            || option.text.to_lowercase().contains(&folded_query);
        if !matches_query
            || options
                .iter()
                .any(|candidate: &ControlSuggestionOption| candidate.value == value)
        {
            continue;
        }
        if options.len() >= limit {
            result_limited = true;
            break;
        }
        options.push(ControlSuggestionOption {
            value,
            label: option.label.clone(),
            text: option.text.clone(),
            source_index,
        });
    }
    if result_limited {
        diagnostics.push(ControlSuggestionDiagnostic {
            code: "result-limit",
            message: "suggestion results exceed the requested bounded limit",
        });
    }
    let open = !options.is_empty();
    ControlSuggestionState {
        key: control.key.clone(),
        open,
        query,
        active_index: open.then_some(0),
        options,
        diagnostics,
    }
}

fn normalize_suggestion_value(
    control: &ControlState,
    binding: &ControlBinding,
    value: &str,
) -> Option<String> {
    let normalized = match control.kind {
        ControlKind::Range => {
            let value = parse_finite(value)?;
            format_number(normalize_range_value(
                value,
                control_numeric_constraints(control, binding),
            ))
        }
        kind if kind.is_temporal() => parse_typed_value(kind, value)?.normalized,
        ControlKind::Color => normalize_color(value)?,
        ControlKind::Number => format_number(parse_finite(value)?),
        _ => binding
            .maxlength
            .as_deref()
            .and_then(|maximum| maximum.parse::<usize>().ok())
            .filter(|_| control.kind.supports_maxlength())
            .map_or_else(
                || value.to_string(),
                |maximum| value.chars().take(maximum).collect(),
            ),
    };
    let mut candidate = control.clone();
    candidate.value = normalized.clone();
    control_value_state(&candidate, binding)
        .diagnostics
        .is_empty()
        .then_some(normalized)
}

fn bounded_utf8(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_string(), false);
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_string(), true)
}

fn suggestion_picker_effect(state: &ControlSuggestionState) -> ControlEffect {
    ControlEffect::SuggestionPickerChanged {
        key: state.key.clone(),
        open: state.open,
        active_index: state.active_index,
    }
}

fn restoration_changed(control: &ControlState, entry: &ControlRestorationEntry) -> bool {
    control.value != entry.value
        || control.checked != entry.checked
        || control.indeterminate != entry.indeterminate
        || control.selected_indices != entry.selected_indices
        || control.selected_index != entry.selected_index
}

fn custom_restoration_state_bytes(state: &CustomElementFormValue) -> Option<usize> {
    match state {
        CustomElementFormValue::Text(value) => Some(value.len()),
        CustomElementFormValue::File(_) => None,
        CustomElementFormValue::Entries(entries) => {
            entries.iter().try_fold(0_usize, |bytes, entry| {
                let value_bytes = match &entry.value {
                    CustomElementFormEntryValue::Text(value) => value.len(),
                    CustomElementFormEntryValue::File(_) => return None,
                };
                Some(
                    bytes
                        .saturating_add(entry.name.len())
                        .saturating_add(value_bytes),
                )
            })
        }
    }
}

fn restore_control(control: &mut ControlState, entry: &ControlRestorationEntry) {
    control.value = entry.value.clone();
    control.checked = entry.checked;
    control.indeterminate = entry.indeterminate;
    if control.kind == ControlKind::Select {
        control.selected_indices = entry
            .selected_indices
            .iter()
            .copied()
            .filter(|index| *index < control.options.len() && !option_is_disabled(control, *index))
            .collect();
        control.selected_index = entry
            .selected_index
            .min(control.options.len().saturating_sub(1));
        sync_selected_value(control);
    }
}

fn reset_editor_after_value_change(editor: &mut ControlEditorState, length: usize) {
    editor.selection = ControlSelection::collapsed(length);
    editor.composition = None;
    editor.composition_range = None;
    editor.scroll_x = 0.0;
    editor.scroll_y = 0.0;
    editor.caret_phase_ms = 0;
    editor.pointer_anchor = None;
    editor.invalid_message = None;
}

fn effect_for_restored_control(control: &ControlState) -> ControlEffect {
    if matches!(control.kind, ControlKind::Checkbox | ControlKind::Radio) {
        ControlEffect::CheckedChanged {
            key: control.key.clone(),
            checked: control.checked,
        }
    } else {
        ControlEffect::ValueChanged {
            key: control.key.clone(),
            value: control.value.clone(),
        }
    }
}

fn node_direction(
    node: &BrowserRenderNode,
    inherited: ControlTextDirection,
) -> (ControlTextDirection, bool) {
    match node.dir.as_deref() {
        Some(direction) if direction.eq_ignore_ascii_case("ltr") => {
            (ControlTextDirection::Ltr, false)
        }
        Some(direction) if direction.eq_ignore_ascii_case("rtl") => {
            (ControlTextDirection::Rtl, false)
        }
        Some(direction) if direction.eq_ignore_ascii_case("auto") => (
            control_text_direction(&render_node_text(node)).unwrap_or(inherited),
            true,
        ),
        _ => (inherited, false),
    }
}

fn render_node_text(node: &BrowserRenderNode) -> String {
    let mut text = node
        .value
        .as_deref()
        .or(node.text.as_deref())
        .unwrap_or_default()
        .to_string();
    for child in &node.children {
        text.push_str(&render_node_text(child));
    }
    text
}

fn control_text_direction(value: &str) -> Option<ControlTextDirection> {
    first_strong_direction(value).map(|direction| match direction {
        Direction::Rtl => ControlTextDirection::Rtl,
        Direction::Ltr => ControlTextDirection::Ltr,
    })
}

fn sync_nodes(
    nodes: &mut [BrowserRenderNode],
    controls: &[ControlState],
    editors: &[ControlEditorState],
    index: &mut usize,
) {
    for node in nodes {
        if node.role == "control" && node.control_type.as_deref() != Some("hidden") && !node.hidden
        {
            if let Some(control) = controls.get(*index) {
                node.value = Some(control.value.clone());
                node.checked = control.checked;
                node.control_focused = control.focused;
                if control.kind == ControlKind::Select {
                    let mut option_index = 0;
                    sync_choice_option_nodes(
                        &mut node.children,
                        &control.selected_indices,
                        &mut option_index,
                    );
                }
                if let Some(editor) = editors.get(*index) {
                    node.aria_invalid = editor.invalid_message.as_ref().map(|_| "true".into());
                    node.accessible_description = match (
                        editor.accessible_description.as_deref(),
                        editor.invalid_message.as_deref(),
                    ) {
                        (Some(description), Some(message)) if !description.is_empty() => {
                            Some(format!("{description}. {message}"))
                        }
                        (_, Some(message)) => Some(message.to_string()),
                        (description, None) => description.map(str::to_string),
                    };
                }
            }
            *index += 1;
        }
        sync_nodes(&mut node.children, controls, editors, index);
    }
}

fn sync_live_value_nodes(
    nodes: &mut [BrowserRenderNode],
    states: &[LiveValueState],
    index: &mut usize,
) {
    for node in nodes {
        if matches!(node.name.as_deref(), Some("output" | "meter" | "progress")) {
            if let Some(state) = states.get(*index) {
                node.value = state
                    .value
                    .map(format_number)
                    .or_else(|| (state.kind == LiveValueKind::Output).then(|| state.text.clone()));
                node.text = Some(if state.kind == LiveValueKind::Output {
                    state.text.clone()
                } else {
                    state.value_text.clone()
                });
            }
            *index += 1;
        }
        sync_live_value_nodes(&mut node.children, states, index);
    }
}

fn control_descriptions(tree: &BrowserRenderTree) -> Vec<Option<String>> {
    fn collect(nodes: &[BrowserRenderNode], descriptions: &mut Vec<Option<String>>) {
        for node in nodes {
            if node.role == "control"
                && node.control_type.as_deref() != Some("hidden")
                && !node.hidden
            {
                descriptions.push(node.accessible_description.clone());
            }
            collect(&node.children, descriptions);
        }
    }

    let mut descriptions = Vec::new();
    collect(&tree.children, &mut descriptions);
    descriptions
}

fn replace_char_range(value: &mut String, start: usize, end: usize, replacement: &str) {
    let start = byte_index(value, start);
    let end = byte_index(value, end);
    value.replace_range(start..end, replacement);
}

fn byte_index(value: &str, character: usize) -> usize {
    value
        .char_indices()
        .nth(character)
        .map_or(value.len(), |(index, _)| index)
}

fn line_boundary(value: &str, character: usize, start: bool) -> usize {
    let characters = value.chars().collect::<Vec<_>>();
    let character = character.min(characters.len());
    if start {
        characters[..character]
            .iter()
            .rposition(|character| *character == '\n')
            .map_or(0, |index| index + 1)
    } else {
        characters[character..]
            .iter()
            .position(|character| *character == '\n')
            .map_or(characters.len(), |index| character + index)
    }
}

fn control_kind(node: &BrowserRenderNode) -> ControlKind {
    match node.name.as_deref() {
        Some("textarea") => ControlKind::TextArea,
        Some("select") => ControlKind::Select,
        Some("button") => ControlKind::Button,
        _ => node
            .control_type
            .as_deref()
            .and_then(ControlKind::parse)
            .unwrap_or(ControlKind::Text),
    }
}

#[derive(Clone, Debug)]
struct ChoiceOption {
    value: String,
    disabled: bool,
    selected: bool,
}

fn collect_choice_options(nodes: &[BrowserRenderNode]) -> Vec<ChoiceOption> {
    fn collect(nodes: &[BrowserRenderNode], inherited_disabled: bool, out: &mut Vec<ChoiceOption>) {
        for node in nodes {
            let disabled = inherited_disabled || node.disabled;
            if node.name.as_deref() == Some("option") {
                out.push(ChoiceOption {
                    value: node
                        .value
                        .clone()
                        .or_else(|| node.text.clone())
                        .unwrap_or_default(),
                    disabled,
                    selected: node.selected,
                });
            } else {
                collect(&node.children, disabled, out);
            }
        }
    }

    let mut options = Vec::new();
    collect(nodes, false, &mut options);
    options
}

fn sync_choice_option_nodes(
    nodes: &mut [BrowserRenderNode],
    selected_indices: &[usize],
    option_index: &mut usize,
) {
    for node in nodes {
        if node.name.as_deref() == Some("option") {
            node.selected = selected_indices.contains(option_index);
            *option_index += 1;
        } else {
            sync_choice_option_nodes(&mut node.children, selected_indices, option_index);
        }
    }
}

fn positive_count(value: &str) -> Option<usize> {
    value.parse().ok().filter(|value| *value > 0)
}

fn sync_selected_value(control: &mut ControlState) {
    control.value = selected_values(control)
        .into_iter()
        .next()
        .unwrap_or_default();
}

fn option_is_disabled(control: &ControlState, index: usize) -> bool {
    control.option_disabled.get(index).copied().unwrap_or(false)
}

fn selected_values(control: &ControlState) -> Vec<String> {
    control
        .selected_indices
        .iter()
        .filter(|index| !option_is_disabled(control, **index))
        .filter_map(|index| control.options.get(*index))
        .cloned()
        .collect()
}

fn enabled_boundary_option(control: &ControlState, first: bool) -> Option<usize> {
    let indices: Box<dyn Iterator<Item = usize>> = if first {
        Box::new(0..control.options.len())
    } else {
        Box::new((0..control.options.len()).rev())
    };
    indices
        .into_iter()
        .find(|index| !option_is_disabled(control, *index))
}

fn next_enabled_option(control: &ControlState, current: usize, forward: bool) -> Option<usize> {
    let length = control.options.len();
    if length == 0 {
        return None;
    }
    (1..=length)
        .map(|distance| {
            if forward {
                (current + distance) % length
            } else {
                (current + length - distance % length) % length
            }
        })
        .find(|index| !option_is_disabled(control, *index))
}

fn sanitize_metrics(metrics: ControlTextMetrics) -> ControlTextMetrics {
    ControlTextMetrics {
        advance: finite_positive(metrics.advance, 8.0),
        line_height: finite_positive(metrics.line_height, 18.0),
        inset_x: finite_non_negative(metrics.inset_x),
        inset_y: finite_non_negative(metrics.inset_y),
        caret_width: finite_positive(metrics.caret_width, 1.5),
    }
}

fn finite_positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

fn char_range(value: &str, start: usize, end: usize) -> String {
    value
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

fn constrained_replacement(
    control: &ControlState,
    binding: &ControlBinding,
    start: usize,
    end: usize,
    replacement: &str,
) -> String {
    let Some(maximum) = binding
        .maxlength
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|_| control.kind.supports_maxlength())
    else {
        return replacement.to_string();
    };
    let retained = control
        .value
        .chars()
        .count()
        .saturating_sub(end.saturating_sub(start));
    replacement
        .chars()
        .take(maximum.saturating_sub(retained))
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct NumericConstraints {
    minimum: Option<f64>,
    maximum: Option<f64>,
    step: f64,
    validates_step: bool,
}

fn numeric_constraints(binding: &ControlBinding) -> NumericConstraints {
    let minimum = binding.min.as_deref().and_then(parse_finite);
    let maximum = binding.max.as_deref().and_then(parse_finite);
    let authored_step = binding.step.as_deref().map(str::trim);
    let validates_step = authored_step != Some("any");
    let step = authored_step
        .and_then(parse_finite)
        .filter(|step| *step > 0.0)
        .unwrap_or(1.0);
    NumericConstraints {
        minimum,
        maximum,
        step,
        validates_step,
    }
}

fn range_constraints(
    minimum: Option<&str>,
    maximum: Option<&str>,
    step: Option<&str>,
) -> NumericConstraints {
    let minimum = minimum.and_then(parse_finite).unwrap_or(0.0);
    let maximum = maximum.and_then(parse_finite).unwrap_or(100.0).max(minimum);
    let validates_step = step.map(str::trim) != Some("any");
    let step = step
        .and_then(parse_finite)
        .filter(|step| *step > 0.0)
        .unwrap_or(1.0);
    NumericConstraints {
        minimum: Some(minimum),
        maximum: Some(maximum),
        step,
        validates_step,
    }
}

fn control_numeric_constraints(
    control: &ControlState,
    binding: &ControlBinding,
) -> NumericConstraints {
    if control.kind == ControlKind::Range {
        range_constraints(
            binding.min.as_deref(),
            binding.max.as_deref(),
            binding.step.as_deref(),
        )
    } else {
        numeric_constraints(binding)
    }
}

fn normalize_range_value(value: f64, constraints: NumericConstraints) -> f64 {
    let minimum = constraints.minimum.unwrap_or(0.0);
    let maximum = constraints.maximum.unwrap_or(100.0);
    let clamped = value.clamp(minimum, maximum);
    if constraints.validates_step {
        (minimum + ((clamped - minimum) / constraints.step).round() * constraints.step)
            .clamp(minimum, maximum)
    } else {
        clamped
    }
}

fn parse_finite(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn format_number(value: f64) -> String {
    let value = if value == -0.0 { 0.0 } else { value };
    value.to_string()
}

fn step_mismatch(value: f64, base: f64, step: f64) -> bool {
    let quotient = (value - base) / step;
    (quotient - quotient.round()).abs() > f64::EPSILON * quotient.abs().max(1.0) * 8.0
}

fn valid_email_address(value: &str) -> bool {
    let value = value.trim();
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
        && !value.chars().any(char::is_whitespace)
}

fn valid_url_value(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return false;
    }
    Url::parse(value).is_ok_and(|url| {
        url.host.as_deref().is_none_or(|host| {
            let bracketed = host.starts_with('[') || host.ends_with(']');
            if bracketed {
                host.strip_prefix('[')
                    .and_then(|host| host.strip_suffix(']'))
                    .is_some_and(|host| host.parse::<std::net::Ipv6Addr>().is_ok())
            } else {
                !host.is_empty() && !host.contains(['[', ']'])
            }
        })
    })
}

fn control_value_state(control: &ControlState, binding: &ControlBinding) -> ControlValueState {
    let mut diagnostics = Vec::new();
    let length = control.value.chars().count();
    if !control.value.is_empty() {
        if binding
            .minlength
            .as_deref()
            .and_then(|value| value.parse::<usize>().ok())
            .is_some_and(|minimum| length < minimum)
        {
            diagnostics.push(ControlValueDiagnostic {
                code: "too-short",
                message: "value is shorter than minlength",
            });
        }
        if binding
            .maxlength
            .as_deref()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|_| control.kind.supports_maxlength())
            .is_some_and(|maximum| length > maximum)
        {
            diagnostics.push(ControlValueDiagnostic {
                code: "too-long",
                message: "value is longer than maxlength",
            });
        }
    }

    match control.kind {
        ControlKind::Email if !control.value.is_empty() => {
            let valid = if control.multiple {
                control.value.split(',').all(valid_email_address)
            } else {
                valid_email_address(&control.value)
            };
            if !valid {
                diagnostics.push(ControlValueDiagnostic {
                    code: "type-mismatch",
                    message: "email value is malformed",
                });
            }
        }
        ControlKind::Url if !control.value.is_empty() && !valid_url_value(&control.value) => {
            diagnostics.push(ControlValueDiagnostic {
                code: "type-mismatch",
                message: "URL value is malformed",
            });
        }
        _ => {}
    }

    let numeric_constraints = control_numeric_constraints(control, binding);
    let mut numeric_value = if matches!(control.kind, ControlKind::Number | ControlKind::Range)
        && !control.value.is_empty()
    {
        match parse_finite(&control.value) {
            Some(value) => {
                if control.kind == ControlKind::Number
                    && numeric_constraints
                        .minimum
                        .is_some_and(|minimum| value < minimum)
                {
                    diagnostics.push(ControlValueDiagnostic {
                        code: "range-underflow",
                        message: "number is below min",
                    });
                }
                if control.kind == ControlKind::Number
                    && numeric_constraints
                        .maximum
                        .is_some_and(|maximum| value > maximum)
                {
                    diagnostics.push(ControlValueDiagnostic {
                        code: "range-overflow",
                        message: "number is above max",
                    });
                }
                if control.kind == ControlKind::Number
                    && numeric_constraints.validates_step
                    && step_mismatch(
                        value,
                        numeric_constraints.minimum.unwrap_or(0.0),
                        numeric_constraints.step,
                    )
                {
                    diagnostics.push(ControlValueDiagnostic {
                        code: "step-mismatch",
                        message: "number is not aligned to step",
                    });
                }
                Some(value)
            }
            None => {
                diagnostics.push(ControlValueDiagnostic {
                    code: "bad-input",
                    message: "number value is malformed",
                });
                None
            }
        }
    } else {
        None
    };

    let mut minimum = numeric_constraints.minimum;
    let mut maximum = numeric_constraints.maximum;
    let mut step = numeric_constraints
        .validates_step
        .then_some(numeric_constraints.step);
    let mut value_text = None;
    if control.kind.is_temporal() {
        let constraints = typed_constraints(
            control.kind,
            binding.min.as_deref(),
            binding.max.as_deref(),
            binding.step.as_deref(),
        );
        for (authored, code, message) in [
            (
                binding.min.as_deref(),
                "invalid-minimum",
                "minimum is not a valid temporal value",
            ),
            (
                binding.max.as_deref(),
                "invalid-maximum",
                "maximum is not a valid temporal value",
            ),
        ] {
            if authored.is_some_and(|value| parse_typed_value(control.kind, value).is_none()) {
                diagnostics.push(ControlValueDiagnostic { code, message });
            }
        }
        if binding.step.as_deref().is_some_and(|authored| {
            authored.trim() != "any" && parse_typed_step(control.kind, authored).is_none()
        }) {
            diagnostics.push(ControlValueDiagnostic {
                code: "invalid-step",
                message: "step is not positive and finite",
            });
        }
        let parsed = (!control.value.is_empty())
            .then(|| parse_typed_value(control.kind, &control.value))
            .flatten();
        if !control.value.is_empty() && parsed.is_none() {
            diagnostics.push(ControlValueDiagnostic {
                code: "bad-input",
                message: "temporal value is malformed",
            });
        }
        if let Some(parsed) = parsed {
            if constraints
                .minimum
                .is_some_and(|minimum| parsed.scalar < minimum)
            {
                diagnostics.push(ControlValueDiagnostic {
                    code: "range-underflow",
                    message: "temporal value is below min",
                });
            }
            if constraints
                .maximum
                .is_some_and(|maximum| parsed.scalar > maximum)
            {
                diagnostics.push(ControlValueDiagnostic {
                    code: "range-overflow",
                    message: "temporal value is above max",
                });
            }
            let base = constraints.minimum.unwrap_or(0);
            if constraints.validates_step && (parsed.scalar - base) % constraints.step != 0 {
                diagnostics.push(ControlValueDiagnostic {
                    code: "step-mismatch",
                    message: "temporal value is not aligned to step",
                });
            }
            numeric_value = Some(parsed.scalar as f64);
            value_text = Some(parsed.normalized);
        }
        minimum = constraints.minimum.map(|value| value as f64);
        maximum = constraints.maximum.map(|value| value as f64);
        step = constraints
            .validates_step
            .then_some(constraints.step as f64);
    } else if control.kind == ControlKind::Color {
        value_text = normalize_color(&control.value);
        if value_text.is_none() {
            diagnostics.push(ControlValueDiagnostic {
                code: "bad-input",
                message: "color value is not a simple hexadecimal color",
            });
        }
        minimum = None;
        maximum = None;
        step = None;
    }

    ControlValueState {
        key: control.key.clone(),
        value: control.value.clone(),
        input_mode: binding.inputmode.clone(),
        selection_supported: control.kind.supports_selection(),
        numeric_value,
        minimum,
        maximum,
        step,
        value_text,
        diagnostics,
    }
}

fn choice_state(control: &ControlState, binding: &ControlBinding) -> Option<ControlChoiceState> {
    let role = match control.kind {
        ControlKind::Checkbox => "checkbox",
        ControlKind::Radio => "radio",
        ControlKind::Select if control.multiple => "listbox",
        ControlKind::Select => "combobox",
        ControlKind::Range => "slider",
        _ => return None,
    };
    let mut diagnostics = Vec::new();
    if control.kind == ControlKind::Select {
        if control.options.is_empty() {
            diagnostics.push(ControlValueDiagnostic {
                code: "no-options",
                message: "select control has no options",
            });
        }
        if control
            .selected_indices
            .iter()
            .any(|index| *index >= control.options.len())
        {
            diagnostics.push(ControlValueDiagnostic {
                code: "selection-out-of-range",
                message: "selected option index is out of range",
            });
        }
    }
    if control.kind == ControlKind::Range {
        for (value, code, message) in [
            (
                binding.min.as_deref(),
                "invalid-minimum",
                "range min is not finite",
            ),
            (
                binding.max.as_deref(),
                "invalid-maximum",
                "range max is not finite",
            ),
        ] {
            if value.is_some_and(|value| parse_finite(value).is_none()) {
                diagnostics.push(ControlValueDiagnostic { code, message });
            }
        }
        if binding.step.as_deref().is_some_and(|step| {
            step.trim() != "any" && parse_finite(step).is_none_or(|step| step <= 0.0)
        }) {
            diagnostics.push(ControlValueDiagnostic {
                code: "invalid-step",
                message: "range step is not positive and finite",
            });
        }
    }
    let constraints = control_numeric_constraints(control, binding);
    let is_range = control.kind == ControlKind::Range;
    Some(ControlChoiceState {
        key: control.key.clone(),
        role,
        disabled: control.disabled,
        checked: matches!(control.kind, ControlKind::Checkbox | ControlKind::Radio)
            .then_some(control.checked),
        indeterminate: control.indeterminate,
        multiple: control.multiple,
        active_index: (control.kind == ControlKind::Select).then_some(control.selected_index),
        options: control
            .options
            .iter()
            .enumerate()
            .map(|(index, value)| ControlChoiceOptionState {
                index,
                value: value.clone(),
                disabled: option_is_disabled(control, index),
                selected: control.selected_indices.contains(&index),
            })
            .collect(),
        value: is_range.then(|| parse_finite(&control.value)).flatten(),
        minimum: is_range.then_some(constraints.minimum).flatten(),
        maximum: is_range.then_some(constraints.maximum).flatten(),
        step: is_range.then_some(constraints.step),
        diagnostics,
    })
}

fn file_item_state(file: &HostFileSelection) -> ControlFileItemState {
    ControlFileItemState {
        opaque_id: file.opaque_id.clone(),
        name: file.name.clone(),
        media_type: file.media_type.clone(),
        size: file.size(),
    }
}

fn sanitize_file_name(value: &str) -> String {
    let leaf = value.rsplit(['/', '\\']).next().unwrap_or(value);
    let clean = leaf
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    if clean.is_empty() {
        "unnamed".to_string()
    } else {
        clean
    }
}

fn json_string(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn normalize_media_type(value: &str) -> Option<String> {
    let value = value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let (category, subtype) = value.split_once('/')?;
    if category.is_empty()
        || subtype.is_empty()
        || !category.bytes().chain(subtype.bytes()).all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'-' | b'^' | b'_' | b'.' | b'+'
                )
        })
    {
        return None;
    }
    Some(value)
}

pub fn parse_accept_filters(value: Option<&str>) -> Vec<FileAcceptFilter> {
    value
        .into_iter()
        .flat_map(|value| value.split(','))
        .filter_map(|item| {
            let item = item.trim().to_ascii_lowercase();
            if item.starts_with('.')
                && item.len() > 1
                && item
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
            {
                return Some(FileAcceptFilter::Extension(item));
            }
            let (category, subtype) = item.split_once('/')?;
            if subtype == "*"
                && !category.is_empty()
                && category.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                return Some(FileAcceptFilter::MediaRange(category.to_string()));
            }
            normalize_media_type(&item).map(FileAcceptFilter::MediaType)
        })
        .collect()
}

fn file_matches_accept(file: &HostFileSelection, filters: &[FileAcceptFilter]) -> bool {
    let name = file.name.to_ascii_lowercase();
    filters.iter().any(|filter| match filter {
        FileAcceptFilter::Extension(extension) => name.ends_with(extension),
        FileAcceptFilter::MediaType(media_type) => file.media_type.as_ref() == Some(media_type),
        FileAcceptFilter::MediaRange(category) => file
            .media_type
            .as_deref()
            .and_then(|media_type| media_type.split_once('/'))
            .is_some_and(|(actual, _)| actual == category),
    })
}

fn push_file_diagnostic(
    diagnostics: &mut Vec<ControlValueDiagnostic>,
    code: &'static str,
    message: &'static str,
) {
    if diagnostics.len() < 16 && !diagnostics.iter().any(|item| item.code == code) {
        diagnostics.push(ControlValueDiagnostic { code, message });
    }
}

fn grapheme_offsets(value: &str) -> Vec<usize> {
    let mut offsets = graphemes(value)
        .into_iter()
        .map(|cluster| value[..cluster.bytes.start].chars().count())
        .collect::<Vec<_>>();
    offsets.push(value.chars().count());
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

fn previous_grapheme_offset(value: &str, offset: usize) -> usize {
    grapheme_offsets(value)
        .into_iter()
        .rev()
        .find(|boundary| *boundary < offset)
        .unwrap_or(0)
}

fn next_grapheme_offset(value: &str, offset: usize) -> usize {
    grapheme_offsets(value)
        .into_iter()
        .find(|boundary| *boundary > offset)
        .unwrap_or_else(|| value.chars().count())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WordClass {
    Space,
    Word,
    Punctuation,
}

fn word_class(character: char) -> WordClass {
    if character.is_whitespace() {
        WordClass::Space
    } else if character.is_alphanumeric() || character == '_' {
        WordClass::Word
    } else {
        WordClass::Punctuation
    }
}

fn word_boundary(value: &str, offset: usize) -> (usize, usize) {
    let characters = value.chars().collect::<Vec<_>>();
    if characters.is_empty() {
        return (0, 0);
    }
    let pivot = offset.min(characters.len().saturating_sub(1));
    let class = word_class(characters[pivot]);
    let mut start = pivot;
    let mut end = pivot + 1;
    while start > 0 && word_class(characters[start - 1]) == class {
        start -= 1;
    }
    while end < characters.len() && word_class(characters[end]) == class {
        end += 1;
    }
    (start, end)
}

fn word_navigation_offset(value: &str, offset: usize, forward: bool) -> usize {
    let characters = value.chars().collect::<Vec<_>>();
    let mut cursor = offset.min(characters.len());
    if forward {
        if cursor < characters.len() {
            let class = word_class(characters[cursor]);
            while cursor < characters.len() && word_class(characters[cursor]) == class {
                cursor += 1;
            }
        }
        while cursor < characters.len() && word_class(characters[cursor]) == WordClass::Space {
            cursor += 1;
        }
    } else {
        while cursor > 0 && word_class(characters[cursor - 1]) == WordClass::Space {
            cursor -= 1;
        }
        if cursor > 0 {
            let class = word_class(characters[cursor - 1]);
            while cursor > 0 && word_class(characters[cursor - 1]) == class {
                cursor -= 1;
            }
        }
    }
    cursor
}

fn text_extent(value: &str, metrics: ControlTextMetrics) -> (f64, f64) {
    let mut longest = 0;
    let mut lines = 1;
    let mut column = 0;
    for character in value.chars() {
        if character == '\n' {
            longest = longest.max(column);
            column = 0;
            lines += 1;
        } else {
            column += 1;
        }
    }
    longest = longest.max(column);
    (
        longest as f64 * metrics.advance,
        lines as f64 * metrics.line_height,
    )
}

fn axis_drag_delta(position: f64, extent: f64, maximum_step: f64) -> f64 {
    if position < 0.0 {
        -(-position).min(maximum_step)
    } else if position > extent {
        (position - extent).min(maximum_step)
    } else {
        0.0
    }
}

fn drag_autoscroll_offset(
    current: f64,
    position: f64,
    extent: f64,
    maximum_step: f64,
    maximum: f64,
) -> f64 {
    (current + axis_drag_delta(position, extent.max(0.0), maximum_step))
        .clamp(0.0, maximum.max(0.0))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn text_to_html_fragment(value: &str) -> String {
    format!("<span>{}</span>", escape_html(value).replace('\n', "<br>"))
}

fn html_fragment_to_text(value: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < value.len() {
        let rest = &value[cursor..];
        if rest.starts_with('<') {
            let Some(end) = rest.find('>') else {
                break;
            };
            let tag = rest[1..end]
                .trim()
                .trim_start_matches('/')
                .split_ascii_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('/')
                .to_ascii_lowercase();
            if matches!(tag.as_str(), "br" | "p" | "div" | "li")
                && !output.ends_with('\n')
                && !output.is_empty()
            {
                output.push('\n');
            }
            cursor += end + 1;
            continue;
        }
        let next = rest.find('<').unwrap_or(rest.len());
        output.push_str(
            &rest[..next]
                .replace("&nbsp;", " ")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&amp;", "&"),
        );
        cursor += next;
    }
    output
}

fn line_column(value: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;
    for character in value.chars().take(offset) {
        if character == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn line_start_offsets(value: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, character) in value.chars().enumerate() {
        if character == '\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn point_to_offset(value: &str, x: f64, y: f64, metrics: ControlTextMetrics) -> usize {
    let metrics = sanitize_metrics(metrics);
    let starts = line_start_offsets(value);
    let line = (finite_non_negative(y) / metrics.line_height).floor() as usize;
    let line = line.min(starts.len().saturating_sub(1));
    let line_start = starts[line];
    let line_length = value
        .chars()
        .skip(line_start)
        .take_while(|character| *character != '\n')
        .count();
    let column = ((finite_non_negative(x) / metrics.advance) + 0.5).floor() as usize;
    line_start + column.min(line_length)
}

fn rect_for_offset(
    value: &str,
    offset: usize,
    viewport: ControlRect,
    scroll_x: f64,
    scroll_y: f64,
    metrics: ControlTextMetrics,
) -> ControlRect {
    let (line, column) = line_column(value, offset);
    ControlRect {
        x: viewport.x + column as f64 * metrics.advance - scroll_x,
        y: viewport.y + line as f64 * metrics.line_height - scroll_y,
        width: metrics.caret_width,
        height: metrics.line_height,
    }
}

fn reveal_axis(current: f64, position: f64, extent: f64, viewport_extent: f64) -> f64 {
    let current = finite_non_negative(current);
    if position < current {
        position.max(0.0)
    } else if position + extent > current + viewport_extent {
        (position + extent - viewport_extent).max(0.0)
    } else {
        current
    }
}

fn composition_rects(
    value: &str,
    composition: &str,
    start: usize,
    viewport: ControlRect,
    scroll_x: f64,
    scroll_y: f64,
    metrics: ControlTextMetrics,
) -> Vec<ControlRect> {
    let mut composed = value.to_string();
    replace_char_range(&mut composed, start, start, composition);
    selection_rects(
        &composed,
        ControlSelection {
            anchor: start,
            focus: start + composition.chars().count(),
        },
        viewport,
        scroll_x,
        scroll_y,
        metrics,
    )
    .into_iter()
    .map(|mut rect| {
        rect.y += rect.height - 2.0;
        rect.height = 2.0;
        rect
    })
    .collect()
}

fn selection_rects(
    value: &str,
    selection: ControlSelection,
    viewport: ControlRect,
    scroll_x: f64,
    scroll_y: f64,
    metrics: ControlTextMetrics,
) -> Vec<ControlRect> {
    let (start, end) = selection.ordered();
    if start == end {
        return Vec::new();
    }
    let (start_line, start_column) = line_column(value, start);
    let (end_line, end_column) = line_column(value, end);
    let starts = line_start_offsets(value);
    (start_line..=end_line)
        .map(|line| {
            let first = if line == start_line { start_column } else { 0 };
            let last = if line == end_line {
                end_column
            } else {
                value
                    .chars()
                    .skip(starts[line])
                    .take_while(|character| *character != '\n')
                    .count()
            };
            ControlRect {
                x: viewport.x + first as f64 * metrics.advance - scroll_x,
                y: viewport.y + line as f64 * metrics.line_height - scroll_y,
                width: ((last.saturating_sub(first)) as f64 * metrics.advance)
                    .max(metrics.caret_width),
                height: metrics.line_height,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_browser_render_tree;

    #[test]
    fn keyboard_pointer_and_disabled_semantics_are_host_neutral() {
        let mut tree = parse_browser_render_tree(
            "<input id='q' value='go'><input id='off' disabled>\
             <input id='check' type='checkbox'><button id='save'>Save</button>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);

        assert_eq!(
            model.focus_next(false),
            Some(ControlEffect::Focused("control:0:id:q".into()))
        );
        assert!(matches!(
            model.text_input("!"),
            Some(ControlEffect::ValueChanged { value, .. }) if value == "go!"
        ));
        assert_eq!(model.focus("control:1:id:off"), None);
        assert_eq!(
            model.pointer_activate("control:2:id:check"),
            Some(ControlEffect::CheckedChanged {
                key: "control:2:id:check".into(),
                checked: true,
            })
        );
        model.sync_render_tree(&mut tree);
        let states = control_states(&tree);
        assert_eq!(states[0].value, "go!");
        assert!(states[2].checked && states[2].focused);
    }

    #[test]
    fn disabled_fieldsets_exempt_only_their_first_legend_subtree() {
        let mut tree = parse_browser_render_tree(
            "<form><fieldset disabled>\
             <legend><input id='legend-control'></legend>\
             <input id='blocked'>\
             <legend><input id='second-legend-control'></legend>\
             <fieldset><legend>Nested</legend><input id='nested-blocked'></fieldset>\
             </fieldset><input id='outside'></form>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);

        let disabled = |id: &str| {
            model
                .controls()
                .iter()
                .find(|control| control.key.ends_with(&format!("id:{id}")))
                .map(|control| control.disabled)
        };
        assert_eq!(disabled("legend-control"), Some(false));
        assert_eq!(disabled("blocked"), Some(true));
        assert_eq!(disabled("second-legend-control"), Some(true));
        assert_eq!(disabled("nested-blocked"), Some(true));
        assert_eq!(disabled("outside"), Some(false));
        assert_eq!(model.focus("control:1:id:blocked"), None);

        model.sync_render_tree(&mut tree);
        let states = control_states(&tree);
        assert!(states[1].disabled);
        assert!(states[2].disabled);
        assert!(states[3].disabled);
    }

    #[test]
    fn select_and_radio_transitions_are_deterministic() {
        let tree = parse_browser_render_tree(
            "<select id='choice'><option>One</option><option>Two</option></select>\
             <input id='a' type='radio' name='group' checked>\
             <input id='b' type='radio' name='group'>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        model.focus("control:0:id:choice");
        assert!(matches!(
            model.key_down(ControlKey::ArrowDown),
            Some(ControlEffect::ValueChanged { value, .. }) if value == "Two"
        ));
        model.pointer_activate("control:2:id:b");
        assert!(!model.controls()[1].checked);
        assert!(model.controls()[2].checked);
    }

    #[test]
    fn radio_groups_and_reset_baselines_are_scoped_to_their_form() {
        let tree = parse_browser_render_tree(
            "<form id='first'><input id='a' type='radio' name='scope' checked value='a'>\
             <input id='b' type='radio' name='scope' value='b'><input id='q' value='initial'></form>\
             <form id='second'><input id='c' type='radio' name='scope' checked value='c'></form>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);

        model.pointer_activate("control:1:id:b");
        model.focus("control:2:id:q");
        model.text_input(" changed");
        assert!(!model.controls()[0].checked);
        assert!(model.controls()[1].checked);
        assert!(model.controls()[3].checked);

        model.reset_form(Some("first"), Some(0));
        assert!(model.controls()[0].checked);
        assert!(!model.controls()[1].checked);
        assert_eq!(model.controls()[2].value, "initial");
        assert!(model.controls()[3].checked);
    }

    #[test]
    fn selection_replacement_and_deletion_are_utf8_safe() {
        let tree = parse_browser_render_tree("<input id='q' value='café'>").unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:q";
        model.focus(key);

        model.set_selection(key, 3, 4);
        model.text_input("è");
        assert_eq!(model.control(key).unwrap().value, "cafè");
        assert_eq!(
            model.editor(key).unwrap().selection,
            ControlSelection::collapsed(4)
        );

        model.key_down(ControlKey::Backspace);
        assert_eq!(model.control(key).unwrap().value, "caf");
        model.set_selection(key, 2, 2);
        model.key_down(ControlKey::Delete);
        assert_eq!(model.control(key).unwrap().value, "ca");
    }

    #[test]
    fn shifted_navigation_composition_and_multiline_boundaries_share_editor_state() {
        let tree = parse_browser_render_tree("<textarea id='notes'>one\ntwo</textarea>").unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:notes";
        assert_eq!(model.control(key).unwrap().value, "one\ntwo");
        model.focus(key);

        model.key_down(ControlKey::Home);
        assert_eq!(
            model.editor(key).unwrap().selection,
            ControlSelection::collapsed(4)
        );
        model.key_down_with_shift(ControlKey::ArrowRight, true);
        model.key_down_with_shift(ControlKey::ArrowRight, true);
        assert_eq!(
            model.editor(key).unwrap().selection,
            ControlSelection {
                anchor: 4,
                focus: 6
            }
        );

        model.update_composition("三");
        assert_eq!(model.control(key).unwrap().value, "one\ntwo");
        model.commit_composition();
        assert_eq!(model.control(key).unwrap().value, "one\n三o");
        assert_eq!(model.editor(key).unwrap().composition, None);

        model.key_down(ControlKey::End);
        model.key_down(ControlKey::Enter);
        assert_eq!(model.control(key).unwrap().value, "one\n三o\n");
    }

    #[test]
    fn validation_metadata_focuses_first_invalid_and_syncs_without_duplication() {
        let mut tree = parse_browser_render_tree(
            "<input id='first' aria-description='Helpful'><input id='second'>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let first = "control:0:id:first";
        model.set_invalid(first, "This field is required.");
        model.focus_first_invalid();
        model.sync_render_tree(&mut tree);
        model.sync_render_tree(&mut tree);

        let node = &tree.children[0];
        assert_eq!(model.focused_key(), Some(first));
        assert_eq!(node.aria_invalid.as_deref(), Some("true"));
        assert_eq!(
            node.accessible_description.as_deref(),
            Some("Helpful. This field is required.")
        );

        model.clear_validation();
        model.sync_render_tree(&mut tree);
        assert_eq!(tree.children[0].aria_invalid, None);
        assert_eq!(
            tree.children[0].accessible_description.as_deref(),
            Some("Helpful")
        );
    }

    #[test]
    fn password_editing_keeps_the_value_private_from_display_projection() {
        let tree =
            parse_browser_render_tree("<input id='password' type='password' value='secret'>")
                .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:password";
        model.focus(key);
        model.set_selection(key, 0, 6);
        model.text_input("private");

        let control = model.control(key).unwrap();
        assert_eq!(control.value, "private");
        assert_eq!(control.display_value(), "*******");
    }

    #[test]
    fn clipboard_edits_unicode_ranges_without_disclosing_passwords() {
        let tree = parse_browser_render_tree(
            "<input id='plain' value='café'><input id='secret' type='password' value='hidden'>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let plain = "control:0:id:plain";
        model.focus(plain);
        model.set_selection(plain, 3, 4);
        assert_eq!(model.copy_selection().as_deref(), Some("é"));
        let cut = model.cut_selection().unwrap();
        assert_eq!(cut.text, "é");
        model.paste_text("è");
        assert_eq!(model.control(plain).unwrap().value, "cafè");

        let secret = "control:1:id:secret";
        model.focus(secret);
        model.set_selection(secret, 0, 6);
        assert_eq!(model.copy_selection(), None);
        assert_eq!(model.cut_selection(), None);
        assert_eq!(model.control(secret).unwrap().value, "hidden");
    }

    #[test]
    fn pointer_drag_viewport_blink_and_composition_geometry_are_deterministic() {
        let tree =
            parse_browser_render_tree("<textarea id='notes'>zero\none two three four</textarea>")
                .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:notes";
        let metrics = ControlTextMetrics::default();
        model.pointer_place(key, 8.0, 0.0, metrics);
        model.pointer_drag(32.0, 18.0, metrics);
        assert_eq!(
            model.editor(key).unwrap().selection,
            ControlSelection {
                anchor: 1,
                focus: 9
            }
        );
        model.pointer_release();
        model.set_selection(key, 23, 23);
        model.update_composition("界");
        let presentation = model
            .editor_presentation(
                key,
                ControlRect {
                    x: 10.0,
                    y: 20.0,
                    width: 64.0,
                    height: 28.0,
                },
                metrics,
            )
            .unwrap();
        assert!(presentation.scroll_x > 0.0);
        assert!(presentation.scroll_y > 0.0);
        assert_eq!(presentation.composition_underlines.len(), 1);
        assert!(presentation.candidate_rect.is_some());
        assert!(presentation.caret.is_some());
        assert!(model.advance_caret_blink(500));
        assert!(model
            .editor_presentation(key, presentation.viewport, metrics)
            .unwrap()
            .caret
            .is_none());
    }

    #[test]
    fn grapheme_and_word_navigation_never_split_user_perceived_characters() {
        let tree = parse_browser_render_tree("<input id='q' value='A👩‍🚀 café two'>").unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:q";
        model.focus(key);
        model.set_selection(key, 4, 4);
        model.key_down(ControlKey::ArrowLeft);
        assert_eq!(
            model.editor(key).unwrap().selection,
            ControlSelection::collapsed(1)
        );
        model.set_selection(key, 4, 4);
        model.key_down(ControlKey::Backspace);
        assert_eq!(model.control(key).unwrap().value, "A café two");

        model.key_down(ControlKey::End);
        model.key_down(ControlKey::WordLeft);
        assert_eq!(
            model.editor(key).unwrap().selection,
            ControlSelection::collapsed(7)
        );
        model.key_down(ControlKey::WordLeft);
        assert_eq!(
            model.editor(key).unwrap().selection,
            ControlSelection::collapsed(2)
        );
        model.key_down_with_shift(ControlKey::WordRight, true);
        assert_eq!(model.editor(key).unwrap().selection.ordered(), (2, 7));
    }

    #[test]
    fn undo_redo_and_clipboard_flavors_are_shared_bounded_transactions() {
        let tree = parse_browser_render_tree("<input id='q' value='one &lt; two'>").unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:q";
        model.focus(key);
        model.set_selection(key, 0, 3);
        let payload = model.copy_selection_payload().unwrap();
        assert_eq!(payload.plain_text.as_deref(), Some("one"));
        assert_eq!(payload.html.as_deref(), Some("<span>one</span>"));

        model.paste_text("three");
        assert_eq!(model.control(key).unwrap().value, "three < two");
        model.key_down(ControlKey::Undo);
        assert_eq!(model.control(key).unwrap().value, "one < two");
        assert_eq!(model.editor(key).unwrap().selection.ordered(), (0, 3));
        model.key_down(ControlKey::Redo);
        assert_eq!(model.control(key).unwrap().value, "three < two");

        model.set_selection(key, 0, 5);
        model.paste_payload(&ControlClipboardPayload {
            plain_text: None,
            html: Some("<b>four &amp; five</b><br>six".into()),
        });
        assert_eq!(model.control(key).unwrap().value, "four & five\nsix < two");

        for _ in 0..(EDIT_HISTORY_LIMIT + 25) {
            model.text_input("!");
        }
        assert_eq!(model.histories[0].undo.len(), EDIT_HISTORY_LIMIT);
    }

    #[test]
    fn click_count_autoscroll_and_accessibility_actions_share_selection_policy() {
        let tree = parse_browser_render_tree(
            "<textarea id='notes'>one two three four five\nsecond line</textarea>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:notes";
        let metrics = ControlTextMetrics::default();

        model.pointer_select(key, 5.0 * metrics.advance, 0.0, metrics, 2);
        assert_eq!(model.editor(key).unwrap().selection.ordered(), (4, 7));
        model.pointer_select(key, 5.0 * metrics.advance, 0.0, metrics, 3);
        assert_eq!(model.editor(key).unwrap().selection.ordered(), (0, 23));

        model.pointer_select(key, 0.0, 0.0, metrics, 1);
        model.pointer_drag_autoscroll(160.0, 80.0, 32.0, 18.0, metrics);
        assert!(model.editor(key).unwrap().scroll_x > 0.0);
        assert!(model.editor(key).unwrap().scroll_y > 0.0);

        model.accessibility_action(ControlAccessibilityAction::SelectAll);
        model.accessibility_action(ControlAccessibilityAction::ReplaceSelection(
            "accessible".into(),
        ));
        assert_eq!(model.control(key).unwrap().value, "accessible");
        model.accessibility_action(ControlAccessibilityAction::Undo);
        assert_eq!(
            model.control(key).unwrap().value,
            "one two three four five\nsecond line"
        );
    }

    #[test]
    fn maxlength_is_live_unicode_aware_and_number_selection_is_restricted() {
        let tree = parse_browser_render_tree(
            "<input id='text' maxlength='4' value='é'>\
             <input id='number' type='number' maxlength='1' inputmode='decimal' value='3'>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let text = "control:0:id:text";
        let number = "control:1:id:number";

        model.focus(text);
        model.text_input("abcde");
        assert_eq!(model.control(text).unwrap().value, "éabc");
        model.set_selection(text, 1, 4);
        model.text_input("xyzw");
        assert_eq!(model.control(text).unwrap().value, "éxyz");

        assert_eq!(model.set_selection(number, 0, 1), None);
        let state = model.value_state(number).unwrap();
        assert!(!state.selection_supported);
        assert_eq!(state.input_mode.as_deref(), Some("decimal"));
        assert_eq!(state.value, "3");
    }

    #[test]
    fn typed_value_states_report_email_url_and_number_diagnostics() {
        let tree = parse_browser_render_tree(
            "<input id='mail' type='email' value='not mail'>\
             <input id='site' type='url' value='http://['>\
             <input id='count' type='number' value='3' min='0' max='10' step='2'>\
             <input id='any' type='number' value='3.5' step='any'>",
        )
        .unwrap();
        let model = BrowserControlModel::from_render_tree(&tree);

        for key in ["control:0:id:mail", "control:1:id:site"] {
            assert_eq!(
                model.value_state(key).unwrap().diagnostics[0].code,
                "type-mismatch"
            );
        }
        let number = model.value_state("control:2:id:count").unwrap();
        assert_eq!(number.numeric_value, Some(3.0));
        assert_eq!(number.minimum, Some(0.0));
        assert_eq!(number.maximum, Some(10.0));
        assert_eq!(number.step, Some(2.0));
        assert_eq!(number.diagnostics[0].code, "step-mismatch");
        assert!(model.value_state("control:3:id:any").unwrap().is_valid());
    }

    #[test]
    fn number_keyboard_and_accessibility_steps_align_clamp_and_undo() {
        let tree = parse_browser_render_tree(
            "<input id='count' type='number' value='3' min='0' max='6' step='2'>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:count";
        model.focus(key);

        model.key_down(ControlKey::ArrowUp);
        assert_eq!(model.control(key).unwrap().value, "4");
        model.accessibility_action(ControlAccessibilityAction::Increment);
        assert_eq!(model.control(key).unwrap().value, "6");
        assert_eq!(
            model.accessibility_action(ControlAccessibilityAction::Increment),
            None
        );
        model.accessibility_action(ControlAccessibilityAction::Decrement);
        assert_eq!(model.control(key).unwrap().value, "4");
        model.accessibility_action(ControlAccessibilityAction::SetValue("invalid".into()));
        assert_eq!(
            model.value_state(key).unwrap().diagnostics[0].code,
            "bad-input"
        );
        model.accessibility_action(ControlAccessibilityAction::Increment);
        assert_eq!(model.control(key).unwrap().value, "0");
        model.accessibility_action(ControlAccessibilityAction::Undo);
        assert_eq!(model.control(key).unwrap().value, "invalid");
    }

    #[test]
    fn multi_select_skips_disabled_options_and_exposes_accessible_choice_state() {
        let tree = parse_browser_render_tree(
            "<select id='tags' name='tag' multiple>\
             <option value='a' selected>A</option>\
             <optgroup label='locked' disabled><option value='b' selected>B</option></optgroup>\
             <option value='c'>C</option><option value='d' disabled>D</option>\
             </select>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:tags";

        let state = model.choice_state(key).unwrap();
        assert_eq!(state.role, "listbox");
        assert_eq!(state.options.len(), 4);
        assert!(state.options[1].disabled);
        assert!(state.options[3].disabled);
        assert_eq!(model.selected_values(key).unwrap(), vec!["a"]);

        model.focus(key);
        model.key_down_with_shift(ControlKey::ArrowDown, true);
        assert_eq!(model.control(key).unwrap().selected_indices, vec![0, 2]);
        assert_eq!(model.selected_values(key).unwrap(), vec!["a", "c"]);
        assert_eq!(
            model.select_option(key, 3, false, true),
            None,
            "disabled options never enter the selection reducer"
        );
        model.accessibility_action(ControlAccessibilityAction::SelectOption {
            index: 2,
            extend: false,
            toggle: true,
        });
        assert_eq!(model.selected_values(key).unwrap(), vec!["a"]);
    }

    #[test]
    fn checkbox_radio_and_range_actions_share_one_non_text_reducer() {
        let tree = parse_browser_render_tree(
            "<input id='check' type='checkbox'>\
             <input id='one' type='radio' name='mode' checked>\
             <input id='two' type='radio' name='mode' disabled>\
             <input id='three' type='radio' name='mode'>\
             <input id='level' type='range' min='0' max='10' step='2' value='3'>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);

        let checkbox = "control:0:id:check";
        model.focus(checkbox);
        model.accessibility_action(ControlAccessibilityAction::SetIndeterminate(true));
        assert!(model.choice_state(checkbox).unwrap().indeterminate);
        model.accessibility_action(ControlAccessibilityAction::Toggle);
        let check_state = model.choice_state(checkbox).unwrap();
        assert_eq!(check_state.checked, Some(true));
        assert!(!check_state.indeterminate);

        model.focus("control:1:id:one");
        model.key_down(ControlKey::ArrowRight);
        assert_eq!(model.focused_key(), Some("control:3:id:three"));
        assert!(model.control("control:3:id:three").unwrap().checked);
        assert!(!model.control("control:1:id:one").unwrap().checked);

        let range = "control:4:id:level";
        model.focus(range);
        assert_eq!(model.control(range).unwrap().value, "4");
        model.key_down(ControlKey::ArrowRight);
        assert_eq!(model.control(range).unwrap().value, "6");
        model.accessibility_action(ControlAccessibilityAction::SetValue("9".into()));
        assert_eq!(model.control(range).unwrap().value, "10");
        model.key_down(ControlKey::Home);
        let range_state = model.choice_state(range).unwrap();
        assert_eq!(range_state.role, "slider");
        assert_eq!(range_state.value, Some(0.0));
        assert_eq!(range_state.minimum, Some(0.0));
        assert_eq!(range_state.maximum, Some(10.0));
        assert_eq!(range_state.step, Some(2.0));
    }

    #[test]
    fn temporal_and_color_values_share_normalization_diagnostics_and_actions() {
        let tree = parse_browser_render_tree(
            "<input id='day' type='date' value='2024-01-02' min='2024-01-01' max='2024-01-09' step='2'>\
             <input id='month' type='month' value='2024-7'>\
             <input id='week' type='week' value='2020-W53'>\
             <input id='clock' type='time' value='09:30:05.120' step='0.5'>\
             <input id='local' type='datetime-local' value='2024-02-29T09:30'>\
             <input id='ink' type='color' value='#A0b1C2'>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);

        let day = "control:0:id:day";
        let day_state = model.value_state(day).unwrap();
        assert!(!day_state.selection_supported);
        assert_eq!(day_state.value_text.as_deref(), Some("2024-01-02"));
        assert_eq!(day_state.diagnostics[0].code, "step-mismatch");
        model.focus(day);
        model.key_down(ControlKey::ArrowUp);
        assert_eq!(model.control(day).unwrap().value, "2024-01-03");
        model.accessibility_action(ControlAccessibilityAction::Increment);
        assert_eq!(model.control(day).unwrap().value, "2024-01-05");
        model.key_down(ControlKey::End);
        assert_eq!(model.control(day).unwrap().value, "2024-01-09");

        assert_eq!(model.control("control:1:id:month").unwrap().value, "");
        assert_eq!(
            model.control("control:2:id:week").unwrap().value,
            "2020-W53"
        );
        assert_eq!(
            model.control("control:3:id:clock").unwrap().value,
            "09:30:05.12"
        );
        assert_eq!(
            model.control("control:4:id:local").unwrap().value,
            "2024-02-29T09:30"
        );

        let color = "control:5:id:ink";
        assert_eq!(model.control(color).unwrap().value, "#a0b1c2");
        model.focus(color);
        model.accessibility_action(ControlAccessibilityAction::SetValue("#00FF7f".into()));
        let color_state = model.value_state(color).unwrap();
        assert_eq!(color_state.value_text.as_deref(), Some("#00ff7f"));
        assert!(color_state.is_valid());
        assert_eq!(
            model.accessibility_action(ControlAccessibilityAction::SetValue("blue".into())),
            None
        );
    }

    #[test]
    fn temporal_metadata_diagnostics_are_reusable() {
        let tree = parse_browser_render_tree(
            "<input id='day' type='date' value='2024-06-01' min='bad' max='2024-05-01' step='zero'>",
        )
        .unwrap();
        let model = BrowserControlModel::from_render_tree(&tree);
        let state = model.value_state("control:0:id:day").unwrap();
        assert_eq!(
            state
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            vec!["invalid-minimum", "invalid-step", "range-overflow"]
        );
    }

    #[test]
    fn file_picker_contract_filters_without_retaining_host_paths() {
        let tree = coding_adventures_html_parser::parse_browser_render_tree(
            "<form id='upload'><input id='asset' name='asset' type='file' accept='image/*,.txt' multiple></form>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let key = "control:0:id:asset";
        assert_eq!(model.control(key).unwrap().kind, ControlKind::File);
        assert_eq!(
            model.pointer_activate(key),
            Some(ControlEffect::FilePickerRequested(
                ControlFilePickerRequest {
                    key: key.into(),
                    accept: vec![
                        FileAcceptFilter::MediaRange("image".into()),
                        FileAcceptFilter::Extension(".txt".into()),
                    ],
                    multiple: true,
                }
            ))
        );

        model.apply_file_selection(
            key,
            vec![
                HostFileSelection::new(
                    "host:1",
                    "/private/user/photo.PNG",
                    Some("IMAGE/PNG; charset=binary".into()),
                    vec![1, 2, 3],
                ),
                HostFileSelection::new(
                    "host:2",
                    "notes.txt",
                    Some("text/plain".into()),
                    b"notes".to_vec(),
                ),
                HostFileSelection::new(
                    "host:3",
                    "program.exe",
                    Some("application/octet-stream".into()),
                    vec![0],
                ),
            ],
        );
        let state = model.file_state(key).unwrap();
        assert_eq!(
            state
                .files
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["photo.PNG", "notes.txt"]
        );
        assert_eq!(state.value_text, "2 files selected");
        assert_eq!(state.diagnostics[0].code, "accept-mismatch");
        assert!(!model.control(key).unwrap().value.contains("private"));

        model.apply_file_selection(
            key,
            vec![
                HostFileSelection::new(
                    "host:large-1",
                    "one.txt",
                    Some("text/plain".into()),
                    vec![0; 9 * 1024 * 1024],
                ),
                HostFileSelection::new(
                    "host:large-2",
                    "two.txt",
                    Some("text/plain".into()),
                    vec![0; 9 * 1024 * 1024],
                ),
            ],
        );
        let bounded = model.file_state(key).unwrap();
        assert_eq!(bounded.files.len(), 1);
        assert_eq!(bounded.diagnostics[0].code, "selection-too-large");

        model.reset_form(Some("upload"), Some(0));
        assert!(model.file_state(key).unwrap().files.is_empty());
        assert_eq!(
            model.file_state(key).unwrap().value_text,
            "No file selected"
        );
    }

    #[test]
    fn dirty_default_state_survives_edits_and_clears_on_reset() {
        let tree = parse_browser_render_tree(
            "<form id='profile'><input id='name' name='name' value='Ada'>\
             <input id='news' name='news' type='checkbox' checked></form>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let name = "control:0:id:name";
        let news = "control:1:id:news";
        assert!(!model.default_state(name).unwrap().dirty_value);

        model.focus(name);
        model.set_selection(name, 0, 3);
        model.text_input("Grace");
        model.pointer_activate(news);
        assert!(model.default_state(name).unwrap().dirty_value);
        assert!(model.default_state(news).unwrap().dirty_checkedness);
        assert_eq!(model.default_state(name).unwrap().default_value, "Ada");

        model.reset_form(Some("profile"), Some(0));
        let name_state = model.default_state(name).unwrap();
        assert_eq!(name_state.value, "Ada");
        assert!(!name_state.dirty_value);
        assert!(model.default_state(news).unwrap().checked);
        assert!(!model.default_state(news).unwrap().dirty_checkedness);
    }

    #[test]
    fn autofill_groups_fields_and_emits_input_before_change() {
        let tree = parse_browser_render_tree(
            "<form autocomplete='on'>\
             <input id='given' name='given-name' autocomplete='section-contact shipping given-name'>\
             <input id='password' type='password' autocomplete='current-password'>\
             <select id='country' name='country' autocomplete='section-contact country-name'>\
             <option>GB</option><option>US</option></select></form>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let descriptors = model.autofill_descriptors();
        assert_eq!(descriptors[0].section.as_deref(), Some("section-contact"));
        assert_eq!(descriptors[0].address_type.as_deref(), Some("shipping"));
        assert_eq!(descriptors[0].purpose, "given-name");
        assert!(descriptors[1].sensitive);

        let outcome = model.apply_autofill(&ControlAutofillTransaction {
            privacy: ControlStatePrivacy::Public,
            values: vec![
                ControlAutofillValue {
                    section: Some("section-contact".into()),
                    purpose: "given-name".into(),
                    value: "Grace".into(),
                },
                ControlAutofillValue {
                    section: None,
                    purpose: "current-password".into(),
                    value: "secret".into(),
                },
                ControlAutofillValue {
                    section: Some("section-contact".into()),
                    purpose: "country-name".into(),
                    value: "US".into(),
                },
            ],
        });
        assert_eq!(model.control("control:0:id:given").unwrap().value, "Grace");
        assert_eq!(model.control("control:1:id:password").unwrap().value, "");
        assert_eq!(model.control("control:2:id:country").unwrap().value, "US");
        assert_eq!(outcome.effects.len(), 2);
        assert_eq!(
            outcome
                .events
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                ControlMutationEventKind::Input,
                ControlMutationEventKind::Change,
                ControlMutationEventKind::Input,
                ControlMutationEventKind::Change,
            ]
        );
    }

    #[test]
    fn history_snapshot_restores_native_state_and_custom_callbacks() {
        let source = "<form><input id='query' name='search' value='before'>\
                      <x-rating id='rating' name='rating'></x-rating></form>";
        let tree = parse_browser_render_tree(source).unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let custom_key = model.form_associated_custom_elements()[0].key.clone();
        model
            .attach_form_associated_custom_element(&custom_key)
            .unwrap();
        model
            .set_custom_element_form_value(
                &custom_key,
                Some(CustomElementFormValue::Text("4".into())),
                Some(CustomElementFormValue::Text("restore:4".into())),
            )
            .unwrap();
        model.take_custom_element_lifecycle_events();
        model.focus("control:0:id:query");
        model.set_selection("control:0:id:query", 0, 6);
        model.text_input("saved");
        let snapshot = model.capture_state(ControlStatePrivacy::Public);

        model.set_selection("control:0:id:query", 0, 5);
        model.text_input("later");
        model.restore_state(&snapshot);
        assert_eq!(model.control("control:0:id:query").unwrap().value, "saved");
        assert!(matches!(
            model.take_custom_element_lifecycle_events().as_slice(),
            [CustomElementLifecycleEvent::FormStateRestore {
                mode: CustomElementStateRestoreMode::Restore,
                state: CustomElementFormValue::Text(state),
                ..
            }] if state == "restore:4"
        ));
    }

    #[test]
    fn history_snapshot_never_persists_custom_file_payloads() {
        let tree = parse_browser_render_tree(
            "<form><x-upload id='upload' name='upload'></x-upload></form>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let custom_key = model.form_associated_custom_elements()[0].key.clone();
        model
            .attach_form_associated_custom_element(&custom_key)
            .unwrap();
        model
            .set_custom_element_form_value(
                &custom_key,
                None,
                Some(CustomElementFormValue::File(HostFileSelection::new(
                    "opaque-file",
                    "secret.txt",
                    Some("text/plain".into()),
                    b"not history state".to_vec(),
                ))),
            )
            .unwrap();

        let snapshot = model.capture_state(ControlStatePrivacy::Credentials);

        assert!(snapshot.custom_elements.is_empty());
        assert_eq!(snapshot.diagnostics[0].code, "state-file-omitted");
        assert_eq!(
            snapshot.diagnostics[0].key.as_deref(),
            Some(custom_key.as_str())
        );
    }

    #[test]
    fn datalist_queries_and_picker_transactions_share_typed_value_policy() {
        let tree = parse_browser_render_tree(
            "<input id='city' list='cities'><datalist id='cities'>\
             <option value='SFO' label='San Francisco'>Bay Area</option>\
             <option value='SEA'>Seattle</option><option value='SFO'>Duplicate</option>\
             <option value='PDX' disabled>Portland</option></datalist>\
             <input id='count' type='number' min='0' max='10' step='2' list='counts'>\
             <datalist id='counts'><option value='4'><option value='3'><option value='many'></datalist>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let city = "control:0:id:city";
        let count = "control:1:id:count";

        model.open_suggestions(city, "bay", 8).unwrap();
        let state = model.suggestion_state(city).unwrap();
        assert_eq!(state.options.len(), 1);
        assert_eq!(state.options[0].value, "SFO");
        assert!(state.to_host_json().contains("\"sourceIndex\":0"));

        model.open_suggestions(city, "", 1).unwrap();
        assert_eq!(model.suggestion_state(city).unwrap().options.len(), 1);
        assert_eq!(
            model.suggestion_state(city).unwrap().diagnostics[0].code,
            "result-limit"
        );

        model.open_suggestions(count, "", 8).unwrap();
        assert_eq!(
            model
                .suggestion_state(count)
                .unwrap()
                .options
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["4"]
        );

        model.focus(city).unwrap();
        model
            .apply_suggestion_picker_action(city, ControlSuggestionPickerAction::Cancel)
            .unwrap();
        model.key_down(ControlKey::ArrowDown).unwrap();
        assert_eq!(
            model.focused_suggestion_state().unwrap().active_index,
            Some(0)
        );
        model
            .accessibility_action(ControlAccessibilityAction::MoveSuggestion { forward: true })
            .unwrap();
        assert_eq!(
            model.focused_suggestion_state().unwrap().active_index,
            Some(1)
        );
        assert_eq!(
            model.key_down(ControlKey::Enter),
            Some(ControlEffect::SuggestionCommitted {
                key: city.into(),
                value: "SEA".into(),
            })
        );
        assert_eq!(model.control(city).unwrap().value, "SEA");
        assert!(model.default_state(city).unwrap().dirty_value);
        assert!(!model.focused_suggestion_state().unwrap().open);
    }

    #[test]
    fn output_dependencies_recalculate_and_reset_transactionally() {
        let mut tree = parse_browser_render_tree(
            "<form id='calc'><input id='a' value='2'><input id='b' value='3'>\
             <output id='sum' for='a b' value='ignored'>waiting</output></form>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        let output = "live:0:id:sum";
        assert_eq!(model.live_value_state(output).unwrap().text, "waiting");
        assert_eq!(
            model
                .output_dependencies(output)
                .iter()
                .map(|dependency| (dependency.id.as_str(), dependency.value.as_str()))
                .collect::<Vec<_>>(),
            vec![("a", "2"), ("b", "3")]
        );

        model.focus("control:0:id:a").unwrap();
        model
            .accessibility_action(ControlAccessibilityAction::SetValue("7".into()))
            .unwrap();
        assert_eq!(
            model.recalculate_output(output, |values| {
                values
                    .iter()
                    .filter_map(|value| value.value.parse::<i32>().ok())
                    .sum::<i32>()
                    .to_string()
            }),
            Some(ControlEffect::LiveValueChanged {
                key: output.into(),
                value: "10".into(),
            })
        );
        model.sync_render_tree(&mut tree);
        assert_eq!(model.live_value_state(output).unwrap().text, "10");

        model.reset_form(Some("calc"), Some(0));
        assert_eq!(model.live_value_state(output).unwrap().text, "waiting");
    }

    #[test]
    fn meter_and_progress_publish_normalized_accessibility_state() {
        let tree = parse_browser_render_tree(
            "<meter id='health' min='0' max='10' low='3' high='7' optimum='9' value='2'>Low</meter>\
             <progress id='download' max='20'>Loading</progress>",
        )
        .unwrap();
        let mut model = BrowserControlModel::from_render_tree(&tree);
        assert_eq!(
            model.recalculate_output("live:0:id:health", |_| {
                panic!("meter must not invoke an output calculator")
            }),
            None
        );
        let meter = model.live_value_state("live:0:id:health").unwrap();
        assert_eq!(meter.value, Some(2.0));
        assert_eq!(meter.position, Some(0.2));
        assert_eq!(meter.meter_region, Some(MeterValueRegion::EvenLessGood));
        assert_eq!(meter.value_text, "2 of 10");

        let progress = model.live_value_state("live:1:id:download").unwrap();
        assert!(progress.indeterminate);
        assert_eq!(progress.position, None);
        assert_eq!(progress.value_text, "indeterminate");

        model
            .set_live_value("live:1:id:download", Some("25"))
            .unwrap();
        let progress = model.live_value_state("live:1:id:download").unwrap();
        assert_eq!(progress.value, Some(20.0));
        assert_eq!(progress.position, Some(1.0));
        assert!(model
            .live_value_states_host_json()
            .contains("\"kind\":\"progress\""));
    }
}
