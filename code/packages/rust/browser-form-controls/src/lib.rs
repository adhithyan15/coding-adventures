//! Host-neutral browser form-control interaction semantics.

use coding_adventures_html_parser::{BrowserRenderNode, BrowserRenderTree};
use layout_controls::{ControlAppearance, ControlKind, ControlState};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKey {
    Backspace,
    Delete,
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
    ValueChanged { key: String, value: String },
    CheckedChanged { key: String, checked: bool },
    Activated(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrowserControlModel {
    controls: Vec<ControlState>,
    initial_controls: Vec<ControlState>,
    bindings: Vec<ControlBinding>,
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
        Self {
            initial_controls: controls.clone(),
            controls,
            bindings,
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
        for ((control, initial), binding) in self
            .controls
            .iter_mut()
            .zip(&self.initial_controls)
            .zip(&self.bindings)
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
        let control = self.focused_mut()?;
        if control.disabled || control.readonly || !control.kind.accepts_text() || text.is_empty() {
            return None;
        }
        control.value.push_str(text);
        Some(ControlEffect::ValueChanged {
            key: control.key.clone(),
            value: control.value.clone(),
        })
    }

    pub fn key_down(&mut self, key: ControlKey) -> Option<ControlEffect> {
        let kind = self.focused()?.kind;
        match (key, kind) {
            (ControlKey::Space, kind) if kind.accepts_text() => return self.text_input(" "),
            (ControlKey::Enter, ControlKind::TextArea) => return self.text_input("\n"),
            (ControlKey::Enter | ControlKey::Space, _) => return self.activate_focused(),
            _ => {}
        }
        let control = self.focused_mut()?;
        if control.disabled {
            return None;
        }
        match key {
            ControlKey::Backspace if control.kind.accepts_text() && !control.readonly => {
                control.value.pop();
            }
            ControlKey::Delete if control.kind.accepts_text() && !control.readonly => {
                control.value.clear();
            }
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
        sync_nodes(&mut tree.children, &self.controls, &mut index);
    }

    fn focused_mut(&mut self) -> Option<&mut ControlState> {
        let key = self.focused_key.as_deref()?;
        self.controls.iter_mut().find(|control| control.key == key)
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

fn sync_nodes(nodes: &mut [BrowserRenderNode], controls: &[ControlState], index: &mut usize) {
    for node in nodes {
        if node.role == "control" && node.control_type.as_deref() != Some("hidden") && !node.hidden
        {
            if let Some(control) = controls.get(*index) {
                node.value = Some(control.value.clone());
                node.checked = control.checked;
                node.control_focused = control.focused;
            }
            *index += 1;
        }
        sync_nodes(&mut node.children, controls, index);
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
}
