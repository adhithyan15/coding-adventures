//! Host-neutral HTML form submission and constraint validation.

use browser_form_controls::{
    BrowserControlModel, ControlBinding, CustomElementFormEntryValue, HostFileSelection,
};
use coding_adventures_html_parser::{BrowserDocument, BrowserForm, BrowserFormControl};
use layout_controls::{ControlKind, ControlState};
use regex::Regex;
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const MAX_FORM_ENTRIES: usize = 1_024;
pub const MAX_ENCODED_BYTES: usize = 1024 * 1024;
pub const MAX_MULTIPART_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DIAGNOSTICS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormMethod {
    Get,
    Post,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormEntry {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFileEntry {
    pub name: String,
    pub file: HostFileSelection,
}

/// Image-submit coordinates normalized from a shared control-local point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageSubmitCoordinates {
    pub x: u32,
    pub y: u32,
}

impl ImageSubmitCoordinates {
    pub const KEYBOARD: Self = Self { x: 0, y: 0 };

    pub fn from_local_point(x: f64, y: f64) -> Self {
        Self {
            x: normalize_coordinate(x),
            y: normalize_coordinate(y),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormNavigation {
    pub method: FormMethod,
    pub url: String,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub entries: Vec<FormEntry>,
    pub files: Vec<FormFileEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormDiagnostic {
    pub code: &'static str,
    pub key: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormValidationMode {
    Check,
    Report,
    Submit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormDataValue {
    Text(String),
    File(HostFileSelection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormDataEntry {
    pub name: String,
    pub value: FormDataValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormLifecycleEvent {
    Invalid {
        form_id: Option<String>,
        form_index: usize,
        diagnostic: FormDiagnostic,
        mode: FormValidationMode,
        cancelable: bool,
        default_prevented: bool,
    },
    Submit {
        form_id: Option<String>,
        form_index: usize,
        submitter_key: Option<String>,
        cancelable: bool,
        default_prevented: bool,
    },
    FormData {
        form_id: Option<String>,
        form_index: usize,
        submitter_key: Option<String>,
        entries: Vec<FormDataEntry>,
    },
    Reset {
        form_id: Option<String>,
        form_index: usize,
        cancelable: bool,
        default_prevented: bool,
    },
}

impl FormLifecycleEvent {
    pub fn prevent_default(&mut self) -> bool {
        match self {
            Self::Invalid {
                cancelable,
                default_prevented,
                ..
            }
            | Self::Submit {
                cancelable,
                default_prevented,
                ..
            }
            | Self::Reset {
                cancelable,
                default_prevented,
                ..
            } if *cancelable => {
                *default_prevented = true;
                true
            }
            _ => false,
        }
    }

    pub fn form_data_mut(&mut self) -> Option<&mut Vec<FormDataEntry>> {
        match self {
            Self::FormData { entries, .. } => Some(entries),
            _ => None,
        }
    }

    pub const fn default_prevented(&self) -> bool {
        match self {
            Self::Invalid {
                default_prevented, ..
            }
            | Self::Submit {
                default_prevented, ..
            }
            | Self::Reset {
                default_prevented, ..
            } => *default_prevented,
            Self::FormData { .. } => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormValidationReport {
    pub valid: bool,
    pub diagnostics: Vec<FormDiagnostic>,
    pub events: Vec<FormLifecycleEvent>,
}

impl FormValidationReport {
    pub fn reportable_diagnostics(&self) -> Vec<FormDiagnostic> {
        self.events
            .iter()
            .filter_map(|event| match event {
                FormLifecycleEvent::Invalid {
                    diagnostic,
                    default_prevented: false,
                    ..
                } => Some(diagnostic.clone()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormDispatchOutcome {
    pub activation: FormActivation,
    pub events: Vec<FormLifecycleEvent>,
}

impl FormDispatchOutcome {
    pub fn reportable_diagnostics(&self) -> Vec<FormDiagnostic> {
        self.events
            .iter()
            .filter_map(|event| match event {
                FormLifecycleEvent::Invalid {
                    diagnostic,
                    default_prevented: false,
                    ..
                } => Some(diagnostic.clone()),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormActivation {
    None,
    Reset {
        form_id: Option<String>,
        form_index: usize,
    },
    Invalid(Vec<FormDiagnostic>),
    Navigate(FormNavigation),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormPlanningError {
    UnknownControl(String),
    MissingForm(String),
    UnsupportedMethod(String),
    UnsupportedEncoding(String),
    InvalidAction(String),
    InvalidSubmitter(String),
    DisabledSubmitter(String),
    TooManyEntries { limit: usize },
    PayloadTooLarge { limit: usize },
}

impl std::fmt::Display for FormPlanningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownControl(key) => write!(formatter, "unknown form control {key}"),
            Self::MissingForm(key) => write!(formatter, "control {key} has no associated form"),
            Self::UnsupportedMethod(method) => {
                write!(formatter, "unsupported form method {method}")
            }
            Self::UnsupportedEncoding(encoding) => {
                write!(formatter, "unsupported form encoding {encoding}")
            }
            Self::InvalidAction(action) => write!(formatter, "invalid form action {action}"),
            Self::InvalidSubmitter(key) => write!(formatter, "invalid form submitter {key}"),
            Self::DisabledSubmitter(key) => write!(formatter, "disabled form submitter {key}"),
            Self::TooManyEntries { limit } => {
                write!(formatter, "form exceeds the {limit}-entry limit")
            }
            Self::PayloadTooLarge { limit } => {
                write!(formatter, "form payload exceeds the {limit}-byte limit")
            }
        }
    }
}

impl std::error::Error for FormPlanningError {}

pub fn plan_activation(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    activated_key: &str,
    document_url: &str,
) -> Result<FormActivation, FormPlanningError> {
    plan_activation_with_image_coordinates(
        document,
        controls,
        activated_key,
        document_url,
        ImageSubmitCoordinates::KEYBOARD,
    )
}

pub fn plan_activation_with_image_coordinates(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    activated_key: &str,
    document_url: &str,
    image_coordinates: ImageSubmitCoordinates,
) -> Result<FormActivation, FormPlanningError> {
    Ok(dispatch_activation_with_image_coordinates(
        document,
        controls,
        activated_key,
        document_url,
        image_coordinates,
        |_| {},
    )?
    .activation)
}

pub fn dispatch_activation_with_image_coordinates<F>(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    activated_key: &str,
    document_url: &str,
    image_coordinates: ImageSubmitCoordinates,
    mut dispatch: F,
) -> Result<FormDispatchOutcome, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let binding = controls
        .binding(activated_key)
        .ok_or_else(|| FormPlanningError::UnknownControl(activated_key.to_string()))?;
    let (form_index, form) = associated_form(document, binding)
        .ok_or_else(|| FormPlanningError::MissingForm(activated_key.to_string()))?;
    match binding.control_type.as_str() {
        "reset" => Ok(dispatch_reset(form, form_index, &mut dispatch)),
        "submit" | "image" => dispatch_submission(
            controls,
            form_index,
            form,
            Some(binding),
            document_url,
            image_coordinates,
            &mut dispatch,
        ),
        _ => Ok(FormDispatchOutcome {
            activation: FormActivation::None,
            events: Vec::new(),
        }),
    }
}

pub fn plan_implicit_submission(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    focused_key: &str,
    document_url: &str,
) -> Result<FormActivation, FormPlanningError> {
    Ok(
        dispatch_implicit_submission(document, controls, focused_key, document_url, |_| {})?
            .activation,
    )
}

pub fn dispatch_implicit_submission<F>(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    focused_key: &str,
    document_url: &str,
    mut dispatch: F,
) -> Result<FormDispatchOutcome, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let focused = controls
        .binding(focused_key)
        .ok_or_else(|| FormPlanningError::UnknownControl(focused_key.to_string()))?;
    let (form_index, form) = associated_form(document, focused)
        .ok_or_else(|| FormPlanningError::MissingForm(focused_key.to_string()))?;
    let submitter = controls.bindings().iter().find(|binding| {
        associated_with(binding, form.id.as_deref(), form_index)
            && matches!(binding.control_type.as_str(), "submit" | "image")
            && controls
                .control(&binding.key)
                .is_some_and(|control| !control.disabled)
    });
    dispatch_submission(
        controls,
        form_index,
        form,
        submitter,
        document_url,
        ImageSubmitCoordinates::KEYBOARD,
        &mut dispatch,
    )
}

pub fn dispatch_request_submit<F>(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    form_index: usize,
    submitter_key: Option<&str>,
    document_url: &str,
    mut dispatch: F,
) -> Result<FormDispatchOutcome, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let form = document
        .forms
        .get(form_index)
        .ok_or_else(|| FormPlanningError::MissingForm(format!("form:{form_index}")))?;
    let submitter = submitter_key
        .map(|key| request_submitter(controls, form, form_index, key))
        .transpose()?;
    dispatch_submission(
        controls,
        form_index,
        form,
        submitter,
        document_url,
        ImageSubmitCoordinates::KEYBOARD,
        &mut dispatch,
    )
}

pub fn check_form_validity<F>(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    form_index: usize,
    dispatch: F,
) -> Result<FormValidationReport, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    dispatch_form_validation(
        document,
        controls,
        form_index,
        FormValidationMode::Check,
        dispatch,
    )
}

pub fn report_form_validity<F>(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    form_index: usize,
    dispatch: F,
) -> Result<FormValidationReport, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    dispatch_form_validation(
        document,
        controls,
        form_index,
        FormValidationMode::Report,
        dispatch,
    )
}

pub fn dispatch_form_reset<F>(
    document: &BrowserDocument,
    form_index: usize,
    mut dispatch: F,
) -> Result<FormDispatchOutcome, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let form = document
        .forms
        .get(form_index)
        .ok_or_else(|| FormPlanningError::MissingForm(format!("form:{form_index}")))?;
    Ok(dispatch_reset(form, form_index, &mut dispatch))
}

fn dispatch_form_validation<F>(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    form_index: usize,
    mode: FormValidationMode,
    mut dispatch: F,
) -> Result<FormValidationReport, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let form = document
        .forms
        .get(form_index)
        .ok_or_else(|| FormPlanningError::MissingForm(format!("form:{form_index}")))?;
    Ok(dispatch_validation_for_form(
        controls,
        form,
        form_index,
        mode,
        &mut dispatch,
    ))
}

fn dispatch_validation_for_form<F>(
    controls: &BrowserControlModel,
    form: &BrowserForm,
    form_index: usize,
    mode: FormValidationMode,
    dispatch: &mut F,
) -> FormValidationReport
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let diagnostics = validate_controls(controls, form.id.as_deref(), form_index);
    let mut events = Vec::with_capacity(diagnostics.len());
    for diagnostic in &diagnostics {
        let mut event = FormLifecycleEvent::Invalid {
            form_id: form.id.clone(),
            form_index,
            diagnostic: diagnostic.clone(),
            mode,
            cancelable: true,
            default_prevented: false,
        };
        dispatch(&mut event);
        events.push(event);
    }
    FormValidationReport {
        valid: diagnostics.is_empty(),
        diagnostics,
        events,
    }
}

fn dispatch_reset<F>(form: &BrowserForm, form_index: usize, dispatch: &mut F) -> FormDispatchOutcome
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let mut event = FormLifecycleEvent::Reset {
        form_id: form.id.clone(),
        form_index,
        cancelable: true,
        default_prevented: false,
    };
    dispatch(&mut event);
    let canceled = event.default_prevented();
    FormDispatchOutcome {
        activation: if canceled {
            FormActivation::None
        } else {
            FormActivation::Reset {
                form_id: form.id.clone(),
                form_index,
            }
        },
        events: vec![event],
    }
}

fn request_submitter<'a>(
    controls: &'a BrowserControlModel,
    form: &BrowserForm,
    form_index: usize,
    key: &str,
) -> Result<&'a ControlBinding, FormPlanningError> {
    let binding = controls
        .binding(key)
        .ok_or_else(|| FormPlanningError::UnknownControl(key.to_string()))?;
    if !matches!(binding.control_type.as_str(), "submit" | "image")
        || !associated_with(binding, form.id.as_deref(), form_index)
    {
        return Err(FormPlanningError::InvalidSubmitter(key.to_string()));
    }
    if controls.control(key).is_none_or(|control| control.disabled) {
        return Err(FormPlanningError::DisabledSubmitter(key.to_string()));
    }
    Ok(binding)
}

fn dispatch_submission<F>(
    controls: &BrowserControlModel,
    form_index: usize,
    form: &BrowserForm,
    submitter: Option<&ControlBinding>,
    document_url: &str,
    image_coordinates: ImageSubmitCoordinates,
    dispatch: &mut F,
) -> Result<FormDispatchOutcome, FormPlanningError>
where
    F: FnMut(&mut FormLifecycleEvent),
{
    let mut events = Vec::new();
    let skip_validation =
        form.novalidate || submitter.is_some_and(|binding| binding.form_novalidate);
    if !skip_validation {
        let report = dispatch_validation_for_form(
            controls,
            form,
            form_index,
            FormValidationMode::Submit,
            dispatch,
        );
        if !report.valid {
            return Ok(FormDispatchOutcome {
                activation: FormActivation::Invalid(report.diagnostics),
                events: report.events,
            });
        }
    }

    let submitter_key = submitter.map(|binding| binding.key.clone());
    let mut submit_event = FormLifecycleEvent::Submit {
        form_id: form.id.clone(),
        form_index,
        submitter_key: submitter_key.clone(),
        cancelable: true,
        default_prevented: false,
    };
    dispatch(&mut submit_event);
    let submit_canceled = submit_event.default_prevented();
    events.push(submit_event);
    if submit_canceled {
        return Ok(FormDispatchOutcome {
            activation: FormActivation::None,
            events,
        });
    }

    let method_name = submitter
        .and_then(|binding| binding.form_method.as_deref())
        .unwrap_or(&form.method)
        .to_ascii_lowercase();
    let method = match method_name.as_str() {
        "get" => FormMethod::Get,
        "post" => FormMethod::Post,
        _ => return Err(FormPlanningError::UnsupportedMethod(method_name)),
    };
    let encoding = submitter
        .and_then(|binding| binding.form_enctype.as_deref())
        .or(form.enctype.as_deref())
        .unwrap_or("application/x-www-form-urlencoded")
        .to_ascii_lowercase();
    if method == FormMethod::Post
        && !matches!(
            encoding.as_str(),
            "application/x-www-form-urlencoded" | "multipart/form-data"
        )
    {
        return Err(FormPlanningError::UnsupportedEncoding(encoding));
    }

    let data =
        collect_successful_controls(controls, form, form_index, submitter, image_coordinates)?;
    let mut form_data_event = FormLifecycleEvent::FormData {
        form_id: form.id.clone(),
        form_index,
        submitter_key,
        entries: data.iter().map(form_data_entry).collect(),
    };
    dispatch(&mut form_data_event);
    let data = match &form_data_event {
        FormLifecycleEvent::FormData { entries, .. } => form_data(entries)?,
        _ => data,
    };
    events.push(form_data_event);
    let entries = data
        .iter()
        .map(|datum| match datum {
            FormDatum::Text(entry) => entry.clone(),
            FormDatum::File(entry) => FormEntry {
                name: entry.name.clone(),
                value: entry.file.name.clone(),
            },
        })
        .collect::<Vec<_>>();
    let files = data
        .iter()
        .filter_map(|datum| match datum {
            FormDatum::File(entry) => Some(entry.clone()),
            FormDatum::Text(_) => None,
        })
        .collect::<Vec<_>>();
    let action = submitter
        .and_then(|binding| binding.resolved_form_action.as_deref())
        .or(form.resolved_action.as_deref())
        .or_else(|| submitter.and_then(|binding| binding.form_action.as_deref()))
        .or(form.action.as_deref())
        .unwrap_or(document_url);
    let action = resolve_action(action, document_url)?;
    let (url, content_type, body) = match (method, encoding.as_str()) {
        (FormMethod::Get, _) => {
            let encoded = bounded_urlencoded(&entries)?;
            (with_query(&action, &encoded)?, None, Vec::new())
        }
        (FormMethod::Post, "application/x-www-form-urlencoded") => {
            let encoded = bounded_urlencoded(&entries)?;
            (
                action,
                Some("application/x-www-form-urlencoded".to_string()),
                encoded.into_bytes(),
            )
        }
        (FormMethod::Post, "multipart/form-data") => {
            let boundary = multipart_boundary(&data);
            let body = encode_multipart(&data, &boundary)?;
            (
                action,
                Some(format!("multipart/form-data; boundary={boundary}")),
                body,
            )
        }
        (FormMethod::Post, _) => unreachable!("encoding checked above"),
    };
    Ok(FormDispatchOutcome {
        activation: FormActivation::Navigate(FormNavigation {
            method,
            url,
            content_type,
            body,
            entries,
            files,
        }),
        events,
    })
}

fn form_data_entry(datum: &FormDatum) -> FormDataEntry {
    match datum {
        FormDatum::Text(entry) => FormDataEntry {
            name: entry.name.clone(),
            value: FormDataValue::Text(entry.value.clone()),
        },
        FormDatum::File(entry) => FormDataEntry {
            name: entry.name.clone(),
            value: FormDataValue::File(entry.file.clone()),
        },
    }
}

fn form_data(entries: &[FormDataEntry]) -> Result<Vec<FormDatum>, FormPlanningError> {
    if entries.len() > MAX_FORM_ENTRIES {
        return Err(FormPlanningError::TooManyEntries {
            limit: MAX_FORM_ENTRIES,
        });
    }
    Ok(entries
        .iter()
        .map(|entry| match &entry.value {
            FormDataValue::Text(value) => FormDatum::Text(FormEntry {
                name: entry.name.clone(),
                value: value.clone(),
            }),
            FormDataValue::File(file) => FormDatum::File(FormFileEntry {
                name: entry.name.clone(),
                file: file.clone(),
            }),
        })
        .collect())
}

fn bounded_urlencoded(entries: &[FormEntry]) -> Result<String, FormPlanningError> {
    let encoded = encode_form_entries(entries);
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(FormPlanningError::PayloadTooLarge {
            limit: MAX_ENCODED_BYTES,
        });
    }
    Ok(encoded)
}

fn validate_controls(
    model: &BrowserControlModel,
    form_id: Option<&str>,
    form_index: usize,
) -> Vec<FormDiagnostic> {
    let mut diagnostics = Vec::new();
    for (control, binding) in model.controls().iter().zip(model.bindings()) {
        if !associated_with(binding, form_id, form_index)
            || control.disabled
            || control.readonly
            || matches!(control.kind, ControlKind::Button)
        {
            continue;
        }
        if control.required {
            let missing = match control.kind {
                ControlKind::Checkbox => !control.checked,
                ControlKind::Radio => !model.controls().iter().zip(model.bindings()).any(
                    |(candidate, candidate_binding)| {
                        associated_with(candidate_binding, form_id, form_index)
                            && candidate.kind == ControlKind::Radio
                            && candidate.name == control.name
                            && candidate.checked
                            && !candidate.disabled
                    },
                ),
                ControlKind::Select => model
                    .selected_values(&binding.key)
                    .is_none_or(|values| values.is_empty()),
                _ => control.value.is_empty(),
            };
            if missing {
                push_diagnostic(
                    &mut diagnostics,
                    "value-missing",
                    control,
                    "required control has no value",
                );
            }
        }
        if let Some(value_state) = model.value_state(&binding.key) {
            for value_diagnostic in value_state.diagnostics {
                push_diagnostic(
                    &mut diagnostics,
                    value_diagnostic.code,
                    control,
                    value_diagnostic.message,
                );
            }
        }
        validate_pattern(control, binding, &mut diagnostics);
        if diagnostics.len() >= MAX_DIAGNOSTICS {
            break;
        }
    }
    for diagnostic in model.custom_element_diagnostics(form_id, form_index) {
        if diagnostics.len() >= MAX_DIAGNOSTICS {
            break;
        }
        diagnostics.push(FormDiagnostic {
            code: diagnostic.code,
            key: Some(diagnostic.key),
            message: diagnostic.message,
        });
    }
    diagnostics
}

fn validate_pattern(
    control: &ControlState,
    binding: &ControlBinding,
    diagnostics: &mut Vec<FormDiagnostic>,
) {
    let Some(pattern) = binding.pattern.as_deref() else {
        return;
    };
    if control.value.is_empty() {
        return;
    }
    let pattern = format!("^(?:{pattern})$");
    match Regex::new(&pattern) {
        Ok(pattern) if !pattern.is_match(&control.value) => push_diagnostic(
            diagnostics,
            "pattern-mismatch",
            control,
            "value does not match pattern",
        ),
        Err(_) => push_diagnostic(
            diagnostics,
            "invalid-pattern",
            control,
            "pattern could not be compiled",
        ),
        Ok(_) => {}
    }
}

fn push_diagnostic(
    diagnostics: &mut Vec<FormDiagnostic>,
    code: &'static str,
    control: &ControlState,
    message: &str,
) {
    if diagnostics.len() < MAX_DIAGNOSTICS {
        diagnostics.push(FormDiagnostic {
            code,
            key: Some(control.key.clone()),
            message: message.to_string(),
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FormDatum {
    Text(FormEntry),
    File(FormFileEntry),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrderedFormData {
    document_order: usize,
    sequence: usize,
    data: Vec<FormDatum>,
}

fn collect_successful_controls(
    model: &BrowserControlModel,
    form: &BrowserForm,
    form_index: usize,
    submitter: Option<&ControlBinding>,
    image_coordinates: ImageSubmitCoordinates,
) -> Result<Vec<FormDatum>, FormPlanningError> {
    let mut groups = Vec::new();
    let mut used = vec![false; model.controls().len()];
    let mut used_orders = Vec::new();
    for (sequence, source) in form.controls.iter().enumerate() {
        let dynamic = find_dynamic_control(model, form.id.as_deref(), form_index, source, &used);
        if let Some((index, _, _)) = dynamic {
            used[index] = true;
        }
        let document_order = dynamic
            .as_ref()
            .map(|(_, _, binding)| binding.document_order)
            .or_else(|| {
                model.form_control_document_order(
                    form.id.as_deref(),
                    form_index,
                    source.id.as_deref(),
                    source.name.as_deref(),
                    &source.control_type,
                    &used_orders,
                )
            })
            .unwrap_or(usize::MAX.saturating_sub(form.controls.len() - sequence));
        used_orders.push(document_order);
        let mut data = Vec::new();
        append_control_entries(
            &mut data,
            model,
            source,
            dynamic,
            submitter,
            image_coordinates,
        );
        if !data.is_empty() {
            groups.push(OrderedFormData {
                document_order,
                sequence,
                data,
            });
        }
    }
    let native_count = groups.len();
    groups.extend(
        model
            .custom_element_submission_groups(form.id.as_deref(), form_index)
            .into_iter()
            .enumerate()
            .map(|(sequence, group)| OrderedFormData {
                document_order: group.document_order,
                sequence: native_count + sequence,
                data: group
                    .entries
                    .into_iter()
                    .map(|entry| match entry.value {
                        CustomElementFormEntryValue::Text(value) => FormDatum::Text(FormEntry {
                            name: entry.name,
                            value,
                        }),
                        CustomElementFormEntryValue::File(file) => FormDatum::File(FormFileEntry {
                            name: entry.name,
                            file,
                        }),
                    })
                    .collect(),
            }),
    );
    groups.sort_by_key(|group| (group.document_order, group.sequence));
    let data = groups
        .into_iter()
        .flat_map(|group| group.data)
        .collect::<Vec<_>>();
    if data.len() > MAX_FORM_ENTRIES {
        return Err(FormPlanningError::TooManyEntries {
            limit: MAX_FORM_ENTRIES,
        });
    }
    Ok(data)
}

fn find_dynamic_control<'a>(
    model: &'a BrowserControlModel,
    form_id: Option<&str>,
    form_index: usize,
    source: &BrowserFormControl,
    used: &[bool],
) -> Option<(usize, &'a ControlState, &'a ControlBinding)> {
    model
        .controls()
        .iter()
        .zip(model.bindings())
        .enumerate()
        .find(|(index, (_, binding))| {
            !used[*index]
                && associated_with(binding, form_id, form_index)
                && source
                    .id
                    .as_ref()
                    .is_some_and(|id| binding.id.as_ref() == Some(id))
        })
        .or_else(|| {
            model
                .controls()
                .iter()
                .zip(model.bindings())
                .enumerate()
                .find(|(index, (control, binding))| {
                    !used[*index]
                        && associated_with(binding, form_id, form_index)
                        && source.name == control.name
                        && source.control_type == binding.control_type
                })
        })
        .map(|(index, (control, binding))| (index, control, binding))
}

fn append_control_entries(
    data: &mut Vec<FormDatum>,
    model: &BrowserControlModel,
    source: &BrowserFormControl,
    dynamic: Option<(usize, &ControlState, &ControlBinding)>,
    submitter: Option<&ControlBinding>,
    image_coordinates: ImageSubmitCoordinates,
) {
    if source.disabled {
        return;
    }
    let active_submitter = dynamic
        .as_ref()
        .and_then(|(_, _, binding)| submitter.map(|submitter| binding.key == submitter.key))
        .unwrap_or(false);
    match source.control_type.as_str() {
        "button" | "reset" => return,
        "submit" | "image" if !active_submitter => return,
        "checkbox" | "radio"
            if !dynamic
                .as_ref()
                .map(|(_, control, _)| control.checked)
                .unwrap_or(source.checked) =>
        {
            return
        }
        _ => {}
    }
    if source.control_type == "image" {
        let prefix = source
            .name
            .as_deref()
            .filter(|name| !name.is_empty())
            .map(|name| format!("{name}."))
            .unwrap_or_default();
        data.push(FormDatum::Text(FormEntry {
            name: format!("{prefix}x"),
            value: image_coordinates.x.to_string(),
        }));
        data.push(FormDatum::Text(FormEntry {
            name: format!("{prefix}y"),
            value: image_coordinates.y.to_string(),
        }));
        return;
    }
    let Some(name) = source.name.as_ref().filter(|name| !name.is_empty()) else {
        append_dirname_entry(data, model, source, dynamic);
        return;
    };
    if source.control_type == "file" {
        if let Some((_, _, binding)) = dynamic {
            if let Some(files) = model.selected_files(&binding.key) {
                data.extend(files.iter().cloned().map(|file| {
                    FormDatum::File(FormFileEntry {
                        name: name.clone(),
                        file,
                    })
                }));
            }
        }
        return;
    }
    let values = match source.control_type.as_str() {
        "select" => dynamic
            .as_ref()
            .map(|(_, control, _)| {
                control
                    .selected_indices
                    .iter()
                    .filter(|index| {
                        !control
                            .option_disabled
                            .get(**index)
                            .copied()
                            .unwrap_or(false)
                    })
                    .filter_map(|index| control.options.get(*index))
                    .cloned()
                    .collect()
            })
            .unwrap_or_else(|| {
                source
                    .option_items
                    .iter()
                    .filter(|option| option.selected && !option.disabled)
                    .map(|option| option.value.clone())
                    .collect()
            }),
        "checkbox" | "radio" => vec![source.value.clone().unwrap_or_else(|| "on".into())],
        "submit" => vec![source.value.clone().unwrap_or_default()],
        _ => dynamic
            .as_ref()
            .map(|(_, control, _)| vec![normalize_line_breaks(&control.value)])
            .unwrap_or_else(|| {
                if source.submission_values.is_empty() {
                    vec![normalize_line_breaks(source.value.as_deref().unwrap_or(""))]
                } else {
                    source
                        .submission_values
                        .iter()
                        .map(|value| normalize_line_breaks(value))
                        .collect()
                }
            }),
    };
    data.extend(values.into_iter().map(|value| {
        FormDatum::Text(FormEntry {
            name: name.clone(),
            value,
        })
    }));
    append_dirname_entry(data, model, source, dynamic);
}

fn append_dirname_entry(
    data: &mut Vec<FormDatum>,
    model: &BrowserControlModel,
    source: &BrowserFormControl,
    dynamic: Option<(usize, &ControlState, &ControlBinding)>,
) {
    if !matches!(source.control_type.as_str(), "text" | "search" | "textarea") {
        return;
    }
    let dirname = dynamic
        .as_ref()
        .and_then(|(_, _, binding)| binding.dirname.as_deref())
        .or(source.dirname.as_deref())
        .filter(|dirname| !dirname.is_empty());
    let Some(dirname) = dirname else {
        return;
    };
    let direction = dynamic
        .and_then(|(_, _, binding)| model.directionality(&binding.key))
        .unwrap_or_default();
    data.push(FormDatum::Text(FormEntry {
        name: dirname.to_string(),
        value: direction.as_str().to_string(),
    }));
}

fn normalize_coordinate(value: f64) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= u32::MAX as f64 {
        u32::MAX
    } else {
        value.floor() as u32
    }
}

fn associated_form<'a>(
    document: &'a BrowserDocument,
    binding: &ControlBinding,
) -> Option<(usize, &'a BrowserForm)> {
    if let Some(owner) = binding.form_owner.as_deref() {
        return document
            .forms
            .iter()
            .enumerate()
            .find(|(_, form)| form.id.as_deref() == Some(owner));
    }
    binding
        .form_index
        .and_then(|index| document.forms.get(index).map(|form| (index, form)))
}

fn associated_with(binding: &ControlBinding, form_id: Option<&str>, form_index: usize) -> bool {
    match binding.form_owner.as_deref() {
        Some(owner) => form_id == Some(owner),
        None => binding.form_index == Some(form_index),
    }
}

fn resolve_action(action: &str, document_url: &str) -> Result<String, FormPlanningError> {
    if action.is_empty() {
        return Ok(document_url.to_string());
    }
    if Url::parse(action).is_ok() {
        return Ok(action.to_string());
    }
    Url::parse(document_url)
        .and_then(|base| base.resolve(action))
        .map(|url| url.to_url_string())
        .map_err(|_| FormPlanningError::InvalidAction(action.to_string()))
}

fn with_query(action: &str, query: &str) -> Result<String, FormPlanningError> {
    let mut url =
        Url::parse(action).map_err(|_| FormPlanningError::InvalidAction(action.to_string()))?;
    url.query = (!query.is_empty()).then(|| query.to_string());
    Ok(url.to_url_string())
}

pub fn encode_form_entries(entries: &[FormEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "{}={}",
                encode_component(&entry.name),
                encode_component(&entry.value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn multipart_boundary(data: &[FormDatum]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for datum in data {
        match datum {
            FormDatum::Text(entry) => {
                hash_bytes(&mut hash, entry.name.as_bytes());
                hash_bytes(&mut hash, entry.value.as_bytes());
            }
            FormDatum::File(entry) => {
                hash_bytes(&mut hash, entry.name.as_bytes());
                hash_bytes(&mut hash, entry.file.opaque_id.as_bytes());
                hash_bytes(&mut hash, entry.file.name.as_bytes());
                hash_bytes(&mut hash, &entry.file.bytes);
            }
        }
    }
    for salt in 0_u64.. {
        let candidate = format!("----venture-{hash:016x}-{salt:x}");
        if !data.iter().any(|datum| match datum {
            FormDatum::Text(entry) => entry
                .value
                .as_bytes()
                .windows(candidate.len())
                .any(|part| part == candidate.as_bytes()),
            FormDatum::File(entry) => entry
                .file
                .bytes
                .windows(candidate.len())
                .any(|part| part == candidate.as_bytes()),
        }) {
            return candidate;
        }
    }
    unreachable!("u64 boundary salt is exhaustive")
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
    *hash ^= 0xff;
    *hash = hash.wrapping_mul(0x100000001b3);
}

fn encode_multipart(data: &[FormDatum], boundary: &str) -> Result<Vec<u8>, FormPlanningError> {
    let mut body = Vec::new();
    for datum in data {
        append_multipart(&mut body, format!("--{boundary}\r\n").as_bytes())?;
        match datum {
            FormDatum::Text(entry) => {
                let name = multipart_quoted(&entry.name);
                append_multipart(
                    &mut body,
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                )?;
                append_multipart(&mut body, entry.value.as_bytes())?;
            }
            FormDatum::File(entry) => {
                let name = multipart_quoted(&entry.name);
                let filename = multipart_quoted(&entry.file.name);
                let media_type = entry
                    .file
                    .media_type
                    .as_deref()
                    .unwrap_or("application/octet-stream");
                append_multipart(
                    &mut body,
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {media_type}\r\n\r\n"
                    )
                    .as_bytes(),
                )?;
                append_multipart(&mut body, &entry.file.bytes)?;
            }
        }
        append_multipart(&mut body, b"\r\n")?;
    }
    append_multipart(&mut body, format!("--{boundary}--\r\n").as_bytes())?;
    Ok(body)
}

fn append_multipart(body: &mut Vec<u8>, bytes: &[u8]) -> Result<(), FormPlanningError> {
    if body.len().saturating_add(bytes.len()) > MAX_MULTIPART_BYTES {
        return Err(FormPlanningError::PayloadTooLarge {
            limit: MAX_MULTIPART_BYTES,
        });
    }
    body.extend_from_slice(bytes);
    Ok(())
}

fn multipart_quoted(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace('"', "%22")
}

fn encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                encoded.push(char::from(*byte));
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn normalize_line_breaks(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::{parse_html, BrowserRenderTree};

    fn model_and_document(source: &str, url: &str) -> (BrowserControlModel, BrowserDocument) {
        let parsed = parse_html(source).unwrap();
        (
            BrowserControlModel::from_render_tree(
                &BrowserRenderTree::from_document_with_document_url(&parsed, url),
            ),
            BrowserDocument::from_document(&parsed),
        )
    }

    #[test]
    fn custom_element_values_join_native_controls_in_document_order() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form id='checkout' action='/save'>\
             <input name='before' value='a'>\
             <x-tags id='tags' name='tag'></x-tags>\
             <input name='after' value='z'>\
             <button id='go'>Save</button></form>",
            url,
        );
        let key = model.form_associated_custom_elements()[0].key.clone();
        model.attach_form_associated_custom_element(&key).unwrap();
        model
            .set_custom_element_form_value(
                &key,
                Some(browser_form_controls::CustomElementFormValue::Entries(
                    vec![
                        browser_form_controls::CustomElementFormEntry {
                            name: "tag".into(),
                            value: CustomElementFormEntryValue::Text("rust".into()),
                        },
                        browser_form_controls::CustomElementFormEntry {
                            name: "tag".into(),
                            value: CustomElementFormEntryValue::Text("browser".into()),
                        },
                    ],
                )),
                Some(browser_form_controls::CustomElementFormValue::Text(
                    "rust,browser".into(),
                )),
            )
            .unwrap();

        let activation = plan_activation(&document, &model, "control:2:id:go", url).unwrap();
        let FormActivation::Navigate(navigation) = activation else {
            panic!("expected navigation");
        };
        assert_eq!(
            navigation.url,
            "http://example.test/save?before=a&tag=rust&tag=browser&after=z"
        );
    }

    #[test]
    fn custom_element_validity_blocks_submission_with_shared_diagnostic() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form><x-rating id='rating' name='rating'></x-rating><button id='go'>Go</button></form>",
            url,
        );
        let key = model.form_associated_custom_elements()[0].key.clone();
        model.attach_form_associated_custom_element(&key).unwrap();
        model
            .set_custom_element_validity(
                &key,
                browser_form_controls::CustomElementValidity {
                    value_missing: true,
                    ..browser_form_controls::CustomElementValidity::default()
                },
                Some("Choose a rating".into()),
                Some("rating".into()),
            )
            .unwrap();
        let FormActivation::Invalid(diagnostics) =
            plan_activation(&document, &model, "control:0:id:go", url).unwrap()
        else {
            panic!("expected invalid activation");
        };
        assert_eq!(diagnostics[0].code, "custom-element-invalid");
        assert_eq!(diagnostics[0].key.as_deref(), Some(key.as_str()));
        assert_eq!(diagnostics[0].message, "Choose a rating");
    }

    #[test]
    fn request_submit_dispatches_submit_then_mutable_formdata() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form id='checkout' action='/save'><input name='before' value='a'>\
             <x-code id='code' name='code'></x-code>\
             <button id='draft' name='intent' value='draft'>Draft</button>\
             <button id='publish' name='intent' value='publish'>Publish</button></form>",
            url,
        );
        let custom_key = model.form_associated_custom_elements()[0].key.clone();
        model
            .attach_form_associated_custom_element(&custom_key)
            .unwrap();
        model
            .set_custom_element_form_value(
                &custom_key,
                Some(browser_form_controls::CustomElementFormValue::Text(
                    "ABC".into(),
                )),
                None,
            )
            .unwrap();

        let outcome = dispatch_request_submit(
            &document,
            &model,
            0,
            Some("control:2:id:publish"),
            url,
            |event| {
                if let Some(entries) = event.form_data_mut() {
                    entries.retain(|entry| entry.name != "before");
                    entries.push(FormDataEntry {
                        name: "scripted".into(),
                        value: FormDataValue::Text("yes".into()),
                    });
                }
            },
        )
        .unwrap();
        assert!(matches!(
            outcome.events[0],
            FormLifecycleEvent::Submit { .. }
        ));
        assert!(matches!(
            outcome.events[1],
            FormLifecycleEvent::FormData { .. }
        ));
        let FormActivation::Navigate(navigation) = outcome.activation else {
            panic!("expected navigation");
        };
        assert_eq!(
            navigation.url,
            "http://example.test/save?code=ABC&intent=publish&scripted=yes"
        );
    }

    #[test]
    fn submit_and_reset_events_can_cancel_default_actions() {
        let url = "http://example.test/form";
        let (model, document) = model_and_document(
            "<form><input name='q' value='rust'><button id='go'>Go</button>\
             <button id='clear' type='reset'>Clear</button></form>",
            url,
        );
        let submitted = dispatch_request_submit(&document, &model, 0, None, url, |event| {
            if matches!(event, FormLifecycleEvent::Submit { .. }) {
                assert!(event.prevent_default());
            }
        })
        .unwrap();
        assert_eq!(submitted.activation, FormActivation::None);
        assert_eq!(submitted.events.len(), 1);

        let reset = dispatch_activation_with_image_coordinates(
            &document,
            &model,
            "control:2:id:clear",
            url,
            ImageSubmitCoordinates::KEYBOARD,
            |event| {
                if matches!(event, FormLifecycleEvent::Reset { .. }) {
                    event.prevent_default();
                }
            },
        )
        .unwrap();
        assert_eq!(reset.activation, FormActivation::None);
        assert!(reset.events[0].default_prevented());
    }

    #[test]
    fn check_and_report_validity_share_cancelable_invalid_events() {
        let url = "http://example.test/form";
        let (model, document) = model_and_document(
            "<form><input id='required' required><button>Go</button></form>",
            url,
        );
        let checked = check_form_validity(&document, &model, 0, |event| {
            assert!(matches!(
                event,
                FormLifecycleEvent::Invalid {
                    mode: FormValidationMode::Check,
                    ..
                }
            ));
            event.prevent_default();
        })
        .unwrap();
        assert!(!checked.valid);
        assert!(checked.reportable_diagnostics().is_empty());

        let reported = report_form_validity(&document, &model, 0, |_| {}).unwrap();
        assert!(!reported.valid);
        assert_eq!(reported.reportable_diagnostics(), reported.diagnostics);
        assert!(matches!(
            reported.events[0],
            FormLifecycleEvent::Invalid {
                mode: FormValidationMode::Report,
                ..
            }
        ));
    }

    #[test]
    fn request_submit_rejects_non_submit_and_disabled_submitters() {
        let url = "http://example.test/form";
        let (model, document) = model_and_document(
            "<form><input id='field'><button id='disabled' disabled>Go</button></form>",
            url,
        );
        assert!(matches!(
            dispatch_request_submit(
                &document,
                &model,
                0,
                Some("control:0:id:field"),
                url,
                |_| {}
            ),
            Err(FormPlanningError::InvalidSubmitter(_))
        ));
        assert!(matches!(
            dispatch_request_submit(
                &document,
                &model,
                0,
                Some("control:1:id:disabled"),
                url,
                |_| {}
            ),
            Err(FormPlanningError::DisabledSubmitter(_))
        ));
    }

    #[test]
    fn serializes_successful_controls_and_active_submitter_in_order() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form action='/search'><input id='q' name='q' required>\
             <input id='off' name='off' disabled value='no'>\
             <input id='a' type='radio' name='scope' value='a'>\
             <input id='b' type='radio' name='scope' value='b' checked>\
             <select id='sort' name='sort'><option value='new'>Newest</option>\
             <option value='old' selected>Oldest</option></select>\
             <button id='go' name='commit' value='yes'>Go</button></form>",
            url,
        );
        model.focus("control:0:id:q");
        model.text_input("rust language");
        let activation = plan_activation(&document, &model, "control:5:id:go", url).unwrap();
        let FormActivation::Navigate(request) = activation else {
            panic!("expected navigation");
        };
        assert_eq!(request.method, FormMethod::Get);
        assert_eq!(
            request.url,
            "http://example.test/search?q=rust+language&scope=b&sort=old&commit=yes"
        );
    }

    #[test]
    fn validation_is_bounded_and_blocks_navigation() {
        let url = "http://example.test/form";
        let (model, document) = model_and_document(
            "<form><input id='mail' name='mail' type='email' required pattern='.+@example\\.com'>\
             <button id='go'>Go</button></form>",
            url,
        );
        let FormActivation::Invalid(diagnostics) =
            plan_activation(&document, &model, "control:1:id:go", url).unwrap()
        else {
            panic!("expected invalid activation");
        };
        assert_eq!(diagnostics[0].code, "value-missing");
        assert!(diagnostics.len() <= MAX_DIAGNOSTICS);
    }

    #[test]
    fn shared_typed_value_diagnostics_block_and_then_allow_submission() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form><input id='site' name='site' type='url' value='http://['>\
             <input id='count' name='count' type='number' min='0' max='10' step='2' value='3'>\
             <button id='go'>Go</button></form>",
            url,
        );
        let FormActivation::Invalid(diagnostics) =
            plan_activation(&document, &model, "control:2:id:go", url).unwrap()
        else {
            panic!("expected invalid activation");
        };
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            vec!["type-mismatch", "step-mismatch"]
        );

        model.focus("control:0:id:site");
        model.accessibility_action(browser_form_controls::ControlAccessibilityAction::SetValue(
            "https://example.test/path".into(),
        ));
        model.focus("control:1:id:count");
        model.accessibility_action(browser_form_controls::ControlAccessibilityAction::Increment);
        assert!(matches!(
            plan_activation(&document, &model, "control:2:id:go", url).unwrap(),
            FormActivation::Navigate(_)
        ));
    }

    #[test]
    fn plans_urlencoded_post_and_reset_effects() {
        let url = "http://example.test/form";
        let (model, document) = model_and_document(
            "<form id='profile' action='/save' method='post'>\
             <textarea id='bio' name='bio'>hello\nworld</textarea>\
             <button id='reset' type='reset'>Reset</button>\
             <button id='save' name='intent' value='save'>Save</button></form>",
            url,
        );
        assert_eq!(
            plan_activation(&document, &model, "control:1:id:reset", url).unwrap(),
            FormActivation::Reset {
                form_id: Some("profile".into()),
                form_index: 0,
            }
        );
        let FormActivation::Navigate(request) =
            plan_activation(&document, &model, "control:2:id:save", url).unwrap()
        else {
            panic!("expected post navigation");
        };
        assert_eq!(request.method, FormMethod::Post);
        assert_eq!(request.url, "http://example.test/save");
        assert_eq!(
            request.content_type.as_deref(),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(request.body, b"bio=hello%0D%0Aworld&intent=save".to_vec());
    }

    #[test]
    fn encoding_uses_utf8_percent_bytes_and_form_spaces() {
        assert_eq!(
            encode_form_entries(&[FormEntry {
                name: "query".into(),
                value: "caf\u{e9} & tea".into(),
            }]),
            "query=caf%C3%A9+%26+tea"
        );
    }

    #[test]
    fn multi_select_serialization_uses_live_enabled_selections() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form><select id='tags' name='tag' multiple required>\
             <option value='a' selected>A</option>\
             <optgroup disabled><option value='b' selected>B</option></optgroup>\
             <option value='c'>C</option><option value='d' disabled>D</option>\
             </select><button id='go'>Go</button></form>",
            url,
        );
        model.select_option("control:0:id:tags", 2, false, true);
        assert_eq!(
            model.select_option("control:0:id:tags", 3, false, true),
            None
        );

        let FormActivation::Navigate(request) =
            plan_activation(&document, &model, "control:1:id:go", url).unwrap()
        else {
            panic!("expected navigation");
        };
        assert_eq!(
            request.entries,
            vec![
                FormEntry {
                    name: "tag".into(),
                    value: "a".into(),
                },
                FormEntry {
                    name: "tag".into(),
                    value: "c".into(),
                },
            ]
        );
    }

    #[test]
    fn temporal_and_color_controls_validate_and_serialize_canonical_values() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form action='/values'>\
             <input id='day' name='day' type='date' value='2024-01-02' min='2024-01-01' step='2'>\
             <input id='clock' name='clock' type='time' value='09:30:05.120' step='0.01'>\
             <input id='ink' name='ink' type='color' value='#A0b1C2'>\
             <button id='go'>Go</button></form>",
            url,
        );
        let FormActivation::Invalid(diagnostics) =
            plan_activation(&document, &model, "control:3:id:go", url).unwrap()
        else {
            panic!("expected step mismatch");
        };
        assert_eq!(diagnostics[0].code, "step-mismatch");

        model.focus("control:0:id:day");
        model.accessibility_action(browser_form_controls::ControlAccessibilityAction::Increment);
        let FormActivation::Navigate(request) =
            plan_activation(&document, &model, "control:3:id:go", url).unwrap()
        else {
            panic!("expected navigation");
        };
        assert_eq!(
            request.url,
            "http://example.test/values?day=2024-01-03&clock=09%3A30%3A05.12&ink=%23a0b1c2"
        );
    }

    #[test]
    fn multipart_submission_is_deterministic_and_path_free() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form action='/upload' method='post' enctype='multipart/form-data'>\
             <input name='title' value='Field notes'>\
             <input id='assets' name='asset' type='file' accept='image/*,.txt' multiple required>\
             <button id='go'>Upload</button></form>",
            url,
        );
        model.apply_file_selection(
            "control:1:id:assets",
            vec![
                browser_form_controls::HostFileSelection::new(
                    "picker:photo",
                    "/Users/example/secret/photo.png",
                    Some("image/png".into()),
                    vec![0x89, b'P', b'N', b'G'],
                ),
                browser_form_controls::HostFileSelection::new(
                    "picker:notes",
                    "notes.txt",
                    Some("text/plain".into()),
                    b"hello\r\nworld".to_vec(),
                ),
            ],
        );

        let first = plan_activation(&document, &model, "control:2:id:go", url).unwrap();
        let second = plan_activation(&document, &model, "control:2:id:go", url).unwrap();
        assert_eq!(first, second);
        let FormActivation::Navigate(request) = first else {
            panic!("expected multipart navigation");
        };
        assert_eq!(request.files.len(), 2);
        assert_eq!(request.files[0].file.name, "photo.png");
        assert_eq!(request.entries[1].value, "photo.png");
        let content_type = request.content_type.unwrap();
        let boundary = content_type
            .strip_prefix("multipart/form-data; boundary=")
            .unwrap();
        assert!(request
            .body
            .starts_with(format!("--{boundary}\r\n").as_bytes()));
        assert!(request
            .body
            .ends_with(format!("--{boundary}--\r\n").as_bytes()));
        let body = String::from_utf8_lossy(&request.body);
        assert!(body.contains("name=\"title\"\r\n\r\nField notes"));
        assert!(body.contains("filename=\"photo.png\"\r\nContent-Type: image/png"));
        assert!(!body.contains("/Users/example/secret"));
    }

    #[test]
    fn multipart_payload_limit_is_enforced_before_navigation() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form method='post' enctype='multipart/form-data'>\
             <input id='assets' name='asset' type='file' multiple>\
             <button id='go'>Upload</button></form>",
            url,
        );
        model.apply_file_selection(
            "control:0:id:assets",
            vec![browser_form_controls::HostFileSelection::new(
                "one",
                "one.bin",
                None,
                vec![0; MAX_MULTIPART_BYTES],
            )],
        );
        assert_eq!(
            plan_activation(&document, &model, "control:1:id:go", url),
            Err(FormPlanningError::PayloadTooLarge {
                limit: MAX_MULTIPART_BYTES,
            })
        );
    }

    #[test]
    fn image_submit_coordinates_expand_in_document_order() {
        let url = "http://example.test/form";
        let (model, document) = model_and_document(
            "<form action='/map'><input name='before' value='a'>\
             <input id='pin' type='image' name='pin' alt='Choose'>\
             <input name='after' value='b'></form>",
            url,
        );
        let FormActivation::Navigate(request) = plan_activation_with_image_coordinates(
            &document,
            &model,
            "control:1:id:pin",
            url,
            ImageSubmitCoordinates::from_local_point(12.9, 7.2),
        )
        .unwrap() else {
            panic!("expected image submission");
        };
        assert_eq!(
            request.entries,
            vec![
                FormEntry {
                    name: "before".into(),
                    value: "a".into()
                },
                FormEntry {
                    name: "pin.x".into(),
                    value: "12".into()
                },
                FormEntry {
                    name: "pin.y".into(),
                    value: "7".into()
                },
                FormEntry {
                    name: "after".into(),
                    value: "b".into()
                },
            ]
        );
        assert_eq!(
            request.url,
            "http://example.test/map?before=a&pin.x=12&pin.y=7&after=b"
        );
    }

    #[test]
    fn keyboard_image_submit_and_dirname_are_deterministic() {
        let url = "http://example.test/form";
        let (mut model, document) = model_and_document(
            "<form action='/search' dir='rtl'>\
             <input id='query' name='q' dirname='q.dir' dir='auto' value='שלום'>\
             <textarea id='notes' dirname='notes.dir' dir='ltr'>neutral 123</textarea>\
             <input id='go' type='image' alt='Search'></form>",
            url,
        );
        assert_eq!(
            model.directionality("control:0:id:query").unwrap().as_str(),
            "rtl"
        );
        model.focus("control:0:id:query");
        model.accessibility_action(browser_form_controls::ControlAccessibilityAction::SetValue(
            "123 -".into(),
        ));
        assert_eq!(
            model.directionality("control:0:id:query").unwrap().as_str(),
            "rtl"
        );
        model.accessibility_action(browser_form_controls::ControlAccessibilityAction::SetValue(
            "Venture".into(),
        ));
        let FormActivation::Navigate(request) =
            plan_activation(&document, &model, "control:2:id:go", url).unwrap()
        else {
            panic!("expected keyboard image submission");
        };
        assert_eq!(
            request.entries,
            vec![
                FormEntry {
                    name: "q".into(),
                    value: "Venture".into()
                },
                FormEntry {
                    name: "q.dir".into(),
                    value: "ltr".into()
                },
                FormEntry {
                    name: "notes.dir".into(),
                    value: "ltr".into()
                },
                FormEntry {
                    name: "x".into(),
                    value: "0".into()
                },
                FormEntry {
                    name: "y".into(),
                    value: "0".into()
                },
            ]
        );
    }

    #[test]
    fn image_coordinate_normalization_is_bounded() {
        assert_eq!(
            ImageSubmitCoordinates::from_local_point(-2.0, f64::NAN),
            ImageSubmitCoordinates::KEYBOARD
        );
        assert_eq!(
            ImageSubmitCoordinates::from_local_point(f64::INFINITY, u32::MAX as f64 + 10.0),
            ImageSubmitCoordinates { x: 0, y: u32::MAX }
        );
    }
}
