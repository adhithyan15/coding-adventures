//! Tests for the places where a silent wrap, truncation or collision would produce a
//! wrong term: arity mismatches, fvar issuing, handle/store identity, level-size ceilings
//! and stack-safe drops.

use lean_kernel::{
    checked_u32, ArityMismatch, ExprStore, FVarIdTaken, Level, LevelOffsetTooLarge, Name,
    MAX_LEVEL_OFFSET,
};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

#[test]
fn level_instantiate_rejects_length_mismatch() {
    let u = Level::param(&n("u"));
    let err = Level::max(&u, &u)
        .instantiate(&[n("u"), n("v")], &[Level::zero()])
        .unwrap_err();
    assert_eq!(
        err,
        ArityMismatch {
            params: 2,
            levels: 1
        }
    );
    // even a parameter-free level is checked: the call itself is malformed
    assert!(Level::zero().instantiate(&[], &[Level::zero()]).is_err());
    assert!(Level::zero().instantiate(&[], &[]).is_ok());
}

#[test]
fn expr_level_instantiation_rejects_length_mismatch() {
    let mut st = ExprStore::new();
    let e = st.sort(&Level::param(&n("u")));
    let err = st.instantiate_level_params(e, &[n("u")], &[]).unwrap_err();
    assert_eq!(
        err,
        ArityMismatch {
            params: 1,
            levels: 0
        }
    );
    let c = st.constant(&n("c"), &[]);
    assert!(st
        .instantiate_level_params(c, &[], &[Level::zero()])
        .is_err());
    assert!(st
        .instantiate_level_params(e, &[n("u")], &[Level::zero()])
        .is_ok());
}

#[test]
fn issued_fvar_ids_never_collide() {
    let mut st = ExprStore::new();
    let a = st.fresh_fvar_id();
    assert_eq!(a.index(), 0);
    // an importer asks for number 10: allowed once, and fresh ids continue above it
    let ten = st.issue_fvar_id_at(10).unwrap();
    assert_eq!(ten.index(), 10);
    assert_eq!(st.issue_fvar_id_at(10), Err(FVarIdTaken(10)));
    assert_eq!(st.issue_fvar_id_at(5), Err(FVarIdTaken(5)));
    let b = st.fresh_fvar_id();
    assert_eq!(b.index(), 11);
    assert_ne!(st.fvar(a), st.fvar(ten));
    assert_ne!(st.fvar(ten), st.fvar(b));
}

#[test]
#[should_panic(expected = "not issued by this store")]
fn fvar_from_another_store_is_refused() {
    let mut s1 = ExprStore::new();
    let mut s2 = ExprStore::new();
    let id = s1.fresh_fvar_id();
    let _ = s2.fresh_fvar_id(); // s2 has issued a number 0 too: still not the same id
    let _ = s2.fvar(id);
}

#[test]
fn handles_from_different_stores_are_never_equal() {
    let mut s1 = ExprStore::new();
    let mut s2 = ExprStore::new();
    let (a, b) = (s1.bvar(0), s2.bvar(0));
    assert_ne!(a, b, "same index in two stores must not alias");
}

#[test]
fn checked_u32_conversion() {
    assert_eq!(checked_u32(0, "x"), 0);
    assert_eq!(checked_u32(u32::MAX as usize, "x"), u32::MAX);
}

#[cfg(target_pointer_width = "64")]
#[test]
#[should_panic(expected = "node limit does not fit in u32")]
fn checked_u32_refuses_to_truncate() {
    // the conversion ExprStore uses for its node index: 2^32 must not wrap to 0
    let _ = checked_u32(u32::MAX as usize + 1, "node limit");
}

#[test]
fn level_offset_ceiling() {
    assert!(Level::try_of_nat(MAX_LEVEL_OFFSET).is_ok());
    assert_eq!(
        Level::try_of_nat(MAX_LEVEL_OFFSET + 1),
        Err(LevelOffsetTooLarge(MAX_LEVEL_OFFSET + 1))
    );
    assert!(Level::try_offset(&Level::zero(), u32::MAX).is_err());
    let at = Level::of_nat(MAX_LEVEL_OFFSET);
    assert_eq!(at.to_offset().1, MAX_LEVEL_OFFSET);
}

#[test]
#[should_panic(expected = "MAX_LEVEL_OFFSET")]
fn plain_of_nat_panics_above_ceiling() {
    let _ = Level::of_nat(MAX_LEVEL_OFFSET + 1);
}

#[test]
fn deep_levels_and_names_drop_without_overflowing_the_stack() {
    let mut l = Level::zero();
    for _ in 0..100_000 {
        l = Level::succ(&l);
    }
    assert_eq!(l.to_offset().1, 100_000);
    // a deep tree of max nodes too
    let mut m = Level::param(&n("u"));
    for _ in 0..100_000 {
        m = Level::max(&m, &Level::zero());
    }
    drop(l);
    drop(m);

    let mut name = Name::anonymous();
    for i in 0..100_000u64 {
        name = if i % 2 == 0 {
            Name::num(&name, i)
        } else {
            Name::str(&name, "c")
        };
    }
    assert_eq!(name.parts().len(), 100_000);
    drop(name);
}

#[test]
fn shared_levels_survive_dropping_one_owner() {
    // the iterative drop must not tear apart nodes another owner still holds
    let base = Level::succ(&Level::param(&n("u")));
    let a = Level::succ(&base);
    let b = Level::max(&base, &a);
    drop(a);
    assert_eq!(base.to_offset().1, 1);
    assert!(b.is_equiv(&Level::succ(&base)));
}
