//! # mosaic-dev — library half
//!
//! The CLI binary lives in `main.rs`; this library carries the
//! *backend-agnostic* logic that has no runtime side-effects, so the unit
//! tests can drive it without spawning Vite, qmlscene, or the Swift
//! toolchain.
//!
//! ## What lives here
//!
//! * [`Backend`] — enum mirroring the user-facing `--backend` flag
//! * [`DummyProps`] — JSON-shaped dummy values derived from a
//!   [`mosmodel_compiler::MosmodelComponent`]
//! * [`wrappers`] — one module per backend that turns a component name
//!   plus its dummy props into a host-wrapper source string
//!
//! ## What lives in `main.rs`
//!
//! * CLI parsing (clap)
//! * `mosaic_package_artifact_builder::build_package` invocation
//! * Spawning child processes (Vite / swift / qmlscene)
//! * File-system watching via `notify`
//! * The tiny HTTP server (HTML / WebComponent)
//!
//! ## Why split it this way?
//!
//! The host-wrapper string is the only thing the user *sees* per backend —
//! a typo in `import { Card } from './react/Card'` is a hard-to-debug
//! Vite error, but a one-line unit test catches it before the user ever
//! runs the dev server.  Process spawning is not interesting to test
//! (the OS already tests `Command::spawn`); the *content* of the files
//! we generate is.

use mosmodel_compiler::{MosmodelComponent, SlotDecl, SlotType};
use serde_json::{json, Map, Value};

// ===========================================================================
// Backend selector
// ===========================================================================

/// Which runtime / wrapper format the user picked at the CLI.
///
/// This mirrors the `--backend` flag exactly.  We carry it through to the
/// wrapper generators rather than passing a string so that misspellings are
/// caught at compile time — every match has to handle every variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    React,
    SwiftUI,
    Qt,
    WebComponent,
    Html,
    Xaml,
}

impl Backend {
    /// Parse the CLI string form.  Returns `None` for unrecognised values
    /// so the caller can produce a friendly error message that lists every
    /// valid choice.
    pub fn from_cli(s: &str) -> Option<Backend> {
        match s {
            "react" => Some(Backend::React),
            "swiftui" => Some(Backend::SwiftUI),
            "qt" => Some(Backend::Qt),
            "webcomponent" => Some(Backend::WebComponent),
            "html" => Some(Backend::Html),
            "xaml" => Some(Backend::Xaml),
            _ => None,
        }
    }

    /// Pretty-printed name used in log messages.
    pub fn label(self) -> &'static str {
        match self {
            Backend::React => "react",
            Backend::SwiftUI => "swiftui",
            Backend::Qt => "qt",
            Backend::WebComponent => "webcomponent",
            Backend::Html => "html",
            Backend::Xaml => "xaml",
        }
    }
}

// ===========================================================================
// Dummy-prop generation
// ===========================================================================

/// Per-slot dummy values that a host wrapper can plug into a freshly
/// built component for preview purposes.
///
/// We store them as a `serde_json::Map<String, Value>` — JSON is the
/// lowest-common-denominator representation across all five wrappers,
/// and the React/TSX, HTML, WebComponent and QML emitters all serialise
/// JSON literals to their own surface syntax with minimal fuss.
///
/// For SwiftUI we hand-translate JSON literals to Swift literals so that
/// `0` becomes `Int(0)` (not `Double(0.0)`) and string slots get string
/// literals with escapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DummyProps {
    /// kebab-case slot name → JSON dummy value.
    pub values: Map<String, Value>,
}

