use coding_adventures_html_parser::{BrowserRenderNode, BrowserRenderTree};

use crate::{HostFileSelection, MAX_SELECTED_FILE_BYTES, MAX_SELECTED_TOTAL_BYTES};

pub const MAX_CUSTOM_ELEMENT_ENTRIES: usize = 256;
pub const MAX_CUSTOM_ELEMENT_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomElementValidity {
    pub value_missing: bool,
    pub type_mismatch: bool,
    pub pattern_mismatch: bool,
    pub too_long: bool,
    pub too_short: bool,
    pub range_underflow: bool,
    pub range_overflow: bool,
    pub step_mismatch: bool,
    pub bad_input: bool,
    pub custom_error: bool,
}

impl CustomElementValidity {
    pub const fn is_valid(&self) -> bool {
        !(self.value_missing
            || self.type_mismatch
            || self.pattern_mismatch
            || self.too_long
            || self.too_short
            || self.range_underflow
            || self.range_overflow
            || self.step_mismatch
            || self.bad_input
            || self.custom_error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomElementFormEntryValue {
    Text(String),
    File(HostFileSelection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomElementFormEntry {
    pub name: String,
    pub value: CustomElementFormEntryValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomElementFormValue {
    Text(String),
    File(HostFileSelection),
    Entries(Vec<CustomElementFormEntry>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomElementFormAssociation {
    pub form_owner: Option<String>,
    pub form_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomElementStateRestoreMode {
    Restore,
    Autocomplete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomElementRestorationEntry {
    pub key: String,
    pub state: CustomElementFormValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomElementLifecycleEvent {
    FormAssociated {
        key: String,
        association: CustomElementFormAssociation,
    },
    FormDisabled {
        key: String,
        disabled: bool,
    },
    FormReset {
        key: String,
    },
    FormStateRestore {
        key: String,
        state: CustomElementFormValue,
        mode: CustomElementStateRestoreMode,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomElementAccessibilityValue {
    pub value_text: Option<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub step: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomElementAccessibilityProjection {
    pub role: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomElementAccessibilityAction {
    SetValue(String),
    Increment,
    Decrement,
    ClearValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormAssociatedCustomElementState {
    pub key: String,
    pub element_id: Option<String>,
    pub definition_name: String,
    pub name: Option<String>,
    pub association: CustomElementFormAssociation,
    pub labels: Vec<String>,
    pub accessible_name: Option<String>,
    pub accessible_description: Option<String>,
    pub role: Option<String>,
    pub attached: bool,
    pub disabled: bool,
    pub will_validate: bool,
    pub validity: CustomElementValidity,
    pub validation_message: Option<String>,
    pub validation_anchor: Option<String>,
    pub value: Option<CustomElementFormValue>,
    pub restoration_state: Option<CustomElementFormValue>,
    pub accessibility_value: CustomElementAccessibilityValue,
    pub document_order: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomElementDiagnostic {
    pub code: &'static str,
    pub key: String,
    pub message: String,
    pub anchor: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomElementAccessibilityState {
    pub key: String,
    pub role: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub labels: Vec<String>,
    pub disabled: bool,
    pub invalid: bool,
    pub validation_message: Option<String>,
    pub validation_anchor: Option<String>,
    pub value: Option<String>,
    pub value_text: Option<String>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub step: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomElementSubmissionGroup {
    pub key: String,
    pub document_order: usize,
    pub entries: Vec<CustomElementFormEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CustomElementInternalsError {
    UnknownElement(String),
    NotAttached(String),
    MissingValidationMessage,
    TooManyEntries { limit: usize },
    TextValueTooLarge { limit: usize },
    FileValueTooLarge { limit: usize },
    ValuePayloadTooLarge { limit: usize },
    UnsupportedAccessibilityAction(String),
}

impl std::fmt::Display for CustomElementInternalsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownElement(key) => write!(formatter, "unknown custom element {key}"),
            Self::NotAttached(key) => write!(formatter, "custom element {key} has no internals"),
            Self::MissingValidationMessage => {
                formatter.write_str("invalid custom-element state requires a message")
            }
            Self::TooManyEntries { limit } => {
                write!(
                    formatter,
                    "custom-element value exceeds the {limit}-entry limit"
                )
            }
            Self::TextValueTooLarge { limit } => {
                write!(
                    formatter,
                    "custom-element text exceeds the {limit}-byte limit"
                )
            }
            Self::FileValueTooLarge { limit } => {
                write!(
                    formatter,
                    "custom-element file exceeds the {limit}-byte limit"
                )
            }
            Self::ValuePayloadTooLarge { limit } => {
                write!(
                    formatter,
                    "custom-element value exceeds the {limit}-byte limit"
                )
            }
            Self::UnsupportedAccessibilityAction(key) => {
                write!(
                    formatter,
                    "custom element {key} does not expose a numeric value"
                )
            }
        }
    }
}

impl std::error::Error for CustomElementInternalsError {}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CustomElementInternalsRegistry {
    elements: Vec<FormAssociatedCustomElementState>,
    lifecycle_events: Vec<CustomElementLifecycleEvent>,
    pending_restores: Vec<(String, CustomElementStateRestoreMode)>,
}

impl CustomElementInternalsRegistry {
    pub(crate) fn from_render_tree(tree: &BrowserRenderTree) -> Self {
        let mut elements = Vec::new();
        let mut next_form_index = 0;
        let mut next_custom_index = 0;
        let mut document_order = 0;
        collect_candidates(
            &tree.children,
            &mut elements,
            &mut next_form_index,
            &mut next_custom_index,
            &mut document_order,
            None,
            false,
        );
        Self {
            elements,
            lifecycle_events: Vec::new(),
            pending_restores: Vec::new(),
        }
    }

    pub(crate) fn elements(&self) -> &[FormAssociatedCustomElementState] {
        &self.elements
    }

    pub(crate) fn element(&self, key: &str) -> Option<&FormAssociatedCustomElementState> {
        self.elements.iter().find(|element| element.key == key)
    }

    pub(crate) fn attach(&mut self, key: &str) -> Result<(), CustomElementInternalsError> {
        let element = self.element_mut(key)?;
        if element.attached {
            return Ok(());
        }
        element.attached = true;
        element.will_validate = !element.disabled;
        let association = element.association.clone();
        let disabled = element.disabled;
        let key = element.key.clone();
        self.lifecycle_events
            .push(CustomElementLifecycleEvent::FormAssociated {
                key: key.clone(),
                association,
            });
        if disabled {
            self.lifecycle_events
                .push(CustomElementLifecycleEvent::FormDisabled {
                    key: key.clone(),
                    disabled,
                });
        }
        if let Some(position) = self
            .pending_restores
            .iter()
            .position(|(pending_key, _)| pending_key == &key)
        {
            let (_, mode) = self.pending_restores.remove(position);
            if let Some(state) = self
                .element(&key)
                .and_then(|element| element.restoration_state.clone())
            {
                self.lifecycle_events
                    .push(CustomElementLifecycleEvent::FormStateRestore { key, state, mode });
            }
        }
        Ok(())
    }

    pub(crate) fn reassociate(
        &mut self,
        key: &str,
        association: CustomElementFormAssociation,
    ) -> Result<(), CustomElementInternalsError> {
        let element = self.attached_mut(key)?;
        if element.association == association {
            return Ok(());
        }
        element.association = association.clone();
        self.lifecycle_events
            .push(CustomElementLifecycleEvent::FormAssociated {
                key: key.to_string(),
                association,
            });
        Ok(())
    }

    pub(crate) fn set_form_value(
        &mut self,
        key: &str,
        value: Option<CustomElementFormValue>,
        restoration_state: Option<CustomElementFormValue>,
    ) -> Result<(), CustomElementInternalsError> {
        validate_value(value.as_ref())?;
        validate_value(restoration_state.as_ref())?;
        let element = self.attached_mut(key)?;
        element.value = value;
        element.restoration_state = restoration_state;
        Ok(())
    }

    pub(crate) fn set_validity(
        &mut self,
        key: &str,
        validity: CustomElementValidity,
        message: Option<String>,
        anchor: Option<String>,
    ) -> Result<(), CustomElementInternalsError> {
        if !validity.is_valid() && message.as_deref().is_none_or(str::is_empty) {
            return Err(CustomElementInternalsError::MissingValidationMessage);
        }
        if let Some(message) = message.as_deref() {
            validate_text(message)?;
        }
        if let Some(anchor) = anchor.as_deref() {
            validate_text(anchor)?;
        }
        let element = self.attached_mut(key)?;
        element.validity = validity;
        element.validation_message = message.filter(|message| !message.is_empty());
        element.validation_anchor = anchor;
        Ok(())
    }

    pub(crate) fn set_disabled(
        &mut self,
        key: &str,
        disabled: bool,
    ) -> Result<(), CustomElementInternalsError> {
        let element = self.attached_mut(key)?;
        if element.disabled == disabled {
            return Ok(());
        }
        element.disabled = disabled;
        element.will_validate = !disabled;
        self.lifecycle_events
            .push(CustomElementLifecycleEvent::FormDisabled {
                key: key.to_string(),
                disabled,
            });
        Ok(())
    }

    pub(crate) fn set_accessibility_value(
        &mut self,
        key: &str,
        value: CustomElementAccessibilityValue,
    ) -> Result<(), CustomElementInternalsError> {
        let element = self.attached_mut(key)?;
        element.accessibility_value = sanitize_accessibility_value(value);
        Ok(())
    }

    pub(crate) fn set_accessibility_projection(
        &mut self,
        key: &str,
        projection: CustomElementAccessibilityProjection,
    ) -> Result<(), CustomElementInternalsError> {
        for value in [
            projection.role.as_deref(),
            projection.name.as_deref(),
            projection.description.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_text(value)?;
        }
        let element = self.attached_mut(key)?;
        if let Some(role) = projection.role {
            element.role = Some(role);
        }
        if let Some(name) = projection.name {
            element.accessible_name = Some(name);
        }
        if let Some(description) = projection.description {
            element.accessible_description = Some(description);
        }
        Ok(())
    }

    pub(crate) fn accessibility_action(
        &mut self,
        key: &str,
        action: CustomElementAccessibilityAction,
    ) -> Result<(), CustomElementInternalsError> {
        let element = self.attached_mut(key)?;
        if element.disabled {
            return Ok(());
        }
        match action {
            CustomElementAccessibilityAction::SetValue(value) => {
                validate_text(&value)?;
                element.value = Some(CustomElementFormValue::Text(value.clone()));
                element.accessibility_value.value_text = Some(value);
            }
            CustomElementAccessibilityAction::ClearValue => {
                element.value = None;
                element.accessibility_value.value_text = None;
            }
            CustomElementAccessibilityAction::Increment => step_accessibility_value(element, 1.0)?,
            CustomElementAccessibilityAction::Decrement => step_accessibility_value(element, -1.0)?,
        }
        Ok(())
    }

    pub(crate) fn accessibility_state(&self, key: &str) -> Option<CustomElementAccessibilityState> {
        let element = self.element(key)?;
        let value = primary_text_value(element.value.as_ref());
        Some(CustomElementAccessibilityState {
            key: element.key.clone(),
            role: element.role.clone(),
            name: element.accessible_name.clone(),
            description: element.accessible_description.clone(),
            labels: element.labels.clone(),
            disabled: element.disabled,
            invalid: !element.validity.is_valid(),
            validation_message: element.validation_message.clone(),
            validation_anchor: element.validation_anchor.clone(),
            value,
            value_text: element.accessibility_value.value_text.clone(),
            minimum: element.accessibility_value.minimum,
            maximum: element.accessibility_value.maximum,
            step: element.accessibility_value.step,
        })
    }

    pub(crate) fn diagnostics(
        &self,
        form_id: Option<&str>,
        form_index: usize,
    ) -> Vec<CustomElementDiagnostic> {
        self.elements
            .iter()
            .filter(|element| {
                element.attached
                    && element.will_validate
                    && associated_with(element, form_id, form_index)
                    && !element.validity.is_valid()
            })
            .map(|element| CustomElementDiagnostic {
                code: "custom-element-invalid",
                key: element.key.clone(),
                message: element
                    .validation_message
                    .clone()
                    .unwrap_or_else(|| "custom element is invalid".to_string()),
                anchor: element.validation_anchor.clone(),
            })
            .collect()
    }

    pub(crate) fn submission_groups(
        &self,
        form_id: Option<&str>,
        form_index: usize,
    ) -> Vec<CustomElementSubmissionGroup> {
        self.elements
            .iter()
            .filter(|element| {
                element.attached
                    && !element.disabled
                    && associated_with(element, form_id, form_index)
            })
            .filter_map(|element| {
                let entries = submission_entries(element);
                (!entries.is_empty()).then(|| CustomElementSubmissionGroup {
                    key: element.key.clone(),
                    document_order: element.document_order,
                    entries,
                })
            })
            .collect()
    }

    pub(crate) fn reset_form(&mut self, form_id: Option<&str>, form_index: usize) {
        for element in &self.elements {
            if element.attached && associated_with(element, form_id, form_index) {
                self.lifecycle_events
                    .push(CustomElementLifecycleEvent::FormReset {
                        key: element.key.clone(),
                    });
            }
        }
    }

    pub(crate) fn restore_state(
        &mut self,
        key: &str,
        mode: CustomElementStateRestoreMode,
    ) -> Result<(), CustomElementInternalsError> {
        let element = self.attached_mut(key)?;
        let Some(state) = element.restoration_state.clone() else {
            return Ok(());
        };
        self.lifecycle_events
            .push(CustomElementLifecycleEvent::FormStateRestore {
                key: key.to_string(),
                state,
                mode,
            });
        Ok(())
    }

    pub(crate) fn restoration_entries(&self) -> Vec<CustomElementRestorationEntry> {
        self.elements
            .iter()
            .filter(|element| element.attached)
            .filter_map(|element| {
                element
                    .restoration_state
                    .clone()
                    .map(|state| CustomElementRestorationEntry {
                        key: element.key.clone(),
                        state,
                    })
            })
            .collect()
    }

    pub(crate) fn restore_entries(
        &mut self,
        entries: &[CustomElementRestorationEntry],
        mode: CustomElementStateRestoreMode,
    ) {
        for entry in entries {
            let Some(element) = self
                .elements
                .iter_mut()
                .find(|element| element.key == entry.key)
            else {
                continue;
            };
            element.restoration_state = Some(entry.state.clone());
            if element.attached {
                self.lifecycle_events
                    .push(CustomElementLifecycleEvent::FormStateRestore {
                        key: entry.key.clone(),
                        state: entry.state.clone(),
                        mode,
                    });
            } else if let Some(pending) = self
                .pending_restores
                .iter_mut()
                .find(|(key, _)| key == &entry.key)
            {
                pending.1 = mode;
            } else {
                self.pending_restores.push((entry.key.clone(), mode));
            }
        }
    }

    pub(crate) fn restore_all(&mut self, mode: CustomElementStateRestoreMode) {
        let entries = self.restoration_entries();
        self.restore_entries(&entries, mode);
    }

    pub(crate) fn take_lifecycle_events(&mut self) -> Vec<CustomElementLifecycleEvent> {
        std::mem::take(&mut self.lifecycle_events)
    }

    fn element_mut(
        &mut self,
        key: &str,
    ) -> Result<&mut FormAssociatedCustomElementState, CustomElementInternalsError> {
        self.elements
            .iter_mut()
            .find(|element| element.key == key)
            .ok_or_else(|| CustomElementInternalsError::UnknownElement(key.to_string()))
    }

    fn attached_mut(
        &mut self,
        key: &str,
    ) -> Result<&mut FormAssociatedCustomElementState, CustomElementInternalsError> {
        let element = self.element_mut(key)?;
        if !element.attached {
            return Err(CustomElementInternalsError::NotAttached(key.to_string()));
        }
        Ok(element)
    }
}

fn collect_candidates(
    nodes: &[BrowserRenderNode],
    elements: &mut Vec<FormAssociatedCustomElementState>,
    next_form_index: &mut usize,
    next_custom_index: &mut usize,
    document_order: &mut usize,
    containing_form: Option<usize>,
    inherited_disabled: bool,
) {
    for node in nodes {
        let order = *document_order;
        *document_order += 1;
        let containing_form = if node.name.as_deref() == Some("form") {
            let index = *next_form_index;
            *next_form_index += 1;
            Some(index)
        } else {
            containing_form
        };
        let disabled =
            inherited_disabled || node.disabled || node.aria_disabled.as_deref() == Some("true");
        if node.custom_element_name.is_some() {
            let definition_name = node
                .custom_element_is
                .clone()
                .or_else(|| node.custom_element_name.clone())
                .unwrap_or_else(|| node.name.clone().unwrap_or_default());
            let key = custom_element_key(node, *next_custom_index);
            *next_custom_index += 1;
            elements.push(FormAssociatedCustomElementState {
                key,
                element_id: node.id.clone(),
                definition_name,
                name: node.control_name.clone(),
                association: CustomElementFormAssociation {
                    form_owner: node.form_owner.clone(),
                    form_index: containing_form,
                },
                labels: node.labels.clone(),
                accessible_name: node.accessible_name.clone(),
                accessible_description: node.accessible_description.clone(),
                role: node.authored_role.clone(),
                attached: false,
                disabled,
                will_validate: false,
                validity: CustomElementValidity::default(),
                validation_message: None,
                validation_anchor: None,
                value: None,
                restoration_state: None,
                accessibility_value: CustomElementAccessibilityValue::default(),
                document_order: order,
            });
        }
        let disabled_fieldset = node.name.as_deref() == Some("fieldset") && node.disabled;
        let first_legend = disabled_fieldset
            .then(|| {
                node.children
                    .iter()
                    .position(|child| child.name.as_deref() == Some("legend"))
            })
            .flatten();
        for (index, child) in node.children.iter().enumerate() {
            collect_candidates(
                std::slice::from_ref(child),
                elements,
                next_form_index,
                next_custom_index,
                document_order,
                containing_form,
                inherited_disabled || (disabled_fieldset && first_legend != Some(index)),
            );
        }
    }
}

fn custom_element_key(node: &BrowserRenderNode, index: usize) -> String {
    node.id
        .as_ref()
        .map(|id| format!("custom:{index}:id:{id}"))
        .unwrap_or_else(|| format!("custom:{index}"))
}

fn validate_value(
    value: Option<&CustomElementFormValue>,
) -> Result<(), CustomElementInternalsError> {
    match value {
        Some(CustomElementFormValue::Text(value)) => validate_text(value),
        Some(CustomElementFormValue::File(file)) => validate_file(file),
        None => Ok(()),
        Some(CustomElementFormValue::Entries(entries)) => {
            if entries.len() > MAX_CUSTOM_ELEMENT_ENTRIES {
                return Err(CustomElementInternalsError::TooManyEntries {
                    limit: MAX_CUSTOM_ELEMENT_ENTRIES,
                });
            }
            let mut payload_bytes = 0_usize;
            for entry in entries {
                validate_text(&entry.name)?;
                payload_bytes = payload_bytes.saturating_add(entry.name.len());
                payload_bytes = payload_bytes.saturating_add(match &entry.value {
                    CustomElementFormEntryValue::Text(value) => {
                        validate_text(value)?;
                        value.len()
                    }
                    CustomElementFormEntryValue::File(file) => {
                        validate_file(file)?;
                        file.size()
                    }
                });
            }
            if payload_bytes > MAX_SELECTED_TOTAL_BYTES {
                return Err(CustomElementInternalsError::ValuePayloadTooLarge {
                    limit: MAX_SELECTED_TOTAL_BYTES,
                });
            }
            Ok(())
        }
    }
}

fn validate_text(value: &str) -> Result<(), CustomElementInternalsError> {
    if value.len() > MAX_CUSTOM_ELEMENT_TEXT_BYTES {
        Err(CustomElementInternalsError::TextValueTooLarge {
            limit: MAX_CUSTOM_ELEMENT_TEXT_BYTES,
        })
    } else {
        Ok(())
    }
}

fn validate_file(file: &HostFileSelection) -> Result<(), CustomElementInternalsError> {
    if file.size() > MAX_SELECTED_FILE_BYTES {
        Err(CustomElementInternalsError::FileValueTooLarge {
            limit: MAX_SELECTED_FILE_BYTES,
        })
    } else {
        Ok(())
    }
}

fn sanitize_accessibility_value(
    mut value: CustomElementAccessibilityValue,
) -> CustomElementAccessibilityValue {
    value.minimum = value.minimum.filter(|number| number.is_finite());
    value.maximum = value.maximum.filter(|number| number.is_finite());
    value.step = value
        .step
        .filter(|number| number.is_finite() && *number > 0.0);
    value
}

fn step_accessibility_value(
    element: &mut FormAssociatedCustomElementState,
    direction: f64,
) -> Result<(), CustomElementInternalsError> {
    let Some(current) = primary_text_value(element.value.as_ref())
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
    else {
        return Err(CustomElementInternalsError::UnsupportedAccessibilityAction(
            element.key.clone(),
        ));
    };
    let step = element.accessibility_value.step.unwrap_or(1.0);
    let mut value = current + step * direction;
    if let Some(minimum) = element.accessibility_value.minimum {
        value = value.max(minimum);
    }
    if let Some(maximum) = element.accessibility_value.maximum {
        value = value.min(maximum);
    }
    let value = format_number(value);
    element.value = Some(CustomElementFormValue::Text(value.clone()));
    element.accessibility_value.value_text = Some(value);
    Ok(())
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn primary_text_value(value: Option<&CustomElementFormValue>) -> Option<String> {
    match value {
        Some(CustomElementFormValue::Text(value)) => Some(value.clone()),
        Some(CustomElementFormValue::File(file)) => Some(file.name.clone()),
        Some(CustomElementFormValue::Entries(entries)) => {
            entries.first().map(|entry| match &entry.value {
                CustomElementFormEntryValue::Text(value) => value.clone(),
                CustomElementFormEntryValue::File(file) => file.name.clone(),
            })
        }
        None => None,
    }
}

fn submission_entries(element: &FormAssociatedCustomElementState) -> Vec<CustomElementFormEntry> {
    match element.value.as_ref() {
        Some(CustomElementFormValue::Text(value)) => element
            .name
            .as_ref()
            .filter(|name| !name.is_empty())
            .map(|name| {
                vec![CustomElementFormEntry {
                    name: name.clone(),
                    value: CustomElementFormEntryValue::Text(value.clone()),
                }]
            })
            .unwrap_or_default(),
        Some(CustomElementFormValue::File(file)) => element
            .name
            .as_ref()
            .filter(|name| !name.is_empty())
            .map(|name| {
                vec![CustomElementFormEntry {
                    name: name.clone(),
                    value: CustomElementFormEntryValue::File(file.clone()),
                }]
            })
            .unwrap_or_default(),
        Some(CustomElementFormValue::Entries(entries)) => entries.clone(),
        None => Vec::new(),
    }
}

fn associated_with(
    element: &FormAssociatedCustomElementState,
    form_id: Option<&str>,
    form_index: usize,
) -> bool {
    match element.association.form_owner.as_deref() {
        Some(owner) => form_id == Some(owner),
        None => element.association.form_index == Some(form_index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::{parse_html, BrowserRenderTree};

    fn registry(source: &str) -> CustomElementInternalsRegistry {
        let document = parse_html(source).unwrap();
        let tree = BrowserRenderTree::from_document(&document);
        CustomElementInternalsRegistry::from_render_tree(&tree)
    }

    #[test]
    fn projects_candidates_and_element_internals_lifecycle() {
        let mut registry = registry(
            "<label for='rating'>Rating</label><form id='review'>\
             <x-rating id='rating' name='score' role='slider' aria-describedby='hint'></x-rating>\
             <span id='hint'>One to five</span></form>",
        );
        let key = registry.elements()[0].key.clone();
        let candidate = registry.element(&key).unwrap();
        assert_eq!(candidate.definition_name, "x-rating");
        assert_eq!(candidate.name.as_deref(), Some("score"));
        assert_eq!(candidate.labels, vec!["Rating"]);
        assert_eq!(candidate.accessible_name.as_deref(), Some("Rating"));
        assert_eq!(
            candidate.accessible_description.as_deref(),
            Some("One to five")
        );
        assert_eq!(candidate.association.form_index, Some(0));

        registry.attach(&key).unwrap();
        registry
            .set_accessibility_projection(
                &key,
                CustomElementAccessibilityProjection {
                    role: Some("spinbutton".into()),
                    name: Some("Review rating".into()),
                    description: None,
                },
            )
            .unwrap();
        registry
            .set_form_value(
                &key,
                Some(CustomElementFormValue::Text("3".into())),
                Some(CustomElementFormValue::Text("restored:3".into())),
            )
            .unwrap();
        registry
            .set_accessibility_value(
                &key,
                CustomElementAccessibilityValue {
                    value_text: Some("3 of 5".into()),
                    minimum: Some(1.0),
                    maximum: Some(5.0),
                    step: Some(1.0),
                },
            )
            .unwrap();
        registry
            .accessibility_action(&key, CustomElementAccessibilityAction::Increment)
            .unwrap();
        let state = registry.accessibility_state(&key).unwrap();
        assert_eq!(state.role.as_deref(), Some("spinbutton"));
        assert_eq!(state.name.as_deref(), Some("Review rating"));
        assert_eq!(state.value.as_deref(), Some("4"));
        assert_eq!(state.minimum, Some(1.0));

        registry
            .set_validity(
                &key,
                CustomElementValidity {
                    custom_error: true,
                    ..CustomElementValidity::default()
                },
                Some("Choose five stars".into()),
                Some("rating-help".into()),
            )
            .unwrap();
        assert_eq!(registry.diagnostics(Some("review"), 0).len(), 1);
        registry.reset_form(Some("review"), 0);
        registry
            .restore_state(&key, CustomElementStateRestoreMode::Restore)
            .unwrap();
        let events = registry.take_lifecycle_events();
        assert!(matches!(
            events[0],
            CustomElementLifecycleEvent::FormAssociated { .. }
        ));
        assert!(events.iter().any(|event| matches!(
            event,
            CustomElementLifecycleEvent::FormReset { key: event_key } if event_key == &key
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            CustomElementLifecycleEvent::FormStateRestore {
                state: CustomElementFormValue::Text(state),
                ..
            } if state == "restored:3"
        )));
    }

    #[test]
    fn reassociation_disabled_state_and_bounded_values_are_shared() {
        let mut registry = registry(
            "<form id='one'></form><form id='two'></form>\
             <x-token id='token' name='token' form='one'></x-token>",
        );
        let key = registry.elements()[0].key.clone();
        registry.attach(&key).unwrap();
        registry
            .reassociate(
                &key,
                CustomElementFormAssociation {
                    form_owner: Some("two".into()),
                    form_index: None,
                },
            )
            .unwrap();
        registry.set_disabled(&key, true).unwrap();
        registry
            .set_form_value(
                &key,
                Some(CustomElementFormValue::Text("secret".into())),
                None,
            )
            .unwrap();
        assert!(registry.submission_groups(Some("one"), 0).is_empty());
        assert!(registry.submission_groups(Some("two"), 1).is_empty());

        registry.set_disabled(&key, false).unwrap();
        assert_eq!(
            registry.submission_groups(Some("two"), 1)[0].entries[0].name,
            "token"
        );
        let oversized = "x".repeat(MAX_CUSTOM_ELEMENT_TEXT_BYTES + 1);
        assert!(matches!(
            registry.set_form_value(&key, Some(CustomElementFormValue::Text(oversized)), None,),
            Err(CustomElementInternalsError::TextValueTooLarge { .. })
        ));
    }
}
