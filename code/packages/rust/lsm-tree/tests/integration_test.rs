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

#[test]
fn test_crash_recovery() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut tree: LSMTree<String, String> = LSMTree::new(dir.path()).unwrap();
        tree.put("key1".to_string(), "val1".to_string()).unwrap();
        tree.put("key2".to_string(), "val2".to_string()).unwrap();
        tree.delete("key1".to_string()).unwrap();
        tree.put("key3".to_string(), "val3".to_string()).unwrap();
    } // Drop sim

    let tree: LSMTree<String, String> = LSMTree::new(dir.path()).unwrap();
    assert_eq!(tree.get(&"key1".to_string()), None); // Tombstone persisted
    assert_eq!(tree.get(&"key2".to_string()), Some("val2".to_string()));
    assert_eq!(tree.get(&"key3".to_string()), Some("val3".to_string()));
    assert_eq!(tree.get(&"missing".to_string()), None);
}

#[test]
fn test_update_existing_key() {
    let dir = tempfile::tempdir().unwrap();
    let mut tree: LSMTree<String, String> = LSMTree::new(dir.path()).unwrap();

    tree.put("k".to_string(), "v1".to_string()).unwrap();
    assert_eq!(tree.get(&"k".to_string()), Some("v1".to_string()));

    tree.put("k".to_string(), "v2".to_string()).unwrap();
    assert_eq!(tree.get(&"k".to_string()), Some("v2".to_string()));
}
