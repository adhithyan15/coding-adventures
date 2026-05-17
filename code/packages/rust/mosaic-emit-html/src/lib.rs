//! # mosaic-emit-html — Pure HTML static snapshot backend.
//!
//! This crate is the static HTML backend for the Mosaic compiler. It
//! implements [`MosaicRenderer`] and is driven by [`MosaicVM`]. The output is
//! a complete `<!DOCTYPE html>` file with no JavaScript, no runtime, and fully
//! inlined styles.
//!
//! ## Use cases
//!
//! - **Design reviews** — share a rendered snapshot without a dev server.
//! - **Screenshot tests** — feed to headless Chrome / Playwright for visual
//!   regression testing.
//! - **Static documentation** — embed rendered component previews in docs.
//! - **e2e test fixtures** — stable reference HTML for integration tests.
//!
//! ## Fixture file
//!
//! An optional JSON fixture file provides concrete values for slots:
//!
//! ```json
//! {
//!   "display-name": "Jane Doe",
//!   "avatar-url": "https://example.com/avatar.png",
//!   "count": 42,
//!   "visible": true,
//!   "items": ["Alpha", "Beta", "Gamma"]
//! }
//! ```
//!
//! Slots absent from the fixture render as `[slot: name]` placeholders.
//!
//! ## Output structure
//!
//! ```html
//! <!DOCTYPE html>
//! <html lang="en">
//! <head>
//!   <meta charset="UTF-8">
//!   <title>ProfileCard</title>
//!   <style>…</style>
//! </head>
//! <body>
//!   <div style="display:flex;flex-direction:column">
//!     <span>Jane Doe</span>
//!   </div>
//! </body>
//! </html>
//! ```

use mosaic_vm::{EmitResult, MosaicRenderer, ResolvedProperty, ResolvedValue};
use mosaic_analyzer::{MosaicEmit, MosaicSlot, MosaicType};

// ===========================================================================
// HtmlRenderer
// ===========================================================================

/// The pure-HTML static snapshot backend for the Mosaic compiler.
///
/// Construct with [`HtmlRenderer::new`], passing fixture values and an optional
/// CSS string. Then drive it with [`MosaicVM::run`].
///
/// # Example
///
/// ```no_run
/// use mosaic_emit_html::HtmlRenderer;
/// use mosaic_vm::MosaicVM;
/// use mosaic_analyzer::analyze;
/// use serde_json::json;
///
/// let fixtures = json!({"display-name": "Jane"}).as_object().cloned().unwrap();
/// let renderer = HtmlRenderer::new(fixtures, None);
/// let file = analyze("component Card { slot display-name: text; Text { content: @display-name; } }").unwrap();
/// let vm = MosaicVM::new(file);
/// let result = vm.run(renderer).unwrap();
/// println!("{}", result.output);
/// ```
pub struct HtmlRenderer {
    component_name: String,
    slots: Vec<MosaicSlot>,
    /// Fixture values for slots provided at compile time.
    /// Keys are kebab-case slot names; values are JSON values.
    fixtures: serde_json::Map<String, serde_json::Value>,
    /// Stack of open element frames during depth-first traversal.
    stack: Vec<HtmlFrame>,
    /// HTML lines accumulated at the root level (outside any frame).
    root_lines: Vec<String>,
    /// Suppression counter: when > 0 we are inside a false `when` block
    /// and must not emit any HTML. Nested false blocks increment further.
    suppress: usize,
    /// Optional CSS to inline inside the `<style>` tag in `<head>`.
    css: Option<String>,
    /// Loop variable bindings: (var_name, fixture_value).
    /// When an `each` block is active, the current fixture array element is
    /// substituted wherever the loop variable appears as a slot ref.
    loop_bindings: Vec<(String, Option<serde_json::Value>)>,
    /// Active `each`-recording state; `Some` while recording the body of an
    /// `each` block that has more than one fixture item.
    each_recording: Option<EachRecordState>,
}

/// A single open element frame on the traversal stack.
struct HtmlFrame {
    /// The HTML close tag (empty for self-closing elements like `<hr>`).
    close_tag: String,
    /// HTML lines accumulated as children of this element.
    lines: Vec<String>,
}

// ===========================================================================
// Each-block event recording
// ===========================================================================

