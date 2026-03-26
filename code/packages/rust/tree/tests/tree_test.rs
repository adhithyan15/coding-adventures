// tree_test.rs -- Comprehensive Tests for the Tree Library
// ========================================================
//
// Organized by category:
//
//  1. Construction -- creating trees, verifying initial state
//  2. add_child -- building trees, error cases
//  3. remove_subtree -- pruning branches, error cases
//  4. Queries -- parent, children, siblings, is_leaf, is_root, depth, height, etc.
//  5. Traversals -- preorder, postorder, level_order
//  6. path_to -- root-to-node paths
//  7. lca -- lowest common ancestor
//  8. subtree -- extracting subtrees
//  9. to_ascii -- ASCII visualization
// 10. Edge cases -- single-node trees, deep chains, wide trees
// 11. graph -- accessing the underlying Graph

use tree::{Tree, TreeError};

// =========================================================================
// Helper: Build a sample tree for many tests
// =========================================================================
//
//         A
//        / \
//       B   C
//      / \   \
//     D   E   F
//    /
//   G

fn make_sample_tree() -> Tree {
    let mut t = Tree::new("A");
    t.add_child("A", "B").unwrap();
    t.add_child("A", "C").unwrap();
    t.add_child("B", "D").unwrap();
    t.add_child("B", "E").unwrap();
    t.add_child("C", "F").unwrap();
    t.add_child("D", "G").unwrap();
    t
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

// =========================================================================
// 1. Construction
// =========================================================================

#[test]
fn test_create_tree_with_root() {
    let t = Tree::new("root");
    assert_eq!(t.root(), "root");
}

#[test]
fn test_new_tree_has_size_one() {
    let t = Tree::new("root");
    assert_eq!(t.size(), 1);
}

#[test]
fn test_new_tree_root_is_leaf() {
    let t = Tree::new("root");
    assert!(t.is_leaf("root").unwrap());
}

#[test]
fn test_new_tree_root_is_root() {
    let t = Tree::new("root");
    assert!(t.is_root("root").unwrap());
}

#[test]
fn test_new_tree_root_has_no_parent() {
    let t = Tree::new("root");
    assert_eq!(t.parent("root").unwrap(), None);
}

#[test]
fn test_new_tree_root_has_no_children() {
    let t = Tree::new("root");
    assert_eq!(t.children("root").unwrap(), Vec::<String>::new());
}

#[test]
fn test_new_tree_root_has_depth_zero() {
    let t = Tree::new("root");
    assert_eq!(t.depth("root").unwrap(), 0);
}

#[test]
fn test_new_tree_height_zero() {
    let t = Tree::new("root");
    assert_eq!(t.height(), 0);
}

#[test]
fn test_new_tree_has_root_in_nodes() {
    let t = Tree::new("root");
    assert!(t.nodes().contains(&"root".to_string()));
}

#[test]
fn test_display() {
    let t = Tree::new("root");
    assert_eq!(format!("{}", t), "Tree(root=\"root\", size=1)");
}

// =========================================================================
// 2. add_child
// =========================================================================

#[test]
fn test_add_one_child() {
    let mut t = Tree::new("root");
    t.add_child("root", "child").unwrap();
    assert_eq!(t.size(), 2);
}

#[test]
fn test_child_has_correct_parent() {
    let mut t = Tree::new("root");
    t.add_child("root", "child").unwrap();
    assert_eq!(t.parent("child").unwrap(), Some("root".to_string()));
}

#[test]
fn test_parent_has_child_in_children_list() {
    let mut t = Tree::new("root");
    t.add_child("root", "child").unwrap();
    assert!(t.children("root").unwrap().contains(&"child".to_string()));
}

#[test]
fn test_add_multiple_children() {
    let mut t = Tree::new("root");
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    t.add_child("root", "C").unwrap();
    assert_eq!(t.children("root").unwrap(), s(&["A", "B", "C"]));
}

#[test]
fn test_add_child_to_non_root() {
    let mut t = Tree::new("root");
    t.add_child("root", "mid").unwrap();
    t.add_child("mid", "leaf").unwrap();
    assert_eq!(t.parent("leaf").unwrap(), Some("mid".to_string()));
}

#[test]
fn test_build_deep_tree() {
    let mut t = Tree::new("level0");
    for i in 1..10 {
        t.add_child(&format!("level{}", i - 1), &format!("level{}", i))
            .unwrap();
    }
    assert_eq!(t.size(), 10);
    assert_eq!(t.depth("level9").unwrap(), 9);
}

#[test]
fn test_add_child_nonexistent_parent() {
    let mut t = Tree::new("root");
    let err = t.add_child("nonexistent", "child").unwrap_err();
    assert_eq!(err, TreeError::NodeNotFound("nonexistent".to_string()));
}

#[test]
fn test_add_duplicate_child() {
    let mut t = Tree::new("root");
    t.add_child("root", "child").unwrap();
    let err = t.add_child("root", "child").unwrap_err();
    assert_eq!(err, TreeError::DuplicateNode("child".to_string()));
}

#[test]
fn test_add_root_as_child() {
    let mut t = Tree::new("root");
    let err = t.add_child("root", "root").unwrap_err();
    assert_eq!(err, TreeError::DuplicateNode("root".to_string()));
}

#[test]
fn test_add_child_makes_parent_not_leaf() {
    let mut t = Tree::new("root");
    assert!(t.is_leaf("root").unwrap());
    t.add_child("root", "child").unwrap();
    assert!(!t.is_leaf("root").unwrap());
}

#[test]
fn test_new_child_is_leaf() {
    let mut t = Tree::new("root");
    t.add_child("root", "child").unwrap();
    assert!(t.is_leaf("child").unwrap());
}

// =========================================================================
// 3. remove_subtree
// =========================================================================

#[test]
fn test_remove_leaf() {
    let mut t = Tree::new("root");
    t.add_child("root", "leaf").unwrap();
    t.remove_subtree("leaf").unwrap();
    assert_eq!(t.size(), 1);
    assert!(!t.has_node("leaf"));
}

#[test]
fn test_remove_subtree_removes_descendants() {
    let mut t = make_sample_tree();
    t.remove_subtree("B").unwrap();
    assert_eq!(t.size(), 3);
    assert!(!t.has_node("B"));
    assert!(!t.has_node("D"));
    assert!(!t.has_node("E"));
    assert!(!t.has_node("G"));
}

#[test]
fn test_remove_subtree_preserves_siblings() {
    let mut t = make_sample_tree();
    t.remove_subtree("B").unwrap();
    assert!(t.has_node("C"));
    assert!(t.has_node("F"));
    assert_eq!(t.children("A").unwrap(), s(&["C"]));
}

#[test]
fn test_remove_deep_subtree() {
    let mut t = make_sample_tree();
    t.remove_subtree("D").unwrap();
    assert_eq!(t.size(), 5);
    assert!(!t.has_node("D"));
    assert!(!t.has_node("G"));
    assert_eq!(t.children("B").unwrap(), s(&["E"]));
}

#[test]
fn test_remove_root_returns_error() {
    let mut t = Tree::new("root");
    let err = t.remove_subtree("root").unwrap_err();
    assert_eq!(err, TreeError::RootRemoval);
}

#[test]
fn test_remove_nonexistent_returns_error() {
    let mut t = Tree::new("root");
    let err = t.remove_subtree("nonexistent").unwrap_err();
    assert_eq!(err, TreeError::NodeNotFound("nonexistent".to_string()));
}

#[test]
fn test_remove_then_readd() {
    let mut t = Tree::new("root");
    t.add_child("root", "child").unwrap();
    t.remove_subtree("child").unwrap();
    t.add_child("root", "child").unwrap();
    assert!(t.has_node("child"));
}

#[test]
fn test_remove_single_child_parent_becomes_leaf() {
    let mut t = Tree::new("root");
    t.add_child("root", "only_child").unwrap();
    t.remove_subtree("only_child").unwrap();
    assert!(t.is_leaf("root").unwrap());
}

// =========================================================================
// 4. Queries
// =========================================================================

#[test]
fn test_parent_of_child() {
    assert_eq!(make_sample_tree().parent("B").unwrap(), Some("A".to_string()));
}

#[test]
fn test_parent_of_grandchild() {
    assert_eq!(
        make_sample_tree().parent("G").unwrap(),
        Some("D".to_string())
    );
}

#[test]
fn test_parent_of_root_is_none() {
    assert_eq!(make_sample_tree().parent("A").unwrap(), None);
}

#[test]
fn test_parent_nonexistent() {
    assert!(make_sample_tree().parent("Z").is_err());
}

#[test]
fn test_children_of_root() {
    assert_eq!(make_sample_tree().children("A").unwrap(), s(&["B", "C"]));
}

#[test]
fn test_children_of_internal() {
    assert_eq!(make_sample_tree().children("B").unwrap(), s(&["D", "E"]));
}

#[test]
fn test_children_of_leaf() {
    assert_eq!(
        make_sample_tree().children("G").unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn test_children_nonexistent() {
    assert!(make_sample_tree().children("Z").is_err());
}

#[test]
fn test_siblings_with_sibling() {
    assert_eq!(make_sample_tree().siblings("B").unwrap(), s(&["C"]));
}

#[test]
fn test_siblings_mutual() {
    assert_eq!(make_sample_tree().siblings("C").unwrap(), s(&["B"]));
}

#[test]
fn test_siblings_only_child() {
    assert_eq!(
        make_sample_tree().siblings("F").unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn test_siblings_root() {
    assert_eq!(
        make_sample_tree().siblings("A").unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn test_siblings_nonexistent() {
    assert!(make_sample_tree().siblings("Z").is_err());
}

#[test]
fn test_siblings_multiple() {
    let mut t = Tree::new("root");
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    t.add_child("root", "C").unwrap();
    t.add_child("root", "D").unwrap();
    assert_eq!(t.siblings("B").unwrap(), s(&["A", "C", "D"]));
}

#[test]
fn test_is_leaf_true() {
    let t = make_sample_tree();
    assert!(t.is_leaf("G").unwrap());
    assert!(t.is_leaf("E").unwrap());
    assert!(t.is_leaf("F").unwrap());
}

#[test]
fn test_is_leaf_false() {
    let t = make_sample_tree();
    assert!(!t.is_leaf("A").unwrap());
    assert!(!t.is_leaf("B").unwrap());
}

#[test]
fn test_is_leaf_nonexistent() {
    assert!(make_sample_tree().is_leaf("Z").is_err());
}

#[test]
fn test_is_root_true() {
    assert!(make_sample_tree().is_root("A").unwrap());
}

#[test]
fn test_is_root_false() {
    assert!(!make_sample_tree().is_root("B").unwrap());
}

#[test]
fn test_is_root_nonexistent() {
    assert!(make_sample_tree().is_root("Z").is_err());
}

#[test]
fn test_depth_root() {
    assert_eq!(make_sample_tree().depth("A").unwrap(), 0);
}

#[test]
fn test_depth_level_one() {
    let t = make_sample_tree();
    assert_eq!(t.depth("B").unwrap(), 1);
    assert_eq!(t.depth("C").unwrap(), 1);
}

#[test]
fn test_depth_level_two() {
    let t = make_sample_tree();
    assert_eq!(t.depth("D").unwrap(), 2);
    assert_eq!(t.depth("E").unwrap(), 2);
    assert_eq!(t.depth("F").unwrap(), 2);
}

#[test]
fn test_depth_level_three() {
    assert_eq!(make_sample_tree().depth("G").unwrap(), 3);
}

#[test]
fn test_depth_nonexistent() {
    assert!(make_sample_tree().depth("Z").is_err());
}

#[test]
fn test_height_sample() {
    assert_eq!(make_sample_tree().height(), 3);
}

#[test]
fn test_height_single_node() {
    assert_eq!(Tree::new("root").height(), 0);
}

#[test]
fn test_height_flat() {
    let mut t = Tree::new("root");
    for i in 0..5 {
        t.add_child("root", &format!("child{}", i)).unwrap();
    }
    assert_eq!(t.height(), 1);
}

#[test]
fn test_height_deep_chain() {
    let mut t = Tree::new("0");
    for i in 1..20 {
        t.add_child(&format!("{}", i - 1), &format!("{}", i))
            .unwrap();
    }
    assert_eq!(t.height(), 19);
}

#[test]
fn test_size_sample() {
    assert_eq!(make_sample_tree().size(), 7);
}

#[test]
fn test_size_after_add() {
    let mut t = Tree::new("root");
    assert_eq!(t.size(), 1);
    t.add_child("root", "A").unwrap();
    assert_eq!(t.size(), 2);
}

#[test]
fn test_nodes_returns_all() {
    assert_eq!(
        make_sample_tree().nodes(),
        s(&["A", "B", "C", "D", "E", "F", "G"])
    );
}

#[test]
fn test_leaves_sample() {
    assert_eq!(make_sample_tree().leaves(), s(&["E", "F", "G"]));
}

#[test]
fn test_leaves_single_node() {
    assert_eq!(Tree::new("root").leaves(), s(&["root"]));
}

#[test]
fn test_leaves_flat() {
    let mut t = Tree::new("root");
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    t.add_child("root", "C").unwrap();
    assert_eq!(t.leaves(), s(&["A", "B", "C"]));
}

#[test]
fn test_has_node_true() {
    assert!(make_sample_tree().has_node("A"));
}

#[test]
fn test_has_node_false() {
    assert!(!make_sample_tree().has_node("Z"));
}

// =========================================================================
// 5. Traversals
// =========================================================================

#[test]
fn test_preorder_sample() {
    assert_eq!(
        make_sample_tree().preorder(),
        s(&["A", "B", "D", "G", "E", "C", "F"])
    );
}

#[test]
fn test_preorder_single_node() {
    assert_eq!(Tree::new("root").preorder(), s(&["root"]));
}

#[test]
fn test_preorder_flat() {
    let mut t = Tree::new("root");
    t.add_child("root", "C").unwrap();
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    assert_eq!(t.preorder(), s(&["root", "A", "B", "C"]));
}

#[test]
fn test_preorder_deep_chain() {
    let mut t = Tree::new("A");
    t.add_child("A", "B").unwrap();
    t.add_child("B", "C").unwrap();
    assert_eq!(t.preorder(), s(&["A", "B", "C"]));
}

#[test]
fn test_postorder_sample() {
    assert_eq!(
        make_sample_tree().postorder(),
        s(&["G", "D", "E", "B", "F", "C", "A"])
    );
}

#[test]
fn test_postorder_single_node() {
    assert_eq!(Tree::new("root").postorder(), s(&["root"]));
}

#[test]
fn test_postorder_flat() {
    let mut t = Tree::new("root");
    t.add_child("root", "C").unwrap();
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    assert_eq!(t.postorder(), s(&["A", "B", "C", "root"]));
}

#[test]
fn test_postorder_deep_chain() {
    let mut t = Tree::new("A");
    t.add_child("A", "B").unwrap();
    t.add_child("B", "C").unwrap();
    assert_eq!(t.postorder(), s(&["C", "B", "A"]));
}

#[test]
fn test_level_order_sample() {
    assert_eq!(
        make_sample_tree().level_order(),
        s(&["A", "B", "C", "D", "E", "F", "G"])
    );
}

#[test]
fn test_level_order_single_node() {
    assert_eq!(Tree::new("root").level_order(), s(&["root"]));
}

#[test]
fn test_level_order_flat() {
    let mut t = Tree::new("root");
    t.add_child("root", "C").unwrap();
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    assert_eq!(t.level_order(), s(&["root", "A", "B", "C"]));
}

#[test]
fn test_level_order_deep_chain() {
    let mut t = Tree::new("A");
    t.add_child("A", "B").unwrap();
    t.add_child("B", "C").unwrap();
    assert_eq!(t.level_order(), s(&["A", "B", "C"]));
}

#[test]
fn test_traversals_same_length() {
    let t = make_sample_tree();
    assert_eq!(t.preorder().len(), 7);
    assert_eq!(t.postorder().len(), 7);
    assert_eq!(t.level_order().len(), 7);
}

#[test]
fn test_traversals_same_elements() {
    let t = make_sample_tree();
    let mut pre = t.preorder();
    let mut post = t.postorder();
    let mut level = t.level_order();
    pre.sort();
    post.sort();
    level.sort();
    assert_eq!(pre, post);
    assert_eq!(pre, level);
}

#[test]
fn test_preorder_root_first() {
    assert_eq!(make_sample_tree().preorder()[0], "A");
}

#[test]
fn test_postorder_root_last() {
    let po = make_sample_tree().postorder();
    assert_eq!(po[po.len() - 1], "A");
}

#[test]
fn test_level_order_root_first() {
    assert_eq!(make_sample_tree().level_order()[0], "A");
}

// =========================================================================
// 6. path_to
// =========================================================================

#[test]
fn test_path_to_root() {
    assert_eq!(make_sample_tree().path_to("A").unwrap(), s(&["A"]));
}

#[test]
fn test_path_to_child() {
    assert_eq!(make_sample_tree().path_to("B").unwrap(), s(&["A", "B"]));
}

#[test]
fn test_path_to_grandchild() {
    assert_eq!(
        make_sample_tree().path_to("D").unwrap(),
        s(&["A", "B", "D"])
    );
}

#[test]
fn test_path_to_deep_node() {
    assert_eq!(
        make_sample_tree().path_to("G").unwrap(),
        s(&["A", "B", "D", "G"])
    );
}

#[test]
fn test_path_to_right_branch() {
    assert_eq!(
        make_sample_tree().path_to("F").unwrap(),
        s(&["A", "C", "F"])
    );
}

#[test]
fn test_path_to_nonexistent() {
    assert!(make_sample_tree().path_to("Z").is_err());
}

#[test]
fn test_path_length_equals_depth_plus_one() {
    let t = make_sample_tree();
    for node in t.nodes() {
        let path = t.path_to(&node).unwrap();
        let d = t.depth(&node).unwrap();
        assert_eq!(path.len(), d + 1);
    }
}

// =========================================================================
// 7. lca
// =========================================================================

#[test]
fn test_lca_same_node() {
    assert_eq!(make_sample_tree().lca("D", "D").unwrap(), "D");
}

#[test]
fn test_lca_siblings() {
    assert_eq!(make_sample_tree().lca("D", "E").unwrap(), "B");
}

#[test]
fn test_lca_parent_child() {
    assert_eq!(make_sample_tree().lca("B", "D").unwrap(), "B");
}

#[test]
fn test_lca_child_parent() {
    assert_eq!(make_sample_tree().lca("D", "B").unwrap(), "B");
}

#[test]
fn test_lca_cousins() {
    assert_eq!(make_sample_tree().lca("D", "F").unwrap(), "A");
}

#[test]
fn test_lca_root_and_leaf() {
    assert_eq!(make_sample_tree().lca("A", "G").unwrap(), "A");
}

#[test]
fn test_lca_deep_nodes() {
    assert_eq!(make_sample_tree().lca("G", "E").unwrap(), "B");
}

#[test]
fn test_lca_leaves_different_subtrees() {
    assert_eq!(make_sample_tree().lca("G", "F").unwrap(), "A");
}

#[test]
fn test_lca_nonexistent_a() {
    assert!(make_sample_tree().lca("Z", "A").is_err());
}

#[test]
fn test_lca_nonexistent_b() {
    assert!(make_sample_tree().lca("A", "Z").is_err());
}

#[test]
fn test_lca_root_with_root() {
    assert_eq!(make_sample_tree().lca("A", "A").unwrap(), "A");
}

// =========================================================================
// 8. subtree
// =========================================================================

#[test]
fn test_subtree_leaf() {
    let sub = make_sample_tree().subtree("G").unwrap();
    assert_eq!(sub.root(), "G");
    assert_eq!(sub.size(), 1);
}

#[test]
fn test_subtree_internal() {
    let sub = make_sample_tree().subtree("B").unwrap();
    assert_eq!(sub.root(), "B");
    assert_eq!(sub.size(), 4);
    assert!(sub.has_node("D"));
    assert!(sub.has_node("E"));
    assert!(sub.has_node("G"));
}

#[test]
fn test_subtree_preserves_structure() {
    let sub = make_sample_tree().subtree("B").unwrap();
    assert_eq!(sub.children("B").unwrap(), s(&["D", "E"]));
    assert_eq!(sub.children("D").unwrap(), s(&["G"]));
    assert!(sub.is_leaf("G").unwrap());
    assert!(sub.is_leaf("E").unwrap());
}

#[test]
fn test_subtree_root() {
    let t = make_sample_tree();
    let sub = t.subtree("A").unwrap();
    assert_eq!(sub.size(), t.size());
    assert_eq!(sub.nodes(), t.nodes());
}

#[test]
fn test_subtree_does_not_modify_original() {
    let t = make_sample_tree();
    let orig_size = t.size();
    let _ = t.subtree("B").unwrap();
    assert_eq!(t.size(), orig_size);
}

#[test]
fn test_subtree_nonexistent() {
    assert!(make_sample_tree().subtree("Z").is_err());
}

#[test]
fn test_subtree_is_independent() {
    let t = make_sample_tree();
    let mut sub = t.subtree("B").unwrap();
    sub.add_child("E", "new_node").unwrap();
    assert!(!t.has_node("new_node"));
}

#[test]
fn test_subtree_right_branch() {
    let sub = make_sample_tree().subtree("C").unwrap();
    assert_eq!(sub.root(), "C");
    assert_eq!(sub.size(), 2);
    assert_eq!(sub.children("C").unwrap(), s(&["F"]));
}

// =========================================================================
// 9. to_ascii
// =========================================================================

#[test]
fn test_ascii_single_node() {
    assert_eq!(Tree::new("root").to_ascii(), "root");
}

#[test]
fn test_ascii_root_with_one_child() {
    let mut t = Tree::new("root");
    t.add_child("root", "child").unwrap();
    assert_eq!(t.to_ascii(), "root\n\u{2514}\u{2500}\u{2500} child");
}

#[test]
fn test_ascii_root_with_two_children() {
    let mut t = Tree::new("root");
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    assert_eq!(
        t.to_ascii(),
        "root\n\u{251C}\u{2500}\u{2500} A\n\u{2514}\u{2500}\u{2500} B"
    );
}

#[test]
fn test_ascii_sample_tree() {
    let expected = "A\n\u{251C}\u{2500}\u{2500} B\n\u{2502}   \u{251C}\u{2500}\u{2500} D\n\u{2502}   \u{2502}   \u{2514}\u{2500}\u{2500} G\n\u{2502}   \u{2514}\u{2500}\u{2500} E\n\u{2514}\u{2500}\u{2500} C\n    \u{2514}\u{2500}\u{2500} F";
    assert_eq!(make_sample_tree().to_ascii(), expected);
}

#[test]
fn test_ascii_deep_chain() {
    let mut t = Tree::new("A");
    t.add_child("A", "B").unwrap();
    t.add_child("B", "C").unwrap();
    assert_eq!(
        t.to_ascii(),
        "A\n\u{2514}\u{2500}\u{2500} B\n    \u{2514}\u{2500}\u{2500} C"
    );
}

#[test]
fn test_ascii_wide_tree() {
    let mut t = Tree::new("root");
    t.add_child("root", "A").unwrap();
    t.add_child("root", "B").unwrap();
    t.add_child("root", "C").unwrap();
    t.add_child("root", "D").unwrap();
    assert_eq!(
        t.to_ascii(),
        "root\n\u{251C}\u{2500}\u{2500} A\n\u{251C}\u{2500}\u{2500} B\n\u{251C}\u{2500}\u{2500} C\n\u{2514}\u{2500}\u{2500} D"
    );
}

// =========================================================================
// 10. Edge Cases
// =========================================================================

#[test]
fn test_single_node_traversals() {
    let t = Tree::new("solo");
    assert_eq!(t.preorder(), s(&["solo"]));
    assert_eq!(t.postorder(), s(&["solo"]));
    assert_eq!(t.level_order(), s(&["solo"]));
}

#[test]
fn test_single_node_leaves() {
    assert_eq!(Tree::new("solo").leaves(), s(&["solo"]));
}

#[test]
fn test_deep_chain_height() {
    let mut t = Tree::new("n0");
    for i in 1..100 {
        t.add_child(&format!("n{}", i - 1), &format!("n{}", i))
            .unwrap();
    }
    assert_eq!(t.height(), 99);
    assert_eq!(t.size(), 100);
}

#[test]
fn test_wide_tree_height() {
    let mut t = Tree::new("root");
    for i in 0..100 {
        t.add_child("root", &format!("child{}", i)).unwrap();
    }
    assert_eq!(t.height(), 1);
    assert_eq!(t.size(), 101);
}

#[test]
fn test_balanced_binary_tree() {
    let mut t = Tree::new("1");
    t.add_child("1", "2").unwrap();
    t.add_child("1", "3").unwrap();
    t.add_child("2", "4").unwrap();
    t.add_child("2", "5").unwrap();
    t.add_child("3", "6").unwrap();
    t.add_child("3", "7").unwrap();
    assert_eq!(t.size(), 7);
    assert_eq!(t.height(), 2);
    assert_eq!(t.leaves(), s(&["4", "5", "6", "7"]));
}

#[test]
fn test_node_names_with_spaces() {
    let mut t = Tree::new("my root");
    t.add_child("my root", "my child").unwrap();
    assert_eq!(
        t.parent("my child").unwrap(),
        Some("my root".to_string())
    );
}

#[test]
fn test_node_names_with_special_chars() {
    let mut t = Tree::new("root:main");
    t.add_child("root:main", "child.1").unwrap();
    assert!(t.has_node("child.1"));
}

#[test]
fn test_path_to_single_node() {
    assert_eq!(Tree::new("solo").path_to("solo").unwrap(), s(&["solo"]));
}

#[test]
fn test_lca_single_node() {
    assert_eq!(Tree::new("solo").lca("solo", "solo").unwrap(), "solo");
}

#[test]
fn test_subtree_single_node() {
    let sub = Tree::new("solo").subtree("solo").unwrap();
    assert_eq!(sub.root(), "solo");
    assert_eq!(sub.size(), 1);
}

#[test]
fn test_remove_and_rebuild() {
    let mut t = Tree::new("root");
    t.add_child("root", "A").unwrap();
    t.add_child("A", "B").unwrap();
    t.remove_subtree("A").unwrap();
    t.add_child("root", "A").unwrap();
    t.add_child("A", "C").unwrap();
    assert_eq!(t.children("A").unwrap(), s(&["C"]));
    assert!(!t.has_node("B"));
}

// =========================================================================
// 11. graph
// =========================================================================

#[test]
fn test_graph_has_correct_nodes() {
    let t = make_sample_tree();
    let mut nodes = t.graph().nodes();
    nodes.sort();
    assert_eq!(nodes, s(&["A", "B", "C", "D", "E", "F", "G"]));
}

#[test]
fn test_graph_edge_count() {
    assert_eq!(make_sample_tree().graph().edges().len(), 6);
}

#[test]
fn test_graph_has_no_cycles() {
    assert!(!make_sample_tree().graph().has_cycle());
}

#[test]
fn test_graph_topological_sort_starts_with_root() {
    let topo = make_sample_tree().graph().topological_sort().unwrap();
    assert_eq!(topo[0], "A");
}
