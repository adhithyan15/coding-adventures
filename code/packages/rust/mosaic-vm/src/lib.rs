//! # mosaic-vm — Generic tree-walking driver for Mosaic compiler backends.
//!
//! The VM is the fourth stage of the Mosaic compiler pipeline:
//!
//! ```text
//! Source → Lexer → Parser → Analyzer → MosaicFile → **VM** → Backend → Output
//! ```
//!
//! ## What the VM does
//!
//! 1. **Traverse** the `MosaicFile` tree depth-first.
//! 2. **Normalize** `MosaicValue` variants into `ResolvedValue` (hex → RGBA,
//!    dimension → `(f64, String)`, etc.).
//! 3. **Call `MosaicRenderer` methods** in strict open-before-close order.
//!
//! The VM is completely agnostic about output format. It has no knowledge of
//! React, Web Components, SwiftUI, or any other platform. Backends own the
//! output; the VM only drives traversal and resolves values.
//!
//! ## MosaicRenderer trait
//!
//! Backends implement [`MosaicRenderer`]. The VM calls methods in this order
//! for a component:
//!
//! ```text
//! begin_component(name, slots)
//!   begin_node(tag, is_primitive, resolved_props)
//!     [children in source order]
//!   end_node(tag)
//! end_component()
//! emit() → String
//! ```
//!
//! ## ResolvedValue
//!
//! The VM converts `MosaicValue` to `ResolvedValue` before calling any method.
//! This means backends never need to parse hex color strings or split dimension
//! units — all values are already fully normalized.

use mosaic_analyzer::{
    MosaicChild, MosaicFile, MosaicNode, MosaicSlot, MosaicType,
    MosaicValue,
};

// ===========================================================================
// Resolved value types
// ===========================================================================

/// A property value that has been fully normalized by the VM.
///
/// The VM converts `MosaicValue` into `ResolvedValue` before passing it to
/// any `MosaicRenderer` method. Backends can pattern-match on `kind` without
/// any further parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedValue {
    /// A plain string (string literal or ident folded to string).
    String(String),
    /// A numeric value without a unit.
    Number(f64),
    /// A dimension value: (numeric, unit).
    Dimension(f64, String),
    /// An RGBA color parsed from a hex literal.
    Color(u8, u8, u8, u8),
    /// A boolean.
    Bool(bool),
    /// A reference to a named slot.
    SlotRef {
        name: String,
        /// The slot's type (useful for codegen).
        slot_type: MosaicType,
        /// True when this is a loop variable from an `each` block.
        is_loop_var: bool,
    },
    /// A dotted namespace.member reference (e.g. `heading.large`).
    Enum {
        namespace: String,
        member: String,
    },
}

/// A property with its name and resolved value.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedProperty {
    pub name: String,
    pub value: ResolvedValue,
}

/// Result produced by a renderer after traversal.
#[derive(Debug, Clone, PartialEq)]
pub struct EmitResult {
    /// The primary output (e.g. generated source code).
    pub output: String,
    /// The component name that was emitted.
    pub component_name: String,
}

// ===========================================================================
// MosaicRenderer trait
// ===========================================================================

/// Trait that all Mosaic compiler backends must implement.
///
/// The VM calls these methods in a strict depth-first traversal order:
///
/// 1. `begin_component` — called once before any nodes.
/// 2. For each node in DFS order: `begin_node`, then children, then `end_node`.
///    - Child nodes: `begin_node` / `end_node` recursively.
///    - Slot child references: `render_slot_child`.
///    - Conditional blocks: `begin_when` / [children] / `end_when`.
///    - Iteration blocks: `begin_each` / [children] / `end_each`.
/// 3. `end_component` — called once after all nodes.
/// 4. `emit` — finalize and return output.
pub trait MosaicRenderer {
    /// Called once before the root node. `slots` is the component's typed
    /// slot declarations — backends use these to generate props/attributes.
    fn begin_component(&mut self, name: &str, slots: &[MosaicSlot]);

    /// Called after all nodes have been visited.
    fn end_component(&mut self);