/// A renderer event recorded during an `each` body traversal.
///
/// The VM calls renderer methods exactly once per AST node. To render all
/// fixture array elements (not just the first), we record every call made
/// during the body traversal and replay the recording for each subsequent
/// item.  Only the loop variable binding changes between replays — all layout
/// structure and literal text stay the same.
#[derive(Clone, Debug)]
enum EachEvent {
    BeginNode {
        tag: String,
        is_primitive: bool,
        props: Vec<ResolvedProperty>,
    },
    EndNode {
        tag: String,
    },
    RenderSlotChild {
        slot_name: String,
        slot_type: MosaicType,
    },
    BeginWhen {
        slot_name: String,
    },
    EndWhen,
    /// A nested `each` encountered inside an outer `each` body. During replay
    /// the inner `each` is expanded with the *same* fixture data as the first
    /// pass (v1 limitation — nested loops use first-item semantics).
    BeginEach {
        slot_name: String,
        item_name: String,
        element_type: MosaicType,
    },
    EndEach,
}

/// State maintained while recording events for an `each` block.
///
/// Created in `begin_each` when the fixture array contains more than one item,
/// consumed in `end_each` to replay the body for the remaining items.
struct EachRecordState {
    /// The loop variable name (e.g. `"task"` in `each @tasks as task`).
    item_name: String,
    /// Items beyond the first (indices 1..N) — replayed after the live pass.
    remaining_items: Vec<serde_json::Value>,
    /// Events recorded from the body traversal.
    events: Vec<EachEvent>,
    /// Nesting depth of `each` blocks encountered *inside* this recording.
    /// When > 0, inner `begin_each`/`end_each` calls are recorded verbatim
    /// rather than starting a new outer recording.
    nesting_depth: usize,
}

impl HtmlRenderer {
    /// Create a new `HtmlRenderer`.
    ///
    /// - `fixtures` — a JSON object mapping slot names to values.
    /// - `css` — optional CSS string to inline in `<style>`.
    pub fn new(
        fixtures: serde_json::Map<String, serde_json::Value>,
        css: Option<String>,
    ) -> Self {
        Self {
            component_name: String::new(),
            slots: Vec::new(),
            fixtures,
            stack: Vec::new(),
            root_lines: Vec::new(),
            suppress: 0,
            css,
            loop_bindings: Vec::new(),
            each_recording: None,
        }
    }

    // -----------------------------------------------------------------------
    // Name-conversion helpers
    // -----------------------------------------------------------------------

    /// Convert a PascalCase component name to a human-readable title.
    ///
    /// We insert spaces before each uppercase letter following a lowercase one:
    /// `ProfileCard` → `Profile Card`.
    fn to_title(name: &str) -> String {
        let mut result = String::new();
        let mut prev_lower = false;
        for ch in name.chars() {
            if ch.is_uppercase() && prev_lower {
                result.push(' ');
            }
            prev_lower = ch.is_lowercase();
            result.push(ch);
        }
        result
    }

    // -----------------------------------------------------------------------
    // Fixture resolution
    // -----------------------------------------------------------------------

    /// Resolve a slot reference to a string value.
    ///
    /// Resolution order:
    /// 1. Active loop bindings (innermost-first).
    /// 2. The fixture map.
    /// 3. Placeholder `[slot: name]`.
    fn resolve_slot(&self, name: &str) -> String {
        // Check loop bindings innermost first.
        for (var_name, val) in self.loop_bindings.iter().rev() {
            if var_name == name {
                return match val {
                    Some(v) => json_to_string(v),
                    None => format!("[{name}]"),
                };
            }
        }
        // Fall back to fixture map.
        match self.fixtures.get(name) {
            Some(v) => json_to_string(v),
            None => format!("[slot: {name}]"),
        }
    }

    /// Resolve a `ResolvedValue` to a displayable string.
    fn resolve_value(&self, v: &ResolvedValue) -> String {
        match v {
            ResolvedValue::String(s) => s.clone(),
            ResolvedValue::Number(n) => n.to_string(),
            ResolvedValue::Dimension(n, unit) => match unit.as_str() {
                "dp" | "sp" => format!("{n}px"),
                "%" => format!("{n}%"),
                _ => format!("{n}{unit}"),
            },
            ResolvedValue::Color(r, g, b, a) => {
                if *a == 255 {
                    format!("rgb({r},{g},{b})")
                } else {
                    let alpha = *a as f64 / 255.0;
                    format!("rgba({r},{g},{b},{alpha:.3})")
                }
            }
            ResolvedValue::Bool(b) => b.to_string(),
            ResolvedValue::Enum { namespace, member } => format!("{namespace}-{member}"),
            ResolvedValue::SlotRef { name, .. } => self.resolve_slot(name),
        }
    }

    // -----------------------------------------------------------------------
    // HTML building helpers
    // -----------------------------------------------------------------------

