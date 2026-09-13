use browser_form_controls::{
    text_editor_presentation, ControlAccessibilityAction, ControlClipboardPayload,
    ControlEditorPresentation, ControlEditorState, ControlEffect, ControlKey,
    ControlNavigationUnit, ControlRect, ControlSelection, ControlTextMetrics,
};
use coding_adventures_html_parser::{BrowserRenderNode, BrowserRenderTree};
use html_to_paint::FocusRegion;
use text_flow::graphemes;

const MAX_EDITING_HOSTS: usize = 512;
const MAX_EDITABLE_BYTES: usize = 256 * 1024;
const MAX_TOTAL_BYTES: usize = 1024 * 1024;
const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentEditableMode {
    Plaintext,
    RichText,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentEditableEffect {
    Focused(String),
    ValueChanged {
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

impl From<ContentEditableEffect> for ControlEffect {
    fn from(effect: ContentEditableEffect) -> Self {
        match effect {
            ContentEditableEffect::Focused(key) => Self::Focused(key),
            ContentEditableEffect::ValueChanged { key, value } => Self::ValueChanged { key, value },
            ContentEditableEffect::SelectionChanged { key, selection } => {
                Self::SelectionChanged { key, selection }
            }
            ContentEditableEffect::CompositionChanged { key, composition } => {
                Self::CompositionChanged { key, composition }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentEditableDiagnostic {
    pub code: &'static str,
    pub key: Option<String>,
    pub message: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentEditableAccessibilityState {
    pub key: String,
    pub mode: ContentEditableMode,
    pub value: String,
    pub selection: ControlSelection,
    pub focused: bool,
    pub composing: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContentEditableSnapshot {
    pub entries: Vec<ContentEditableSnapshotEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentEditableSnapshotEntry {
    pub key: String,
    pub value: String,
    pub selection: ControlSelection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EditSnapshot {
    value: String,
    selection: ControlSelection,
}

#[derive(Clone, Debug, PartialEq)]
struct EditingHost {
    key: String,
    mode: ContentEditableMode,
    value: String,
    editor: ControlEditorState,
    undo: Vec<EditSnapshot>,
    redo: Vec<EditSnapshot>,
    dirty: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContentEditableModel {
    hosts: Vec<EditingHost>,
    focused_key: Option<String>,
    diagnostics: Vec<ContentEditableDiagnostic>,
}

impl ContentEditableModel {
    pub fn from_page(tree: &BrowserRenderTree, regions: &[FocusRegion]) -> Self {
        let mut descriptors = Vec::new();
        collect_editing_hosts(&tree.children, false, &mut descriptors);
        let region_keys = regions
            .iter()
            .filter(|region| region.editing_host)
            .map(|region| region.key.clone())
            .collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        if descriptors.len() > MAX_EDITING_HOSTS {
            diagnostics.push(ContentEditableDiagnostic {
                code: "contenteditable-host-limit",
                key: None,
                message: "contenteditable hosts beyond the shared bound were ignored",
            });
        }
        let mut total = 0usize;
        let hosts = descriptors
            .into_iter()
            .zip(region_keys)
            .take(MAX_EDITING_HOSTS)
            .filter_map(|((mode, mut value), key)| {
                if total >= MAX_TOTAL_BYTES {
                    diagnostics.push(ContentEditableDiagnostic {
                        code: "contenteditable-document-byte-limit",
                        key: Some(key),
                        message: "contenteditable value was omitted by the document byte bound",
                    });
                    return None;
                }
                let budget = MAX_EDITABLE_BYTES.min(MAX_TOTAL_BYTES - total);
                if value.len() > budget {
                    truncate_utf8(&mut value, budget);
                    diagnostics.push(ContentEditableDiagnostic {
                        code: "contenteditable-value-truncated",
                        key: Some(key.clone()),
                        message: "contenteditable value was truncated to the shared byte bound",
                    });
                }
                total += value.len();
                let caret = value.chars().count();
                Some(EditingHost {
                    editor: ControlEditorState {
                        key: key.clone(),
                        selection: ControlSelection::collapsed(caret),
                        ..ControlEditorState::default()
                    },
                    key,
                    mode,
                    value,
                    undo: Vec::new(),
                    redo: Vec::new(),
                    dirty: false,
                })
            })
            .collect();
        Self {
            hosts,
            focused_key: None,
            diagnostics,
        }
    }

    pub fn diagnostics(&self) -> &[ContentEditableDiagnostic] {
        &self.diagnostics
    }

    pub fn contains(&self, key: &str) -> bool {
        self.hosts.iter().any(|host| host.key == key)
    }

    pub fn focused_key(&self) -> Option<&str> {
        self.focused_key.as_deref()
    }

    pub fn focus(&mut self, key: &str) -> Option<ContentEditableEffect> {
        self.contains(key).then(|| {
            self.focused_key = Some(key.to_string());
            if let Some(host) = self.host_mut(key) {
                host.editor.caret_phase_ms = 0;
            }
            ContentEditableEffect::Focused(key.to_string())
        })
    }

    pub fn blur(&mut self) {
        self.focused_key = None;
        for host in &mut self.hosts {
            host.editor.composition = None;
            host.editor.composition_range = None;
            host.editor.pointer_anchor = None;
        }
    }

    pub fn accessibility_states(&self) -> Vec<ContentEditableAccessibilityState> {
        self.hosts
            .iter()
            .map(|host| ContentEditableAccessibilityState {
                key: host.key.clone(),
                mode: host.mode,
                value: host.value.clone(),
                selection: host.editor.selection,
                focused: self.focused_key.as_deref() == Some(host.key.as_str()),
                composing: host.editor.composition.is_some(),
                can_undo: !host.undo.is_empty(),
                can_redo: !host.redo.is_empty(),
            })
            .collect()
    }

    pub fn capture_state(&self) -> ContentEditableSnapshot {
        ContentEditableSnapshot {
            entries: self
                .hosts
                .iter()
                .map(|host| ContentEditableSnapshotEntry {
                    key: host.key.clone(),
                    value: host.value.clone(),
                    selection: host.editor.selection,
                })
                .collect(),
        }
    }

    pub fn restore_state(&mut self, snapshot: &ContentEditableSnapshot) {
        for entry in snapshot.entries.iter().take(MAX_EDITING_HOSTS) {
            let Some(host) = self.host_mut(&entry.key) else {
                continue;
            };
            host.value = bounded_text(&entry.value);
            let length = host.value.chars().count();
            host.editor.selection = ControlSelection {
                anchor: entry.selection.anchor.min(length),
                focus: entry.selection.focus.min(length),
            };
            host.editor.composition = None;
            host.editor.composition_range = None;
            host.undo.clear();
            host.redo.clear();
            host.dirty = true;
        }
    }

    pub fn text_input(&mut self, text: &str) -> Option<ContentEditableEffect> {
        self.replace_selection(text)
    }

    pub fn key_down(&mut self, key: ControlKey, shift: bool) -> Option<ContentEditableEffect> {
        match key {
            ControlKey::Undo => self.undo(),
            ControlKey::Redo => self.redo(),
            ControlKey::SelectAll => {
                let focused = self.focused_key.clone()?;
                let length = self.host(&focused)?.value.chars().count();
                self.set_selection(&focused, 0, length)
            }
            ControlKey::Backspace => self.delete(true),
            ControlKey::Delete => self.delete(false),
            ControlKey::ArrowLeft => self.move_caret(false, shift, false),
            ControlKey::ArrowRight => self.move_caret(true, shift, false),
            ControlKey::WordLeft => self.move_caret(false, shift, true),
            ControlKey::WordRight => self.move_caret(true, shift, true),
            ControlKey::Home => self.move_line_boundary(false, shift),
            ControlKey::End => self.move_line_boundary(true, shift),
            ControlKey::Enter => self.replace_selection("\n"),
            ControlKey::Space => self.replace_selection(" "),
            _ => None,
        }
    }

    pub fn accessibility_action(
        &mut self,
        action: ControlAccessibilityAction,
    ) -> Option<ContentEditableEffect> {
        match action {
            ControlAccessibilityAction::Move {
                forward,
                unit,
                extend,
            } => match unit {
                ControlNavigationUnit::Grapheme => self.move_caret(forward, extend, false),
                ControlNavigationUnit::Word => self.move_caret(forward, extend, true),
                ControlNavigationUnit::Line => self.move_line_boundary(forward, extend),
                ControlNavigationUnit::Document => {
                    let key = self.focused_key.clone()?;
                    let destination = if forward {
                        self.host(&key)?.value.chars().count()
                    } else {
                        0
                    };
                    let anchor = if extend {
                        self.host(&key)?.editor.selection.anchor
                    } else {
                        destination
                    };
                    self.set_selection(&key, anchor, destination)
                }
            },
            ControlAccessibilityAction::SetSelection(selection) => {
                let key = self.focused_key.clone()?;
                self.set_selection(&key, selection.anchor, selection.focus)
            }
            ControlAccessibilityAction::ReplaceSelection(value) => self.replace_selection(&value),
            ControlAccessibilityAction::SetValue(value) => {
                let key = self.focused_key.clone()?;
                let length = self.host(&key)?.value.chars().count();
                self.set_selection(&key, 0, length)?;
                self.replace_selection(&value)
            }
            ControlAccessibilityAction::SelectAll => {
                let key = self.focused_key.clone()?;
                let length = self.host(&key)?.value.chars().count();
                self.set_selection(&key, 0, length)
            }
            ControlAccessibilityAction::Undo => self.undo(),
            ControlAccessibilityAction::Redo => self.redo(),
            _ => None,
        }
    }

    pub fn set_selection(
        &mut self,
        key: &str,
        anchor: usize,
        focus: usize,
    ) -> Option<ContentEditableEffect> {
        let host = self.host_mut(key)?;
        let length = host.value.chars().count();
        host.editor.selection = ControlSelection {
            anchor: anchor.min(length),
            focus: focus.min(length),
        };
        host.editor.caret_phase_ms = 0;
        Some(ContentEditableEffect::SelectionChanged {
            key: key.to_string(),
            selection: host.editor.selection,
        })
    }

    pub fn update_composition(&mut self, text: &str) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host_mut(&key)?;
        host.editor.composition = Some(bounded_text(text));
        host.editor.composition_range = Some(host.editor.selection);
        host.editor.caret_phase_ms = 0;
        Some(ContentEditableEffect::CompositionChanged {
            key,
            composition: host.editor.composition.clone(),
        })
    }

    pub fn commit_composition(&mut self) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let composition = self.host_mut(&key)?.editor.composition.take()?;
        self.replace_selection(&composition)
    }

    pub fn cancel_composition(&mut self) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host_mut(&key)?;
        host.editor.composition.take()?;
        host.editor.composition_range = None;
        Some(ContentEditableEffect::CompositionChanged {
            key,
            composition: None,
        })
    }

    pub fn copy_payload(&self) -> Option<ControlClipboardPayload> {
        let host = self.focused_host()?;
        let (start, end) = host.editor.selection.ordered();
        (start != end)
            .then(|| ControlClipboardPayload::from_plain_text(char_range(&host.value, start, end)))
    }

    pub fn cut(&mut self) -> Option<(String, ContentEditableEffect)> {
        let text = self.copy_payload()?.plain_text?;
        let effect = self.replace_selection("")?;
        Some((text, effect))
    }

    pub fn paste(&mut self, payload: &ControlClipboardPayload) -> Option<ContentEditableEffect> {
        self.replace_selection(&payload.preferred_text()?)
    }

    pub fn pointer_down(
        &mut self,
        key: &str,
        x: f64,
        y: f64,
        metrics: ControlTextMetrics,
        click_count: u8,
    ) -> Option<ContentEditableEffect> {
        self.focus(key)?;
        let host = self.host_mut(key)?;
        let offset = point_to_offset(&host.value, x, y, metrics);
        let selection = match click_count.min(3) {
            2 => word_selection(&host.value, offset),
            3 => line_selection(&host.value, offset),
            _ => ControlSelection::collapsed(offset),
        };
        host.editor.selection = selection;
        host.editor.pointer_anchor = Some(selection.anchor);
        Some(ContentEditableEffect::SelectionChanged {
            key: key.to_string(),
            selection,
        })
    }

    pub fn pointer_drag(
        &mut self,
        x: f64,
        y: f64,
        metrics: ControlTextMetrics,
    ) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host_mut(&key)?;
        let anchor = host.editor.pointer_anchor?;
        let focus = point_to_offset(&host.value, x, y, metrics);
        host.editor.selection = ControlSelection { anchor, focus };
        Some(ContentEditableEffect::SelectionChanged {
            key,
            selection: host.editor.selection,
        })
    }

    pub fn pointer_up(&mut self) {
        for host in &mut self.hosts {
            host.editor.pointer_anchor = None;
        }
    }

    pub fn advance_caret_blink(&mut self, elapsed_ms: u64) -> bool {
        let Some(key) = self.focused_key.clone() else {
            return false;
        };
        let Some(host) = self.host_mut(&key) else {
            return false;
        };
        let before = host.editor.caret_phase_ms < 500;
        host.editor.caret_phase_ms = (host.editor.caret_phase_ms + elapsed_ms) % 1000;
        before != (host.editor.caret_phase_ms < 500)
    }

    pub fn presentation(
        &mut self,
        key: &str,
        bounds: ControlRect,
        metrics: ControlTextMetrics,
    ) -> Option<ControlEditorPresentation> {
        let focused = self.focused_key.as_deref() == Some(key);
        let host = self.host_mut(key)?;
        Some(text_editor_presentation(
            &host.value,
            &mut host.editor,
            bounds,
            metrics,
            focused,
            true,
        ))
    }

    pub fn sync_render_tree(&self, tree: &mut BrowserRenderTree) {
        let mut editable_index = 0;
        sync_nodes(&mut tree.children, false, &self.hosts, &mut editable_index);
    }

    fn replace_selection(&mut self, replacement: &str) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host_mut(&key)?;
        let (start, end) = host.editor.selection.ordered();
        if start == end && replacement.is_empty() {
            return None;
        }
        let remaining = MAX_EDITABLE_BYTES.saturating_sub(
            host.value
                .len()
                .saturating_sub(byte_len_for_char_range(&host.value, start, end)),
        );
        let replacement = bounded_to_bytes(replacement, remaining);
        host.undo.push(EditSnapshot {
            value: host.value.clone(),
            selection: host.editor.selection,
        });
        if host.undo.len() > HISTORY_LIMIT {
            host.undo.remove(0);
        }
        host.redo.clear();
        replace_char_range(&mut host.value, start, end, &replacement);
        let caret = start + replacement.chars().count();
        host.editor.selection = ControlSelection::collapsed(caret);
        host.editor.composition = None;
        host.editor.composition_range = None;
        host.editor.caret_phase_ms = 0;
        host.dirty = true;
        Some(ContentEditableEffect::ValueChanged {
            key,
            value: host.value.clone(),
        })
    }

    fn delete(&mut self, backward: bool) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host(&key)?;
        let selection = host.editor.selection;
        if !selection.is_collapsed() {
            return self.replace_selection("");
        }
        let boundaries = grapheme_char_boundaries(&host.value);
        let caret = selection.focus;
        let adjacent = if backward {
            boundaries.into_iter().rfind(|offset| *offset < caret)
        } else {
            boundaries.into_iter().find(|offset| *offset > caret)
        }?;
        let (start, end) = if backward {
            (adjacent, caret)
        } else {
            (caret, adjacent)
        };
        self.set_selection(&key, start, end)?;
        self.replace_selection("")
    }

    fn move_caret(
        &mut self,
        forward: bool,
        extend: bool,
        by_word: bool,
    ) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host(&key)?;
        let current = host.editor.selection.focus;
        let destination = if by_word {
            word_destination(&host.value, current, forward)
        } else {
            let boundaries = grapheme_char_boundaries(&host.value);
            if forward {
                boundaries.into_iter().find(|offset| *offset > current)
            } else {
                boundaries.into_iter().rfind(|offset| *offset < current)
            }
            .unwrap_or(current)
        };
        let anchor = if extend {
            host.editor.selection.anchor
        } else {
            destination
        };
        self.set_selection(&key, anchor, destination)
    }

    fn move_line_boundary(&mut self, end: bool, extend: bool) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host(&key)?;
        let chars = host.value.chars().collect::<Vec<_>>();
        let current = host.editor.selection.focus.min(chars.len());
        let destination = if end {
            chars[current..]
                .iter()
                .position(|character| *character == '\n')
                .map_or(chars.len(), |offset| current + offset)
        } else {
            chars[..current]
                .iter()
                .rposition(|character| *character == '\n')
                .map_or(0, |offset| offset + 1)
        };
        let anchor = if extend {
            host.editor.selection.anchor
        } else {
            destination
        };
        self.set_selection(&key, anchor, destination)
    }

    fn undo(&mut self) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host_mut(&key)?;
        let previous = host.undo.pop()?;
        host.redo.push(EditSnapshot {
            value: host.value.clone(),
            selection: host.editor.selection,
        });
        host.value = previous.value;
        host.editor.selection = previous.selection;
        host.dirty = true;
        Some(ContentEditableEffect::ValueChanged {
            key,
            value: host.value.clone(),
        })
    }

    fn redo(&mut self) -> Option<ContentEditableEffect> {
        let key = self.focused_key.clone()?;
        let host = self.host_mut(&key)?;
        let next = host.redo.pop()?;
        host.undo.push(EditSnapshot {
            value: host.value.clone(),
            selection: host.editor.selection,
        });
        host.value = next.value;
        host.editor.selection = next.selection;
        host.dirty = true;
        Some(ContentEditableEffect::ValueChanged {
            key,
            value: host.value.clone(),
        })
    }

    fn focused_host(&self) -> Option<&EditingHost> {
        self.host(self.focused_key.as_deref()?)
    }

    fn host(&self, key: &str) -> Option<&EditingHost> {
        self.hosts.iter().find(|host| host.key == key)
    }

    fn host_mut(&mut self, key: &str) -> Option<&mut EditingHost> {
        self.hosts.iter_mut().find(|host| host.key == key)
    }
}

fn collect_editing_hosts(
    nodes: &[BrowserRenderNode],
    ancestor_editable: bool,
    out: &mut Vec<(ContentEditableMode, String)>,
) {
    for node in nodes {
        let explicit_mode = node.editing_mode.as_deref();
        let editable = explicit_mode.and_then(|mode| match mode {
            "plaintext" => Some(ContentEditableMode::Plaintext),
            "richtext" => Some(ContentEditableMode::RichText),
            _ => None,
        });
        let is_host = editable.is_some() && !ancestor_editable;
        if let Some(mode) = editable.filter(|_| is_host) {
            out.push((mode, rendered_text(node)));
        }
        let descendants_editable = match explicit_mode {
            Some("plaintext" | "richtext") => true,
            Some("false") => false,
            _ => ancestor_editable,
        };
        collect_editing_hosts(&node.children, descendants_editable, out);
    }
}

fn rendered_text(node: &BrowserRenderNode) -> String {
    node.text.clone().unwrap_or_else(|| {
        node.children
            .iter()
            .map(rendered_text)
            .collect::<Vec<_>>()
            .join("")
    })
}

fn sync_nodes(
    nodes: &mut [BrowserRenderNode],
    ancestor_editable: bool,
    hosts: &[EditingHost],
    editable_index: &mut usize,
) {
    for node in nodes {
        let editable = node.editing_mode.is_some();
        let is_host = editable && !ancestor_editable;
        if is_host {
            if let Some(host) = hosts.get(*editable_index).filter(|host| host.dirty) {
                node.text = Some(host.value.clone());
                node.children.clear();
            }
            *editable_index += 1;
        }
        sync_nodes(
            &mut node.children,
            ancestor_editable || is_host,
            hosts,
            editable_index,
        );
    }
}

fn point_to_offset(value: &str, x: f64, y: f64, metrics: ControlTextMetrics) -> usize {
    let advance = metrics.advance.max(0.1);
    let line_height = metrics.line_height.max(0.1);
    let line = (y.max(0.0) / line_height).floor() as usize;
    let starts = line_starts(value);
    let start = starts[line.min(starts.len().saturating_sub(1))];
    let length = value
        .chars()
        .skip(start)
        .take_while(|character| *character != '\n')
        .count();
    start + (((x.max(0.0) / advance) + 0.5).floor() as usize).min(length)
}

fn line_starts(value: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, character) in value.chars().enumerate() {
        if character == '\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn line_selection(value: &str, offset: usize) -> ControlSelection {
    let chars = value.chars().collect::<Vec<_>>();
    let offset = offset.min(chars.len());
    let start = chars[..offset]
        .iter()
        .rposition(|character| *character == '\n')
        .map_or(0, |index| index + 1);
    let end = chars[offset..]
        .iter()
        .position(|character| *character == '\n')
        .map_or(chars.len(), |index| offset + index);
    ControlSelection {
        anchor: start,
        focus: end,
    }
}

fn word_selection(value: &str, offset: usize) -> ControlSelection {
    let chars = value.chars().collect::<Vec<_>>();
    let offset = offset.min(chars.len().saturating_sub(1));
    let word = chars
        .get(offset)
        .is_some_and(|character| character.is_alphanumeric());
    let mut start = offset;
    while start > 0 && chars[start - 1].is_alphanumeric() == word {
        start -= 1;
    }
    let mut end = offset;
    while end < chars.len() && chars[end].is_alphanumeric() == word {
        end += 1;
    }
    ControlSelection {
        anchor: start,
        focus: end,
    }
}

fn word_destination(value: &str, offset: usize, forward: bool) -> usize {
    let chars = value.chars().collect::<Vec<_>>();
    if forward {
        let mut index = offset.min(chars.len());
        while index < chars.len() && chars[index].is_alphanumeric() {
            index += 1;
        }
        while index < chars.len() && !chars[index].is_alphanumeric() {
            index += 1;
        }
        index
    } else {
        let mut index = offset.min(chars.len());
        while index > 0 && !chars[index - 1].is_alphanumeric() {
            index -= 1;
        }
        while index > 0 && chars[index - 1].is_alphanumeric() {
            index -= 1;
        }
        index
    }
}

fn grapheme_char_boundaries(value: &str) -> Vec<usize> {
    let mut boundaries = vec![0];
    boundaries.extend(
        graphemes(value)
            .into_iter()
            .map(|grapheme| value[..grapheme.bytes.end].chars().count()),
    );
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
}

fn char_range(value: &str, start: usize, end: usize) -> String {
    value
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

fn byte_len_for_char_range(value: &str, start: usize, end: usize) -> usize {
    char_range(value, start, end).len()
}

fn replace_char_range(value: &mut String, start: usize, end: usize, replacement: &str) {
    let start_byte = value
        .char_indices()
        .nth(start)
        .map_or(value.len(), |(index, _)| index);
    let end_byte = value
        .char_indices()
        .nth(end)
        .map_or(value.len(), |(index, _)| index);
    value.replace_range(start_byte..end_byte, replacement);
}

fn bounded_text(value: &str) -> String {
    bounded_to_bytes(value, MAX_EDITABLE_BYTES)
}

fn bounded_to_bytes(value: &str, limit: usize) -> String {
    let mut output = value.to_string();
    truncate_utf8(&mut output, limit);
    output
}

fn truncate_utf8(value: &mut String, limit: usize) {
    if value.len() <= limit {
        return;
    }
    let mut boundary = limit;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_browser_render_tree;

    fn model(source: &str) -> (BrowserRenderTree, ContentEditableModel) {
        let tree = parse_browser_render_tree(source).unwrap();
        fn regions(
            nodes: &[BrowserRenderNode],
            inherited_editable: bool,
            out: &mut Vec<FocusRegion>,
        ) {
            for node in nodes {
                let explicit_mode = node.editing_mode.as_deref();
                let explicitly_editable = matches!(explicit_mode, Some("plaintext" | "richtext"));
                let editing_host = explicitly_editable && !inherited_editable;
                if node.editing_mode.is_some() {
                    out.push(FocusRegion {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0,
                        height: 40.0,
                        key: node
                            .id
                            .as_deref()
                            .map(|id| format!("focus:id:{id}"))
                            .unwrap_or_else(|| format!("focus:{}", out.len())),
                        role: "textbox".into(),
                        accessible_name: node.accessible_name.clone(),
                        editing_mode: node.editing_mode.clone(),
                        editing_host,
                        focus_order: out.len(),
                        tab_index: 0,
                        top_layer_index: None,
                        fixed: false,
                        clips: Vec::new(),
                    });
                }
                let descendants_editable = match explicit_mode {
                    Some("plaintext" | "richtext") => true,
                    Some("false") => false,
                    _ => inherited_editable,
                };
                regions(&node.children, descendants_editable, out);
            }
        }
        let mut regions_out = Vec::new();
        regions(&tree.children, false, &mut regions_out);
        let model = ContentEditableModel::from_page(&tree, &regions_out);
        (tree, model)
    }

    #[test]
    fn editing_is_bounded_transactional_and_not_a_form_control() {
        let (mut tree, mut model) = model(
            "<form><input id='field' value='form'></form>\
             <section id='notes' contenteditable='plaintext-only'>Draft</section>",
        );
        assert_eq!(model.accessibility_states().len(), 1);
        assert!(model.focus("focus:id:notes").is_some());
        assert!(model.key_down(ControlKey::SelectAll, false).is_some());
        assert!(matches!(
            model.text_input("Shared"),
            Some(ContentEditableEffect::ValueChanged { value, .. }) if value == "Shared"
        ));
        assert!(model.key_down(ControlKey::Undo, false).is_some());
        assert_eq!(model.accessibility_states()[0].value, "Draft");
        assert!(model.key_down(ControlKey::Redo, false).is_some());
        model.sync_render_tree(&mut tree);
        assert_eq!(tree.children[1].text.as_deref(), Some("Shared"));
        assert_eq!(tree.children[0].children[0].value.as_deref(), Some("form"));
    }

    #[test]
    fn composition_clipboard_and_shared_geometry_follow_one_editor() {
        let (_, mut model) =
            model("<p id='notes' contenteditable aria-label='Notes'>hello world</p>");
        model.focus("focus:id:notes").unwrap();
        model.set_selection("focus:id:notes", 6, 11).unwrap();
        assert_eq!(
            model.copy_payload().unwrap().plain_text.as_deref(),
            Some("world")
        );
        model.update_composition("界").unwrap();
        let presentation = model
            .presentation(
                "focus:id:notes",
                ControlRect {
                    x: 10.0,
                    y: 20.0,
                    width: 200.0,
                    height: 40.0,
                },
                ControlTextMetrics::default(),
            )
            .unwrap();
        assert!(!presentation.composition_underlines.is_empty());
        assert!(presentation.candidate_rect.is_some());
        model.commit_composition().unwrap();
        assert_eq!(model.accessibility_states()[0].value, "hello 界");
    }

    #[test]
    fn explicit_false_boundaries_preserve_editing_host_alignment() {
        let (_, model) = model(
            "<div contenteditable='false' tabindex='0'>blocked</div>\
             <section id='outer' contenteditable>outer <b contenteditable>nested</b></section>\
             <div contenteditable='false'><p id='island' contenteditable>island</p></div>",
        );
        let states = model.accessibility_states();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].key, "focus:id:outer");
        assert_eq!(states[1].key, "focus:id:island");
    }
}