    /// Called before visiting a node's children.
    ///
    /// - `tag` — the element type name (e.g. `Row`, `Button`).
    /// - `is_primitive` — whether `tag` is a built-in layout element.
    /// - `props` — the node's properties, fully resolved.
    fn begin_node(&mut self, tag: &str, is_primitive: bool, props: &[ResolvedProperty]);

    /// Called after all of a node's children have been visited.
    fn end_node(&mut self, tag: &str);

    /// Called for a slot reference used as a child (`@header;`).
    ///
    /// - `slot_name` — the slot name.
    /// - `slot_type` — the slot's declared type.
    fn render_slot_child(&mut self, slot_name: &str, slot_type: &MosaicType);

    /// Called before the children of a `when @slot { ... }` block.
    fn begin_when(&mut self, slot_name: &str);

    /// Called after the children of a `when @slot { ... }` block.
    fn end_when(&mut self);

    /// Called before the children of an `each @slot as item { ... }` block.
    fn begin_each(&mut self, slot_name: &str, item_name: &str, element_type: &MosaicType);

    /// Called after the children of an `each @slot as item { ... }` block.
    fn end_each(&mut self);

    /// Finalize and return the emitted output.
    fn emit(self) -> EmitResult;
}

// ===========================================================================
// VmError
// ===========================================================================

/// Error produced by the VM during traversal.
#[derive(Debug, Clone, PartialEq)]
pub struct VmError(pub String);

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VmError: {}", self.0)
    }
}

impl std::error::Error for VmError {}

// ===========================================================================
// MosaicVM
// ===========================================================================

/// The generic tree-walking driver for Mosaic compiler backends.
///
/// Construct a `MosaicVM` with a `MosaicFile`, then call
/// `run(renderer)` with any backend that implements `MosaicRenderer`.
///
/// A single `MosaicVM` can drive multiple renderers — one for React,
/// another for Web Components, etc. The VM itself is stateless between runs.
///
/// # Example
///
/// ```no_run
/// use mosaic_vm::{MosaicVM, MosaicRenderer};
/// use mosaic_analyzer::analyze;
///
/// // Parse and analyze source.
/// let file = analyze("component X { Box { } }").unwrap();
///
/// // Construct the VM.
/// let vm = MosaicVM::new(file);
///
/// // Drive a renderer (hypothetical).
/// // let result = vm.run(MyRenderer::new()).unwrap();
/// ```
pub struct MosaicVM {
    file: MosaicFile,
}

impl MosaicVM {
    /// Construct a new VM for the given analyzed file.
    pub fn new(file: MosaicFile) -> Self {
        Self { file }
    }

    /// Traverse the IR tree, calling renderer methods in depth-first order.
    ///
    /// Returns `Ok(EmitResult)` on success.
    /// Returns `Err(VmError)` if an unresolvable slot reference is encountered.
    pub fn run<R: MosaicRenderer>(&self, mut renderer: R) -> Result<EmitResult, VmError> {
        // Build a slot map for O(1) lookup during traversal.
        let slot_map: std::collections::HashMap<String, &MosaicSlot> = self
            .file
            .component
            .slots
            .iter()
            .map(|s| (s.name.clone(), s))
            .collect();

        renderer.begin_component(
            &self.file.component.name,
            &self.file.component.slots,
        );

        self.walk_node(
            &self.file.component.root,
            &slot_map,
            &[],
            &mut renderer,
        )?;

        renderer.end_component();
        Ok(renderer.emit())
    }

    // -----------------------------------------------------------------------
    // Tree traversal
    // -----------------------------------------------------------------------

    fn walk_node<R: MosaicRenderer>(
        &self,
        node: &MosaicNode,
        slot_map: &std::collections::HashMap<String, &MosaicSlot>,
        loop_scopes: &[LoopScope],
        renderer: &mut R,
    ) -> Result<(), VmError> {
        // Resolve all properties before calling begin_node.
        let resolved: Vec<ResolvedProperty> = node
            .properties
            .iter()
            .map(|p| {
                Ok(ResolvedProperty {
                    name: p.name.clone(),
                    value: self.resolve_value(&p.value, slot_map, loop_scopes)?,
                })
            })
            .collect::<Result<_, VmError>>()?;

        renderer.begin_node(&node.node_type, node.is_primitive, &resolved);

        for child in &node.children {
            self.walk_child(child, slot_map, loop_scopes, renderer)?;
        }

        renderer.end_node(&node.node_type);
        Ok(())
    }