    /// Push a line into the current open frame or root accumulator.
    fn push_line(&mut self, line: String) {
        if self.suppress > 0 {
            return; // Suppressed by a false `when` block.
        }
        if let Some(frame) = self.stack.last_mut() {
            frame.lines.push(line);
        } else {
            self.root_lines.push(line);
        }
    }

    /// Build an inline style string from resolved properties.
    ///
    /// # Security
    ///
    /// Resolved values (including fixture strings from the host) are
    /// HTML-escaped before being embedded in the `style="…"` attribute.
    /// This prevents an attacker-controlled fixture value such as
    /// `red" onmouseover="alert(1)` from breaking out of the attribute and
    /// injecting event handlers.
    fn build_style(&self, props: &[ResolvedProperty]) -> String {
        let skip = ["content", "source", "a11y-label", "a11y-role", "a11y-hidden", "style"];
        let entries: Vec<String> = props
            .iter()
            .filter(|p| !skip.contains(&p.name.as_str()))
            .map(|p| {
                let css_name = &p.name;
                // Escape the resolved value at the output boundary to prevent
                // HTML attribute break-out (CSS injection / attribute injection).
                let val = html_escape(&self.resolve_value(&p.value));
                format!("{css_name}:{val}")
            })
            .collect();
        entries.join(";")
    }

    /// Get the CSS class from a `style: namespace.member` prop.
    ///
    /// # Security
    ///
    /// The class value is HTML-escaped before use. Although Mosaic's `style`
    /// properties are typically enum literals (e.g. `heading.large`), any
    /// `String` value that originated from a slot reference or fixture file
    /// must be escaped to prevent attribute break-out.
    fn get_class(props: &[ResolvedProperty]) -> String {
        props
            .iter()
            .find(|p| p.name == "style")
            .map(|p| match &p.value {
                ResolvedValue::Enum { namespace, member } => {
                    // Enum variants are compiler-generated identifiers — escape
                    // for defence-in-depth.
                    html_escape(&format!("mosaic-{namespace}-{member}"))
                }
                ResolvedValue::String(s) => html_escape(s),
                _ => String::new(),
            })
            .unwrap_or_default()
    }

