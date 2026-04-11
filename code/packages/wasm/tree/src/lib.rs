use std::fmt::Display;

use tree::Tree;
use wasm_bindgen::prelude::*;

fn to_js_error(error: impl Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
pub struct WasmTree {
    inner: Tree,
}

#[wasm_bindgen]
impl WasmTree {
    #[wasm_bindgen(constructor)]
    pub fn new(root: &str) -> Self {
        Self {
            inner: Tree::new(root),
        }
    }

    #[wasm_bindgen(js_name = "addChild")]
    pub fn add_child(&mut self, parent: &str, child: &str) -> Result<(), JsValue> {
        self.inner.add_child(parent, child).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "removeSubtree")]
    pub fn remove_subtree(&mut self, node: &str) -> Result<(), JsValue> {
        self.inner.remove_subtree(node).map_err(to_js_error)
    }

    pub fn root(&self) -> String {
        self.inner.root().to_string()
    }

    pub fn parent(&self, node: &str) -> Result<Option<String>, JsValue> {
        self.inner.parent(node).map_err(to_js_error)
    }

    pub fn children(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.children(node).map_err(to_js_error)
    }

    pub fn siblings(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.siblings(node).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "isLeaf")]
    pub fn is_leaf(&self, node: &str) -> Result<bool, JsValue> {
        self.inner.is_leaf(node).map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "isRoot")]
    pub fn is_root(&self, node: &str) -> Result<bool, JsValue> {
        self.inner.is_root(node).map_err(to_js_error)
    }

    pub fn depth(&self, node: &str) -> Result<usize, JsValue> {
        self.inner.depth(node).map_err(to_js_error)
    }

    pub fn height(&self) -> usize {
        self.inner.height()
    }

    pub fn size(&self) -> usize {
        self.inner.size()
    }

    pub fn nodes(&self) -> Vec<String> {
        self.inner.nodes()
    }

    pub fn leaves(&self) -> Vec<String> {
        self.inner.leaves()
    }

    #[wasm_bindgen(js_name = "hasNode")]
    pub fn has_node(&self, node: &str) -> bool {
        self.inner.has_node(node)
    }

    pub fn preorder(&self) -> Vec<String> {
        self.inner.preorder()
    }

    pub fn postorder(&self) -> Vec<String> {
        self.inner.postorder()
    }

    #[wasm_bindgen(js_name = "levelOrder")]
    pub fn level_order(&self) -> Vec<String> {
        self.inner.level_order()
    }

    #[wasm_bindgen(js_name = "pathTo")]
    pub fn path_to(&self, node: &str) -> Result<Vec<String>, JsValue> {
        self.inner.path_to(node).map_err(to_js_error)
    }

    pub fn lca(&self, a: &str, b: &str) -> Result<String, JsValue> {
        self.inner.lca(a, b).map_err(to_js_error)
    }

    pub fn subtree(&self, node: &str) -> Result<Self, JsValue> {
        self.inner
            .subtree(node)
            .map(|inner| Self { inner })
            .map_err(to_js_error)
    }

    #[wasm_bindgen(js_name = "toAscii")]
    pub fn to_ascii(&self) -> String {
        self.inner.to_ascii()
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        self.inner.to_string()
    }
}

impl Default for WasmTree {
    fn default() -> Self {
        Self::new("root")
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    fn make_tree() -> WasmTree {
        let mut tree = WasmTree::new("Program");
        tree.add_child("Program", "Assignment").unwrap();
        tree.add_child("Program", "Print").unwrap();
        tree.add_child("Assignment", "Name").unwrap();
        tree.add_child("Assignment", "BinaryOp").unwrap();
        tree
    }

    #[test]
    fn wasm_tree_wraps_core_operations() {
        let tree = make_tree();
        assert_eq!(tree.root(), "Program");
        assert_eq!(tree.size(), 5);
        assert_eq!(tree.height(), 2);
        assert_eq!(
            tree.parent("Assignment").unwrap(),
            Some("Program".to_string())
        );
        assert!(tree.has_node("BinaryOp"));
    }

    #[test]
    fn wasm_tree_exposes_traversals_and_subtrees() {
        let tree = make_tree();
        assert_eq!(
            tree.preorder(),
            vec!["Program", "Assignment", "BinaryOp", "Name", "Print"]
        );
        assert_eq!(tree.leaves(), vec!["BinaryOp", "Name", "Print"]);
        assert_eq!(
            tree.path_to("BinaryOp").unwrap(),
            vec!["Program", "Assignment", "BinaryOp"]
        );
        assert_eq!(tree.lca("BinaryOp", "Name").unwrap(), "Assignment");
        assert!(tree.to_ascii().contains("Program"));

        let subtree = tree.subtree("Assignment").unwrap();
        assert_eq!(subtree.root(), "Assignment");
        assert_eq!(subtree.size(), 3);
    }
}
