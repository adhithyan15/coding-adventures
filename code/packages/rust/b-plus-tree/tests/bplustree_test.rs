//! Integration tests for `coding-adventures-b-plus-tree`.
//!
//! These tests exercise the public API from the perspective of an external
//! caller, complementing the unit tests in `src/lib.rs`.

use coding_adventures_b_plus_tree::BPlusTree;

// ---------------------------------------------------------------------------
// Smoke test — basic CRUD with string keys
// ---------------------------------------------------------------------------

#[test]
fn crud_with_string_keys() {
    let mut t: BPlusTree<String, u64> = BPlusTree::new(2);
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
// Full scan sorted
// ---------------------------------------------------------------------------

#[test]
fn full_scan_sorted_large() {
    let mut t: BPlusTree<i32, i32> = BPlusTree::new(4);
    let mut input: Vec<i32> = (0..200).rev().collect();
    for &k in &input {
        t.insert(k, k * 3);
    }
    let scan: Vec<i32> = t.full_scan().iter().map(|(k, _)| **k).collect();
    input.sort();
    assert_eq!(scan, input);
    assert!(t.is_valid());
}

// ---------------------------------------------------------------------------
// range_scan correctness
// ---------------------------------------------------------------------------

#[test]
fn range_scan_boundaries() {
    let mut t: BPlusTree<i32, i32> = BPlusTree::new(3);
    for i in 0..=100 {
        t.insert(i, i);
    }
    let r = t.range_scan(&0, &0);
    assert_eq!(r.len(), 1);

    let r = t.range_scan(&100, &100);
    assert_eq!(r.len(), 1);

    let r = t.range_scan(&50, &60);
    let keys: Vec<i32> = r.iter().map(|(k, _)| **k).collect();
    assert_eq!(keys, (50..=60).collect::<Vec<_>>());
    assert!(t.is_valid());
}

// ---------------------------------------------------------------------------
// Leaf linked list — walk the chain and verify exact key set
// ---------------------------------------------------------------------------

#[test]
fn leaf_list_exact_keys_after_mixed_ops() {
    let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
    for i in 0..50 {
        t.insert(i, i);
    }
    // Delete every third key.
    for i in (0..50).step_by(3) {
        t.delete(&i);
        assert!(t.is_valid(), "invalid after deleting {i}");
    }

    // The full_scan uses the leaf linked list internally.
    let scan_keys: Vec<i32> = t.full_scan().iter().map(|(k, _)| **k).collect();
    let expected: Vec<i32> = (0..50).filter(|i| i % 3 != 0).collect();
    assert_eq!(scan_keys, expected);
}

// ---------------------------------------------------------------------------
// Sequential delete maintains validity
// ---------------------------------------------------------------------------

#[test]
fn sequential_delete_maintains_validity() {
    let n = 100;
    let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
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
// Iterator yields all entries in order
// ---------------------------------------------------------------------------

#[test]
fn iter_all_entries_in_order() {
    let mut t: BPlusTree<i32, i32> = BPlusTree::new(3);
    for i in [40, 10, 30, 20, 50] {
        t.insert(i, i * 2);
    }
    let collected: Vec<(i32, i32)> = t.iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(collected, vec![(10, 20), (20, 40), (30, 60), (40, 80), (50, 100)]);
}

// ---------------------------------------------------------------------------
// IntoIterator (consuming)
// ---------------------------------------------------------------------------

#[test]
fn into_iter_all_entries_in_order() {
    let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
    for i in 0..20 {
        t.insert(i, i);
    }
    let entries: Vec<(i32, i32)> = t.into_iter().collect();
    let keys: Vec<i32> = entries.iter().map(|(k, _)| *k).collect();
    assert_eq!(keys, (0..20).collect::<Vec<_>>());
}

// ---------------------------------------------------------------------------
// Large scale — 10 000 keys
// ---------------------------------------------------------------------------

#[test]
fn large_scale_10k() {
    let mut t: BPlusTree<i32, i32> = BPlusTree::new(3);
    for i in (0..10_000).rev() {
        t.insert(i, i * 2);
    }
    assert_eq!(t.len(), 10_000);
    assert!(t.is_valid());

    // Full scan must be sorted.
    let scan: Vec<i32> = t.full_scan().iter().map(|(k, _)| **k).collect();
    assert_eq!(scan, (0..10_000).collect::<Vec<_>>());

    // Range scan mid section.
    let r = t.range_scan(&4000, &5000);
    assert_eq!(r.len(), 1001);

    // Delete all even keys.
    for i in (0..10_000).step_by(2) {
        t.delete(&i);
    }
    assert_eq!(t.len(), 5_000);
    assert!(t.is_valid());

    let scan: Vec<i32> = t.full_scan().iter().map(|(k, _)| **k).collect();
    let expected: Vec<i32> = (0..10_000).filter(|i| i % 2 != 0).collect();
    assert_eq!(scan, expected);
}