    /// Build the opening HTML tag for a primitive Mosaic node.
    ///
    /// Returns `(open_tag, close_tag)`. Self-closing tags have an empty close.
    fn primitive_open(&self, tag: &str, props: &[ResolvedProperty]) -> (String, String) {
        let extra_style = self.build_style(props);
        let class = Self::get_class(props);

        match tag {
            "Box" => {
                let combined = combine_styles("", &extra_style);
                (format_div(&combined, &class, props), "</div>".into())
            }
            "Stack" => {
                let combined = combine_styles("position:relative", &extra_style);
                (format_div(&combined, &class, props), "</div>".into())
            }
            "Column" => {
                let combined = combine_styles("display:flex;flex-direction:column", &extra_style);
                (format_div(&combined, &class, props), "</div>".into())
            }
            "Row" => {
                let combined = combine_styles("display:flex;flex-direction:row", &extra_style);
                (format_div(&combined, &class, props), "</div>".into())
            }
            "Text" => {
                let content = props
                    .iter()
                    .find(|p| p.name == "content")
                    .map(|p| html_escape(&self.resolve_value(&p.value)))
                    .unwrap_or_default();
                let mut attrs = String::new();
                if !extra_style.is_empty() {
                    attrs.push_str(&format!(" style=\"{extra_style}\""));
                }
                if !class.is_empty() {
                    attrs.push_str(&format!(" class=\"{class}\""));
                }
                // Text is self-contained — content and close tag together.
                (format!("<span{attrs}>{content}</span>"), String::new())
            }
            "Image" => {
                let src = props
                    .iter()
                    .find(|p| p.name == "source")
                    .map(|p| html_escape(&self.resolve_value(&p.value)))
                    .unwrap_or_default();
                let alt = props
                    .iter()
                    .find(|p| p.name == "a11y-label")
                    .map(|p| html_escape(&self.resolve_value(&p.value)))
                    .unwrap_or_default();
                (format!("<img src=\"{src}\" alt=\"{alt}\">"), String::new())
            }
            "Spacer" => {
                let combined = combine_styles("flex:1", &extra_style);
                (format_div(&combined, &class, props), "</div>".into())
            }
            "Scroll" => {
                let combined = combine_styles("overflow:auto", &extra_style);
                (format_div(&combined, &class, props), "</div>".into())
            }
            "Divider" => ("<hr>".into(), String::new()),
            "Icon" => {
                let class_val = if class.is_empty() {
                    "icon".to_string()
                } else {
                    format!("icon {class}")
                };
                (format!("<span class=\"{class_val}\">"), "</span>".into())
            }
            "Grid" => {
                // Resolve the headers and rows fixture arrays.
                let headers_slot = props
                    .iter()
                    .find(|p| p.name == "headers" || p.name == "column-headers")
                    .and_then(|p| {
                        if let ResolvedValue::SlotRef { name, .. } = &p.value {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "headers".to_string());

                let rows_slot = props
                    .iter()
                    .find(|p| p.name == "rows" || p.name == "viewport-rows")
                    .and_then(|p| {
                        if let ResolvedValue::SlotRef { name, .. } = &p.value {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "rows".to_string());

                let headers: Vec<String> = self
                    .fixtures
                    .get(&headers_slot)
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().map(|h| html_escape(&json_to_string(h))).collect())
                    .unwrap_or_else(|| vec![format!("[{headers_slot}]")]);

                let rows: Vec<String> = self
                    .fixtures
                    .get(&rows_slot)
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().map(|r| html_escape(&json_to_string(r))).collect())
                    .unwrap_or_else(|| vec![format!("[{rows_slot}]")]);

                let style_attr = if extra_style.is_empty() {
                    String::new()
                } else {
                    format!(" style=\"{extra_style}\"")
                };

                let header_cells: String = headers
                    .iter()
                    .map(|h| format!("<th>{h}</th>"))
                    .collect();
                let row_cells: String = rows
                    .iter()
                    .map(|r| format!("<tr><td>{r}</td></tr>"))
                    .collect();

                let table = format!(
                    "<table{style_attr}><thead><tr>{header_cells}</tr></thead><tbody>{row_cells}</tbody></table>",
                );
                (table, String::new())
            }
            _ => {
                // Unknown primitive: generic div.
                let combined = combine_styles("", &extra_style);
                (format_div(&combined, &class, props), "</div>".into())
            }
        }
    }

    /// Minimal CSS reset emitted when no external CSS is provided.
    fn minimal_reset() -> &'static str {
        "*, *::before, *::after { box-sizing: border-box; }\nbody { margin: 0; font-family: sans-serif; }"
    }

    // -----------------------------------------------------------------------
    // Inner rendering methods (called directly during replay, bypassing recording)
    // -----------------------------------------------------------------------

    /// Core logic for opening a node — pushes an `HtmlFrame` and emits the
    /// opening tag.  Called both on the live pass and during each-block replay.
    fn begin_node_inner(&mut self, tag: &str, is_primitive: bool, props: &[ResolvedProperty]) {
        if self.suppress > 0 {
            self.stack.push(HtmlFrame {
                close_tag: String::new(),
                lines: Vec::new(),
            });
            return;
        }

        let (open, close) = if is_primitive {
            self.primitive_open(tag, props)
        } else {
            let tag_lower = tag.to_lowercase();
            (
                format!("<div data-component=\"{tag_lower}\">"),
                "</div>".into(),
            )
        };

        if close.is_empty() {
            self.push_line(open);
            self.stack.push(HtmlFrame {
                close_tag: String::new(),
                lines: Vec::new(),
            });
        } else {
            self.push_line(open);
            self.stack.push(HtmlFrame {
                close_tag: close,
                lines: Vec::new(),
            });
        }
    }

    /// Core logic for closing a node — pops the top `HtmlFrame` and emits its
    /// accumulated inner lines followed by the close tag.
    fn end_node_inner(&mut self, _tag: &str) {
        if let Some(frame) = self.stack.pop() {
            if self.suppress > 0 {
                return;
            }
            let inner = frame.lines.join("\n");
            let close = frame.close_tag;
            if !close.is_empty() {
                if inner.is_empty() {
                    self.push_line(close);
                } else {
                    self.push_line(inner);
                    self.push_line(close);
                }
            } else if !inner.is_empty() {
                self.push_line(inner);
            }
        }
    }

    /// Core logic for a slot-child placeholder.
    fn render_slot_child_inner(&mut self, slot_name: &str, _slot_type: &MosaicType) {
        if self.suppress > 0 {
            return;
        }
        let escaped_name = html_escape(slot_name);
        let placeholder = format!(
            "<div class=\"mos-slot\" data-slot=\"{escaped_name}\"><!-- slot: {escaped_name} --></div>"
        );
        self.push_line(placeholder);
    }

    /// Core logic for a `when` block open.
    fn begin_when_inner(&mut self, slot_name: &str) {
        if self.suppress > 0 {
            self.suppress += 1;
            return;
        }
        let is_true = match self.fixtures.get(slot_name) {
            None => true,
            Some(v) => v.as_bool().unwrap_or(false),
        };
        if !is_true {
            self.suppress += 1;
        }
    }

    /// Core logic for a `when` block close.
    fn end_when_inner(&mut self) {
        if self.suppress > 0 {
            self.suppress -= 1;
        }
    }

    /// Inner `begin_each` used during replay of a nested each.
    ///
    /// During replay of an outer `each` body, any inner `each` block
    /// encountered in the event stream is expanded with the first fixture item
    /// only (v1 limitation — nested loops are rare and single-level is
    /// sufficient for the current demo).
    fn begin_each_inner(&mut self, slot_name: &str, item_name: &str, _element_type: &MosaicType) {
        if self.suppress > 0 {
            return;
        }
        let first_item = self
            .fixtures
            .get(slot_name)
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .cloned();
        self.loop_bindings.push((item_name.to_string(), first_item));
    }

    /// Inner `end_each` used during replay of a nested each.
    fn end_each_inner(&mut self) {
        if self.suppress > 0 {
            return;
        }
        self.loop_bindings.pop();
    }

    /// Replay a recorded event log under a specific loop variable binding.
    ///
    /// Called once per additional fixture item (items[1..N]).  The stack is at
    /// the same depth as at the start of the `each` body because the live pass
    /// (item 0) balanced every push with a pop.
    ///
    /// # Arguments
    ///
    /// - `item_name` — the loop variable name to bind during this pass.
    /// - `item` — the JSON value for this iteration's fixture item.
    /// - `events` — the recorded event log (cloned from recording state).
    fn replay_events(
        &mut self,
        item_name: &str,
        item: serde_json::Value,
        events: &[EachEvent],
    ) {
        self.loop_bindings.push((item_name.to_string(), Some(item)));
        for event in events {
            match event {
                EachEvent::BeginNode { tag, is_primitive, props } => {
                    self.begin_node_inner(tag, *is_primitive, props);
                }
                EachEvent::EndNode { tag } => {
                    self.end_node_inner(tag);
                }
                EachEvent::RenderSlotChild { slot_name, slot_type } => {
                    self.render_slot_child_inner(slot_name, slot_type);
                }
                EachEvent::BeginWhen { slot_name } => {
                    self.begin_when_inner(slot_name);
                }
                EachEvent::EndWhen => {
                    self.end_when_inner();
                }
                EachEvent::BeginEach { slot_name, item_name: nested_name, element_type } => {
                    // Nested each during replay: expand first item only (v1).
                    self.begin_each_inner(slot_name, nested_name, element_type);
                }
                EachEvent::EndEach => {
                    self.end_each_inner();
                }
            }
        }
        self.loop_bindings.pop();
    }
}

impl MosaicRenderer for HtmlRenderer {
    fn begin_component(&mut self, name: &str, slots: &[MosaicSlot], _emits: &[MosaicEmit]) {
        self.component_name = name.to_string();
        self.slots = slots.to_vec();
    }

