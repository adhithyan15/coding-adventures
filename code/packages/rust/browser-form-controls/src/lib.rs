//! Host-neutral browser form-control interaction semantics.

use coding_adventures_html_parser::{BrowserRenderNode, BrowserRenderTree};
use layout_controls::{ControlAppearance, ControlKind, ControlState};
use text_flow::graphemes;
use url_parser::Url;

const EDIT_HISTORY_LIMIT: usize = 100;

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
    SelectAll,
    Undo,
    Redo,
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
        Self {
            initial_controls: controls.clone(),
            controls,
            bindings,
            editors,
            histories,
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
                || control.selected_index != initial.selected_index;
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
        match (key, kind) {
            (ControlKey::Space, kind) if kind.accepts_text() => return self.text_input(" "),
            (ControlKey::Enter, ControlKind::TextArea) => return self.text_input("\n"),
            (ControlKey::Enter | ControlKey::Space, _) => return self.activate_focused(),
            _ => {}
        }
        if kind.accepts_text() {
            match key {
                ControlKey::ArrowUp if kind == ControlKind::Number => {
                    return self.step_focused_number(true)
                }
                ControlKey::ArrowDown if kind == ControlKind::Number => {
                    return self.step_focused_number(false)
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
            ControlAccessibilityAction::Increment => self.step_focused_number(true),
            ControlAccessibilityAction::Decrement => self.step_focused_number(false),
            ControlAccessibilityAction::SelectAll => self.key_down(ControlKey::SelectAll),
            ControlAccessibilityAction::Undo => self.undo(),
            ControlAccessibilityAction::Redo => self.redo(),
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
        if control.disabled || control.readonly || !control.kind.accepts_text() {
            return None;
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

    fn step_focused_number(&mut self, forward: bool) -> Option<ControlEffect> {
        let index = self.focused_index()?;
        let control = &self.controls[index];
        if control.disabled || control.readonly || control.kind != ControlKind::Number {
            return None;
        }
        let constraints = numeric_constraints(&self.bindings[index]);
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
                step: node.step.clone(),
                inputmode: node.inputmode.clone(),
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

    let constraints = numeric_constraints(binding);
    let numeric_value = if control.kind == ControlKind::Number && !control.value.is_empty() {
        match parse_finite(&control.value) {
            Some(value) => {
                if constraints.minimum.is_some_and(|minimum| value < minimum) {
                    diagnostics.push(ControlValueDiagnostic {
                        code: "range-underflow",
                        message: "number is below min",
                    });
                }
                if constraints.maximum.is_some_and(|maximum| value > maximum) {
                    diagnostics.push(ControlValueDiagnostic {
                        code: "range-overflow",
                        message: "number is above max",
                    });
                }
                if constraints.validates_step
                    && step_mismatch(value, constraints.minimum.unwrap_or(0.0), constraints.step)
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

    ControlValueState {
        key: control.key.clone(),
        value: control.value.clone(),
        input_mode: binding.inputmode.clone(),
        selection_supported: control.kind.supports_selection(),
        numeric_value,
        minimum: constraints.minimum,
        maximum: constraints.maximum,
        step: constraints.validates_step.then_some(constraints.step),
        diagnostics,
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
}
