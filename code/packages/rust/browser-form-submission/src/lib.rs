//! Host-neutral HTML form submission and constraint validation.

use browser_form_controls::{BrowserControlModel, ControlBinding};
use coding_adventures_html_parser::{BrowserDocument, BrowserForm, BrowserFormControl};
use layout_controls::{ControlKind, ControlState};
use regex::Regex;
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const MAX_FORM_ENTRIES: usize = 1_024;
pub const MAX_ENCODED_BYTES: usize = 1024 * 1024;
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
pub struct FormNavigation {
    pub method: FormMethod,
    pub url: String,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub entries: Vec<FormEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormDiagnostic {
    pub code: &'static str,
    pub key: Option<String>,
    pub message: String,
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
            Self::TooManyEntries { limit } => {
                write!(formatter, "form exceeds the {limit}-entry limit")
            }
            Self::PayloadTooLarge { limit } => {
                write!(formatter, "encoded form exceeds the {limit}-byte limit")
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
    let binding = controls
        .binding(activated_key)
        .ok_or_else(|| FormPlanningError::UnknownControl(activated_key.to_string()))?;
    let (form_index, form) = associated_form(document, binding)
        .ok_or_else(|| FormPlanningError::MissingForm(activated_key.to_string()))?;
    match binding.control_type.as_str() {
        "reset" => Ok(FormActivation::Reset {
            form_id: form.id.clone(),
            form_index,
        }),
        "submit" | "image" => plan_submission(
            document,
            controls,
            form_index,
            form,
            Some(binding),
            document_url,
        ),
        _ => Ok(FormActivation::None),
    }
}

pub fn plan_implicit_submission(
    document: &BrowserDocument,
    controls: &BrowserControlModel,
    focused_key: &str,
    document_url: &str,
) -> Result<FormActivation, FormPlanningError> {
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
    plan_submission(
        document,
        controls,
        form_index,
        form,
        submitter,
        document_url,
    )
}

fn plan_submission(
    _document: &BrowserDocument,
    controls: &BrowserControlModel,
    form_index: usize,
    form: &BrowserForm,
    submitter: Option<&ControlBinding>,
    document_url: &str,
) -> Result<FormActivation, FormPlanningError> {
    let skip_validation =
        form.novalidate || submitter.is_some_and(|binding| binding.form_novalidate);
    if !skip_validation {
        let diagnostics = validate_controls(controls, form.id.as_deref(), form_index);
        if !diagnostics.is_empty() {
            return Ok(FormActivation::Invalid(diagnostics));
        }
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
    if method == FormMethod::Post && encoding != "application/x-www-form-urlencoded" {
        return Err(FormPlanningError::UnsupportedEncoding(encoding));
    }

    let entries = collect_successful_controls(controls, form, form_index, submitter)?;
    let encoded = encode_form_entries(&entries);
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(FormPlanningError::PayloadTooLarge {
            limit: MAX_ENCODED_BYTES,
        });
    }
    let action = submitter
        .and_then(|binding| binding.resolved_form_action.as_deref())
        .or(form.resolved_action.as_deref())
        .or_else(|| submitter.and_then(|binding| binding.form_action.as_deref()))
        .or(form.action.as_deref())
        .unwrap_or(document_url);
    let action = resolve_action(action, document_url)?;
    let (url, content_type, body) = match method {
        FormMethod::Get => (with_query(&action, &encoded)?, None, Vec::new()),
        FormMethod::Post => (
            action,
            Some("application/x-www-form-urlencoded".to_string()),
            encoded.into_bytes(),
        ),
    };
    Ok(FormActivation::Navigate(FormNavigation {
        method,
        url,
        content_type,
        body,
        entries,
    }))
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

fn collect_successful_controls(
    model: &BrowserControlModel,
    form: &BrowserForm,
    form_index: usize,
    submitter: Option<&ControlBinding>,
) -> Result<Vec<FormEntry>, FormPlanningError> {
    let mut entries = Vec::new();
    let mut used = vec![false; model.controls().len()];
    for source in &form.controls {
        let dynamic = find_dynamic_control(model, form.id.as_deref(), form_index, source, &used);
        if let Some((index, _, _)) = dynamic {
            used[index] = true;
        }
        append_control_entries(&mut entries, source, dynamic, submitter);
        if entries.len() > MAX_FORM_ENTRIES {
            return Err(FormPlanningError::TooManyEntries {
                limit: MAX_FORM_ENTRIES,
            });
        }
    }
    Ok(entries)
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
    entries: &mut Vec<FormEntry>,
    source: &BrowserFormControl,
    dynamic: Option<(usize, &ControlState, &ControlBinding)>,
    submitter: Option<&ControlBinding>,
) {
    if source.disabled {
        return;
    }
    let Some(name) = source.name.as_ref().filter(|name| !name.is_empty()) else {
        return;
    };
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
        "file" => source.submission_values.clone(),
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
    entries.extend(values.into_iter().map(|value| FormEntry {
        name: name.clone(),
        value,
    }));
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
}