    fn end_component(&mut self) {}

    fn begin_node(&mut self, tag: &str, is_primitive: bool, props: &[ResolvedProperty]) {
        // If we're recording an `each` body (and not inside a nested inner each),
        // capture this event so it can be replayed for subsequent fixture items.
        if let Some(rec) = self.each_recording.as_mut() {
            if rec.nesting_depth == 0 {
                rec.events.push(EachEvent::BeginNode {
                    tag: tag.to_string(),
                    is_primitive,
                    props: props.to_vec(),
                });
            }
        }
        self.begin_node_inner(tag, is_primitive, props);
    }

    fn end_node(&mut self, tag: &str) {
        if let Some(rec) = self.each_recording.as_mut() {
            if rec.nesting_depth == 0 {
                rec.events.push(EachEvent::EndNode {
                    tag: tag.to_string(),
                });
            }
        }
        self.end_node_inner(tag);
    }

    fn render_slot_child(&mut self, slot_name: &str, slot_type: &MosaicType) {
        // Security: HTML-escape the slot name before embedding it in an attribute
        // value and an HTML comment. Slot names are constrained to identifier syntax
        // by the analyzer, but we escape at the output boundary for defence-in-depth.
        if let Some(rec) = self.each_recording.as_mut() {
            if rec.nesting_depth == 0 {
                rec.events.push(EachEvent::RenderSlotChild {
                    slot_name: slot_name.to_string(),
                    slot_type: slot_type.clone(),
                });
            }
        }
        self.render_slot_child_inner(slot_name, slot_type);
    }

