use binary_tree::{BinaryTree, BinaryTreeNode};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmBinaryTree {
    inner: BinaryTree<i32>,
}

#[wasm_bindgen]
impl WasmBinaryTree {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: BinaryTree::new(),
        }
    }

    #[wasm_bindgen(js_name = "withRoot")]
    pub fn with_root(value: i32) -> Self {
        Self {
            inner: BinaryTree::with_root(Some(BinaryTreeNode::new(value))),
        }
    }

    #[wasm_bindgen(js_name = "fromLevelOrder")]
    pub fn from_level_order(values: Vec<JsValue>) -> Self {
        let values: Vec<Option<i32>> = values
            .into_iter()
            .map(|value| {
                if value.is_null() || value.is_undefined() {
                    None
                } else {
                    Some(
                        value
                            .as_f64()
                            .expect("binary-tree level order values must be numbers")
                            as i32,
                    )
                }
            })
            .collect();
        Self {
            inner: BinaryTree::from_level_order(values),
        }
    }

    #[wasm_bindgen(js_name = "rootValue")]
    pub fn root_value(&self) -> Option<i32> {
        self.inner.root().map(|node| node.value)
    }

    #[wasm_bindgen(js_name = "isFull")]
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    #[wasm_bindgen(js_name = "isComplete")]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    #[wasm_bindgen(js_name = "isPerfect")]
    pub fn is_perfect(&self) -> bool {
        self.inner.is_perfect()
    }

    pub fn height(&self) -> isize {
        self.inner.height()
    }

    pub fn size(&self) -> usize {
        self.inner.size()
    }

    #[wasm_bindgen(js_name = "leftChild")]
    pub fn left_child(&self, value: i32) -> Option<i32> {
        self.inner.left_child(&value).map(|node| node.value)
    }

    #[wasm_bindgen(js_name = "rightChild")]
    pub fn right_child(&self, value: i32) -> Option<i32> {
        self.inner.right_child(&value).map(|node| node.value)
    }

    pub fn inorder(&self) -> Vec<i32> {
        self.inner.inorder()
    }

    pub fn preorder(&self) -> Vec<i32> {
        self.inner.preorder()
    }

    pub fn postorder(&self) -> Vec<i32> {
        self.inner.postorder()
    }

    #[wasm_bindgen(js_name = "levelOrder")]
    pub fn level_order(&self) -> Vec<i32> {
        self.inner.level_order()
    }

    #[wasm_bindgen(js_name = "toArray")]
    pub fn to_array(&self) -> Vec<JsValue> {
        self.inner
            .to_array()
            .into_iter()
            .map(|value| value.map_or(JsValue::NULL, |value| JsValue::from_f64(value as f64)))
            .collect()
    }

    #[wasm_bindgen(js_name = "toAscii")]
    pub fn to_ascii(&self) -> String {
        self.inner.to_ascii()
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        self.inner.to_ascii()
    }
}

impl Default for WasmBinaryTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_binary_tree_wraps_core_operations() {
        let tree = WasmBinaryTree {
            inner: BinaryTree::from_level_order(vec![Some(1), Some(2), Some(3), None, Some(4)]),
        };
        assert_eq!(tree.root_value(), Some(1));
        assert_eq!(tree.height(), 2);
        assert_eq!(tree.size(), 4);
        assert_eq!(tree.left_child(1), Some(2));
        assert_eq!(tree.right_child(1), Some(3));
    }

    #[test]
    fn wasm_binary_tree_exposes_traversals_and_rendering() {
        let tree = WasmBinaryTree::with_root(10);
        assert_eq!(tree.preorder(), vec![10]);
        assert!(tree.to_ascii().contains("10"));
        assert_eq!(tree.inner.to_array(), vec![Some(10)]);
        assert!(tree.is_full());
        assert!(tree.is_complete());
        assert!(tree.is_perfect());
    }
}