impl DummyProps {
    /// Derive dummy props from a parsed mosmodel component.
    ///
    /// Rules — chosen to produce "obviously placeholder" values that a
    /// human eyeballing the preview can recognise immediately:
    ///
    /// | mosmodel slot type   | dummy value                              |
    /// |----------------------|------------------------------------------|
    /// | `text`               | the slot name itself, as a string        |
    /// | `one-of ...`         | the first declared value, as a string   |
    /// | `number`             | `0`                                      |
    /// | `bool`               | `false`                                  |
    /// | `image`              | `""` (empty URL string)                  |
    /// | `color`              | `"#cccccc"` (a neutral grey)             |
    /// | `node`               | `null`                                   |
    /// | `list<T>`            | `[]` (empty list)                        |
    /// | `Component(_)`       | `null`                                   |
    ///
    /// Slots that have an explicit default in the `.mil` file inherit the
    /// default value verbatim, overriding the rule above. This matches
    /// what a real user would see when they don't pass the prop.
    pub fn from_component(component: &MosmodelComponent) -> Self {
        let mut values = Map::new();
        for slot in &component.slots {
            values.insert(slot.name.clone(), dummy_for_slot(slot));
        }
        DummyProps { values }
    }

    /// Iterate over the (slot_name, json_value) pairs in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.values.iter()
    }

    /// `true` if no slots were declared.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Pick a dummy value for one slot.  Honours explicit `.mil` defaults
/// first; otherwise applies the table in [`DummyProps::from_component`].
fn dummy_for_slot(slot: &SlotDecl) -> Value {
    // Defaults declared in source win — they are authored, intentional
    // values and the preview should reflect what the host would see when
    // it omits the prop.
    if let Some(default) = &slot.default {
        return match default {
            mosmodel_compiler::SlotDefault::Text(s) => Value::String(s.clone()),
            mosmodel_compiler::SlotDefault::Number(n) => json!(n),
            mosmodel_compiler::SlotDefault::Bool(b) => Value::Bool(*b),
        };
    }

    match &slot.r#type {
        SlotType::Text => Value::String(slot.name.clone()),
        // Closed-set previews need a legal member, not the slot name.
        SlotType::OneOf(values) => Value::String(values.first().cloned().unwrap_or_default()),
        SlotType::Number => json!(0),
        SlotType::Bool => Value::Bool(false),
        SlotType::Image => Value::String(String::new()),
        SlotType::Color => Value::String("#cccccc".to_string()),
        SlotType::Node => Value::Null,
        SlotType::List(_) => Value::Array(Vec::new()),
        SlotType::Component(_) => Value::Null,
    }
}

// ===========================================================================
// Wrapper generation — one module per backend
// ===========================================================================

pub mod wrappers {
    //! Each function in here takes a component name + dummy props and
    //! returns the *source text* for a host wrapper.  No I/O; the caller
    //! decides where (or whether) to write it.
    //!
    //! Wrappers are *intentionally* simple — they're not meant to be
    //! production hosts.  Their only job is to instantiate the component
    //! with plausible-looking values so the author can eyeball the
    //! rendering pipeline end-to-end.

    use super::*;

    // -----------------------------------------------------------------------
    // React: index.html + main.tsx
    // -----------------------------------------------------------------------

    /// `index.html` for Vite — loads `main.tsx` as a module.
    ///
    /// We pin the meta-charset and viewport so the preview is readable on
    /// a hi-DPI screen without the user editing anything.
    pub fn react_index_html(component: &str) -> String {
        format!(
            "<!doctype html>\n\
             <html>\n\
             <head>\n\
             <meta charset=\"utf-8\"/>\n\
             <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/>\n\
             <title>mosaic-dev: {component}</title>\n\
             </head>\n\
             <body>\n\
             <div id=\"root\"></div>\n\
             <script type=\"module\" src=\"/main.tsx\"></script>\n\
             </body>\n\
             </html>\n"
        )
    }