    fn begin_when(&mut self, slot_name: &str) {
        if let Some(rec) = self.each_recording.as_mut() {
            if rec.nesting_depth == 0 {
                rec.events.push(EachEvent::BeginWhen {
                    slot_name: slot_name.to_string(),
                });
            }
        }
        self.begin_when_inner(slot_name);
    }

    fn end_when(&mut self) {
        if let Some(rec) = self.each_recording.as_mut() {
            if rec.nesting_depth == 0 {
                rec.events.push(EachEvent::EndWhen);
            }
        }
        self.end_when_inner();
    }

    fn begin_each(&mut self, slot_name: &str, item_name: &str, element_type: &MosaicType) {
        if self.suppress > 0 {
            return;
        }

        // If we're already recording an outer `each` body, this is a nested
        // `each`. Record it verbatim and increment nesting depth so inner events
        // don't get double-recorded as outer events.
        if let Some(rec) = self.each_recording.as_mut() {
            rec.events.push(EachEvent::BeginEach {
                slot_name: slot_name.to_string(),
                item_name: item_name.to_string(),
                element_type: element_type.clone(),
            });
            rec.nesting_depth += 1;
            // Still need a loop binding so resolve_slot doesn't emit placeholders
            // for the loop variable during the live (item 0) pass.
            let first_item = self
                .fixtures
                .get(slot_name)
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .cloned();
            self.loop_bindings.push((item_name.to_string(), first_item));
            return;
        }

        // Top-level `each` — collect all fixture items.
        let items: Vec<serde_json::Value> = self
            .fixtures
            .get(slot_name)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let first_item = items.first().cloned();

        // If there are 2+ items, start recording so we can replay for items[1..].
        if items.len() > 1 {
            self.each_recording = Some(EachRecordState {
                item_name: item_name.to_string(),
                remaining_items: items[1..].to_vec(),
                events: Vec::new(),
                nesting_depth: 0,
            });
        }

        // Push the first item (or None placeholder) as the active loop binding.
        self.loop_bindings.push((item_name.to_string(), first_item));
    }

    fn end_each(&mut self) {
        if self.suppress > 0 {
            return;
        }

        // If we're inside an outer recording and this closes a nested `each`:
        if let Some(rec) = self.each_recording.as_mut() {
            if rec.nesting_depth > 0 {
                rec.events.push(EachEvent::EndEach);
                rec.nesting_depth -= 1;
                self.loop_bindings.pop();
                return;
            }
        }

        // Pop the loop binding used for item 0 (the live pass).
        self.loop_bindings.pop();

        // Replay the recorded event log for each remaining item.
        if let Some(rec) = self.each_recording.take() {
            let item_name = rec.item_name.clone();
            let events = rec.events.clone();
            for item in rec.remaining_items {
                self.replay_events(&item_name, item, &events);
            }
        }
    }

    fn emit(self) -> EmitResult {
        // Security: HTML-escape the title even though component names are
        // currently restricted to identifier syntax. Defence-in-depth: always
        // escape at the output boundary regardless of upstream validation.
        let title = html_escape(&Self::to_title(&self.component_name));
        let css = self
            .css
            .as_deref()
            .unwrap_or(Self::minimal_reset())
            .to_string();
        let html_body = self.root_lines.join("\n");

        let output = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>{title}</title>
  <style>
{css}
  </style>
</head>
<body>
{body}
</body>
</html>
"#,
            title = title,
            css = css,
            body = html_body,
        );

        EmitResult {
            output,
            component_name: self.component_name,
        }
    }
}

// ===========================================================================
// Free helpers
// ===========================================================================

/// HTML-escape a string for safe embedding in attribute values or text content.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Sanitize a CSS string for safe embedding inside a `<style>` tag.
///
/// Rejects CSS that contains `</style` (case-insensitive) — the substring that
/// would close the `<style>` block prematurely, allowing an attacker to inject
/// arbitrary HTML (including `<script>` tags) after the closing tag.
///
/// # Errors
///
/// Returns `Err(String)` with a human-readable message if the CSS is rejected.
///
/// # Example
///
/// ```
/// use mosaic_emit_html::sanitize_css;
/// assert!(sanitize_css("body { color: red; }").is_ok());
/// assert!(sanitize_css("</style><script>alert(1)</script>").is_err());
/// ```
pub fn sanitize_css(css: &str) -> Result<String, String> {
    // Case-insensitive search: `</STYLE` is equally dangerous.
    if css.to_ascii_lowercase().contains("</style") {
        return Err(
            "CSS file contains '</style' which would break out of the <style> block \
             and allow HTML injection. Remove or escape the offending substring."
                .into(),
        );
    }
    Ok(css.to_string())
}