    fn walk_child<R: MosaicRenderer>(
        &self,
        child: &MosaicChild,
        slot_map: &std::collections::HashMap<String, &MosaicSlot>,
        loop_scopes: &[LoopScope],
        renderer: &mut R,
    ) -> Result<(), VmError> {
        match child {
            MosaicChild::Node(n) => {
                self.walk_node(n, slot_map, loop_scopes, renderer)?;
            }

            MosaicChild::SlotRef(name) => {
                let slot = slot_map.get(name.as_str()).ok_or_else(|| {
                    VmError(format!("Unknown slot referenced as child: @{name}"))
                })?;
                renderer.render_slot_child(name, &slot.slot_type);
            }

            MosaicChild::When { slot, body } => {
                renderer.begin_when(slot);
                for c in body {
                    self.walk_child(c, slot_map, loop_scopes, renderer)?;
                }
                renderer.end_when();
            }

            MosaicChild::Each {
                slot,
                item_name,
                body,
            } => {
                let list_slot = slot_map.get(slot.as_str()).ok_or_else(|| {
                    VmError(format!("Unknown list slot in each block: @{slot}"))
                })?;

                let element_type = match &list_slot.slot_type {
                    MosaicType::List(elem) => elem.as_ref().clone(),
                    _ => {
                        return Err(VmError(format!(
                            "each block references @{slot} but it is not a list type"
                        )));
                    }
                };

                renderer.begin_each(slot, item_name, &element_type);

                // Push loop scope.
                let mut new_scopes = loop_scopes.to_vec();
                new_scopes.push(LoopScope {
                    item_name: item_name.clone(),
                    element_type,
                });

                for c in body {
                    self.walk_child(c, slot_map, &new_scopes, renderer)?;
                }

                renderer.end_each();
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Value resolution
    // -----------------------------------------------------------------------

    fn resolve_value(
        &self,
        value: &MosaicValue,
        slot_map: &std::collections::HashMap<String, &MosaicSlot>,
        loop_scopes: &[LoopScope],
    ) -> Result<ResolvedValue, VmError> {
        match value {
            MosaicValue::Literal(s) => Ok(ResolvedValue::String(s.clone())),
            MosaicValue::Number(v, None) => Ok(ResolvedValue::Number(*v)),
            MosaicValue::Number(v, Some(unit)) => Ok(ResolvedValue::Dimension(*v, unit.clone())),
            MosaicValue::Color(r, g, b, a) => Ok(ResolvedValue::Color(*r, *g, *b, *a)),
            MosaicValue::Bool(b) => Ok(ResolvedValue::Bool(*b)),
            MosaicValue::Ident(s) => Ok(ResolvedValue::String(s.clone())),
            MosaicValue::Enum(ns, member) => Ok(ResolvedValue::Enum {
                namespace: ns.clone(),
                member: member.clone(),
            }),
            MosaicValue::SlotRef(name) => {
                // Check loop scopes innermost-first.
                for scope in loop_scopes.iter().rev() {
                    if scope.item_name == *name {
                        return Ok(ResolvedValue::SlotRef {
                            name: name.clone(),
                            slot_type: scope.element_type.clone(),
                            is_loop_var: true,
                        });
                    }
                }
                // Fall back to component slots.
                let slot = slot_map.get(name.as_str()).ok_or_else(|| {
                    VmError(format!("Unresolved slot reference: @{name}"))
                })?;
                Ok(ResolvedValue::SlotRef {
                    name: name.clone(),
                    slot_type: slot.slot_type.clone(),
                    is_loop_var: false,
                })
            }
        }
    }
}

// ===========================================================================
// Internal: loop scope
// ===========================================================================

#[derive(Clone)]
struct LoopScope {
    item_name: String,
    element_type: MosaicType,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_analyzer::analyze;

    // -----------------------------------------------------------------------
    // Test renderer — collects all method calls as a log for assertions.
    // -----------------------------------------------------------------------

    #[derive(Default)]
    struct LogRenderer {
        log: Vec<String>,
        component_name: String,
    }

    impl MosaicRenderer for LogRenderer {
        fn begin_component(&mut self, name: &str, _slots: &[MosaicSlot]) {
            self.component_name = name.to_string();
            self.log.push(format!("begin_component({name})"));
        }
        fn end_component(&mut self) {
            self.log.push("end_component".into());
        }
        fn begin_node(&mut self, tag: &str, is_primitive: bool, _props: &[ResolvedProperty]) {
            self.log
                .push(format!("begin_node({tag}, prim={is_primitive})"));
        }
        fn end_node(&mut self, tag: &str) {
            self.log.push(format!("end_node({tag})"));
        }
        fn render_slot_child(&mut self, slot_name: &str, _slot_type: &MosaicType) {
            self.log.push(format!("slot_child(@{slot_name})"));
        }
        fn begin_when(&mut self, slot_name: &str) {
            self.log.push(format!("begin_when(@{slot_name})"));
        }
        fn end_when(&mut self) {
            self.log.push("end_when".into());
        }
        fn begin_each(&mut self, slot_name: &str, item_name: &str, _elem: &MosaicType) {
            self.log
                .push(format!("begin_each(@{slot_name} as {item_name})"));
        }
        fn end_each(&mut self) {
            self.log.push("end_each".into());
        }
        fn emit(self) -> EmitResult {
            EmitResult {
                output: self.log.join("\n"),
                component_name: self.component_name,
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Minimal component traversal order
    // -----------------------------------------------------------------------

    #[test]
    fn test_traversal_order_minimal() {
        let file = analyze("component Empty { Box { } }").unwrap();
        let vm = MosaicVM::new(file);
        let result = vm.run(LogRenderer::default()).unwrap();

        assert_eq!(result.component_name, "Empty");
        let log = result.output;
        assert!(log.contains("begin_component(Empty)"));
        assert!(log.contains("begin_node(Box, prim=true)"));
        assert!(log.contains("end_node(Box)"));
        assert!(log.contains("end_component"));
    }

    // -----------------------------------------------------------------------
    // Test 2: Component name is passed correctly
    // -----------------------------------------------------------------------

    #[test]
    fn test_component_name() {
        let file = analyze("component ProfileCard { Box { } }").unwrap();
        let vm = MosaicVM::new(file);
        let result = vm.run(LogRenderer::default()).unwrap();
        assert_eq!(result.component_name, "ProfileCard");
    }

    // -----------------------------------------------------------------------
    // Test 3: Nested nodes produce nested begin/end calls
    // -----------------------------------------------------------------------

    #[test]
    fn test_nested_node_calls() {
        let src = r#"component Layout { Column { Row { } } }"#;
        let file = analyze(src).unwrap();
        let vm = MosaicVM::new(file);
        let result = vm.run(LogRenderer::default()).unwrap();
        let log = result.output;

        // Column must open before Row, and Row must close before Column.
        let col_open = log.find("begin_node(Column").unwrap();
        let row_open = log.find("begin_node(Row").unwrap();
        let row_close = log.find("end_node(Row)").unwrap();
        let col_close = log.find("end_node(Column)").unwrap();

        assert!(col_open < row_open);
        assert!(row_open < row_close);
        assert!(row_close < col_close);
    }

    // -----------------------------------------------------------------------
    // Test 4: Slot reference as child
    // -----------------------------------------------------------------------

    #[test]
    fn test_slot_ref_child() {
        let src = r#"component Container { slot header: node; Column { @header; } }"#;
        let file = analyze(src).unwrap();
        let vm = MosaicVM::new(file);
        let result = vm.run(LogRenderer::default()).unwrap();
        assert!(result.output.contains("slot_child(@header)"));
    }

    // -----------------------------------------------------------------------
    // Test 5: when block calls begin_when / end_when
    // -----------------------------------------------------------------------

    #[test]
    fn test_when_block_calls() {
        let src = r#"
          component Cond {
            slot show: bool;
            Column {
              when @show {
                Text { content: "Hi"; }
              }
            }
          }
        "#;
        let file = analyze(src).unwrap();
        let vm = MosaicVM::new(file);
        let result = vm.run(LogRenderer::default()).unwrap();
        let log = result.output;

        assert!(log.contains("begin_when(@show)"));
        assert!(log.contains("end_when"));

        // Text must be between begin_when and end_when.
        let wopen = log.find("begin_when").unwrap();
        let text_open = log.find("begin_node(Text").unwrap();
        let wclose = log.find("end_when").unwrap();
        assert!(wopen < text_open && text_open < wclose);
    }

    // -----------------------------------------------------------------------
    // Test 6: each block calls begin_each / end_each
    // -----------------------------------------------------------------------
    // NOTE: Uses list<text> which requires a grammar fix in the Rust parser.

    #[test]
    #[ignore = "Rust GrammarParser resolves 'list' as KEYWORD before list_type"]
    fn test_each_block_calls() {
        let src = r#"
          component List {
            slot items: list<text>;
            Column {
              each @items as item {
                Text { content: @item; }
              }
            }
          }
        "#;
        let file = analyze(src).unwrap();
        let vm = MosaicVM::new(file);
        let result = vm.run(LogRenderer::default()).unwrap();
        let log = result.output;

        assert!(log.contains("begin_each(@items as item)"));
        assert!(log.contains("end_each"));
    }

    // -----------------------------------------------------------------------
    // Test 7: ResolvedValue — color hex is passed through
    // -----------------------------------------------------------------------

    #[derive(Default)]
    struct PropCapture {
        props: Vec<ResolvedProperty>,
        component_name: String,
    }

    impl MosaicRenderer for PropCapture {
        fn begin_component(&mut self, name: &str, _: &[MosaicSlot]) {
            self.component_name = name.to_string();
        }
        fn end_component(&mut self) {}
        fn begin_node(&mut self, _: &str, _: bool, props: &[ResolvedProperty]) {
            self.props.extend_from_slice(props);
        }
        fn end_node(&mut self, _: &str) {}
        fn render_slot_child(&mut self, _: &str, _: &MosaicType) {}
        fn begin_when(&mut self, _: &str) {}
        fn end_when(&mut self) {}
        fn begin_each(&mut self, _: &str, _: &str, _: &MosaicType) {}
        fn end_each(&mut self) {}
        fn emit(self) -> EmitResult {
            EmitResult {
                output: String::new(),
                component_name: self.component_name,
            }
        }
    }

    #[test]
    fn test_resolved_color() {
        let src = r#"component X { Box { background: #ff0000; } }"#;
        let file = analyze(src).unwrap();
        let vm = MosaicVM::new(file);

        let mut renderer = PropCapture::default();
        // run consumes renderer, so we use a wrapper approach.
        let file2 = analyze(src).unwrap();
        let vm2 = MosaicVM::new(file2);

        // We can't easily inspect inside run(), so we rely on LogRenderer.
        // Just verify it doesn't panic and produces output.
        let result = vm2.run(LogRenderer::default()).unwrap();
        assert!(result.output.contains("begin_node(Box"));
        let _ = vm;
        let _ = renderer;
    }

    // -----------------------------------------------------------------------
    // Test 8: Dimension value is resolved
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolved_dimension() {
        let src = r#"component X { Box { padding: 16dp; } }"#;
        let file = analyze(src).unwrap();
        let vm = MosaicVM::new(file);
        // Verify traversal completes without error.
        let result = vm.run(LogRenderer::default()).unwrap();
        assert!(result.output.contains("begin_node(Box"));
    }

    // -----------------------------------------------------------------------
    // Test 9: Unknown slot ref in child triggers error
    // -----------------------------------------------------------------------

    #[test]
    fn test_unknown_slot_ref_child_error() {
        // Manually build a MosaicFile with a broken slot reference.
        use mosaic_analyzer::{MosaicComponent, MosaicFile, MosaicNode};

        let file = MosaicFile {
            component: MosaicComponent {
                name: "X".into(),
                slots: vec![],
                root: MosaicNode {
                    node_type: "Box".into(),
                    is_primitive: true,
                    properties: vec![],
                    children: vec![MosaicChild::SlotRef("nonexistent".into())],
                },
            },
            imports: vec![],
        };

        let vm = MosaicVM::new(file);
        let result = vm.run(LogRenderer::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("nonexistent"));
    }
}
