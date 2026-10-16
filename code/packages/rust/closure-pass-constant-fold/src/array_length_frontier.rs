#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forced_collisions_preserve_first_occurrence_and_charge_every_step() {
        for parents in [
            vec![None, None],
            vec![Some("a"), Some("a"), None, Some("a")],
            vec![Some("z"), None, Some("a"), Some("z"), Some("b")],
        ] {
            let mut budget = Budget { remaining_work: usize::MAX, max_buffer_bytes: usize::MAX };
            let expected = collect_with(parents.iter().copied(), &mut budget, |_| 0, reserve_buffers).unwrap();
            assert_eq!(expected, if parents[0].is_none() { vec![] }
                else if parents[0] == Some("a") { vec!["a"] }
                else { vec!["z", "a", "b"] });
            let used = usize::MAX - budget.remaining_work;
            // Every smaller budget refuses, covering preflight visits/bytes,
            // initialization, hashing, collided probes and equality charges.
            for limit in 0..=used {
                let mut budget = Budget { remaining_work: limit, max_buffer_bytes: usize::MAX };
                let result = collect_with(parents.iter().copied(), &mut budget, |_| 0, reserve_buffers);
                if limit == used { assert_eq!(result.unwrap(), expected); }
                else { assert!(result.unwrap_err().contains("work limit")); }
            }
        }
    }

    #[test]
    fn storage_is_checked_before_allocation_and_both_buffers_count() {
        let (_, bytes) = buffer_layout(3, usize::MAX).unwrap();
        assert_eq!(bytes, 3 * std::mem::size_of::<&str>() + 8 * std::mem::size_of::<Option<&str>>());
        for cap in [bytes - 1, bytes] {
            let mut budget = Budget { remaining_work: 1000, max_buffer_bytes: cap };
            let result = collect_with([Some("a"), Some("a"), Some("b")].into_iter(), &mut budget, |_| 0, reserve_buffers);
            if cap == bytes { assert_eq!(result.unwrap(), ["a", "b"]); }
            else { assert!(result.unwrap_err().contains("buffer limit")); }
        }
        assert!(buffer_layout(usize::MAX, usize::MAX).unwrap_err().contains("overflow"));
        // Multiplication can fit while rounding or byte accounting overflows.
        assert!(buffer_layout(usize::MAX / 2, usize::MAX).is_err());
        assert!(buffer_layout(usize::MAX / 64, usize::MAX).is_err());
    }

    #[test]
    fn missing_identities_do_not_allocate_or_hash_and_allocation_failure_is_an_error() {
        let mut budget = Budget { remaining_work: 2, max_buffer_bytes: 0 };
        let out = collect_with([None, None].into_iter(), &mut budget,
            |_| panic!("no available IDs"), |_, _| panic!("no buffer needed")).unwrap();
        assert!(out.is_empty());
        let mut budget = Budget { remaining_work: 100, max_buffer_bytes: 1000 };
        let error = collect_with([Some("a")].into_iter(), &mut budget,
            |_| panic!("allocation precedes hashing"), |_, _| Err("CV array frontier allocation refused (injected)".into())).unwrap_err();
        assert!(error.contains("allocation refused"));
        let mut budget = Budget { remaining_work: 0, max_buffer_bytes: 1000 };
        assert!(collect_with([Some("a")].into_iter(), &mut budget,
            |_| panic!("preflight precedes hashing"), |_, _| panic!("preflight precedes allocation")).is_err());
    }

    #[test]
    fn long_ids_are_metered_and_random_placement_never_changes_order() {
        let long = "x".repeat(4096);
        let parents = [Some(long.as_str()), Some("b"), Some(long.as_str())];
        let mut budget = Budget { remaining_work: usize::MAX, max_buffer_bytes: usize::MAX };
        let expected = collect_with(parents.into_iter(), &mut budget, |_| 0, reserve_buffers).unwrap();
        let used = usize::MAX - budget.remaining_work;
        let mut short = Budget { remaining_work: used - 1, max_buffer_bytes: usize::MAX };
        assert!(collect_with(parents.into_iter(), &mut short, |_| 0, reserve_buffers).is_err());
        for _ in 0..16 { assert_eq!(collect(parents.into_iter()).unwrap(), expected); }
    }
}