    /// `main.tsx` — imports the freshly built React component out of the
    /// `react/` artifact directory and renders it under `#root`.
    ///
    /// We pass `dispatch` as a console.log so the author can see emits
    /// fire in DevTools.  All slot values come from `dummy` as a JSON
    /// literal spread.
    pub fn react_main_tsx(component: &str, dummy: &DummyProps) -> String {
        // `serde_json::to_string_pretty` produces valid JSON which is *also*
        // valid as a JS/TS object literal for the keys/values we emit here
        // (no `undefined`, no functions, no `NaN`). That lets the TSX file
        // drop the JSON straight into the source.
        let dummy_json = serde_json::to_string_pretty(&dummy.values)
            .unwrap_or_else(|_| "{}".to_string());

        format!(
            "import React from 'react';\n\
             import {{ createRoot }} from 'react-dom/client';\n\
             import {{ {component} }} from './react/{component}';\n\
             \n\
             const dummyProps = {dummy_json};\n\
             \n\
             function App() {{\n\
             \u{20}\u{20}return (\n\
             \u{20}\u{20}\u{20}\u{20}<{component}\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}{{...dummyProps}}\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}dispatch={{(e: unknown) => console.log('emit', e)}}\n\
             \u{20}\u{20}\u{20}\u{20}/>\n\
             \u{20}\u{20});\n\
             }}\n\
             \n\
             const root = createRoot(document.getElementById('root')!);\n\
             root.render(<App />);\n"
        )
    }

    // -----------------------------------------------------------------------
    // HTML
    // -----------------------------------------------------------------------

    /// A standalone HTML page that inlines the built static HTML snippet.
    ///
    /// The HTML backend's `build_package` output is a `.html` body fragment
    /// per component; we wrap it in a full document with a banner so the
    /// preview is recognisably a dev environment, not the production
    /// markup.
    pub fn html_index(component: &str, component_html: &str) -> String {
        format!(
            "<!doctype html>\n\
             <html>\n\
             <head>\n\
             <meta charset=\"utf-8\"/>\n\
             <title>mosaic-dev: {component}</title>\n\
             <style>\n\
             body {{ font-family: system-ui, sans-serif; margin: 0; padding: 16px; }}\n\
             .mosaic-dev-banner {{ background:#222; color:#eee; padding:8px 12px; \
             border-radius:4px; margin-bottom:16px; font-size:13px; }}\n\
             </style>\n\
             </head>\n\
             <body>\n\
             <div class=\"mosaic-dev-banner\">mosaic-dev — html — {component}</div>\n\
             {component_html}\n\
             </body>\n\
             </html>\n"
        )
    }

    // -----------------------------------------------------------------------
    // WebComponent
    // -----------------------------------------------------------------------

    /// A standalone HTML page that loads the auto-registered custom
    /// element via a `<script>` tag and instantiates it.
    ///
    /// Slot values become element attributes — that's the canonical way to
    /// pass scalar props into a custom element.  Lists and other non-string
    /// values are JSON-stringified so the element can `JSON.parse` them
    /// from `getAttribute`.
    pub fn webcomponent_index(component: &str, dummy: &DummyProps) -> String {
        let tag = kebab_case(component);
        let mut attrs = String::new();
        for (name, value) in dummy.iter() {
            let attr_value = match value {
                Value::String(s) => html_escape(s),
                other => html_escape(&other.to_string()),
            };
            attrs.push_str(&format!(" {name}=\"{attr_value}\""));
        }

        format!(
            "<!doctype html>\n\
             <html>\n\
             <head>\n\
             <meta charset=\"utf-8\"/>\n\
             <title>mosaic-dev: {component}</title>\n\
             </head>\n\
             <body>\n\
             <div style=\"font-family:system-ui;padding:8px;background:#222;\
             color:#eee\">mosaic-dev — webcomponent — {component}</div>\n\
             <script type=\"module\" src=\"./webcomponent/{component}.js\"></script>\n\
             <{tag}{attrs}></{tag}>\n\
             </body>\n\
             </html>\n"
        )
    }

    // -----------------------------------------------------------------------
    // SwiftUI
    // -----------------------------------------------------------------------

