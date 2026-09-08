//! Host-neutral browser form-control interaction semantics.

use coding_adventures_html_parser::{BrowserRenderNode, BrowserRenderTree};
use layout_controls::{ControlAppearance, ControlKind, ControlState};

pub const VERSION: &str = "0.1.0";

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
    Space,
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
            "space" => Self::Space,
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
            Self::Space => "space",
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
    SelectionChanged {
        key: String,
        selection: ControlSelection,
    },
    CompositionChanged {
        key: String,
        composition: Option<String>,
    },
}

/// Character-indexed selection that never splits a UTF-8 scalar value.
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlEditorState {
    pub key: String,
    pub selection: ControlSelection,
    pub composition: Option<String>,
    pub invalid_message: Option<String>,
    pub accessible_description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrowserControlModel {
    controls: Vec<ControlState>,
    initial_controls: Vec<ControlState>,
    bindings: Vec<ControlBinding>,
    editors: Vec<ControlEditorState>,
    focused_key: Option<String>,
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
}

impl BrowserControlModel {
    pub fn from_render_tree(tree: &BrowserRenderTree) -> Self {
        let mut controls = Vec::new();
        let mut bindings = Vec::new();
        let mut control_index = 0;
        let mut form_index = 0;
        collect_model_nodes(
            &tree.children,
            &mut controls,
            &mut bindings,
            &mut control_index,
            &mut form_index,
            None,
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
                invalid_message: None,
                accessible_description,
            })
            .collect();
        Self {
            initial_controls: controls.clone(),
            controls,
            bindings,
            editors,
            focused_key,
        }
    }

    pub fn controls(&self) -> &[ControlState] {
        &self.controls
    }

    pub fn bindings(&self) -> &[ControlBinding] {
        &self.bindings
    }

    pub fn binding(&self, key: &str) -> Option<&ControlBinding> {
        self.bindings.iter().find(|binding| binding.key == key)
    }

    pub fn editor(&self, key: &str) -> Option<&ControlEditorState> {
        self.editors.iter().find(|editor| editor.key == key)
    }

    pub fn control(&self, key: &str) -> Option<&ControlState> {
        self.controls.iter().find(|control| control.key == key)
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
        for (((control, initial), binding), editor) in self
            .controls
            .iter_mut()
            .zip(&self.initial_controls)
            .zip(&self.bindings)
            .zip(&mut self.editors)
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
                || control.selected_index != initial.selected_index;
            *control = initial.clone();
            control.focused = focused;
            editor.selection = ControlSelection::collapsed(control.value.chars().count());
            editor.composition = None;
            editor.invalid_message = None;
            if changed {
                effects.push(ControlEffect::ValueChanged {
                    key: control.key.clone(),
                    value: control.value.clone(),
                });
            }
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
        self.controls[target].focused = true;
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
        let kind = self.focused()?.kind;
        match (key, kind) {
            (ControlKey::Space, kind) if kind.accepts_text() => return self.text_input(" "),
            (ControlKey::Enter, ControlKind::TextArea) => return self.text_input("\n"),
            (ControlKey::Enter | ControlKey::Space, _) => return self.activate_focused(),
            _ => {}
        }
        if kind.accepts_text() {
            match key {
                ControlKey::ArrowLeft => return self.move_caret(-1, shift, false),
                ControlKey::ArrowRight => return self.move_caret(1, shift, false),
                ControlKey::Home => return self.move_caret(0, shift, true),
                ControlKey::End => return self.move_caret(0, shift, false),
                ControlKey::Backspace => return self.delete_from_focused(true),
                ControlKey::Delete => return self.delete_from_focused(false),
                _ => {}
            }
        }
        let control = self.focused_mut()?;
        if control.disabled {
            return None;
        }
        match key {
            ControlKey::ArrowUp if control.kind == ControlKind::Select => {
                control.selected_index = control.selected_index.saturating_sub(1);
                sync_selected_value(control);
            }
            ControlKey::ArrowDown if control.kind == ControlKind::Select => {
                if !control.options.is_empty() {
                    control.selected_index =
                        (control.selected_index + 1).min(control.options.len() - 1);
                    sync_selected_value(control);
                }
            }
            ControlKey::Home if control.kind == ControlKind::Select => {
                control.selected_index = 0;
                sync_selected_value(control);
            }
            ControlKey::End if control.kind == ControlKind::Select => {
                control.selected_index = control.options.len().saturating_sub(1);
                sync_selected_value(control);
            }
            _ => return None,
        }
        Some(ControlEffect::ValueChanged {
            key: control.key.clone(),
            value: control.value.clone(),
        })
    }

    pub fn sync_render_tree(&self, tree: &mut BrowserRenderTree) {
        let mut index = 0;
        sync_nodes(
            &mut tree.children,
            &self.controls,
            &self.editors,
            &mut index,
        );
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
        if !self.controls[index].kind.accepts_text() {
            return None;
        }
        let length = self.controls[index].value.chars().count();
        let selection = ControlSelection {
            anchor: anchor.min(length),
            focus: focus.min(length),
        };
        self.editors[index].selection = selection;
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

    fn focused_mut(&mut self) -> Option<&mut ControlState> {
        let key = self.focused_key.as_deref()?;
        self.controls.iter_mut().find(|control| control.key == key)
    }

    fn focused_index(&self) -> Option<usize> {
        let key = self.focused_key.as_deref()?;
        self.controls.iter().position(|control| control.key == key)
    }

    fn replace_focused_selection(&mut self, text: &str) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &mut self.controls[index];
        if control.disabled || control.readonly || !control.kind.accepts_text() {
            return None;
        }
        let selection = self.editors[index].selection;
        let (start, end) = selection.ordered();
        replace_char_range(&mut control.value, start, end, text);
        let caret = start + text.chars().count();
        self.editors[index].selection = ControlSelection::collapsed(caret);
        self.editors[index].composition = None;
        self.editors[index].invalid_message = None;
        Some(ControlEffect::ValueChanged {
            key: control.key.clone(),
            value: control.value.clone(),
        })
    }

    fn delete_from_focused(&mut self, backward: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if control.disabled || control.readonly || !control.kind.accepts_text() {
            return None;
        }
        let length = control.value.chars().count();
        let (mut start, mut end) = self.editors[index].selection.ordered();
        if start == end {
            if backward {
                if start == 0 {
                    return None;
                }
                start -= 1;
            } else {
                if end >= length {
                    return None;
                }
                end += 1;
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
        let length = control.value.chars().count();
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
            current.focus.saturating_add_signed(delta).min(length)
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
        Some(ControlEffect::SelectionChanged {
            key: control.key.clone(),
            selection,
        })
    }

    fn activate_focused(&mut self) -> Option<ControlEffect> {
        let index = self.controls.iter().position(|control| {
            self.focused_key.as_deref() == Some(control.key.as_str()) && !control.disabled
        })?;
        match self.controls[index].kind {
            ControlKind::Checkbox => {
                self.controls[index].checked = !self.controls[index].checked;
                Some(ControlEffect::CheckedChanged {
                    key: self.controls[index].key.clone(),
                    checked: self.controls[index].checked,
                })
            }
            ControlKind::Radio => {
                let name = self.controls[index].name.clone();
                let owner = self.bindings[index].form_owner.clone();
                let form_index = self.bindings[index].form_index;
                for (control, binding) in self.controls.iter_mut().zip(&self.bindings) {
                    let same_form = match owner.as_deref() {
                        Some(owner) => binding.form_owner.as_deref() == Some(owner),
                        None => binding.form_owner.is_none() && binding.form_index == form_index,
                    };
                    if same_form && control.kind == ControlKind::Radio && control.name == name {
                        control.checked = false;
                    }
                }
                self.controls[index].checked = true;
                Some(ControlEffect::CheckedChanged {
                    key: self.controls[index].key.clone(),
                    checked: true,
                })
            }
            ControlKind::Button => Some(ControlEffect::Activated(self.controls[index].key.clone())),
            ControlKind::Select => {
                if self.controls[index].options.is_empty() {
                    return None;
                }
                self.controls[index].selected_index =
                    (self.controls[index].selected_index + 1) % self.controls[index].options.len();
                sync_selected_value(&mut self.controls[index]);
                Some(ControlEffect::ValueChanged {
                    key: self.controls[index].key.clone(),
                    value: self.controls[index].value.clone(),
                })
            }
            _ => None,
        }
    }
}

pub fn control_states(tree: &BrowserRenderTree) -> Vec<ControlState> {
    let mut states = Vec::new();
    let mut index = 0;
    collect_nodes(&tree.children, &mut states, &mut index);
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
        _ => node.value.clone().unwrap_or_default(),
    };
    state.placeholder = node.placeholder.clone();
    state.options = node.options.clone();
    state.selected_index = node
        .value
        .as_ref()
        .and_then(|value| state.options.iter().position(|option| option == value))
        .unwrap_or(0);
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
    state.multiple = node.multiple;
    state.checked = node.checked;
    state.focused = node.control_focused;
    state.appearance = ControlAppearance::Auto;
    state
}

fn collect_nodes(nodes: &[BrowserRenderNode], states: &mut Vec<ControlState>, index: &mut usize) {
    for node in nodes {
        if node.role == "control" && node.control_type.as_deref() != Some("hidden") && !node.hidden
        {
            let key = control_key(node, *index);
            *index += 1;
            states.push(project_control(node, key));
        }
        collect_nodes(&node.children, states, index);
    }
}

fn collect_model_nodes(
    nodes: &[BrowserRenderNode],
    states: &mut Vec<ControlState>,
    bindings: &mut Vec<ControlBinding>,
    control_index: &mut usize,
    next_form_index: &mut usize,
    containing_form: Option<usize>,
) {
    for node in nodes {
        let containing_form = if node.name.as_deref() == Some("form") {
            let index = *next_form_index;
            *next_form_index += 1;
            Some(index)
        } else {
            containing_form
        };
        if node.role == "control" && node.control_type.as_deref() != Some("hidden") && !node.hidden
        {
            let key = control_key(node, *control_index);
            *control_index += 1;
            states.push(project_control(node, key.clone()));
            bindings.push(ControlBinding {
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
            });
        }
        collect_model_nodes(
            &node.children,
            states,
            bindings,
            control_index,
            next_form_index,
            containing_form,
        );
    }
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

fn positive_count(value: &str) -> Option<usize> {
    value.parse().ok().filter(|value| *value > 0)
}

fn sync_selected_value(control: &mut ControlState) {
    if let Some(value) = control.options.get(control.selected_index) {
        control.value = value.clone();
    }
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
}
