//! The count half of the observer's caps (see `tests/wxh_caps.rs`): bombs that
//! hold almost nothing still cannot abandon more than `OBS_MAX_ABANDONED`
//! worker threads; the next observed call is refused.

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxh_subject {
        include!("support/wxh_subject_fixed.rs");
    }
    include!("generated/wxh_evidence.rs");

    pub fn trips() -> (usize, bool) {
        let mut tripped = 0;
        loop {
            match obs_exec(|| {
                let v: Vec<u8> = Vec::with_capacity(OBSERVER_CAP_BYTES as usize + 1);
                V::Bool(v.is_empty())
            }) {
                Exec::Tripped(_) => tripped += 1,
                Exec::Refused => return (tripped, true),
                _ => return (tripped, false),
            }
            if tripped > OBS_MAX_ABANDONED + 8 {
                return (tripped, false);
            }
        }
    }

    pub const MAX_ABANDONED: usize = OBS_MAX_ABANDONED;
}

#[test]
fn at_most_the_configured_number_of_workers_is_ever_abandoned() {
    let (tripped, refused) = harness::trips();
    assert!(refused, "the count cap never fired");
    assert_eq!(tripped, harness::MAX_ABANDONED);
}