    /// `Package.swift` for the host SwiftPM project that depends on the
    /// generated package and is executable.
    ///
    /// We hard-code swift-tools-version 5.9 because the SwiftUI emitter
    /// already requires it; bumping versions is a follow-up.
    pub fn swiftui_package_swift(host_name: &str, pkg_path_rel: &str) -> String {
        format!(
            "// swift-tools-version:5.9\n\
             import PackageDescription\n\
             \n\
             let package = Package(\n\
             \u{20}\u{20}\u{20}\u{20}name: \"{host_name}\",\n\
             \u{20}\u{20}\u{20}\u{20}platforms: [.macOS(.v13)],\n\
             \u{20}\u{20}\u{20}\u{20}dependencies: [\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}.package(path: \"{pkg_path_rel}\")\n\
             \u{20}\u{20}\u{20}\u{20}],\n\
             \u{20}\u{20}\u{20}\u{20}targets: [\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}.executableTarget(\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}name: \"{host_name}\"\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20})\n\
             \u{20}\u{20}\u{20}\u{20}]\n\
             )\n"
        )
    }

    /// `main.swift` — instantiates a SwiftUI App with a window containing
    /// the previewed component.
    ///
    /// SwiftUI doesn't have a JSON-spread analogue, so we hand-translate
    /// each slot value to a Swift literal.  Slot names are kebab-case in
    /// `.mil`; SwiftUI initializer args are camelCase, so we translate.
    pub fn swiftui_main_swift(component: &str, dummy: &DummyProps) -> String {
        let mut args = String::new();
        let mut first = true;
        for (name, value) in dummy.iter() {
            if !first {
                args.push_str(", ");
            }
            first = false;
            let camel = kebab_to_camel(name);
            let literal = swift_literal(value);
            args.push_str(&format!("{camel}: {literal}"));
        }

        format!(
            "import SwiftUI\n\
             \n\
             @main\n\
             struct MosaicDevApp: App {{\n\
             \u{20}\u{20}\u{20}\u{20}var body: some Scene {{\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}WindowGroup(\"mosaic-dev: {component}\") {{\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}{component}({args})\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}}}\n\
             \u{20}\u{20}\u{20}\u{20}}}\n\
             }}\n"
        )
    }

    // -----------------------------------------------------------------------
    // Qt / QML
    // -----------------------------------------------------------------------

    /// `main.qml` — imports the generated Qt module and instantiates the
    /// component with literal property assignments.
    ///
    /// QML property names use lowerCamelCase, so we translate slot names
    /// the same way as for SwiftUI.  Strings are quoted, numbers and
    /// booleans are emitted bare, and `null` values are skipped (QML has
    /// no `null` literal for arbitrary property types — the component
    /// will fall back to its declared default).
    pub fn qt_main_qml(component: &str, dummy: &DummyProps) -> String {
        let mut props = String::new();
        for (name, value) in dummy.iter() {
            let qml_value = match qml_literal(value) {
                Some(v) => v,
                None => continue,
            };
            let camel = kebab_to_camel(name);
            props.push_str(&format!(
                "\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}{camel}: {qml_value}\n"
            ));
        }

        format!(
            "import QtQuick 2.15\n\
             import QtQuick.Window 2.15\n\
             \n\
             Window {{\n\
             \u{20}\u{20}\u{20}\u{20}width: 800\n\
             \u{20}\u{20}\u{20}\u{20}height: 600\n\
             \u{20}\u{20}\u{20}\u{20}visible: true\n\
             \u{20}\u{20}\u{20}\u{20}title: \"mosaic-dev: {component}\"\n\
             \n\
             \u{20}\u{20}\u{20}\u{20}{component} {{\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}anchors.fill: parent\n\
             {props}\
             \u{20}\u{20}\u{20}\u{20}}}\n\
             }}\n"
        )
    }

    // -----------------------------------------------------------------------
    // Translation helpers
    // -----------------------------------------------------------------------

