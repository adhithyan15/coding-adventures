use lsm_tree::LSMTree;


#[test]
fn test_put_and_get() {
    let dir = tempfile::tempdir().unwrap();
    let mut tree: LSMTree<String, String> = LSMTree::new(dir.path()).unwrap();

    tree.put("alice".to_string(), "wonderland".to_string()).unwrap();
    tree.put("bob".to_string(), "builder".to_string()).unwrap();

    assert_eq!(tree.get(&"alice".to_string()), Some("wonderland".to_string()));
    assert_eq!(tree.get(&"bob".to_string()), Some("builder".to_string()));
    assert_eq!(tree.get(&"carol".to_string()), None);

    tree.delete("alice".to_string()).unwrap();
    assert_eq!(tree.get(&"alice".to_string()), None);
}
