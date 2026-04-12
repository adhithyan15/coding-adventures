//! Integration tests for `coding-adventures-b-tree`.
//!
//! These tests exercise the public API from the perspective of an external
//! caller, complementing the unit tests in `src/lib.rs`.

use coding_adventures_b_tree::BTree;

// ---------------------------------------------------------------------------
// Smoke test — basic CRUD
// ---------------------------------------------------------------------------

#[test]
fn crud_with_string_keys() {
    let mut t: BTree<String, u64> = BTree::new(2);
    t.insert("banana".to_string(), 2);
    t.insert("apple".to_string(), 1);
    t.insert("cherry".to_string(), 3);

    assert_eq!(t.search(&"apple".to_string()), Some(&1));
    assert_eq!(t.search(&"banana".to_string()), Some(&2));
    assert_eq!(t.search(&"cherry".to_string()), Some(&3));
    assert_eq!(t.search(&"durian".to_string()), None);
    assert!(t.is_valid());
}

// ---------------------------------------------------------------------------
// Inorder traversal produces sorted output
// ---------------------------------------------------------------------------

#[test]
fn inorder_sorted_large() {
    let mut t: BTree<i32, i32> = BTree::new(4);
    let mut input: Vec<i32> = (0..200).rev().collect();
    for &k in &input {
        t.insert(k, k * 3);
    }
    let io: Vec<i32> = t.inorder().iter().map(|(k, _)| **k).collect();
    input.sort();
    assert_eq!(io, input);
    assert!(t.is_valid());
}

// ---------------------------------------------------------------------------
// range_query correctness
// ---------------------------------------------------------------------------

#[test]
fn range_query_boundaries() {
    let mut t: BTree<i32, i32> = BTree::new(3);
    for i in 0..=100 {
        t.insert(i, i);
    }
    // Inclusive on both ends.
    let r = t.range_query(&0, &0);
    assert_eq!(r.len(), 1);
    assert_eq!(*r[0].0, 0);

    let r = t.range_query(&100, &100);
    assert_eq!(r.len(), 1);
    assert_eq!(*r[0].0, 100);

    let r = t.range_query(&50, &60);
    let keys: Vec<i32> = r.iter().map(|(k, _)| **k).collect();
    assert_eq!(keys, (50..=60).collect::<Vec<_>>());
    assert!(t.is_valid());
}

// ---------------------------------------------------------------------------
// Delete every key one by one, checking validity each time
// ---------------------------------------------------------------------------

#[test]
fn sequential_delete_maintains_validity() {
    let n = 100;
    let mut t: BTree<i32, i32> = BTree::new(2);
    for i in 0..n {
        t.insert(i, i);
    }
    for i in 0..n {
        assert!(t.delete(&i), "key {i} should exist");
        assert!(t.is_valid(), "invalid after deleting {i}");
    }
    assert!(t.is_empty());
}

// ---------------------------------------------------------------------------
// Reverse-order delete
// ---------------------------------------------------------------------------

#[test]
fn reverse_delete_maintains_validity() {
    let n = 80;
    let mut t: BTree<i32, i32> = BTree::new(3);
    for i in 0..n {
        t.insert(i, i);
    }
    for i in (0..n).rev() {
        assert!(t.delete(&i));
        assert!(t.is_valid(), "invalid after deleting {i}");
    }
    assert!(t.is_empty());
}

// ---------------------------------------------------------------------------
// Large scale — 10 000 keys, random-ish (decreasing then increasing)
// ---------------------------------------------------------------------------

#[test]
fn large_scale_10k_t3() {
    let mut t: BTree<i32, i32> = BTree::new(3);
    for i in (0..5_000).rev() {
        t.insert(i, i);
    }
    for i in 5_000..10_000 {
        t.insert(i, i);
    }
    assert_eq!(t.len(), 10_000);
    assert!(t.is_valid());
    assert_eq!(t.min_key(), Some(&0));
    assert_eq!(t.max_key(), Some(&9_999));

    for i in 0..10_000 {
        assert_eq!(t.search(&i), Some(&i), "key {i} missing");
    }
}