    /// `Card` → `card`, `FormulaBar` → `formula-bar`, `XAML` → `x-a-m-l`.
    ///
    /// Custom-element specs require at least one dash, so the lowercased
    /// single-word case (e.g. `card`) is suffixed with `-mosaic` to keep
    /// the browser happy.
    fn kebab_case(pascal: &str) -> String {
        let mut out = String::new();
        for (i, c) in pascal.chars().enumerate() {
            if c.is_ascii_uppercase() && i > 0 {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        }
        if !out.contains('-') {
            out.push_str("-mosaic");
        }
        out
    }

    /// `viewport-rows` → `viewportRows`.
    fn kebab_to_camel(kebab: &str) -> String {
        let mut out = String::new();
        let mut upper_next = false;
        for c in kebab.chars() {
            if c == '-' {
                upper_next = true;
            } else if upper_next {
                out.push(c.to_ascii_uppercase());
                upper_next = false;
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Render a JSON value as a Swift literal.
    ///
    /// - strings → `"escaped"`
    /// - numbers → `Int(_)` if integral, otherwise `Double(_)`
    /// - bools → `true` / `false`
    /// - null → `nil`
    /// - arrays → `[]` (we don't recurse into element literals; that
    ///   would require knowing the element's Swift type, which we'd have
    ///   to plumb through from the slot's `ListInnerType`. Future work.)
    fn swift_literal(value: &Value) -> String {
        match value {
            Value::String(s) => {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{escaped}\"")
            }
            // Swift's compiler is happy to coerce numeric literals to the
            // declared property type, so a single `n.to_string()` covers
            // both integral and fractional cases.
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "nil".to_string(),
            Value::Array(_) => "[]".to_string(),
            Value::Object(_) => "[:]".to_string(),
        }
    }

    /// Render a JSON value as a QML literal, or `None` if QML cannot
    /// represent it as a property assignment.
    fn qml_literal(value: &Value) -> Option<String> {
        match value {
            Value::String(s) => {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                Some(format!("\"{escaped}\""))
            }
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Array(_) => Some("[]".to_string()),
            Value::Null => None,
            Value::Object(_) => None,
        }
    }

    /// Minimal HTML-attribute escaping — enough to keep dummy values out
    /// of attribute injection territory.
    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

// ===========================================================================
// Convenience: derive a component summary from a .mil source string
// ===========================================================================

/// Read `<package_root>/src/<component>.mil`, parse it with mosmodel,
/// and return the typed component.
///
/// We expose this from the library (not `main.rs`) so unit tests can
/// drive it without going through the CLI.
pub fn parse_component_interface(
    package_root: &std::path::Path,
    component: &str,
) -> Result<MosmodelComponent, String> {
    let mil_path = package_root.join("src").join(format!("{component}.mil"));
    let src = std::fs::read_to_string(&mil_path)
        .map_err(|e| format!("cannot read {}: {e}", mil_path.display()))?;
    let out = mosmodel_compiler::compile(&src).map_err(|errs| {
        errs.first()
            .map(|e| format!("{e:?}"))
            .unwrap_or_else(|| "unknown mosmodel error".to_string())
    })?;
    Ok(out.component)
}

// ===========================================================================
// Convenience re-exports
//
// Callers depending only on `mosaic_dev` shouldn't need to add a second
// path-dependency on `mosmodel-compiler` to name the slot-default type
// in test helpers.
// ===========================================================================

pub use mosmodel_compiler::SlotDefault as MosmodelSlotDefault;

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mosmodel_compiler::{EmitDecl, ListInnerType};

    /// Build a synthetic component with one slot per scalar type plus a
    /// list and a couple of emits.  Lets every wrapper test exercise the
    /// full type matrix without re-parsing `.mil` source.
    fn fixture_component() -> MosmodelComponent {
        MosmodelComponent {
            component: "Card".to_string(),
            slots: vec![
                SlotDecl {
                    name: "title".to_string(),
                    r#type: SlotType::Text,
                    required: true,
                    default: None,
                },
                SlotDecl {
                    name: "count".to_string(),
                    r#type: SlotType::Number,
                    required: true,
                    default: None,
                },
                SlotDecl {
                    name: "expanded".to_string(),
                    r#type: SlotType::Bool,
                    required: true,
                    default: None,
                },
                SlotDecl {
                    name: "tags".to_string(),
                    r#type: SlotType::List(Box::new(ListInnerType::Text)),
                    required: true,
                    default: None,
                },
                SlotDecl {
                    name: "thumbnail".to_string(),
                    r#type: SlotType::Image,
                    required: true,
                    default: None,
                },
                SlotDecl {
                    name: "accent".to_string(),
                    r#type: SlotType::Color,
                    required: true,
                    default: None,
                },
            ],
            emits: vec![EmitDecl {
                name: "onClick".to_string(),
                params: vec![],
            }],
        }
    }

    // ---- DummyProps ------------------------------------------------------

    #[test]
    fn dummy_props_assigns_one_value_per_slot() {
        let dummy = DummyProps::from_component(&fixture_component());
        assert_eq!(dummy.values.len(), 6);
        // Names preserved verbatim from the .mil declarations.
        assert!(dummy.values.contains_key("title"));
        assert!(dummy.values.contains_key("tags"));
    }

    #[test]
    fn dummy_props_picks_sensible_defaults_per_type() {
        let dummy = DummyProps::from_component(&fixture_component());
        assert_eq!(dummy.values["title"], Value::String("title".to_string()));
        assert_eq!(dummy.values["count"], json!(0));
        assert_eq!(dummy.values["expanded"], Value::Bool(false));
        assert_eq!(dummy.values["tags"], Value::Array(vec![]));
        assert_eq!(dummy.values["accent"], Value::String("#cccccc".to_string()));
    }

    #[test]
    fn dummy_props_honours_explicit_mil_defaults() {
        let mut c = fixture_component();
        c.slots[0].default = Some(MosmodelSlotDefault::Text("Hello".to_string()));
        let dummy = DummyProps::from_component(&c);
        assert_eq!(dummy.values["title"], Value::String("Hello".to_string()));
    }

    #[test]
    fn one_of_preview_uses_first_declared_member() {
        let output = mosmodel_compiler::compile(
            "component Button {\n\
             slot variant : one-of secondary primary danger ;\n\
             slot size : one-of compact ;\n\
             }\n",
        )
        .expect("one-of interface compiles");
        let dummy = DummyProps::from_component(&output.component);
        assert_eq!(dummy.values["variant"], json!("secondary"));
        assert_eq!(dummy.values["size"], json!("compact"));

        let html = wrappers::webcomponent_index("Button", &dummy);
        assert!(html.contains(" variant=\"secondary\""));
        assert!(html.contains(" size=\"compact\""));
    }

    // ---- React wrapper ---------------------------------------------------

    #[test]
    fn react_wrapper_generates_valid_tsx() {
        let dummy = DummyProps::from_component(&fixture_component());
        let tsx = wrappers::react_main_tsx("Card", &dummy);

        // Must import the freshly built component out of `./react/`.
        assert!(tsx.contains("from './react/Card'"));
        // Must spread dummy props.
        assert!(tsx.contains("{...dummyProps}"));
        // Dummy values inlined as a JSON literal.
        assert!(tsx.contains("\"title\""));
        // Renders into the root div.
        assert!(tsx.contains("createRoot"));
        assert!(tsx.contains("getElementById('root')"));
    }

    #[test]
    fn react_index_html_loads_the_module() {
        let html = wrappers::react_index_html("Card");
        assert!(html.contains("<div id=\"root\""));
        assert!(html.contains("src=\"/main.tsx\""));
        assert!(html.contains("mosaic-dev: Card"));
    }

    // ---- HTML wrapper ----------------------------------------------------

    #[test]
    fn html_wrapper_wraps_built_snippet() {
        let body = wrappers::html_index("Card", "<div>built body</div>");
        assert!(body.contains("<!doctype html>"));
        assert!(body.contains("mosaic-dev — html — Card"));
        assert!(body.contains("<div>built body</div>"));
    }

    // ---- WebComponent wrapper -------------------------------------------

    #[test]
    fn webcomponent_wrapper_uses_kebab_case_tag_and_attrs() {
        let dummy = DummyProps::from_component(&fixture_component());
        let html = wrappers::webcomponent_index("FormulaBar", &dummy);
        // Tag is kebab-case.
        assert!(html.contains("<formula-bar"));
        assert!(html.contains("</formula-bar>"));
        // Loads the auto-registered JS bundle.
        assert!(html.contains("./webcomponent/FormulaBar.js"));
        // String attrs land verbatim.
        assert!(html.contains(" title=\"title\""));
        // Numbers serialize without quotes.
        assert!(html.contains(" count=\"0\""));
    }

    #[test]
    fn webcomponent_wrapper_handles_single_word_components() {
        let dummy = DummyProps::from_component(&fixture_component());
        let html = wrappers::webcomponent_index("Card", &dummy);
        // Single-word names need an injected dash to satisfy the
        // custom-element spec — verify the workaround fires.
        assert!(html.contains("<card-mosaic"));
    }

    // ---- SwiftUI wrapper -------------------------------------------------

    #[test]
    fn swiftui_wrapper_generates_valid_swift() {
        let dummy = DummyProps::from_component(&fixture_component());
        let swift = wrappers::swiftui_main_swift("Card", &dummy);
        assert!(swift.contains("@main"));
        assert!(swift.contains("import SwiftUI"));
        assert!(swift.contains("struct MosaicDevApp: App"));
        assert!(swift.contains("Card("));
        // Slot args translated to camelCase.
        // (`thumbnail` and `accent` are already camelCase; check the
        // multiword case explicitly by injecting one.)
        let mut c = fixture_component();
        c.slots.push(SlotDecl {
            name: "edit-content".to_string(),
            r#type: SlotType::Text,
            required: true,
            default: None,
        });
        let d2 = DummyProps::from_component(&c);
        let s2 = wrappers::swiftui_main_swift("Card", &d2);
        assert!(s2.contains("editContent:"));
    }

    #[test]
    fn swiftui_package_swift_is_executable() {
        let pkg = wrappers::swiftui_package_swift("Host", "../swiftui");
        assert!(pkg.contains(".executableTarget"));
        assert!(pkg.contains(".package(path: \"../swiftui\")"));
        assert!(pkg.contains("swift-tools-version:5.9"));
    }

    // ---- Qt wrapper ------------------------------------------------------

    #[test]
    fn qt_wrapper_generates_valid_qml() {
        let dummy = DummyProps::from_component(&fixture_component());
        let qml = wrappers::qt_main_qml("Card", &dummy);
        assert!(qml.contains("import QtQuick"));
        assert!(qml.contains("Window {"));
        assert!(qml.contains("Card {"));
        // Strings quoted.
        assert!(qml.contains("title: \"title\""));
        // Numbers bare.
        assert!(qml.contains("count: 0"));
        // Bools bare.
        assert!(qml.contains("expanded: false"));
        // Multiword slot names translated to lowerCamelCase.
        let mut c = fixture_component();
        c.slots.push(SlotDecl {
            name: "edit-content".to_string(),
            r#type: SlotType::Text,
            required: true,
            default: None,
        });
        let d2 = DummyProps::from_component(&c);
        let q2 = wrappers::qt_main_qml("Card", &d2);
        assert!(q2.contains("editContent:"));
    }

    // ---- Backend parsing ------------------------------------------------

    #[test]
    fn backend_parsing_round_trips_every_variant() {
        for s in ["react", "swiftui", "qt", "webcomponent", "html", "xaml"] {
            let b = Backend::from_cli(s).unwrap_or_else(|| panic!("parse {s}"));
            assert_eq!(b.label(), s);
        }
        assert!(Backend::from_cli("nonexistent").is_none());
    }
}
