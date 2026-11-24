//! Tests at the bound-variable index ceiling: arithmetic that would overflow `u32` must
//! fail loudly, never wrap into a different term.

use lean_kernel::{BVarIndexTooLarge, ExprStore, Name, MAX_BVAR_INDEX};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

#[test]
fn bvar_ceiling_is_enforced() {
    let mut st = ExprStore::new();
    let top = st.try_bvar(MAX_BVAR_INDEX - 1).unwrap();
    assert_eq!(st.loose_bvar_range(top), MAX_BVAR_INDEX);
    assert_eq!(
        st.try_bvar(MAX_BVAR_INDEX),
        Err(BVarIndexTooLarge(MAX_BVAR_INDEX))
    );
    assert_eq!(st.try_bvar(u32::MAX), Err(BVarIndexTooLarge(u32::MAX)));
    // operations at the ceiling still work where the result stays inside it
    let c = st.constant(&n("c"), &[]);
    let below = st.try_bvar(MAX_BVAR_INDEX - 2).unwrap();
    assert_eq!(st.instantiate1(top, c), below);
    assert_eq!(st.lower_loose_bvars(top, 1, 1), below);
    assert_eq!(st.lift_loose_bvars(below, 0, 1), top);
    let app = st.app(top, c);
    assert_eq!(st.loose_bvar_range(app), MAX_BVAR_INDEX);
}

#[test]
#[should_panic(expected = "MAX_BVAR_INDEX")]
fn bvar_at_ceiling_panics() {
    let mut st = ExprStore::new();
    let _ = st.bvar(MAX_BVAR_INDEX);
}

#[test]
#[should_panic(expected = "MAX_BVAR_INDEX")]
fn lift_past_the_ceiling_panics_instead_of_wrapping() {
    let mut st = ExprStore::new();
    let top = st.bvar(MAX_BVAR_INDEX - 1);
    let _ = st.lift_loose_bvars(top, 0, 1);
}

#[test]
#[should_panic(expected = "overflowed u32")]
fn lift_by_huge_amount_panics_instead_of_wrapping() {
    let mut st = ExprStore::new();
    let b = st.bvar(5);
    let _ = st.lift_loose_bvars(b, 0, u32::MAX);
}