/// Convert a JSON value to a displayable string.
fn json_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(_) => "[array]".to_string(),
        serde_json::Value::Object(_) => "[object]".to_string(),
    }
}

/// Combine two CSS style strings with a semicolon separator.
fn combine_styles(base: &str, extra: &str) -> String {
    match (base.is_empty(), extra.is_empty()) {
        (true, true) => String::new(),
        (true, false) => extra.to_string(),
        (false, true) => base.to_string(),
        (false, false) => format!("{base};{extra}"),
    }
}

/// Format a `<div>` with optional style, class, and a11y attributes.
///
/// # Security
///
/// `style` is produced by `build_style`, which already HTML-escapes each
/// individual value. `class` is produced by `get_class`, which also HTML-escapes.
/// Both are wrapped in double-quoted attributes here; the escaping at the source
/// ensures no `"` character can break out of the attribute.
fn format_div(style: &str, class: &str, props: &[ResolvedProperty]) -> String {
    let mut attrs = Vec::new();
    if !style.is_empty() {
        attrs.push(format!("style=\"{style}\""));
    }
    if !class.is_empty() {
        attrs.push(format!("class=\"{class}\""));
    }
    for p in props {
        match p.name.as_str() {
            "a11y-label" => {
                if let ResolvedValue::String(s) = &p.value {
                    attrs.push(format!("aria-label=\"{}\"", html_escape(s)));
                }
            }
            "a11y-role" => {
                if let ResolvedValue::String(s) = &p.value {
                    attrs.push(format!("role=\"{}\"", html_escape(s)));
                }
            }
            "a11y-hidden" => {
                attrs.push("aria-hidden=\"true\"".into());
            }
            _ => {}
        }
    }
    if attrs.is_empty() {
        "<div>".into()
    } else {
        format!("<div {}>", attrs.join(" "))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_vm::MosaicVM;
    use mosaic_analyzer::analyze;
    use serde_json::json;

    /// Build an `HtmlRenderer` with the given fixture JSON, run it on `src`.
    fn emit_with(src: &str, fixtures: serde_json::Value, css: Option<String>) -> String {
        let map = fixtures.as_object().cloned().unwrap_or_default();
        let renderer = HtmlRenderer::new(map, css);
        let file = analyze(src).unwrap();
        let vm = MosaicVM::new(file);
        vm.run(renderer).unwrap().output
    }

    /// Emit with an empty fixture map and no CSS.
    fn emit(src: &str) -> String {
        emit_with(src, json!({}), None)
    }

    // -----------------------------------------------------------------------
    // Test 1: Output has correct HTML document structure
    // -----------------------------------------------------------------------

    #[test]
    fn test_html_document_structure() {
        let out = emit(r#"component X { Box { } }"#);
        assert!(out.starts_with("<!DOCTYPE html>"), "Expected DOCTYPE: {out}");
        assert!(out.contains("<html lang=\"en\">"), "Expected <html>: {out}");
        assert!(out.contains("<head>"), "Expected <head>: {out}");
        assert!(out.contains("<body>"), "Expected <body>: {out}");
        assert!(out.contains("</html>"), "Expected </html>: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 2: <title> contains component name
    // -----------------------------------------------------------------------

    #[test]
    fn test_component_title() {
        let out = emit(r#"component ProfileCard { Box { } }"#);
        assert!(out.contains("<title>Profile Card</title>"), "Expected title: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 3: Box renders as <div>
    // -----------------------------------------------------------------------

    #[test]
    fn test_box_renders_div() {
        let out = emit(r#"component X { Box { } }"#);
        assert!(out.contains("<div"), "Expected <div: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 4: Column → flex-direction:column inline style
    // -----------------------------------------------------------------------

    #[test]
    fn test_column_flex_style() {
        let out = emit(r#"component X { Column { } }"#);
        assert!(
            out.contains("flex-direction:column"),
            "Expected flex-direction:column: {out}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Row → flex-direction:row inline style
    // -----------------------------------------------------------------------

    #[test]
    fn test_row_flex_style() {
        let out = emit(r#"component X { Row { } }"#);
        assert!(
            out.contains("flex-direction:row"),
            "Expected flex-direction:row: {out}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Text content resolved from fixture
    // -----------------------------------------------------------------------

    #[test]
    fn test_text_content_from_fixture() {
        let out = emit_with(
            r#"component Card { slot title: text; Text { content: @title; } }"#,
            json!({"title": "Hello World"}),
            None,
        );
        assert!(out.contains("Hello World"), "Expected fixture value: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 7: Text content uses placeholder when slot not in fixture
    // -----------------------------------------------------------------------

    #[test]
    fn test_text_content_placeholder() {
        let out = emit(r#"component Card { slot title: text; Text { content: @title; } }"#);
        assert!(
            out.contains("[slot: title]"),
            "Expected placeholder: {out}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: when block with fixture = true renders its body
    // -----------------------------------------------------------------------

    #[test]
    fn test_when_true_renders() {
        let out = emit_with(
            r#"component X {
              slot show: bool;
              Column {
                when @show {
                  Text { content: "Visible"; }
                }
              }
            }"#,
            json!({"show": true}),
            None,
        );
        assert!(out.contains("Visible"), "Expected body when show=true: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 9: when block with fixture = false suppresses its body
    // -----------------------------------------------------------------------

    #[test]
    fn test_when_false_suppresses() {
        let out = emit_with(
            r#"component X {
              slot show: bool;
              Column {
                when @show {
                  Text { content: "Hidden"; }
                }
              }
            }"#,
            json!({"show": false}),
            None,
        );
        assert!(!out.contains("Hidden"), "Expected body suppressed when show=false: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 10: when block with no fixture defaults to showing (design review)
    // -----------------------------------------------------------------------

    #[test]
    fn test_when_missing_renders() {
        let out = emit(
            r#"component X {
              slot show: bool;
              Column {
                when @show {
                  Text { content: "DefaultVisible"; }
                }
              }
            }"#,
        );
        assert!(
            out.contains("DefaultVisible"),
            "Expected body shown when fixture missing: {out}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: each block with fixture array renders ALL items
    // -----------------------------------------------------------------------

    #[test]
    fn test_each_with_fixture() {
        let out = emit_with(
            r#"component X {
              slot items: list<text>;
              Column {
                each @items as item {
                  Text { content: @item; }
                }
              }
            }"#,
            json!({"items": ["FirstItem", "SecondItem", "ThirdItem"]}),
            None,
        );
        assert!(out.contains("FirstItem"), "Expected first fixture item: {out}");
        assert!(out.contains("SecondItem"), "Expected second fixture item: {out}");
        assert!(out.contains("ThirdItem"), "Expected third fixture item: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 12: each block without fixture renders placeholder
    // -----------------------------------------------------------------------

    #[test]
    fn test_each_without_fixture() {
        let out = emit(
            r#"component X {
              slot items: list<text>;
              Column {
                each @items as item {
                  Text { content: @item; }
                }
              }
            }"#,
        );
        // Without fixture, loop var resolves to [item] placeholder.
        assert!(out.contains("[item]"), "Expected placeholder for loop var: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 13: CSS string appears inside <style> tag
    // -----------------------------------------------------------------------

    #[test]
    fn test_css_inlined() {
        let css = "body { background: red; }".to_string();
        let out = emit_with(r#"component X { Box { } }"#, json!({}), Some(css));
        assert!(
            out.contains("body { background: red; }"),
            "Expected CSS in <style>: {out}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: Slot values with HTML characters are escaped
    // -----------------------------------------------------------------------

    #[test]
    fn test_slot_value_html_escaped() {
        let out = emit_with(
            r#"component X { slot code: text; Text { content: @code; } }"#,
            json!({"code": "<script>alert(1)</script>"}),
            None,
        );
        assert!(
            out.contains("&lt;script&gt;"),
            "Expected escaped <script>: {out}"
        );
        assert!(
            !out.contains("<script>"),
            "Must NOT contain raw <script>: {out}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 15: Divider → <hr>
    // -----------------------------------------------------------------------

    #[test]
    fn test_divider_hr() {
        let out = emit(r#"component X { Column { Divider { } } }"#);
        assert!(out.contains("<hr>"), "Expected <hr>: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 16: Minimal CSS reset is emitted when no CSS provided
    // -----------------------------------------------------------------------

    #[test]
    fn test_minimal_reset_emitted() {
        let out = emit(r#"component X { Box { } }"#);
        assert!(out.contains("box-sizing: border-box"), "Expected reset CSS: {out}");
    }

    // -----------------------------------------------------------------------
    // Test 17: Slot node child renders placeholder div
    // -----------------------------------------------------------------------

    #[test]
    fn test_slot_node_placeholder() {
        let out = emit(r#"component X { slot header: node; Column { @header; } }"#);
        assert!(
            out.contains("mos-slot"),
            "Expected mos-slot placeholder: {out}"
        );
        assert!(
            out.contains("data-slot=\"header\""),
            "Expected data-slot attr: {out}"
        );
    }
}
