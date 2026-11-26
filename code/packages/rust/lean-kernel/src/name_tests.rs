//! Unit tests of `Name`'s cached hash and equality (VER32 PIF-002 security review). They
//! live in their own file, beside `name.rs`, because forcing a hash collision needs the
//! private constructor, and test code is not part of the trusted line budget.

use super::*;
use std::collections::hash_map::DefaultHasher;

fn h(n: &Name) -> u64 {
    let mut s = DefaultHasher::new();
    n.hash(&mut s);
    s.finish()
}

/// A name with a forced hash, to exercise the collision path.
fn forged(prefix: &Name, c: &str, hash: u64) -> Name {
    Name::node(Kind::Str(prefix.clone(), c.to_string()), hash)
}

#[test]
fn equal_names_have_equal_hashes_and_distinct_names_differ() {
    let a = Name::from_dotted("Nat.add");
    let b = Name::from_dotted("Nat.add");
    assert!(a == b && h(&a) == h(&b));
    assert_ne!(a, Name::from_dotted("Nat.sub"));
    let p = Name::from_dotted("p");
    assert_ne!(Name::str(&p, "3"), Name::num(&p, 3));
    assert_ne!(Name::anonymous(), p);
    assert_eq!(Name::anonymous(), Name::anonymous());
}

#[test]
fn a_hash_collision_does_not_make_different_names_equal() {
    let root = Name::anonymous();
    let (x, y) = (forged(&root, "x", 7), forged(&root, "y", 7));
    assert_eq!(h(&x), h(&y));
    assert_ne!(x, y);
    assert_eq!(x, forged(&root, "x", 7));
    assert_ne!(x.cmp(&y), Ordering::Equal);
    assert_eq!(x.cmp(&forged(&root, "x", 7)), Ordering::Equal);
    assert_eq!(x.cmp(&y), y.cmp(&x).reverse());
    let n = Name::node(Kind::Num(root.clone(), 1), 7);
    assert_ne!(x, n);
    assert_ne!(x, forged(&x, "z", 7));
}

#[test]
fn ordering_matches_the_derived_one() {
    let a = Name::from_dotted("a");
    let n1 = Name::num(&a, 1);
    assert!(Name::anonymous() < a && a < Name::from_dotted("a.b") && Name::from_dotted("a.b") < n1);
    assert!(Name::from_dotted("a.b") < Name::from_dotted("a.c"));
    // derived order compares the prefix first: anonymous < a, so b < a.z
    assert!(Name::from_dotted("b") < Name::from_dotted("a.z"));
}

#[test]
fn hashing_a_name_is_constant_time() {
    // 32 components of 64 KiB (about 2 MiB), hashed and compared 100,000 times
    let big = (0..32).fold(Name::anonymous(), |p, i| {
        Name::str(&p, &char::from(b'a' + i).to_string().repeat(65536))
    });
    let twin = big.clone();
    let t = std::time::Instant::now();
    let mut acc = 0u64;
    for _ in 0..100_000 {
        acc ^= h(&big);
        assert!(big == twin);
    }
    assert!(acc == 0 || acc != 1);
    assert!(
        t.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        t.elapsed()
    );
}
